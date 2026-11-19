use crate::hash::Hash;
use crate::mmr::MmrStore;
use crate::smt::SmtStore;
use std::cell::Cell;
use std::collections::BTreeMap;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    pub fn u128(&mut self) -> u128 {
        (u128::from(self.next()) << 64) | u128::from(self.next())
    }

    pub fn hash(&mut self) -> Hash {
        let mut out = [0u8; 32];
        out.chunks_mut(8)
            .for_each(|c| c.copy_from_slice(&self.next().to_le_bytes()));
        out
    }
}

#[derive(Default)]
pub struct Counting<S> {
    pub inner: S,
    pub reads: Cell<u64>,
    pub writes: u64,
}

impl<S> Counting<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            reads: Cell::new(0),
            writes: 0,
        }
    }

    pub fn reset(&mut self) {
        self.reads.set(0);
        self.writes = 0;
    }
}

impl MmrStore for Counting<Vec<Hash>> {
    fn size(&self) -> u64 {
        self.inner.size()
    }

    fn get(&self, pos: u64) -> Option<Hash> {
        self.reads.set(self.reads.get() + 1);
        MmrStore::get(&self.inner, pos)
    }

    fn push(&mut self, hash: Hash) {
        self.writes += 1;
        MmrStore::push(&mut self.inner, hash);
    }
}

impl SmtStore for Counting<BTreeMap<(u8, u128), Hash>> {
    fn get(&self, depth: u8, prefix: u128) -> Option<Hash> {
        self.reads.set(self.reads.get() + 1);
        SmtStore::get(&self.inner, depth, prefix)
    }

    fn set(&mut self, depth: u8, prefix: u128, hash: Hash) {
        self.writes += 1;
        self.inner.set(depth, prefix, hash);
    }

    fn remove(&mut self, depth: u8, prefix: u128) {
        self.writes += 1;
        SmtStore::remove(&mut self.inner, depth, prefix);
    }
}
