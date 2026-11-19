use crate::record::leaf_data;
use serde_bytes::ByteBuf;
use witness_canister_common::{CertifiedKey, CertifiedRecord, Shards};
use witness_format::crypto::hash_ds;
use witness_merkle::hash::{mmr_leaf, smt_leaf};
use witness_merkle::{Hash, InclusionProof, SmtProof, certified_data, key_of};

pub const SHARDS_KEY: u128 = u128::MAX;
pub const SHARDS_DOMAIN: &[u8] = b"witness/shards/v1";
pub const ROOTS_LEN: usize = 32 + 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roots {
    pub mmr_root: Hash,
    pub leaf_count: u64,
    pub smt_root: Hash,
}

impl Roots {
    pub fn certified(&self) -> Hash {
        certified_data(&self.mmr_root, self.leaf_count, &self.smt_root)
    }

    pub fn mmr_suffix(&self) -> [u8; ROOTS_LEN] {
        let mut out = [0u8; ROOTS_LEN];
        out[..32].copy_from_slice(&self.mmr_root);
        out[32..].copy_from_slice(&self.leaf_count.to_be_bytes());
        out
    }
}

pub fn shard_map_hash(s: &Shards) -> Hash {
    let mut buf = Vec::with_capacity(4 + 32 * (s.index.len() + s.media.len()));
    for list in [&s.index, &s.media] {
        buf.extend_from_slice(&(list.len() as u16).to_be_bytes());
        for i in list.iter() {
            let p = i.canister.as_slice();
            buf.push(i.shard);
            buf.push(u8::from(i.open));
            buf.push(p.len() as u8);
            buf.extend_from_slice(p);
        }
    }
    hash_ds(SHARDS_DOMAIN, &[&buf])
}

pub fn record_path(path: &[Hash], smt_root: &Hash) -> Vec<ByteBuf> {
    path.iter()
        .chain(core::iter::once(smt_root))
        .map(|h| ByteBuf::from(h.to_vec()))
        .collect()
}

pub fn record_certified(r: &CertifiedRecord) -> Option<Hash> {
    let (smt_root, path) = r.mmr_proof.split_last()?;
    let smt_root: Hash = smt_root.as_slice().try_into().ok()?;
    let path = path
        .iter()
        .map(|h| h.as_slice().try_into().ok())
        .collect::<Option<Vec<Hash>>>()?;
    let proof = InclusionProof {
        index: r.record_ix,
        leaf_count: r.leaf_count,
        path,
    };
    let leaf = mmr_leaf(&leaf_data(&r.claim, &r.c2pa_claim, &r.cose, r.anchored_ns));
    let mmr_root = proof.compute_root(&leaf)?;
    Some(certified_data(&mmr_root, r.leaf_count, &smt_root))
}

pub fn smt_bundle(proof: &SmtProof, roots: &Roots) -> Vec<u8> {
    let mut out = proof.to_bytes();
    out.extend_from_slice(&roots.mmr_suffix());
    out
}

fn smt_bundle_certified(bundle: &[u8], key: u128, value: &Hash) -> Option<Hash> {
    let (proof, tail) = bundle.split_at_checked(bundle.len().checked_sub(ROOTS_LEN)?)?;
    let mmr_root: Hash = tail[..32].try_into().ok()?;
    let leaf_count = u64::from_be_bytes(tail[32..].try_into().ok()?);
    let proof = SmtProof::from_bytes(proof).ok()?;
    let smt_root = proof.compute_root(key, &smt_leaf(key, value))?;
    Some(certified_data(&mmr_root, leaf_count, &smt_root))
}

pub fn key_certified(k: &CertifiedKey) -> Option<Hash> {
    let info = witness_format::KeyInfo::try_from(k.key.clone()).ok()?;
    smt_bundle_certified(&k.smt_proof, key_of(&info.key_id), &info.leaf_hash())
}

pub fn shards_bundle(certificate: &[u8], proof: &SmtProof, roots: &Roots) -> Vec<u8> {
    let mut out =
        Vec::with_capacity(4 + certificate.len() + ROOTS_LEN + 16 + 32 * proof.siblings.len());
    out.extend_from_slice(&(certificate.len() as u32).to_be_bytes());
    out.extend_from_slice(certificate);
    out.extend_from_slice(&smt_bundle(proof, roots));
    out
}

