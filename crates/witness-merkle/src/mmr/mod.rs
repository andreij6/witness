mod consistency;
mod inclusion;
pub(crate) mod pos;

use crate::error::Error;
use crate::hash::{Hash, ZERO, bag_step, mmr_leaf, mmr_node, mmr_root};
use pos::{Mountain, Mountains, leaves_for_size};

pub use consistency::ConsistencyProof;
pub use inclusion::InclusionProof;

pub const MAX_LEAVES: u64 = 1 << 62;

pub trait MmrStore {
    fn size(&self) -> u64;
    fn get(&self, pos: u64) -> Option<Hash>;
    fn push(&mut self, hash: Hash);
}

impl MmrStore for Vec<Hash> {
    fn size(&self) -> u64 {
        self.len() as u64
    }

    fn get(&self, pos: u64) -> Option<Hash> {
        usize::try_from(pos)
            .ok()
            .and_then(|p| self.as_slice().get(p))
            .copied()
    }

    fn push(&mut self, hash: Hash) {
        Vec::push(self, hash);
    }
}

#[derive(Debug, Clone)]
pub struct Mmr<S> {
    store: S,
    leaves: u64,
}

impl<S: MmrStore> Mmr<S> {
    pub fn new(store: S) -> Result<Self, Error> {
        let leaves = leaves_for_size(store.size()).ok_or(Error::CorruptStore)?;
        Ok(Self { store, leaves })
    }

    pub const fn leaf_count(&self) -> u64 {
        self.leaves
    }

    pub const fn store(&self) -> &S {
        &self.store
    }

    pub fn into_store(self) -> S {
        self.store
    }

    pub fn append(&mut self, data: &[u8]) -> Result<u64, Error> {
        self.append_leaf(mmr_leaf(data))
    }

    pub fn append_leaf(&mut self, leaf: Hash) -> Result<u64, Error> {
        let index = self.leaves;
        if index >= MAX_LEAVES {
            return Err(Error::Full);
        }
        let merges = index.trailing_ones() as usize;
        let mut chain = [ZERO; 64];
        chain[0] = leaf;
        for (k, m) in Mountains::new(index).rev().take(merges).enumerate() {
            chain[k + 1] = mmr_node(&self.node(m)?, &chain[k]);
        }
        chain[..=merges].iter().for_each(|h| self.store.push(*h));
        self.leaves += 1;
        Ok(index)
    }

    pub fn peaks(&self) -> Result<Vec<Hash>, Error> {
        Mountains::new(self.leaves).map(|m| self.node(m)).collect()
    }

    pub fn root(&self) -> Result<Hash, Error> {
        self.root_at(self.leaves)
    }

    pub fn root_at(&self, leaf_count: u64) -> Result<Hash, Error> {
        self.check_size(leaf_count)?;
        let bag = self.bag(Mountains::new(leaf_count))?;
        Ok(mmr_root(leaf_count, &bag.unwrap_or(ZERO)))
    }

    fn bag(
        &self,
        mountains: impl DoubleEndedIterator<Item = Mountain>,
    ) -> Result<Option<Hash>, Error> {
        mountains
            .rev()
            .try_fold(None, |acc, m| Ok(Some(bag_step(acc, &self.node(m)?))))
    }

    fn node(&self, m: Mountain) -> Result<Hash, Error> {
        let pos = m.pos();
        self.store.get(pos).ok_or(Error::MissingNode(pos))
    }

