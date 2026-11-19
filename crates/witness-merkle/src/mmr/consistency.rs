use super::pos::{Mountain, Mountains, is_peak_of};
use super::{Mmr, MmrStore};
use crate::error::Error;
use crate::hash::{Hash, ZERO, bag_step, mmr_node, mmr_root};
use core::slice::Iter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyProof {
    pub old_size: u64,
    pub new_size: u64,
    pub path: Vec<Hash>,
}

impl<S: MmrStore> Mmr<S> {
    pub fn prove_consistency(
        &self,
        old_size: u64,
        new_size: u64,
    ) -> Result<ConsistencyProof, Error> {
        self.check_size(new_size)?;
        if old_size > new_size {
            return Err(Error::OutOfRange);
        }
        let mut path = Vec::with_capacity(old_size.count_ones() as usize + 2 * 64);
        for m in Mountains::new(old_size) {
            path.push(self.node(m)?);
        }
        let mut news = Mountains::new(new_size).peekable();
        while let Some(m) = news.next_if(|m| m.start < old_size) {
            self.collect(old_size, m, &mut path)?;
        }
        if let Some(bag) = self.bag(news)? {
            path.push(bag);
        }
        Ok(ConsistencyProof {
            old_size,
            new_size,
            path,
        })
    }

    fn collect(&self, old_size: u64, m: Mountain, path: &mut Vec<Hash>) -> Result<(), Error> {
        if is_peak_of(old_size, m.start, m.height) {
            return Ok(());
        }
        if m.start >= old_size {
            path.push(self.node(m)?);
            return Ok(());
        }
        let [l, r] = halves(m).ok_or(Error::CorruptStore)?;
        self.collect(old_size, l, path)?;
        self.collect(old_size, r, path)
    }
}

fn halves(m: Mountain) -> Option<[Mountain; 2]> {
    let height = m.height.checked_sub(1)?;
    Some([
        Mountain {
            start: m.start,
            height,
        },
        Mountain {
            start: m.start + (1 << height),
            height,
        },
    ])
}

struct Rebuild<'a> {
    old_size: u64,
    olds: Iter<'a, Hash>,
    news: Iter<'a, Hash>,
}

impl Rebuild<'_> {
    fn node(&mut self, m: Mountain) -> Option<Hash> {
        if is_peak_of(self.old_size, m.start, m.height) {
            return self.olds.next().copied();
        }
        if m.start >= self.old_size {
            return self.news.next().copied();
        }
        let [l, r] = halves(m)?;
        let left = self.node(l)?;
        let right = self.node(r)?;
        Some(mmr_node(&left, &right))
    }
}

impl ConsistencyProof {
    pub fn verify(&self, old_root: &Hash, new_root: &Hash) -> bool {
        self.roots()
            .is_some_and(|(o, n)| o == *old_root && n == *new_root)
    }

