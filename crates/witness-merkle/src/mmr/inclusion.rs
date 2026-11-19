use super::pos::{Mountain, Mountains};
use super::{Mmr, MmrStore};
use crate::error::Error;
use crate::hash::{Hash, mmr_bag, mmr_leaf, mmr_node, mmr_root};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InclusionProof {
    pub index: u64,
    pub leaf_count: u64,
    pub path: Vec<Hash>,
}

struct Layout {
    target: Mountain,
    left: usize,
    right: bool,
}

impl Layout {
    fn of(index: u64, leaf_count: u64) -> Option<Self> {
        if index >= leaf_count {
            return None;
        }
        let mut left = 0;
        let mut ms = Mountains::new(leaf_count);
        let target = ms
            .by_ref()
            .inspect(|_| left += 1)
            .find(|m| m.contains(index))?;
        Some(Self {
            target,
            left: left - 1,
            right: ms.len() > 0,
        })
    }

    const fn len(&self) -> usize {
        self.target.height as usize + self.left + self.right as usize
    }
}

impl<S: MmrStore> Mmr<S> {
    pub fn prove(&self, index: u64) -> Result<InclusionProof, Error> {
        self.prove_at(index, self.leaves)
    }

    pub fn prove_at(&self, index: u64, leaf_count: u64) -> Result<InclusionProof, Error> {
        self.check_size(leaf_count)?;
        let layout = Layout::of(index, leaf_count).ok_or(Error::OutOfRange)?;
        let Mountain { start, height } = layout.target;
        let local = index - start;
        let mut path = Vec::with_capacity(layout.len());
        for k in 0..height {
            let sibling = start + (((local >> k) ^ 1) << k);
            path.push(self.node(Mountain {
                start: sibling,
                height: k,
            })?);
        }
        let all = Mountains::new(leaf_count);
        let right = all.clone().skip(layout.left + 1);
        if let Some(bag) = self.bag(right)? {
            path.push(bag);
        }
        for m in all.take(layout.left).rev() {
            path.push(self.node(m)?);
        }
        debug_assert_eq!(path.len(), layout.len());
        Ok(InclusionProof {
            index,
            leaf_count,
            path,
        })
    }
}

impl InclusionProof {
    pub fn verify(&self, root: &Hash, data: &[u8]) -> bool {
        self.verify_leaf(root, &mmr_leaf(data))
    }

    pub fn verify_leaf(&self, root: &Hash, leaf: &Hash) -> bool {
        self.compute_root(leaf).is_some_and(|r| r == *root)
    }