pub fn shards_certified<'a>(shards: &Shards, bundle: &'a [u8]) -> Option<(&'a [u8], Hash)> {
    let (len, rest) = bundle.split_first_chunk::<4>()?;
    let (cert, smt) = rest.split_at_checked(usize::try_from(u32::from_be_bytes(*len)).ok()?)?;
    let h = smt_bundle_certified(smt, SHARDS_KEY, &shard_map_hash(shards))?;
    Some((cert, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use std::collections::BTreeMap;
    use witness_canister_common::{Fresh, KeyInfo, Prov, Security, ShardInfo, StatusChange};
    use witness_merkle::{Mmr, Smt};

    fn shards() -> Shards {
        Shards {
            index: vec![ShardInfo {
                shard: 0,
                canister: Principal::from_slice(&[1; 10]),
                open: true,
            }],
            media: vec![ShardInfo {
                shard: 0,
                canister: Principal::from_slice(&[2; 29]),
                open: true,
            }],
        }
    }

    fn record(ix: u64) -> CertifiedRecord {
        CertifiedRecord {
            record_ix: ix,
            record_id: String::new(),
            claim: vec![ix as u8, 1],
            c2pa_claim: vec![2],
            cose: vec![3],
            anchored_ns: ix.wrapping_add(100),
            mmr_proof: vec![],
            leaf_count: 0,
            certificate: vec![],
            media_shard: 0,
        }
    }

    fn build(n: u64) -> (Mmr<Vec<Hash>>, Vec<CertifiedRecord>) {
        let mut mmr = Mmr::new(Vec::new()).unwrap();
        let recs: Vec<_> = (0..n).map(record).collect();
        for r in &recs {
            mmr.append(&leaf_data(&r.claim, &r.c2pa_claim, &r.cose, r.anchored_ns))
                .unwrap();
        }
        (mmr, recs)
    }

    #[test]
    fn record_proofs_verify_for_every_leaf() {
        let smt_root = [9u8; 32];
        for n in 1..=12 {
            let (mmr, recs) = build(n);
            let roots = Roots {
                mmr_root: mmr.root().unwrap(),
                leaf_count: n,
                smt_root,
            };
            for mut r in recs {
                r.leaf_count = n;
                r.mmr_proof = record_path(&mmr.prove(r.record_ix).unwrap().path, &smt_root);
                assert_eq!(record_certified(&r), Some(roots.certified()));
            }
        }
    }

    #[test]
    fn record_proof_rejects_tampering() {
        let (mmr, recs) = build(5);
        let smt_root = [9u8; 32];
        let roots = Roots {
            mmr_root: mmr.root().unwrap(),
            leaf_count: 5,
            smt_root,
        };
        let mut r = recs[2].clone();
        r.leaf_count = 5;
        r.mmr_proof = record_path(&mmr.prove(2).unwrap().path, &smt_root);
        let good = roots.certified();
        let tampered = [
            CertifiedRecord {
                anchored_ns: 1,
                ..r.clone()
            },
            CertifiedRecord {
                claim: vec![0],
                ..r.clone()
            },
            CertifiedRecord {
                cose: vec![0],
                ..r.clone()
            },
            CertifiedRecord {
                c2pa_claim: vec![0],
                ..r.clone()
            },
            CertifiedRecord {
                record_ix: 3,
                ..r.clone()
            },
            CertifiedRecord {
                leaf_count: 6,
                ..r.clone()
            },
        ];
        for t in tampered {
            assert_ne!(record_certified(&t), Some(good));
        }
        let mut bad_smt = r.clone();
        *bad_smt.mmr_proof.last_mut().unwrap() = ByteBuf::from(vec![8; 32]);
        assert_ne!(record_certified(&bad_smt), Some(good));
        let mut short = r.clone();
        short.mmr_proof[0] = ByteBuf::from(vec![1; 31]);
        assert_eq!(record_certified(&short), None);
        let empty = CertifiedRecord {
            mmr_proof: vec![],
            ..r
        };
        assert_eq!(record_certified(&empty), None);
    }

    #[test]
    fn record_proof_survives_hostile_sizes() {
        let mut r = record(u64::MAX);
        r.leaf_count = u64::MAX;
        r.mmr_proof = vec![ByteBuf::from(vec![0; 32]); 70];
        let _ = record_certified(&r);
    }

    fn key() -> KeyInfo {
        let pubkey = vec![4; 91];
        KeyInfo {
            key_id: witness_format::KeyId::from_spki(&pubkey)
                .as_bytes()
                .to_vec(),
            pubkey,
            security: Security::Tee,
            provisioning: Prov::Rkp,
            freshness: Fresh::Fresh,
            patch: 1,
            enrolled_ns: 1,
            active_until_ns: 2,
            history: vec![StatusChange {
                status: witness_canister_common::KeyStatus::Active,
                since_ns: 1,
            }],
            chain_serials: vec![],
        }
    }

    #[test]
    fn key_bundle_verifies() {
        let k = key();
        let info = witness_format::KeyInfo::try_from(k.clone()).unwrap();
        let mut smt = Smt::new(BTreeMap::new());
        smt.insert(key_of(&info.key_id), &info.leaf_hash());
        smt.insert(SHARDS_KEY, &[1; 32]);
        let roots = Roots {
            mmr_root: [3; 32],
            leaf_count: 7,
            smt_root: smt.root(),
        };
        let ck = CertifiedKey {
            key: k.clone(),
            smt_proof: smt_bundle(&smt.prove(key_of(&info.key_id)), &roots),
            certificate: vec![],
        };
        assert_eq!(key_certified(&ck), Some(roots.certified()));
        let mut changed = ck.clone();
        changed.key.patch = 2;
        assert_ne!(key_certified(&changed), Some(roots.certified()));
        let mut cut = ck.clone();
        cut.smt_proof.pop();
        assert_ne!(key_certified(&cut), Some(roots.certified()));
        let mut short = ck.clone();
        short.smt_proof.truncate(10);
        assert_eq!(key_certified(&short), None);
        let mut bad = ck;
        bad.key.key_id = vec![1; 3];
        assert_eq!(key_certified(&bad), None);
    }

    #[test]
    fn shards_bundle_verifies() {
        let s = shards();
        let mut smt = Smt::new(BTreeMap::new());
        smt.insert(SHARDS_KEY, &shard_map_hash(&s));
        let roots = Roots {
            mmr_root: [3; 32],
            leaf_count: 0,
            smt_root: smt.root(),
        };
        let b = shards_bundle(b"cert", &smt.prove(SHARDS_KEY), &roots);
        assert_eq!(
            shards_certified(&s, &b),
            Some((&b"cert"[..], roots.certified()))
        );
        let mut other = s.clone();
        other.index[0].open = false;
        assert_ne!(
            shards_certified(&other, &b).map(|x| x.1),
            Some(roots.certified())
        );
        assert_eq!(shards_certified(&s, &b[..3]), None);
        let mut long = b.clone();
        long[..4].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(shards_certified(&s, &long), None);
    }

    #[test]
    fn shard_hash_is_deterministic_and_sensitive() {
        let s = shards();
        assert_eq!(shard_map_hash(&s), shard_map_hash(&s.clone()));
        let swapped = Shards {
            index: s.media.clone(),
            media: s.index.clone(),
        };
        assert_ne!(shard_map_hash(&s), shard_map_hash(&swapped));
        let empty_split = Shards {
            index: vec![],
            media: [s.index.clone(), s.media.clone()].concat(),
        };
        assert_ne!(shard_map_hash(&s), shard_map_hash(&empty_split));
        assert_ne!(shard_map_hash(&Shards::default()), shard_map_hash(&s));
    }

    #[test]
    fn roots_suffix_layout() {
        let r = Roots {
            mmr_root: [1; 32],
            leaf_count: 258,
            smt_root: [2; 32],
        };
        let s = r.mmr_suffix();
        assert_eq!(&s[..32], &[1; 32]);
        assert_eq!(&s[32..], &258u64.to_be_bytes());
        assert_eq!(r.certified(), certified_data(&[1; 32], 258, &[2; 32]));
    }
}
