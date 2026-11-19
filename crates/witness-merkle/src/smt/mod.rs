mod empty;
mod proof;

use crate::hash::{Hash, smt_leaf, smt_node};
use std::collections::BTreeMap;
use witness_format::KeyId;

pub use empty::empty;
pub use proof::SmtProof;

pub const DEPTH: u8 = 128;

pub trait SmtStore {
    fn get(&self, depth: u8, prefix: u128) -> Option<Hash>;
    fn set(&mut self, depth: u8, prefix: u128, hash: Hash);
    fn remove(&mut self, depth: u8, prefix: u128);
}

impl SmtStore for BTreeMap<(u8, u128), Hash> {
    fn get(&self, depth: u8, prefix: u128) -> Option<Hash> {
        BTreeMap::get(self, &(depth, prefix)).copied()
    }

    fn set(&mut self, depth: u8, prefix: u128, hash: Hash) {
        self.insert((depth, prefix), hash);
    }

    fn remove(&mut self, depth: u8, prefix: u128) {
        BTreeMap::remove(self, &(depth, prefix));
    }
}

pub fn key_of(id: &KeyId) -> u128 {
    u128::from_be_bytes(id.to_bytes())
}

pub(crate) const fn prefix(key: u128, depth: u8) -> u128 {
    match key.checked_shr((DEPTH - depth) as u32) {
        Some(p) => p,
        None => 0,
    }
}

pub(crate) fn combine(height: u8, is_right: bool, node: &Hash, sibling: &Hash) -> Hash {
    let e = empty(height);
    match (node == e && sibling == e, is_right) {
        (true, _) => *empty(height + 1),
        (false, true) => smt_node(sibling, node),
        (false, false) => smt_node(node, sibling),
    }
}

#[derive(Debug, Clone, Default)]
pub struct Smt<S> {
    store: S,
}

impl<S: SmtStore> Smt<S> {
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    pub const fn store(&self) -> &S {
        &self.store
    }

    pub fn into_store(self) -> S {
        self.store
    }

    pub fn root(&self) -> Hash {
        self.node(0, 0)
    }

    pub fn leaf(&self, key: u128) -> Hash {
        self.node(DEPTH, key)
    }

    pub fn insert(&mut self, key: u128, value: &Hash) -> Hash {
        self.write_path(key, smt_leaf(key, value))
    }

    pub fn remove(&mut self, key: u128) -> Hash {
        self.write_path(key, *empty(0))
    }

    pub fn prove(&self, key: u128) -> SmtProof {
        let mut proof = SmtProof::default();
        for depth in (1..=DEPTH).rev() {
            let height = DEPTH - depth;
            let sibling = self.node(depth, prefix(key, depth) ^ 1);
            if sibling != *empty(height) {
                proof.bitmap |= 1 << height;
                proof.siblings.push(sibling);
            }
        }
        proof
    }

    fn node(&self, depth: u8, prefix: u128) -> Hash {
        self.store
            .get(depth, prefix)
            .unwrap_or(*empty(DEPTH - depth))
    }

    fn put(&mut self, depth: u8, prefix: u128, hash: Hash) {
        if hash == *empty(DEPTH - depth) {
            self.store.remove(depth, prefix);
        } else {
            self.store.set(depth, prefix, hash);
        }
    }