    pub fn compute_root(&self, leaf: &Hash) -> Option<Hash> {
        let layout = Layout::of(self.index, self.leaf_count)?;
        if self.path.len() != layout.len() {
            return None;
        }
        let height = layout.target.height as usize;
        let local = self.index - layout.target.start;
        let (siblings, rest) = self.path.split_at(height);
        let peak = siblings.iter().enumerate().fold(*leaf, |acc, (k, s)| {
            if (local >> k) & 1 == 1 {
                mmr_node(s, &acc)
            } else {
                mmr_node(&acc, s)
            }
        });
        let (right, left) = rest.split_at(usize::from(layout.right));
        let acc = right.first().map_or(peak, |r| mmr_bag(&peak, r));
        let bag = left.iter().fold(acc, |acc, l| mmr_bag(l, &acc));
        Some(mmr_root(self.leaf_count, &bag))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmr::tests::{build, data};
    use crate::testutil::{Counting, Rng};

    #[test]
    fn every_leaf_verifies_for_sizes_up_to_64() {
        for n in 1..=64u64 {
            let mmr = build(n);
            let root = mmr.root().unwrap();
            for i in 0..n {
                let p = mmr.prove(i).unwrap();
                assert!(p.verify(&root, &data(i)), "n={n} i={i}");
                assert!(!p.verify(&root, &data(i + 1)));
            }
        }
    }

    #[test]
    fn proof_length_is_height_plus_peaks() {
        for n in 1..=200u64 {
            let mmr = build(n);
            for i in 0..n {
                let p = mmr.prove(i).unwrap();
                let l = Layout::of(i, n).unwrap();
                assert_eq!(p.path.len(), l.len());
                assert!(p.path.len() <= 2 * 64);
                assert!(
                    p.path.len() <= (64 - n.leading_zeros()) as usize + n.count_ones() as usize
                );
            }
        }
    }

    #[test]
    fn historical_proofs_verify_against_historical_roots() {
        let mmr = build(150);
        let mut rng = Rng::new(11);
        for _ in 0..400 {
            let n = 1 + rng.below(150);
            let i = rng.below(n);
            let p = mmr.prove_at(i, n).unwrap();
            assert!(p.verify(&mmr.root_at(n).unwrap(), &data(i)));
            if n < 150 {
                assert!(!p.verify(&mmr.root().unwrap(), &data(i)));
            }
        }
    }

    #[test]
    fn out_of_range_requests_fail() {
        let mmr = build(5);
        assert_eq!(mmr.prove(5).unwrap_err(), Error::OutOfRange);
        assert_eq!(mmr.prove(u64::MAX).unwrap_err(), Error::OutOfRange);
        assert_eq!(mmr.prove_at(0, 6).unwrap_err(), Error::OutOfRange);
        assert_eq!(mmr.prove_at(3, 3).unwrap_err(), Error::OutOfRange);
        assert_eq!(build(0).prove(0).unwrap_err(), Error::OutOfRange);
    }

    #[test]
    fn tampering_any_field_fails() {
        let mmr = build(37);
        let root = mmr.root().unwrap();
        for i in 0..37 {
            let p = mmr.prove(i).unwrap();
            for k in 0..p.path.len() {
                let mut t = p.clone();
                t.path[k][k % 32] ^= 1;
                assert!(!t.verify(&root, &data(i)));
            }
            let mut t = p.clone();
            t.index ^= 1;
            assert!(!t.verify(&root, &data(i)));
            let mut t = p.clone();
            t.leaf_count += 1;
            assert!(!t.verify(&root, &data(i)));
            let mut t = p.clone();
            t.path.push([0; 32]);
            assert!(!t.verify(&root, &data(i)));
            if !p.path.is_empty() {
                let mut t = p.clone();
                t.path.pop();
                assert!(!t.verify(&root, &data(i)));
                let mut t = p.clone();
                t.path.reverse();
                if t.path != p.path {
                    assert!(!t.verify(&root, &data(i)));
                }
            }
            let mut bad_root = root;
            bad_root[0] ^= 0x80;
            assert!(!p.verify(&bad_root, &data(i)));
        }
    }

    #[test]
    fn adversarial_headers_do_not_panic() {
        let cases = [
            (0u64, 0u64),
            (u64::MAX, u64::MAX),
            (u64::MAX - 1, u64::MAX),
            (0, u64::MAX),
            (1 << 63, 1 << 63),
            ((1 << 63) - 1, 1 << 63),
        ];
        for (index, leaf_count) in cases {
            for len in [0usize, 1, 63, 64, 127, 128, 200] {
                let p = InclusionProof {
                    index,
                    leaf_count,
                    path: vec![[7; 32]; len],
                };
                assert!(!p.verify(&[0; 32], b"x"));
            }
        }
    }

    #[test]
    fn max_size_layout_length_accepted_but_root_mismatch() {
        let p = InclusionProof {
            index: u64::MAX - 1,
            leaf_count: u64::MAX,
            path: vec![[1; 32]; 63],
        };
        assert!(p.compute_root(&[0; 32]).is_some());
        let p = InclusionProof {
            index: 0,
            leaf_count: u64::MAX,
            path: vec![[1; 32]; 64],
        };
        assert!(p.compute_root(&[0; 32]).is_some());
    }

    #[test]
    fn leaf_hash_variant_matches_data_variant() {
        let mmr = build(9);
        let root = mmr.root().unwrap();
        let p = mmr.prove(4).unwrap();
        assert!(p.verify_leaf(&root, &mmr_leaf(&data(4))));
        assert!(!p.verify_leaf(&root, &data_hash_unprefixed(4)));
    }

    fn data_hash_unprefixed(i: u64) -> Hash {
        witness_format::crypto::sha256(&data(i))
    }

    #[test]
    fn prove_reads_are_logarithmic() {
        let mut mmr = Mmr::new(Counting::new(Vec::new())).unwrap();
        let n = 5000u64;
        for i in 0..n {
            mmr.append(&data(i)).unwrap();
        }
        let bound = 64 - n.leading_zeros() as u64 + 2 * u64::from(n.count_ones());
        let mut rng = Rng::new(5);
        for _ in 0..300 {
            let i = rng.below(n);
            mmr.store.reset();
            let p = mmr.prove(i).unwrap();
            assert!(mmr.store().reads.get() <= bound);
            assert!(p.verify(&mmr.root().unwrap(), &data(i)));
        }
    }

    #[test]
    fn proofs_are_deterministic() {
        let a = build(99);
        let b = build(99);
        for i in [0, 1, 50, 97, 98] {
            assert_eq!(a.prove(i).unwrap(), b.prove(i).unwrap());
        }
    }
}
