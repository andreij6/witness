use super::{BucketKey, MIH_CHUNKS, PROBES_PER_CHUNK, PostingStore, probe_keys};
use crate::hash::PdqHash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Candidate {
    pub local_ix: u32,
    pub matched_chunks: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ranked {
    pub candidates: Vec<Candidate>,
    pub truncated: bool,
}

#[derive(Default)]
pub struct Prober {
    seen: Vec<u64>,
    touched: Vec<u32>,
    hits: Vec<u32>,
}

impl Prober {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(local_ixs: u32) -> Self {
        Self {
            seen: vec![0; (local_ixs as usize).div_ceil(64)],
            ..Self::default()
        }
    }

    fn mark(&mut self, local_ix: u32) {
        let (word, bit) = (local_ix as usize / 64, local_ix % 64);
        if word >= self.seen.len() {
            self.seen.resize(word + 1, 0);
        }
        if self.seen[word] >> bit & 1 == 0 {
            self.seen[word] |= 1 << bit;
            self.touched.push(local_ix);
        }
    }

    fn close_chunk(&mut self) {
        for &ix in &self.touched {
            self.seen[ix as usize / 64] &= !(1 << (ix % 64));
        }
        self.hits.append(&mut self.touched);
    }

    pub fn probe<S: PostingStore>(&mut self, store: &S, query: &PdqHash, limit: usize) -> Ranked {
        self.hits.clear();
        let keys = probe_keys(query);
        for chunk_keys in keys.chunks_exact(PROBES_PER_CHUNK) {
            for &key in chunk_keys {
                self.visit(store, key);
            }
            self.close_chunk();
        }
        self.rank(limit)
    }

    fn visit<S: PostingStore>(&mut self, store: &S, key: BucketKey) {
        store.visit(key, |posting| self.mark(posting.local_ix));
    }

    fn rank(&mut self, limit: usize) -> Ranked {
        self.hits.sort_unstable();
        let mut candidates: Vec<Candidate> = Vec::new();
        for run in self.hits.chunk_by(|a, b| a == b) {
            debug_assert!(run.len() <= MIH_CHUNKS);
            candidates.push(Candidate {
                local_ix: run[0],
                matched_chunks: run.len() as u8,
            });
        }
        candidates.sort_unstable_by_key(|c| (core::cmp::Reverse(c.matched_chunks), c.local_ix));
        let truncated = match limit {
            _ if candidates.len() <= limit => false,
            0 => true,
            _ => candidates[limit].matched_chunks == candidates[limit - 1].matched_chunks,
        };
        candidates.truncate(limit);
        Ranked {
            candidates,
            truncated,
        }
    }
}
