use super::{DEPTH, combine, empty};
use crate::error::Error;
use crate::hash::{Hash, smt_leaf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmtProof {
    pub bitmap: u128,
    pub siblings: Vec<Hash>,
}

impl SmtProof {
    pub const HEADER_LEN: usize = 16;

    pub fn verify(&self, root: &Hash, key: u128, value: Option<&Hash>) -> bool {
        let leaf = value.map_or(*empty(0), |v| smt_leaf(key, v));
        self.compute_root(key, &leaf).is_some_and(|r| r == *root)
    }

    pub fn compute_root(&self, key: u128, leaf: &Hash) -> Option<Hash> {
        if self.siblings.len() != self.bitmap.count_ones() as usize {
            return None;
        }
        let mut siblings = self.siblings.iter();
        let mut cur = *leaf;
        for height in 0..DEPTH {
            let sibling = if (self.bitmap >> height) & 1 == 1 {
                siblings.next().filter(|s| *s != empty(height))?
            } else {
                empty(height)
            };
            cur = combine(height, (key >> height) & 1 == 1, &cur, sibling);
        }
        Some(cur)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::HEADER_LEN + 32 * self.siblings.len());
        out.extend_from_slice(&self.bitmap.to_be_bytes());
        self.siblings.iter().for_each(|s| out.extend_from_slice(s));
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let (header, body) = bytes.split_first_chunk::<16>().ok_or(Error::Malformed)?;
        let bitmap = u128::from_be_bytes(*header);
        let (siblings, rest) = body.as_chunks::<32>();
        if !rest.is_empty() || siblings.len() != bitmap.count_ones() as usize {
            return Err(Error::Malformed);
        }
        Ok(Self {
            bitmap,
            siblings: siblings.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smt::Smt;
    use crate::smt::tests::{MapSmt, naive_root};
    use crate::testutil::{Counting, Rng};
    use std::collections::BTreeMap;

    fn populated(seed: u64, n: usize) -> (MapSmt, BTreeMap<u128, Hash>) {
        let mut rng = Rng::new(seed);
        let mut smt = MapSmt::default();
        let mut model = BTreeMap::new();
        for _ in 0..n {
            let (k, v) = (rng.u128(), rng.hash());
            smt.insert(k, &v);
            model.insert(k, v);
        }
        (smt, model)
    }

    #[test]
    fn membership_proofs_verify() {
        let (smt, model) = populated(1, 64);
        let root = smt.root();
        assert_eq!(root, naive_root(&model));
        for (k, v) in &model {
            let p = smt.prove(*k);
            assert!(p.verify(&root, *k, Some(v)));
            assert!(!p.verify(&root, *k, None));
            assert!(!p.verify(&root, *k, Some(&[0; 32])));
            assert!(!p.verify(&root, k ^ 1, Some(v)));
        }
    }

    #[test]
    fn non_membership_proofs_verify() {
        let (smt, model) = populated(2, 64);
        let root = smt.root();
        let mut rng = Rng::new(99);
        for _ in 0..100 {
            let k = rng.u128();
            if model.contains_key(&k) {
                continue;
            }
            let p = smt.prove(k);
            assert!(p.verify(&root, k, None));
            assert!(!p.verify(&root, k, Some(&rng.hash())));
        }
        for k in model.keys() {
            let neighbour = k ^ 1;
            if !model.contains_key(&neighbour) {
                assert!(smt.prove(neighbour).verify(&root, neighbour, None));
            }
        }
    }

    #[test]
    fn empty_tree_proof_is_empty_and_proves_absence() {
        let smt = MapSmt::default();
        for k in [0u128, 1, u128::MAX] {
            let p = smt.prove(k);
            assert_eq!(p, SmtProof::default());
            assert!(p.verify(&smt.root(), k, None));
            assert!(!p.verify(&smt.root(), k, Some(&[1; 32])));
        }
    }

    #[test]
    fn adjacent_keys_have_leaf_level_sibling() {
        let mut smt = MapSmt::default();
        smt.insert(10, &[1; 32]);
        smt.insert(11, &[2; 32]);
        let p = smt.prove(10);
        assert_eq!(p.bitmap, 1);
        assert_eq!(p.siblings.len(), 1);
        assert!(p.verify(&smt.root(), 10, Some(&[1; 32])));
        let p = smt.prove(0);
        assert_eq!(p.bitmap, 1 << 3);
        assert!(p.verify(&smt.root(), 0, None));
    }

    #[test]
    fn extreme_keys() {
        let mut smt = MapSmt::default();
        smt.insert(0, &[1; 32]);
        smt.insert(u128::MAX, &[2; 32]);
        let root = smt.root();
        let p0 = smt.prove(0);
        let pm = smt.prove(u128::MAX);
        assert_eq!(p0.bitmap, 1 << 127);
        assert_eq!(pm.bitmap, 1 << 127);
        assert!(p0.verify(&root, 0, Some(&[1; 32])));
        assert!(pm.verify(&root, u128::MAX, Some(&[2; 32])));
        assert!(!p0.verify(&root, u128::MAX, Some(&[2; 32])));
    }

    #[test]
    fn tampering_fails() {
        let (smt, model) = populated(3, 40);
        let root = smt.root();
        for (k, v) in model.iter().take(15) {
            let p = smt.prove(*k);
            for i in 0..p.siblings.len() {
                let mut t = p.clone();
                t.siblings[i][0] ^= 1;
                assert!(!t.verify(&root, *k, Some(v)));
            }
            for bit in [0u32, 1, 63, 64, 126, 127] {
                let mut t = p.clone();
                t.bitmap ^= 1 << bit;
                assert!(!t.verify(&root, *k, Some(v)));
            }
            let mut t = p.clone();
            t.siblings.push([3; 32]);
            assert!(!t.verify(&root, *k, Some(v)));
            let mut bad = root;
            bad[5] ^= 2;
            assert!(!p.verify(&bad, *k, Some(v)));
        }
    }

    #[test]
    fn non_canonical_default_sibling_rejected() {
        let smt = MapSmt::default();
        let p = SmtProof {
            bitmap: 1,
            siblings: vec![*empty(0)],
        };
        assert_eq!(p.compute_root(0, empty(0)), None);
        assert!(!p.verify(&smt.root(), 0, None));
        let p = SmtProof {
            bitmap: 1 << 127,
            siblings: vec![*empty(127)],
        };
        assert!(!p.verify(&smt.root(), 0, None));
    }

    #[test]
    fn stale_proof_fails_after_update() {
        let (mut smt, model) = populated(4, 10);
        let (k, v) = model
            .iter()
            .next()
            .map(|(k, v)| (*k, *v))
            .unwrap_or_default();
        let p = smt.prove(k);
        smt.insert(k, &[0xee; 32]);
        assert!(!p.verify(&smt.root(), k, Some(&v)));
        assert!(smt.prove(k).verify(&smt.root(), k, Some(&[0xee; 32])));
        smt.remove(k);
        assert!(smt.prove(k).verify(&smt.root(), k, None));
    }

    #[test]
    fn bytes_roundtrip() {
        let (smt, model) = populated(5, 30);
        for k in model.keys().take(10) {
            let p = smt.prove(*k);
            let b = p.to_bytes();
            assert_eq!(b.len(), 16 + 32 * p.siblings.len());
            assert_eq!(SmtProof::from_bytes(&b).unwrap(), p);
        }
        let e = SmtProof::default();
        assert_eq!(SmtProof::from_bytes(&e.to_bytes()).unwrap(), e);
    }

    #[test]
    fn malformed_bytes_rejected() {
        assert_eq!(SmtProof::from_bytes(&[]), Err(Error::Malformed));
        assert_eq!(SmtProof::from_bytes(&[0; 15]), Err(Error::Malformed));
        assert_eq!(SmtProof::from_bytes(&[0; 17]), Err(Error::Malformed));
        assert_eq!(SmtProof::from_bytes(&[0; 48]), Err(Error::Malformed));
        let mut one = vec![0u8; 16];
        one[15] = 1;
        assert_eq!(SmtProof::from_bytes(&one), Err(Error::Malformed));
        one.extend_from_slice(&[7; 32]);
        assert!(SmtProof::from_bytes(&one).is_ok());
        one.push(0);
        assert_eq!(SmtProof::from_bytes(&one), Err(Error::Malformed));
        assert_eq!(SmtProof::from_bytes(&[0xff; 16]), Err(Error::Malformed));
    }

    #[test]
    fn random_garbage_never_verifies_or_panics() {
        let (smt, _) = populated(6, 8);
        let root = smt.root();
        let mut rng = Rng::new(7);
        for _ in 0..500 {
            let len = 16 + 32 * rng.below(6) as usize + rng.below(3) as usize;
            let mut bytes = vec![0u8; len];
            bytes.iter_mut().for_each(|b| *b = rng.next() as u8);
            if let Ok(p) = SmtProof::from_bytes(&bytes) {
                assert!(!p.verify(&root, rng.u128(), Some(&rng.hash())));
            }
        }
    }

    #[test]
    fn prove_reads_exactly_depth_nodes() {
        let mut smt = Smt::new(Counting::new(BTreeMap::new()));
        let mut rng = Rng::new(8);
        for _ in 0..50 {
            smt.insert(rng.u128(), &rng.hash());
        }
        let root = smt.root();
        smt.store.reset();
        let k = rng.u128();
        let p = smt.prove(k);
        assert_eq!(smt.store().reads.get(), u64::from(DEPTH));
        assert_eq!(smt.store().writes, 0);
        assert!(p.verify(&root, k, None));
        assert!(p.siblings.len() <= 12);
    }
}
