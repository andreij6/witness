#![allow(dead_code)]

use witness_merkle::Hash;

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

pub fn record(i: u64) -> Vec<u8> {
    let mut v = b"record-".to_vec();
    v.extend_from_slice(&i.to_be_bytes());
    v
}