    fn check_size(&self, leaf_count: u64) -> Result<(), Error> {
        if leaf_count <= self.leaves {
            Ok(())
        } else {
            Err(Error::OutOfRange)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::mmr_bag;
    use crate::testutil::{Counting, Rng};

    pub(crate) fn data(i: u64) -> [u8; 8] {
        i.to_le_bytes()
    }

    pub(crate) fn build(n: u64) -> Mmr<Vec<Hash>> {
        let mut mmr = Mmr::new(Vec::new()).unwrap();
        (0..n).for_each(|i| {
            mmr.append(&data(i)).unwrap();
        });
        mmr
    }

    fn naive_tree(leaves: &[Hash]) -> Hash {
        if leaves.len() == 1 {
            return leaves[0];
        }
        let (l, r) = leaves.split_at(leaves.len() / 2);
        mmr_node(&naive_tree(l), &naive_tree(r))
    }

    pub(crate) fn naive_root(n: u64) -> Hash {
        let leaves: Vec<Hash> = (0..n).map(|i| mmr_leaf(&data(i))).collect();
        let mut peaks = Vec::new();
        let mut start = 0usize;
        for h in (0..64).rev() {
            if (n >> h) & 1 == 1 {
                peaks.push(naive_tree(&leaves[start..start + (1 << h)]));
                start += 1 << h;
            }
        }
        let bag = peaks.iter().rev().fold(None, |acc: Option<Hash>, p| {
            Some(acc.map_or(*p, |a| mmr_bag(p, &a)))
        });
        mmr_root(n, &bag.unwrap_or(ZERO))
    }

    #[test]
    fn empty_root_is_defined_and_stable() {
        let mmr = build(0);
        assert_eq!(mmr.leaf_count(), 0);
        assert_eq!(mmr.root().unwrap(), mmr_root(0, &ZERO));
        assert!(mmr.peaks().unwrap().is_empty());
    }

    #[test]
    fn single_leaf_root() {
        let mmr = build(1);
        assert_eq!(mmr.root().unwrap(), mmr_root(1, &mmr_leaf(&data(0))));
        assert_eq!(mmr.store().len(), 1);
    }

    #[test]
    fn roots_match_naive_for_all_small_sizes() {
        let mut mmr = Mmr::new(Vec::new()).unwrap();
        for n in 0..=300u64 {
            assert_eq!(mmr.root().unwrap(), naive_root(n), "n={n}");
            assert_eq!(mmr.store().len() as u64, pos::mmr_size(n));
            assert_eq!(mmr.append(&data(n)).unwrap(), n);
        }
    }

    #[test]
    fn root_at_matches_historical_roots() {
        let mut mmr = Mmr::new(Vec::new()).unwrap();
        let mut history = vec![mmr.root().unwrap()];
        for i in 0..200 {
            mmr.append(&data(i)).unwrap();
            history.push(mmr.root().unwrap());
        }
        for (n, r) in history.iter().enumerate() {
            assert_eq!(mmr.root_at(n as u64).unwrap(), *r);
        }
        assert_eq!(mmr.root_at(201), Err(Error::OutOfRange));
    }

    #[test]
    fn peaks_count_matches_popcount() {
        for n in 0..130u64 {
            assert_eq!(build(n).peaks().unwrap().len(), n.count_ones() as usize);
        }
    }

    #[test]
    fn reopen_from_store_preserves_state() {
        let mut mmr = build(77);
        let root = mmr.root().unwrap();
        let reopened = Mmr::new(mmr.store().clone()).unwrap();
        assert_eq!(reopened.leaf_count(), 77);
        assert_eq!(reopened.root().unwrap(), root);
        let mut reopened = reopened;
        mmr.append(b"x").unwrap();
        reopened.append(b"x").unwrap();
        assert_eq!(mmr.root().unwrap(), reopened.root().unwrap());
        assert_eq!(mmr.into_store(), reopened.into_store());
    }

    #[test]
    fn corrupt_store_size_rejected() {
        assert_eq!(Mmr::new(vec![ZERO; 2]).unwrap_err(), Error::CorruptStore);
        assert_eq!(Mmr::new(vec![ZERO; 5]).unwrap_err(), Error::CorruptStore);
        assert!(Mmr::new(vec![ZERO; 4]).is_ok());
    }

    struct Holey(Vec<Hash>, u64);

    impl MmrStore for Holey {
        fn size(&self) -> u64 {
            self.0.size()
        }
        fn get(&self, pos: u64) -> Option<Hash> {
            if pos == self.1 {
                None
            } else {
                MmrStore::get(&self.0, pos)
            }
        }
        fn push(&mut self, hash: Hash) {
            self.0.push(hash);
        }
    }

    #[test]
    fn missing_node_reported_without_partial_write() {
        let mut mmr = Mmr::new(Holey(build(3).into_store(), 3)).unwrap();
        assert_eq!(mmr.root().unwrap_err(), Error::MissingNode(3));
        assert_eq!(mmr.append(b"y").unwrap_err(), Error::MissingNode(3));
        assert_eq!(mmr.leaf_count(), 3);
        assert_eq!(mmr.store().0.len(), 4);
    }

    struct Huge;

    impl MmrStore for Huge {
        fn size(&self) -> u64 {
            pos::mmr_size(MAX_LEAVES)
        }
        fn get(&self, _: u64) -> Option<Hash> {
            Some(ZERO)
        }
        fn push(&mut self, _: Hash) {}
    }

    #[test]
    fn capacity_limit_enforced() {
        let mut mmr = Mmr::new(Huge).unwrap();
        assert_eq!(mmr.leaf_count(), MAX_LEAVES);
        assert_eq!(mmr.append(b"z").unwrap_err(), Error::Full);
        assert!(mmr.root().is_ok());
    }

    #[test]
    fn append_hash_equals_append_data() {
        let mut a = Mmr::new(Vec::new()).unwrap();
        let mut b = Mmr::new(Vec::new()).unwrap();
        for i in 0..40 {
            a.append(&data(i)).unwrap();
            b.append_leaf(mmr_leaf(&data(i))).unwrap();
        }
        assert_eq!(a.root().unwrap(), b.root().unwrap());
    }

    #[test]
    fn append_reads_and_writes_are_logarithmic_and_amortized_constant() {
        let mut mmr = Mmr::new(Counting::new(Vec::new())).unwrap();
        let mut total_writes = 0u64;
        for i in 0..4096u64 {
            let store = &mut mmr.store;
            store.reset();
            mmr.append(&data(i)).unwrap();
            let s = mmr.store();
            assert_eq!(s.reads.get(), u64::from(i.trailing_ones()));
            assert_eq!(s.writes, 1 + u64::from(i.trailing_ones()));
            total_writes += s.writes;
        }
        assert!(total_writes < 2 * 4096);
    }

    #[test]
    fn root_reads_popcount_nodes() {
        let mut mmr = Mmr::new(Counting::new(Vec::new())).unwrap();
        let mut rng = Rng::new(3);
        for _ in 0..1000 {
            mmr.append(&rng.hash()).unwrap();
        }
        mmr.store.reset();
        mmr.root().unwrap();
        assert_eq!(mmr.store().reads.get(), u64::from(1000u64.count_ones()));
    }

    #[test]
    fn different_data_different_root() {
        let mut a = build(10);
        let mut b = build(10);
        a.append(b"a").unwrap();
        b.append(b"b").unwrap();
        assert_ne!(a.root().unwrap(), b.root().unwrap());
    }

    #[test]
    fn vec_store_get_out_of_range() {
        let v: Vec<Hash> = vec![ZERO];
        assert_eq!(MmrStore::get(&v, 1), None);
        assert_eq!(MmrStore::get(&v, u64::MAX), None);
        assert_eq!(MmrStore::get(&v, 0), Some(ZERO));
    }
}
