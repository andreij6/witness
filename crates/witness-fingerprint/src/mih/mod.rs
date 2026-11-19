mod permutation;
mod probe;

pub use permutation::{PERMUTATION, PERMUTATION_SEED, chunks, permute};
pub use probe::{Candidate, Prober, Ranked};

use crate::hash::PdqHash;

pub const MIH_CHUNKS: usize = 16;
pub const MIH_CHUNK_BITS: usize = 16;
pub const MIH_RADIUS: u32 = (MIH_CHUNKS as u32) * 2 - 1;
pub const PROBES_PER_CHUNK: usize = MIH_CHUNK_BITS + 1;
pub const PROBES_PER_HASH: usize = MIH_CHUNKS * PROBES_PER_CHUNK;
pub const POSTING_BYTES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BucketKey {
    pub chunk: u8,
    pub value: u16,
}

impl BucketKey {
    pub const fn packed(self) -> u32 {
        ((self.chunk as u32) << 16) | self.value as u32
    }

    pub const fn stable_key(self, sub: u16) -> u64 {
        ((self.chunk as u64) << 32) | ((self.value as u64) << 16) | sub as u64
    }

    pub const fn from_stable_key(key: u64) -> (Self, u16) {
        let key_part = Self {
            chunk: (key >> 32) as u8,
            value: (key >> 16) as u16,
        };
        (key_part, key as u16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Posting {
    pub local_ix: u32,
    pub shape: u8,
}

impl Posting {
    pub const fn encode(self) -> [u8; POSTING_BYTES] {
        let ix = self.local_ix.to_be_bytes();
        [ix[0], ix[1], ix[2], ix[3], self.shape]
    }

    pub const fn decode(bytes: &[u8; POSTING_BYTES]) -> Self {
        Self {
            local_ix: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            shape: bytes[4],
        }
    }
}

pub trait PostingStore {
    fn insert(&mut self, key: BucketKey, posting: Posting);
    fn visit(&self, key: BucketKey, sink: impl FnMut(Posting));
}

pub fn index_keys(hash: &PdqHash) -> [BucketKey; MIH_CHUNKS] {
    let values = chunks(hash);
    core::array::from_fn(|chunk| BucketKey {
        chunk: chunk as u8,
        value: values[chunk],
    })
}

pub fn probe_keys(hash: &PdqHash) -> [BucketKey; PROBES_PER_HASH] {
    let values = chunks(hash);
    core::array::from_fn(|i| {
        let (chunk, flip) = (i / PROBES_PER_CHUNK, i % PROBES_PER_CHUNK);
        let value = if flip == 0 {
            values[chunk]
        } else {
            values[chunk] ^ (1 << (flip - 1))
        };
        BucketKey {
            chunk: chunk as u8,
            value,
        }
    })
}

pub fn insert<S: PostingStore>(store: &mut S, hash: &PdqHash, posting: Posting) {
    index_keys(hash)
        .into_iter()
        .for_each(|key| store.insert(key, posting));
}

#[cfg(test)]
mod tests;
