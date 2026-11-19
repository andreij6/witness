use ic_stable_structures::storable::Blob;
use ic_stable_structures::{Memory, StableBTreeMap};
use witness_fingerprint::{BucketKey, POSTING_BYTES, Posting, PostingStore};

pub const BLOCK_POSTINGS: usize = 1024 / POSTING_BYTES;
pub const BLOCK_BYTES: usize = BLOCK_POSTINGS * POSTING_BYTES;

type Block = Blob<BLOCK_BYTES>;

pub struct Buckets<M: Memory> {
    map: StableBTreeMap<u64, Block, M>,
}

fn block(bytes: &[u8]) -> Block {
    Block::try_from(bytes).expect("block within capacity")
}

impl<M: Memory> Buckets<M> {
    pub fn init(memory: M) -> Self {
        Self {
            map: StableBTreeMap::init(memory),
        }
    }

    pub fn blocks(&self) -> u64 {
        self.map.len()
    }

    fn walk(&self, key: BucketKey) -> impl Iterator<Item = (u16, Block)> + '_ {
        let mut next = Some(0u16);
        std::iter::from_fn(move || {
            let sub = next?;
            let stored = self.map.get(&key.stable_key(sub))?;
            next = (stored.len() == BLOCK_BYTES)
                .then(|| sub.checked_add(1))
                .flatten();
            Some((sub, stored))
        })
    }

    fn tail(&self, key: BucketKey) -> Option<(u16, Block)> {
        let e = self.map.range(..=key.stable_key(u16::MAX)).next_back()?;
        let (owner, sub) = BucketKey::from_stable_key(*e.key());
        (owner == key).then(|| (sub, e.value()))
    }

    pub fn append(&mut self, key: BucketKey, postings: impl IntoIterator<Item = Posting>) {
        let (mut sub, mut bytes) = match self.tail(key) {
            Some((sub, stored)) => (sub, stored.as_slice().to_vec()),
            None => (0, Vec::new()),
        };
        let mut dirty = false;
        for posting in postings {
            if bytes.len() == BLOCK_BYTES {
                self.map.insert(key.stable_key(sub), block(&bytes));
                sub = sub.checked_add(1).expect("bucket block overflow");
                bytes.clear();
            }
            bytes.extend_from_slice(&posting.encode());
            dirty = true;
        }
        if dirty {
            self.map.insert(key.stable_key(sub), block(&bytes));
        }
    }

    pub fn remove(&mut self, key: BucketKey, posting: Posting) -> bool {
        let target = posting.encode();
        let mut found = None;
        let mut last = None;
        for (sub, stored) in self.walk(key) {
            if found.is_none() {
                found = stored
                    .as_slice()
                    .chunks_exact(POSTING_BYTES)
                    .position(|p| p == target)
                    .map(|at| (sub, stored, at * POSTING_BYTES));
            }
            last = Some((sub, stored));
        }
        let (Some((found_sub, found_block, at)), Some((last_sub, last_block))) = (found, last)
        else {
            return false;
        };
        let mut tail_block = last_block.as_slice().to_vec();
        let mut tail = [0u8; POSTING_BYTES];
        tail.copy_from_slice(&tail_block[tail_block.len() - POSTING_BYTES..]);
        tail_block.truncate(tail_block.len() - POSTING_BYTES);
        if found_sub == last_sub {
            if at < tail_block.len() {
                tail_block[at..at + POSTING_BYTES].copy_from_slice(&tail);
            }
        } else {
            let mut patched = found_block.as_slice().to_vec();
            patched[at..at + POSTING_BYTES].copy_from_slice(&tail);
            self.map.insert(key.stable_key(found_sub), block(&patched));
        }
        if tail_block.is_empty() {
            self.map.remove(&key.stable_key(last_sub));
        } else {
            self.map
                .insert(key.stable_key(last_sub), block(&tail_block));
        }
        true
    }

    pub fn count(&self, key: BucketKey) -> usize {
        self.walk(key).map(|(_, b)| b.len() / POSTING_BYTES).sum()
    }
}

impl<M: Memory> PostingStore for Buckets<M> {
    fn insert(&mut self, key: BucketKey, posting: Posting) {
        self.append(key, [posting]);
    }

