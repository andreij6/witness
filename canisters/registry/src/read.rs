use crate::proof::{SHARDS_KEY, record_path, shards_bundle, smt_bundle};
use crate::store::{Stores, pair};
use serde_bytes::ByteBuf;
use witness_canister_common::limits::MAX_BY_CAPTURE;
use witness_canister_common::{CertifiedKey, CertifiedRecord, KeyInfo, Shards};
use witness_format::{KeyId, RecordId};
use witness_merkle::key_of;

pub fn record(st: &mut Stores, ix: u64, certificate: Vec<u8>) -> Option<CertifiedRecord> {
    let e = st.record(ix)?;
    let roots = st.roots();
    let proof = st.mmr().prove(ix).ok()?;
    Some(CertifiedRecord {
        record_ix: ix,
        record_id: e.record_id().to_string(),
        mmr_proof: record_path(&proof.path, &roots.smt_root),
        leaf_count: roots.leaf_count,
        claim: e.claim,
        c2pa_claim: e.c2pa_claim,
        cose: e.cose,
        anchored_ns: e.anchored_ns,
        certificate,
        media_shard: e.media_shard,
    })
}

pub fn record_by_id(st: &mut Stores, id: &str, certificate: Vec<u8>) -> Option<CertifiedRecord> {
    let ix = st.record_ix(&RecordId::parse(id).ok()?)?;
    record(st, ix, certificate)
}

pub fn key(st: &mut Stores, key_id: &[u8], certificate: Vec<u8>) -> Option<CertifiedKey> {
    let id = KeyId::try_from(key_id).ok()?;
    let info = st.key(&id)?;
    let roots = st.roots();
    let proof = st.smt().prove(key_of(&id));
    Some(CertifiedKey {
        key: KeyInfo::from(&info),
        smt_proof: smt_bundle(&proof, &roots),
        certificate,
    })
}

pub fn by_capture(st: &Stores, capture: u64) -> Vec<String> {
    st.by_capture
        .keys_range(pair(capture, 0)..=pair(capture, u64::MAX))
        .take(MAX_BY_CAPTURE)
        .filter_map(|k| {
            let ix = u64::from_be_bytes(k[8..].try_into().ok()?);
            Some(st.record(ix)?.record_id().to_string())
        })
        .collect()
}

pub fn shards(st: &mut Stores, certificate: &[u8]) -> (Shards, ByteBuf) {
    let roots = st.roots();
    let proof = st.smt().prove(SHARDS_KEY);
    (
        st.shards(),
        ByteBuf::from(shards_bundle(certificate, &proof, &roots)),
    )
}

pub fn consistency(st: &Stores, old_size: u64) -> Result<Vec<ByteBuf>, String> {
    let mmr = st.mmr();
    let p = mmr
        .prove_consistency(old_size, mmr.leaf_count())
        .map_err(|e| format!("{e:?}"))?;
    Ok(p.path.iter().map(|h| ByteBuf::from(h.to_vec())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enroll::tests::{NOW, ready};
    use crate::proof::{key_certified, record_certified, shards_certified};
    use crate::store::with;
    use crate::submit::submit;
    use crate::submit::tests::{device, sealed};
    use witness_canister_common::{RegistryInit, SubmitResult};
    use witness_format::Kind;
    use witness_merkle::{ConsistencyProof, Hash};

    fn anchor_n(st: &mut Stores, n: u64) -> Vec<String> {
        ready(st);
        let dev = device(st, 1);
        (0..n)
            .map(|i| {
                let s = sealed(
                    &dev,
                    dev.device.claim(100 + i % 2, Kind::Original, i + 1),
                    i,
                );
                let (r, _) = submit(st, vec![s], NOW).unwrap();
                match &r[0] {
                    SubmitResult::Ok(a) => a.record_id.clone(),
                    other => panic!("{other:?}"),
                }
            })
            .collect()
    }

    #[test]
    fn records_carry_verifiable_proofs() {
        with(|st| {
            let ids = anchor_n(st, 5);
            let certified = st.roots().certified();
            for (ix, id) in ids.iter().enumerate() {
                let r = record(st, ix as u64, b"cert".to_vec()).unwrap();
                assert_eq!(&r.record_id, id);
                assert_eq!(r.certificate, b"cert");
                assert_eq!(record_certified(&r), Some(certified));
                assert_eq!(
                    record_by_id(st, id, vec![]).map(|x| x.record_ix),
                    Some(ix as u64)
                );
            }
            assert_eq!(record(st, 5, vec![]), None);
            assert_eq!(record_by_id(st, "not-an-id", vec![]), None);
            assert_eq!(record_by_id(st, "0000000000000000", vec![]), None);
        });
    }

    #[test]
    fn keys_carry_verifiable_proofs() {
        with(|st| {
            anchor_n(st, 1);
            let id = st.keys.keys().next().unwrap();
            let k = key(st, &id, vec![1]).unwrap();
            assert_eq!(key_certified(&k), Some(st.roots().certified()));
            assert_eq!(k.key.key_id, id.to_vec());
            assert_eq!(key(st, &[0; 16], vec![]), None);
            assert_eq!(key(st, &[0; 3], vec![]), None);
        });
    }

    #[test]
    fn by_capture_lists_earliest_first() {
        with(|st| {
            let ids = anchor_n(st, 5);
            assert_eq!(
                by_capture(st, 100),
                vec![ids[0].clone(), ids[2].clone(), ids[4].clone()]
            );
            assert_eq!(by_capture(st, 101), vec![ids[1].clone(), ids[3].clone()]);
            assert!(by_capture(st, 7).is_empty());
        });
    }

    #[test]
    fn shards_are_certified() {
        with(|st| {
            let p = |n: u8| candid::Principal::from_slice(&[n; 10]);
            st.configure(RegistryInit {
                index: vec![p(1)],
                media: vec![p(2)],
                pages: p(3),
            });
            let (s, b) = shards(st, b"cc");
            assert_eq!(s.index[0].canister, p(1));
            assert_eq!(
                shards_certified(&s, &b),
                Some((&b"cc"[..], st.roots().certified()))
            );
        });
    }

    #[test]
    fn consistency_proofs_link_roots() {
        with(|st| {
            anchor_n(st, 3);
            let old_root = st.mmr().root_at(1).unwrap();
            let new_root = st.mmr().root().unwrap();
            let path: Vec<Hash> = consistency(st, 1)
                .unwrap()
                .iter()
                .map(|b| b.as_slice().try_into().unwrap())
                .collect();
            let p = ConsistencyProof {
                old_size: 1,
                new_size: 3,
                path,
            };
            assert!(p.verify(&old_root, &new_root));
            assert!(consistency(st, 4).is_err());
            assert!(consistency(st, 3).is_ok());
            assert!(consistency(st, 0).is_ok());
        });
    }
}
