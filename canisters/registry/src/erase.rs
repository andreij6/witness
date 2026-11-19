use crate::outbox::{Body, Dest, Out};
use crate::store::{Stores, pair};
use witness_canister_common::{EraseAuth, EraseRequest};
use witness_format::{Kind, WitnessClaim};

pub fn erase_mine(st: &mut Stores, req: &EraseRequest, now: u64) -> Result<(), String> {
    let id = req.record_id().map_err(|e| e.to_string())?;
    let ix = st.record_ix(&id).ok_or("unknown record")?;
    let entry = st.record(ix).ok_or("unknown record")?;
    let claim = WitnessClaim::from_cbor(&entry.claim).map_err(|_| "corrupt record")?;
    match req.auth {
        EraseAuth::Device(_) => {
            let key = st.key(&claim.key.key_id).ok_or("unknown key")?;
            req.verify_device(&key.pubkey).map_err(|e| e.to_string())?;
        }
        EraseAuth::Ii => return Err("internet identity not linked".into()),
    }
    if st.tombstones.contains_key(&ix) {
        return Ok(());
    }
    st.tombstones.insert(ix, now);
    st.enqueue(&Out {
        dest: Dest::Index(entry.index_shard),
        body: Body::EraseIx(ix),
    });
    if let Kind::Derivative { parent, .. } = &claim.kind {
        if let Some(p) = st.record_ix(parent) {
            st.derivatives.remove(&pair(p, ix));
        }
        let text = id.to_string();
        st.enqueue(&Out {
            dest: Dest::Pages,
            body: Body::Tombstone(text.clone()),
        });
        st.enqueue(&Out {
            dest: Dest::Media(entry.media_shard),
            body: Body::EraseId(text),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enroll::tests::{NOW, ready};
    use crate::outbox::{Budget, Call, plan};
    use crate::store::with;
    use crate::submit::submit;
    use crate::submit::tests::{Dev, device, sealed};
    use p256::ecdsa::signature::Signer as _;
    use witness_canister_common::registry::erase_signing_bytes;
    use witness_canister_common::{Submission, SubmitResult};
    use witness_format::{CaptureId, RecordId};

    fn sign(dev: &Dev, id: &RecordId, nonce: &[u8]) -> Vec<u8> {
        let msg = erase_signing_bytes(id, nonce).unwrap();
        let sig: p256::ecdsa::Signature = dev.device.key.sign(&msg);
        sig.to_bytes().to_vec()
    }

    fn req(dev: &Dev, s: &Submission) -> EraseRequest {
        let id = s.record_id();
        EraseRequest {
            record_id: id.to_string(),
            nonce: vec![1, 2, 3],
            auth: EraseAuth::Device(sign(dev, &id, &[1, 2, 3])),
        }
    }

    fn anchor(st: &mut Stores, s: Submission) {
        let (r, _) = submit(st, vec![s], NOW).unwrap();
        assert!(matches!(r[0], SubmitResult::Ok(_)), "{r:?}");
    }

    fn setup(st: &mut Stores) -> (Dev, Submission, Submission) {
        ready(st);
        let dev = device(st, 1);
        let o = sealed(&dev, dev.device.claim(1, Kind::Original, 1), 1);
        let kind = Kind::Derivative {
            parent: o.record_id(),
            root: CaptureId::new(1, 0).unwrap(),
            edits: vec![],
        };
        let d = sealed(&dev, dev.device.claim(2, kind, 2), 2);
        anchor(st, o.clone());
        anchor(st, d.clone());
        (dev, o, d)
    }

    #[test]
    fn device_signed_erase_tombstones_and_fans_out() {
        with(|st| {
            let (dev, _, d) = setup(st);
            let roots = st.roots();
            erase_mine(st, &req(&dev, &d), NOW + 5).unwrap();
            assert_eq!(st.tombstones.get(&1), Some(NOW + 5));
            assert!(!st.derivatives.contains_key(&pair(0, 1)));
            assert_eq!(st.roots(), roots);
            assert!(st.record(1).is_some());
            let p = plan(st, 0, Budget::unbounded(100, 64));
            let calls: Vec<_> = p.batches.iter().map(|b| (b.dest, b.call.clone())).collect();
            assert!(
                p.dropped.len() == 3,
                "posting, shell and media allow dropped: {:?}",
                p.dropped
            );
            assert!(calls.contains(&(
                Dest::Pages,
                Call::Tombstones(vec![d.record_id().to_string()])
            )));
            assert!(calls.contains(&(Dest::Media(0), Call::EraseId(d.record_id().to_string()))));
            let n = st.outbox.len();
            erase_mine(st, &req(&dev, &d), NOW + 6).unwrap();
            assert_eq!(st.outbox.len(), n);
            assert_eq!(st.tombstones.get(&1), Some(NOW + 5));
        });
    }

    #[test]
    fn original_erase_only_touches_index() {
        with(|st| {
            let (dev, o, _) = setup(st);
            let before = st.outbox.len();
            erase_mine(st, &req(&dev, &o), NOW).unwrap();
            assert_eq!(st.outbox.len(), before + 1);
            let last = st
                .outbox
                .keys_range(..Dest::Pages.lane())
                .next_back()
                .and_then(|k| st.outbox.get(&k))
                .map(|v| Out::decode(&v).unwrap())
                .unwrap();
            assert_eq!(
                last,
                Out {
                    dest: Dest::Index(0),
                    body: Body::EraseIx(0)
                }
            );
        });
    }

    #[test]
    fn rejects_bad_auth_and_unknown_records() {
        with(|st| {
            let (dev, o, d) = setup(st);
            let other = device(st, 2);
            let mut r = req(&dev, &o);
            r.nonce = vec![9];
            assert_eq!(erase_mine(st, &r, NOW).unwrap_err(), "bad signature");
            assert_eq!(
                erase_mine(st, &req(&other, &o), NOW).unwrap_err(),
                "bad signature"
            );
            let mut wrong = req(&dev, &d);
            wrong.record_id = o.record_id().to_string();
            assert_eq!(erase_mine(st, &wrong, NOW).unwrap_err(), "bad signature");
            let ii = EraseRequest {
                auth: EraseAuth::Ii,
                ..req(&dev, &o)
            };
            assert!(erase_mine(st, &ii, NOW).unwrap_err().contains("not linked"));
            let bad_id = EraseRequest {
                record_id: "nope".into(),
                ..req(&dev, &o)
            };
            assert_eq!(erase_mine(st, &bad_id, NOW).unwrap_err(), "bad id");
            let unknown = sealed(&dev, dev.device.claim(3, Kind::Original, 3), 3);
            assert_eq!(
                erase_mine(st, &req(&dev, &unknown), NOW).unwrap_err(),
                "unknown record"
            );
            let empty_nonce = EraseRequest {
                nonce: vec![],
                ..req(&dev, &o)
            };
            assert!(erase_mine(st, &empty_nonce, NOW).is_err());
            assert!(st.tombstones.is_empty());
        });
    }
}