    pub fn roots(&self) -> Option<(Hash, Hash)> {
        if self.old_size > self.new_size {
            return None;
        }
        let olds = self
            .path
            .as_slice()
            .get(..self.old_size.count_ones() as usize)?;
        let news = &self.path[olds.len()..];
        let old_bag = olds
            .iter()
            .rev()
            .fold(None, |acc, p| Some(bag_step(acc, p)));
        let old_root = mmr_root(self.old_size, &old_bag.unwrap_or(ZERO));
        let mut rb = Rebuild {
            old_size: self.old_size,
            olds: olds.iter(),
            news: news.iter(),
        };
        let mut peaks = [ZERO; 64];
        let mut count = 0;
        let mut has_right = false;
        for m in Mountains::new(self.new_size) {
            if m.start >= self.old_size {
                has_right = true;
                break;
            }
            peaks[count] = rb.node(m)?;
            count += 1;
        }
        let right = if has_right {
            Some(*rb.news.next()?)
        } else {
            None
        };
        if rb.olds.next().is_some() || rb.news.next().is_some() {
            return None;
        }
        let new_bag = peaks[..count]
            .iter()
            .rev()
            .fold(right, |acc, p| Some(bag_step(acc, p)));
        Some((old_root, mmr_root(self.new_size, &new_bag.unwrap_or(ZERO))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmr::tests::build;
    use crate::testutil::{Counting, Rng};

    #[test]
    fn all_pairs_up_to_64_verify() {
        let mmr = build(64);
        let roots: Vec<Hash> = (0..=64).map(|n| mmr.root_at(n).unwrap()).collect();
        for new in 0..=64u64 {
            for old in 0..=new {
                let p = mmr.prove_consistency(old, new).unwrap();
                assert_eq!(
                    p.roots(),
                    Some((roots[old as usize], roots[new as usize])),
                    "{old}->{new}"
                );
                assert!(p.verify(&roots[old as usize], &roots[new as usize]));
                if old != new {
                    assert!(!p.verify(&roots[new as usize], &roots[old as usize]));
                }
            }
        }
    }

    #[test]
    fn equal_sizes_proof_is_just_old_peaks() {
        let mmr = build(13);
        let p = mmr.prove_consistency(13, 13).unwrap();
        assert_eq!(p.path, mmr.peaks().unwrap());
        let r = mmr.root().unwrap();
        assert!(p.verify(&r, &r));
    }

    #[test]
    fn from_empty_proof_is_single_bag() {
        let mmr = build(21);
        let p = mmr.prove_consistency(0, 21).unwrap();
        assert_eq!(p.path.len(), 1);
        assert!(p.verify(&mmr.root_at(0).unwrap(), &mmr.root().unwrap()));
        let e = build(0).prove_consistency(0, 0).unwrap();
        assert!(e.path.is_empty());
        assert!(e.verify(&mmr.root_at(0).unwrap(), &mmr.root_at(0).unwrap()));
    }

    #[test]
    fn invalid_requests_rejected() {
        let mmr = build(10);
        assert_eq!(mmr.prove_consistency(5, 11).unwrap_err(), Error::OutOfRange);
        assert_eq!(mmr.prove_consistency(6, 5).unwrap_err(), Error::OutOfRange);
    }

    #[test]
    fn tampering_fails() {
        let mmr = build(100);
        let mut rng = Rng::new(17);
        for _ in 0..200 {
            let new = rng.below(101);
            let old = rng.below(new + 1);
            let p = mmr.prove_consistency(old, new).unwrap();
            let (ro, rn) = (mmr.root_at(old).unwrap(), mmr.root_at(new).unwrap());
            assert!(p.verify(&ro, &rn));
            for k in 0..p.path.len() {
                let mut t = p.clone();
                t.path[k][31] ^= 4;
                assert!(!t.verify(&ro, &rn));
            }
            let mut t = p.clone();
            t.path.push(ZERO);
            assert!(!t.verify(&ro, &rn));
            if !p.path.is_empty() {
                let mut t = p.clone();
                t.path.pop();
                assert!(!t.verify(&ro, &rn));
            }
            let mut t = p.clone();
            t.new_size += 1;
            assert!(!t.verify(&ro, &rn));
            if old > 0 {
                let mut t = p.clone();
                t.old_size -= 1;
                assert!(!t.verify(&ro, &rn));
            }
        }
    }

    #[test]
    fn forked_history_detected() {
        let honest = build(40);
        let mut forked = build(20);
        for i in 20..40u64 {
            forked.append(&(i + 1000).to_le_bytes()).unwrap();
        }
        let p = forked.prove_consistency(30, 40).unwrap();
        assert!(!p.verify(&honest.root_at(30).unwrap(), &forked.root().unwrap()));
        assert!(p.verify(&forked.root_at(30).unwrap(), &forked.root().unwrap()));
        let p = forked.prove_consistency(20, 40).unwrap();
        assert!(p.verify(&honest.root_at(20).unwrap(), &forked.root().unwrap()));
    }

    #[test]
    fn adversarial_sizes_do_not_panic_and_fail() {
        let sizes = [
            0u64,
            1,
            2,
            3,
            1 << 62,
            (1 << 63) - 1,
            1 << 63,
            u64::MAX - 1,
            u64::MAX,
        ];
        for &old in &sizes {
            for &new in &sizes {
                for len in [0usize, 1, 2, 64, 65, 129, 300] {
                    let p = ConsistencyProof {
                        old_size: old,
                        new_size: new,
                        path: vec![[9; 32]; len],
                    };
                    assert!(!p.verify(&ZERO, &ZERO));
                }
            }
        }
    }

    #[test]
    fn proof_size_and_reads_are_logarithmic() {
        let mut mmr = Mmr::new(Counting::new(Vec::new())).unwrap();
        let n = 3000u64;
        for i in 0..n {
            mmr.append(&i.to_le_bytes()).unwrap();
        }
        let mut rng = Rng::new(23);
        for _ in 0..200 {
            let new = 1 + rng.below(n);
            let old = rng.below(new + 1);
            mmr.store.reset();
            let p = mmr.prove_consistency(old, new).unwrap();
            let bits = u64::from(64 - new.leading_zeros());
            let bound = u64::from(old.count_ones()) + 2 * bits + u64::from(new.count_ones());
            assert!(mmr.store().reads.get() <= bound, "{old}->{new}");
            assert!(p.path.len() as u64 <= bound);
            assert!(p.verify(&mmr.root_at(old).unwrap(), &mmr.root_at(new).unwrap()));
        }
    }

    #[test]
    fn halves_of_leaf_is_none() {
        assert!(
            halves(Mountain {
                start: 3,
                height: 0
            })
            .is_none()
        );
        let [l, r] = halves(Mountain {
            start: 8,
            height: 3,
        })
        .unwrap();
        assert_eq!((l.start, l.height, r.start, r.height), (8, 2, 12, 2));
    }
}
