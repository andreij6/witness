use crate::ca::{self, NS_PER_S};
use crate::config::SUBMITS_PER_DAY;
use crate::outbox::{Body, Dest, Out};
use crate::proof::Roots;
use crate::quota::Daily;
use crate::record::RecordEntry;
use crate::store::{Stores, counter_key, pair};
use crate::types::{Allow, Shell};
use candid::Principal;
use serde_bytes::ByteBuf;
use witness_c2pa::{Error as C2paError, TrustAnchor, verify_seal};
use witness_canister_common::limits::MAX_SUBMIT_BATCH;
use witness_canister_common::{Anchored, Posting, Submission, SubmitErr, SubmitResult};
use witness_format::{FingerprintBundle, KeyStatus, Kind, Provisioning, WitnessClaim, cbor};

pub const MAX_PER_CAPTURE: usize = witness_canister_common::limits::MAX_BY_CAPTURE;

fn anchored(st: &Stores, ix: u64) -> Option<Anchored> {
    let e = st.record(ix)?;
    Some(Anchored {
        record_ix: ix,
        record_id: e.record_id().to_string(),
        anchored_ns: e.anchored_ns,
        media_shard: e.media_shard,
    })
}

fn seal_err(e: C2paError) -> SubmitErr {
    match e {
        C2paError::Signature
        | C2paError::Signer
        | C2paError::Chain(_)
        | C2paError::KeyMismatch
        | C2paError::SerialMismatch
        | C2paError::AssertionHash
        | C2paError::UnresolvedAssertion
        | C2paError::DuplicateAssertion => SubmitErr::BadSignature,
        other => SubmitErr::Malformed(other.to_string()),
    }
}

fn wm_root(claim: &WitnessClaim) -> u64 {
    match &claim.kind {
        Kind::Original => claim.capture_id.to_u64(),
        Kind::Derivative { root, .. } => root.to_u64(),
    }
}

struct Parent {
    ix: u64,
    root_ix: u64,
}

fn parent_of(st: &Stores, claim: &WitnessClaim) -> Result<Option<Parent>, SubmitErr> {
    let Kind::Derivative { parent, root, .. } = &claim.kind else {
        return Ok(None);
    };
    let ix = st.record_ix(parent).ok_or(SubmitErr::ParentUnknown)?;
    let entry = st.record(ix).ok_or(SubmitErr::ParentUnknown)?;
    let pc = WitnessClaim::from_cbor(&entry.claim).map_err(|_| SubmitErr::ParentUnknown)?;
    if pc.key.key_id != claim.key.key_id {
        return Err(SubmitErr::Malformed("parent signed by another key".into()));
    }
    if wm_root(&pc) != root.to_u64() {
        return Err(SubmitErr::Malformed("root capture mismatch".into()));
    }
    if !pc.kind.presentation_only() {
        return Err(SubmitErr::Malformed("parent has content edits".into()));
    }
    Ok(Some(Parent {
        ix,
        root_ix: entry.root_ix,
    }))
}

fn validate(
    st: &Stores,
    s: &Submission,
    anchor: &TrustAnchor,
    now: u64,
) -> Result<(WitnessClaim, Option<Parent>, Daily), SubmitErr> {
    let seal = verify_seal(
        &s.claim,
        &s.c2pa_claim,
        &s.cose,
        anchor,
        Some(now / NS_PER_S),
    )
    .map_err(seal_err)?;
    let claim = seal.claim;
    let key = st.key(&claim.key.key_id).ok_or(SubmitErr::UnknownKey)?;
    if key.pubkey != seal.signer_spki {
        return Err(SubmitErr::BadSignature);
    }
    if matches!(
        key.history.current(),
        KeyStatus::Suspended | KeyStatus::Revoked
    ) {
        return Err(SubmitErr::KeyDistrusted);
    }
    if claim.kind.is_original() && !key.accepts_original_at_ms(claim.time.wall_ms) {
        return Err(SubmitErr::KeyExpired);
    }
    let parent = parent_of(st, &claim)?;
    if st
        .counters
        .contains_key(&counter_key(&claim.key.key_id, claim.key.counter))
    {
        return Err(SubmitErr::CounterReused);
    }
    if let Some(fp) = &s.fingerprints {
        let bundle: FingerprintBundle = cbor::from_slice_canonical(fp)
            .map_err(|e| SubmitErr::Malformed(format!("fingerprints: {e:?}")))?;
        if bundle.fp_commit() != claim.fp_commit {
            return Err(SubmitErr::FingerprintMismatch);
        }
    }
    let prev = st
        .submit_quota
        .get(claim.key.key_id.as_bytes())
        .map(|b| Daily::from_bytes(&b));
    let daily = Daily::admit(prev, now, SUBMITS_PER_DAY).map_err(SubmitErr::RetryAfterNs)?;
    Ok((claim, parent, daily))
}

