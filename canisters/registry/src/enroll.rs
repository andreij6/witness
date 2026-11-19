use crate::ca;
use crate::config::PENDING_TTL_NS;
use crate::outbox::{Body, Dest, Out};
use crate::proof::Roots;
use crate::quota::EnrollWindow;
use crate::store::Stores;
use crate::types::Allow;
use crate::types::Enrollment;
use serde_bytes::ByteBuf;
use witness_attest::{Context, LateOpening, Policy, RevocationList};
use witness_format::{KeyId, KeyInfo, Provisioning};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    pub info: KeyInfo,
    pub quota_key: [u8; 32],
    pub window_start: u64,
    pub tbs: Vec<u8>,
}

pub fn begin(
    st: &mut Stores,
    req: &Enrollment,
    policy: &Policy<'_>,
    now: u64,
) -> Result<Ticket, String> {
    let state = st.state.get();
    let secret = state.secret().ok_or("not ready: challenge secret")?;
    let anchor = ca::anchor(state.ca_root().ok_or("not ready: witness ca")?)?;
    let key_id = req
        .key_id
        .as_ref()
        .map(|k| KeyId::try_from(k.as_slice()).map_err(|_| "bad key_id"))
        .transpose()?;
    let revocations = RevocationList::empty();
    let enrollment = witness_attest::Enrollment {
        chain: &req.chain,
        key_id,
        late: req.late.as_ref().map(|l| LateOpening {
            install_nonce: &l.install_nonce,
            wall_ms: l.wall_ms,
        }),
    };
    let ctx = Context {
        now_ns: now,
        challenge_secret: &secret,
        revocations: &revocations,
    };
    let attested = witness_attest::verify(&enrollment, policy, &ctx)
        .map_err(|e| format!("attestation: {e:?}"))?;
    let id = attested.key_id.to_bytes();
    if st.keys.contains_key(&id) {
        return Err("already enrolled".into());
    }
    if st
        .pending
        .get(&id)
        .is_some_and(|since| now.saturating_sub(since) < PENDING_TTL_NS)
    {
        return Err("enrollment pending".into());
    }
    let backoff = attested.provisioning != Provisioning::Rkp;
    let prev = st
        .enroll_quota
        .get(&attested.quota_key)
        .map(|b| EnrollWindow::from_bytes(&b));
    let window = EnrollWindow::admit(prev, now, backoff)
        .map_err(|wait| format!("quota: retry after {wait} ns"))?;
    let serial = st.update_state(crate::state::State::take_serial);
    let tbs = ca::leaf_to_sign(serial, now, &attested.pubkey, &anchor)?;
    st.enroll_quota
        .insert(attested.quota_key, window.to_bytes());
    st.pending.insert(id, now);
    Ok(Ticket {
        info: attested.key_info(now, policy.active_ns),
        quota_key: attested.quota_key,
        window_start: window.start_ns,
        tbs,
    })
}

pub fn commit(st: &mut Stores, t: &Ticket) -> Roots {
    st.pending.remove(&t.info.key_id.to_bytes());
    st.put_key(&t.info);
    let shards = st.config.get().media.len();
    for shard in 0..u8::try_from(shards).unwrap_or(u8::MAX) {
        st.enqueue(&Out {
            dest: Dest::Media(shard),
            body: Body::Sync {
                record_ix: None,
                allow: Allow::Key {
                    key_id: ByteBuf::from(t.info.key_id.as_bytes().to_vec()),
                    pubkey: ByteBuf::from(t.info.pubkey.clone()),
                },
            },
        });
    }
    st.roots()
}

pub fn rollback(st: &mut Stores, t: &Ticket) {
    st.pending.remove(&t.info.key_id.to_bytes());
    if let Some(b) = st.enroll_quota.get(&t.quota_key) {
        let w = EnrollWindow::from_bytes(&b).release(t.window_start);
        st.enroll_quota.insert(t.quota_key, w.to_bytes());
    }
}

#[cfg(test)]
pub(crate) mod tests;