    fn visit(&self, key: BucketKey, mut sink: impl FnMut(Posting)) {
        for (_, stored) in self.walk(key) {
            for chunk in stored.as_slice().chunks_exact(POSTING_BYTES) {
                let mut raw = [0u8; POSTING_BYTES];
                raw.copy_from_slice(chunk);
                sink(Posting::decode(&raw));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{hash, mem};
    use witness_fingerprint::{MIH_CHUNKS, Prober, insert};

    fn buckets() -> Buckets<crate::testkit::Mem> {
        Buckets::init(mem())
    }

    fn key(chunk: u8, value: u16) -> BucketKey {
        BucketKey { chunk, value }
    }

    fn p(local_ix: u32, shape: u8) -> Posting {
        Posting { local_ix, shape }
    }

    fn collect(b: &Buckets<crate::testkit::Mem>, k: BucketKey) -> Vec<Posting> {
        let mut out = Vec::new();
        b.visit(k, |x| out.push(x));
        out
    }

    #[test]
    fn block_fits_one_kib() {
        assert_eq!(BLOCK_POSTINGS, 204);
        const { assert!(BLOCK_BYTES <= 1024) };
        assert_eq!(BLOCK_BYTES % POSTING_BYTES, 0);
    }

    #[test]
    fn append_then_visit_in_order() {
        let mut b = buckets();
        b.append(key(3, 7), [p(1, 0), p(2, 4)]);
        b.append(key(3, 7), [p(9, 1)]);
        assert_eq!(collect(&b, key(3, 7)), [p(1, 0), p(2, 4), p(9, 1)]);
        assert_eq!(b.count(key(3, 7)), 3);
        assert_eq!(b.blocks(), 1);
    }

    #[test]
    fn neighbouring_keys_do_not_bleed() {
        let mut b = buckets();
        b.append(key(3, 7), [p(1, 0)]);
        b.append(key(3, 8), [p(2, 0)]);
        b.append(key(4, 7), [p(3, 0)]);
        b.append(key(2, u16::MAX), [p(4, 0)]);
        assert_eq!(collect(&b, key(3, 7)), [p(1, 0)]);
        assert_eq!(collect(&b, key(3, 8)), [p(2, 0)]);
        assert_eq!(collect(&b, key(4, 7)), [p(3, 0)]);
        assert_eq!(collect(&b, key(2, u16::MAX)), [p(4, 0)]);
        assert!(collect(&b, key(3, 6)).is_empty());
    }

    #[test]
    fn empty_append_writes_nothing() {
        let mut b = buckets();
        b.append(key(0, 0), []);
        assert_eq!(b.blocks(), 0);
        assert!(collect(&b, key(0, 0)).is_empty());
    }

    #[test]
    fn overflow_spills_into_contiguous_blocks_never_truncates() {
        let mut b = buckets();
        let n = BLOCK_POSTINGS * 2 + 5;
        b.append(key(1, 1), (0..n as u32).map(|i| p(i, 0)));
        assert_eq!(b.blocks(), 3);
        let got = collect(&b, key(1, 1));
        assert_eq!(got.len(), n);
        assert!(got.iter().enumerate().all(|(i, x)| x.local_ix == i as u32));
        b.append(key(1, 1), [p(77_777, 3)]);
        assert_eq!(b.count(key(1, 1)), n + 1);
        assert_eq!(collect(&b, key(1, 1)).last(), Some(&p(77_777, 3)));
    }

    #[test]
    fn single_appends_fill_exact_block_boundary() {
        let mut b = buckets();
        for i in 0..BLOCK_POSTINGS as u32 {
            b.insert(key(0, 5), p(i, 1));
        }
        assert_eq!(b.blocks(), 1);
        b.insert(key(0, 5), p(9_999, 1));
        assert_eq!(b.blocks(), 2);
        assert_eq!(b.count(key(0, 5)), BLOCK_POSTINGS + 1);
    }

    #[test]
    fn remove_swaps_tail_in_same_block() {
        let mut b = buckets();
        b.append(key(2, 2), [p(1, 0), p(2, 0), p(3, 0)]);
        assert!(b.remove(key(2, 2), p(1, 0)));
        assert_eq!(collect(&b, key(2, 2)), [p(3, 0), p(2, 0)]);
        assert!(b.remove(key(2, 2), p(2, 0)));
        assert_eq!(collect(&b, key(2, 2)), [p(3, 0)]);
        assert!(b.remove(key(2, 2), p(3, 0)));
        assert_eq!(b.blocks(), 0);
    }

    #[test]
    fn remove_across_blocks_keeps_buckets_dense() {
        let mut b = buckets();
        let n = BLOCK_POSTINGS + 1;
        b.append(key(5, 5), (0..n as u32).map(|i| p(i, 2)));
        assert_eq!(b.blocks(), 2);
        assert!(b.remove(key(5, 5), p(0, 2)));
        assert_eq!(b.blocks(), 1);
        let got = collect(&b, key(5, 5));
        assert_eq!(got.len(), n - 1);
        assert_eq!(got[0], p(n as u32 - 1, 2));
        b.append(key(5, 5), [p(1_000_000, 0)]);
        assert_eq!(b.blocks(), 2);
    }

    #[test]
    fn remove_missing_is_noop() {
        let mut b = buckets();
        assert!(!b.remove(key(1, 1), p(1, 1)));
        b.append(key(1, 1), [p(1, 1)]);
        assert!(!b.remove(key(1, 1), p(1, 2)));
        assert!(!b.remove(key(1, 2), p(1, 1)));
        assert_eq!(collect(&b, key(1, 1)), [p(1, 1)]);
    }

    #[test]
    fn remove_matches_whole_postings_not_byte_offsets() {
        let mut b = buckets();
        b.append(key(0, 1), [p(0x0000_0102, 3), p(0x0405_0000, 1)]);
        assert!(!b.remove(key(0, 1), p(0x0203_0405, 0)));
        assert_eq!(b.count(key(0, 1)), 2);
    }

    #[test]
    fn prober_over_stable_buckets_finds_inserted_hash() {
        let mut b = buckets();
        for i in 0..200u32 {
            insert(&mut b, &hash(i as u64), p(i, 0));
        }
        let mut prober = Prober::new();
        let ranked = prober.probe(&b, &hash(42), 10);
        assert_eq!(ranked.candidates[0].local_ix, 42);
        assert_eq!(ranked.candidates[0].matched_chunks as usize, MIH_CHUNKS);
    }

    #[derive(Clone, Default)]
    struct Counting {
        inner: crate::testkit::Mem,
        read: std::rc::Rc<std::cell::Cell<u64>>,
    }

    impl Memory for Counting {
        fn size(&self) -> u64 {
            self.inner.size()
        }
        fn grow(&self, pages: u64) -> i64 {
            self.inner.grow(pages)
        }
        fn read(&self, offset: u64, dst: &mut [u8]) {
            self.read.set(self.read.get() + dst.len() as u64);
            self.inner.read(offset, dst)
        }
        fn write(&self, offset: u64, src: &[u8]) {
            self.inner.write(offset, src)
        }
    }

    fn append_cost(blocks: usize) -> u64 {
        let m = Counting::default();
        let mut b = Buckets::init(m.clone());
        let k = key(3, 3);
        b.append(k, (0..(BLOCK_POSTINGS * blocks) as u32).map(|i| p(i, 0)));
        b.append(key(3, 4), [p(1, 1)]);
        b.append(key(3, 2), [p(1, 1)]);
        let before = m.read.get();
        b.append(k, [p(u32::MAX, 0)]);
        let cost = m.read.get() - before;
        assert_eq!(b.count(k), BLOCK_POSTINGS * blocks + 1);
        let mut last = None;
        b.visit(k, |x| last = Some(x));
        assert_eq!(last, Some(p(u32::MAX, 0)));
        cost
    }

    #[test]
    fn append_reads_only_the_tail_block_not_the_whole_bucket() {
        let short = append_cost(40);
        let long = append_cost(160);
        assert!(long < short * 2, "short={short} long={long}");
    }

    #[test]
    fn tail_is_the_highest_block_of_its_bucket_only() {
        let mut b = buckets();
        assert_eq!(b.tail(key(1, 1)).map(|t| t.0), None);
        b.append(
            key(1, 1),
            (0..(BLOCK_POSTINGS * 2 + 1) as u32).map(|i| p(i, 0)),
        );
        b.append(key(1, 2), [p(5, 5)]);
        assert_eq!(b.tail(key(1, 1)).map(|t| t.0), Some(2));
        assert_eq!(b.tail(key(1, 2)).map(|t| t.0), Some(0));
        assert_eq!(b.tail(key(1, 0)).map(|t| t.0), None);
        assert_eq!(b.tail(key(0, u16::MAX)).map(|t| t.0), None);
    }

    #[test]
    fn deterministic_layout() {
        let build = || {
            let mut b = buckets();
            for i in 0..50u32 {
                insert(&mut b, &hash(i as u64 % 7), p(i, (i % 3) as u8));
            }
            (0..MIH_CHUNKS as u8)
                .flat_map(|c| (0..64u16).map(move |v| key(c, v)))
                .map(|k| collect(&b, k))
                .collect::<Vec<_>>()
        };
        assert_eq!(build(), build());
    }
}