fn apply(
    st: &mut Stores,
    s: Submission,
    claim: &WitnessClaim,
    parent: Option<Parent>,
    daily: Daily,
    now: u64,
) -> Anchored {
    let shards = st.shards();
    let index_shard = shards.open_index().map_or(0, |i| i.shard);
    let media_shard = shards.media.last().map_or(0, |m| m.shard);
    let record_ix = st.mmr().leaf_count();
    let entry = RecordEntry {
        anchored_ns: now,
        root_ix: parent.as_ref().map_or(record_ix, |p| p.root_ix),
        index_shard,
        media_shard,
        claim: s.claim,
        c2pa_claim: s.c2pa_claim,
        cose: s.cose,
    };
    let appended = st.mmr().append(&entry.leaf_data()).expect("mmr append");
    let logged = st.records.append(&entry.encode()).expect("record log");
    assert!(
        appended == record_ix && logged == record_ix,
        "record log out of step"
    );
    let record_id = entry.record_id();
    st.by_id.insert(record_id.to_bytes(), record_ix);
    st.counters
        .insert(counter_key(&claim.key.key_id, claim.key.counter), record_ix);
    st.submit_quota
        .insert(claim.key.key_id.to_bytes(), daily.to_bytes());
    let capture = claim.capture_id.to_u64();
    if st
        .by_capture
        .range(pair(capture, 0)..=pair(capture, u64::MAX))
        .take(MAX_PER_CAPTURE)
        .count()
        < MAX_PER_CAPTURE
    {
        st.by_capture.insert(pair(capture, record_ix), ());
    }
    if let Some(p) = &parent {
        st.derivatives.insert(pair(p.ix, record_ix), ());
    }
    st.enqueue(&Out {
        dest: Dest::Index(index_shard),
        body: Body::Put(Posting {
            record_ix,
            wm_root: wm_root(claim),
            asset_hash: claim.asset_hash.to_vec(),
            pixel_hash: claim.pixel_hash.to_vec(),
            fingerprints: s.fingerprints,
            root_ix: parent.map(|_| entry.root_ix),
        }),
    });
    if !claim.kind.is_original() {
        let dev = st
            .key(&claim.key.key_id)
            .is_some_and(|k| k.provisioning == Provisioning::Dev);
        let media = shards
            .media_shard(media_shard)
            .map_or(Principal::anonymous(), |m| m.canister);
        st.enqueue(&Out {
            dest: Dest::Pages,
            body: Body::Shell(Shell {
                record_id: record_id.to_string(),
                record_ix,
                anchored_ns: now,
                claim: entry.claim,
                media_shard,
                media,
                dev,
            }),
        });
        st.enqueue(&Out {
            dest: Dest::Media(media_shard),
            body: Body::Sync {
                record_ix: Some(record_ix),
                allow: Allow::Record {
                    record_id: record_id.to_string(),
                    key_id: ByteBuf::from(claim.key.key_id.as_bytes().to_vec()),
                },
            },
        });
    }
    Anchored {
        record_ix,
        record_id: record_id.to_string(),
        anchored_ns: now,
        media_shard,
    }
}

pub fn submit_one(st: &mut Stores, s: Submission, anchor: &TrustAnchor, now: u64) -> SubmitResult {
    if !s.within_limits() {
        return SubmitResult::Err(SubmitErr::TooLarge);
    }
    if let Some(a) = st.record_ix(&s.record_id()).and_then(|ix| anchored(st, ix)) {
        return SubmitResult::Dup(a);
    }
    match validate(st, &s, anchor, now) {
        Ok((claim, parent, daily)) => SubmitResult::Ok(apply(st, s, &claim, parent, daily, now)),
        Err(e) => SubmitResult::Err(e),
    }
}

pub fn submit(
    st: &mut Stores,
    batch: Vec<Submission>,
    now: u64,
) -> Result<(Vec<SubmitResult>, Roots), String> {
    if batch.len() > MAX_SUBMIT_BATCH {
        return Err(format!("batch larger than {MAX_SUBMIT_BATCH}"));
    }
    let anchor = ca::anchor(st.state.get().ca_root().ok_or("not ready: witness ca")?)?;
    let results = batch
        .into_iter()
        .map(|s| submit_one(st, s, &anchor, now))
        .collect();
    Ok((results, st.roots()))
}

#[cfg(test)]
pub(crate) mod tests;