    fn write_path(&mut self, key: u128, leaf: Hash) -> Hash {
        let mut cur = leaf;
        self.put(DEPTH, key, cur);
        for depth in (1..=DEPTH).rev() {
            let p = prefix(key, depth);
            let sibling = self.node(depth, p ^ 1);
            cur = combine(DEPTH - depth, p & 1 == 1, &cur, &sibling);
            self.put(depth - 1, p >> 1, cur);
        }
        cur
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hash::ZERO;
    use crate::testutil::{Counting, Rng};

    pub(crate) type MapSmt = Smt<BTreeMap<(u8, u128), Hash>>;

    fn naive(sorted: &[(u128, Hash)], depth: u8) -> Hash {
        if sorted.is_empty() {
            return *empty(DEPTH - depth);
        }
        if depth == DEPTH {
            return smt_leaf(sorted[0].0, &sorted[0].1);
        }
        let bit = 1u128 << (DEPTH - depth - 1);
        let split = sorted.partition_point(|(k, _)| k & bit == 0);
        let (l, r) = sorted.split_at(split);
        smt_node(&naive(l, depth + 1), &naive(r, depth + 1))
    }

    pub(crate) fn naive_root(model: &BTreeMap<u128, Hash>) -> Hash {
        let sorted: Vec<(u128, Hash)> = model.iter().map(|(k, v)| (*k, *v)).collect();
        naive(&sorted, 0)
    }

    #[test]
    fn empty_tree_root_is_top_empty_hash() {
        let smt = MapSmt::default();
        assert_eq!(smt.root(), *empty(DEPTH));
        assert_eq!(smt.leaf(5), ZERO);
        assert!(smt.store().is_empty());
    }

    #[test]
    fn prefix_bounds() {
        assert_eq!(prefix(u128::MAX, 0), 0);
        assert_eq!(prefix(u128::MAX, 1), 1);
        assert_eq!(prefix(u128::MAX, DEPTH), u128::MAX);
        assert_eq!(prefix(1u128 << 127, 1), 1);
        assert_eq!(prefix(0xabu128 << 120, 8), 0xab);
    }

    #[test]
    fn single_insert_matches_naive() {
        let mut smt = MapSmt::default();
        let v = [5u8; 32];
        let root = smt.insert(42, &v);
        assert_eq!(root, smt.root());
        assert_eq!(root, naive_root(&BTreeMap::from([(42, v)])));
        assert_eq!(smt.leaf(42), smt_leaf(42, &v));
        assert_eq!(smt.store().len(), 129);
    }

    #[test]
    fn insert_then_remove_restores_empty_and_frees_store() {
        let mut smt = MapSmt::default();
        smt.insert(7, &[1; 32]);
        smt.insert(u128::MAX, &[2; 32]);
        smt.remove(7);
        smt.remove(u128::MAX);
        assert_eq!(smt.root(), *empty(DEPTH));
        assert!(smt.store().is_empty());
    }

    #[test]
    fn remove_absent_key_is_noop() {
        let mut smt = MapSmt::default();
        let r = smt.insert(1, &[1; 32]);
        assert_eq!(smt.remove(2), r);
        assert_eq!(smt.store().len(), 129);
    }

    #[test]
    fn order_independent() {
        let mut rng = Rng::new(31);
        let entries: Vec<(u128, Hash)> = (0..50).map(|_| (rng.u128(), rng.hash())).collect();
        let mut a = MapSmt::default();
        let mut b = MapSmt::default();
        entries.iter().for_each(|(k, v)| {
            a.insert(*k, v);
        });
        entries.iter().rev().for_each(|(k, v)| {
            b.insert(*k, v);
        });
        assert_eq!(a.root(), b.root());
        assert_eq!(a.into_store(), b.into_store());
    }

    #[test]
    fn random_sequences_match_naive() {
        for seed in 0..12 {
            let mut rng = Rng::new(seed);
            let mut smt = MapSmt::default();
            let mut model = BTreeMap::new();
            let pool: Vec<u128> = (0..24)
                .map(|i| match i % 4 {
                    0 => rng.u128(),
                    1 => rng.below(16) as u128,
                    2 => u128::MAX - rng.below(16) as u128,
                    _ => rng.u128() & !0xff,
                })
                .collect();
            for _ in 0..60 {
                let key = pool[rng.below(pool.len() as u64) as usize];
                let root = if rng.below(4) == 0 {
                    model.remove(&key);
                    smt.remove(key)
                } else {
                    let v = rng.hash();
                    model.insert(key, v);
                    smt.insert(key, &v)
                };
                assert_eq!(root, naive_root(&model), "seed={seed}");
            }
        }
    }

    #[test]
    fn update_costs_exactly_depth_reads_and_depth_plus_one_writes() {
        let mut smt = Smt::new(Counting::new(BTreeMap::new()));
        let mut rng = Rng::new(41);
        for _ in 0..200 {
            smt.store.reset();
            if rng.below(5) == 0 {
                smt.remove(rng.u128());
            } else {
                smt.insert(rng.u128(), &rng.hash());
            }
            assert_eq!(smt.store().reads.get(), u64::from(DEPTH));
            assert_eq!(smt.store().writes, u64::from(DEPTH) + 1);
        }
    }

    #[test]
    fn store_holds_only_non_default_nodes() {
        let mut smt = MapSmt::default();
        let mut rng = Rng::new(43);
        for _ in 0..30 {
            smt.insert(rng.u128(), &rng.hash());
        }
        for (&(depth, _), h) in smt.store() {
            assert_ne!(*h, *empty(DEPTH - depth));
        }
        assert!(smt.store().len() <= 30 * 129);
    }

    #[test]
    fn key_of_is_big_endian() {
        let mut b = [0u8; 16];
        b[0] = 0x80;
        b[15] = 1;
        assert_eq!(key_of(&KeyId::from_bytes(b)), (1u128 << 127) | 1);
    }

    #[test]
    fn combine_shortcut_matches_hash() {
        for h in 0..DEPTH {
            assert_eq!(
                combine(h, false, empty(h), empty(h)),
                smt_node(empty(h), empty(h))
            );
            assert_eq!(combine(h, true, empty(h), empty(h)), *empty(h + 1));
        }
        let a = [1u8; 32];
        assert_eq!(combine(3, true, &a, empty(3)), smt_node(empty(3), &a));
        assert_eq!(combine(3, false, &a, empty(3)), smt_node(&a, empty(3)));
    }
}
