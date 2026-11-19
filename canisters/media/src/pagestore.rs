use ic_stable_structures::Memory;
use sha2::{Digest, Sha256};

pub const PAGE: u64 = 64 * 1024;
const NONE: u32 = u32::MAX;
const HEAD: u64 = 0;
const FREE_COUNT: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfMemory;

pub struct PageStore<M: Memory> {
    mem: M,
}

impl<M: Memory> PageStore<M> {
    pub fn init(mem: M) -> Self {
        if mem.size() == 0 {
            assert!(mem.grow(1) >= 0, "cannot grow page store");
            mem.write(HEAD, &NONE.to_le_bytes());
            mem.write(FREE_COUNT, &0u32.to_le_bytes());
        }
        Self { mem }
    }

    fn read_u32(&self, offset: u64) -> u32 {
        let mut b = [0u8; 4];
        self.mem.read(offset, &mut b);
        u32::from_le_bytes(b)
    }

    fn write_u32(&self, offset: u64, v: u32) {
        self.mem.write(offset, &v.to_le_bytes());
    }

    pub fn total_pages(&self) -> u64 {
        self.mem.size().saturating_sub(1)
    }

    pub fn free_pages(&self) -> u64 {
        self.read_u32(FREE_COUNT) as u64
    }

    pub fn alloc(&self, n: u64) -> Result<Vec<u32>, OutOfMemory> {
        let mut out = Vec::with_capacity(n as usize);
        let mut head = self.read_u32(HEAD);
        let mut free = self.read_u32(FREE_COUNT);
        while (out.len() as u64) < n && head != NONE {
            out.push(head);
            head = self.read_u32(head as u64 * PAGE);
            free -= 1;
        }
        let fresh = n - out.len() as u64;
        if fresh > 0 {
            let first = self.mem.size();
            if first + fresh > NONE as u64 || self.mem.grow(fresh) < 0 {
                return Err(OutOfMemory);
            }
            out.extend((first..first + fresh).map(|p| p as u32));
        }
        self.write_u32(HEAD, head);
        self.write_u32(FREE_COUNT, free);
        Ok(out)
    }

    pub fn free(&self, pages: &[u32]) {
        let mut head = self.read_u32(HEAD);
        for p in pages {
            self.write_u32(*p as u64 * PAGE, head);
            head = *p;
        }
        self.write_u32(HEAD, head);
        let free = self.read_u32(FREE_COUNT) + pages.len() as u32;
        self.write_u32(FREE_COUNT, free);
    }

    fn spans(pages: &[u32], offset: u64, len: usize) -> impl Iterator<Item = (u64, usize, usize)> {
        let mut pos = offset;
        let end = offset + len as u64;
        let mut done = 0usize;
        core::iter::from_fn(move || {
            if pos >= end {
                return None;
            }
            let page = pages[(pos / PAGE) as usize] as u64;
            let within = pos % PAGE;
            let take = (PAGE - within).min(end - pos) as usize;
            let item = (page * PAGE + within, done, take);
            pos += take as u64;
            done += take;
            Some(item)
        })
    }

    pub fn write(&self, pages: &[u32], offset: u64, bytes: &[u8]) {
        for (addr, at, take) in Self::spans(pages, offset, bytes.len()) {
            self.mem.write(addr, &bytes[at..at + take]);
        }
    }

    pub fn read(&self, pages: &[u32], offset: u64, out: &mut [u8]) {
        let len = out.len();
        for (addr, at, take) in Self::spans(pages, offset, len) {
            self.mem.read(addr, &mut out[at..at + take]);
        }
    }

    pub fn sha256(&self, pages: &[u32], len: u64) -> [u8; 32] {
        let mut h = Sha256::new();
        let mut buf = vec![0u8; PAGE as usize];
        let mut pos = 0u64;
        for p in pages {
            if pos >= len {
                break;
            }
            let take = (len - pos).min(PAGE) as usize;
            self.mem.read(*p as u64 * PAGE, &mut buf[..take]);
            h.update(&buf[..take]);
            pos += take as u64;
        }
        h.finalize().into()
    }
}

pub const fn pages_for(len: u64) -> u64 {
    len.div_ceil(PAGE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_stable_structures::VectorMemory;

    fn store() -> PageStore<VectorMemory> {
        PageStore::init(VectorMemory::default())
    }

    fn pattern(len: usize, seed: u8) -> Vec<u8> {
        (0..len)
            .map(|i| (i as u8).wrapping_mul(31) ^ seed)
            .collect()
    }

    #[test]
    fn pages_for_rounds_up() {
        assert_eq!(pages_for(0), 0);
        assert_eq!(pages_for(1), 1);
        assert_eq!(pages_for(PAGE), 1);
        assert_eq!(pages_for(PAGE + 1), 2);
    }

    #[test]
    fn fresh_store_has_header_only() {
        let s = store();
        assert_eq!(s.total_pages(), 0);
        assert_eq!(s.free_pages(), 0);
    }

    #[test]
    fn alloc_grows_then_reuses_freed_pages() {
        let s = store();
        let a = s.alloc(3).unwrap_or_default();
        assert_eq!(a, vec![1, 2, 3]);
        assert_eq!(s.total_pages(), 3);
        s.free(&a[..2]);
        assert_eq!(s.free_pages(), 2);
        let b = s.alloc(3).unwrap_or_default();
        assert_eq!(b, vec![2, 1, 4]);
        assert_eq!(s.free_pages(), 0);
        assert_eq!(s.total_pages(), 4);
    }

    #[test]
    fn alloc_zero_is_empty() {
        let s = store();
        assert_eq!(s.alloc(0), Ok(vec![]));
        assert_eq!(s.total_pages(), 0);
    }

    #[test]
    fn write_read_round_trip_across_pages() {
        let s = store();
        let len = 3 * PAGE as usize + 17;
        let pages = s.alloc(pages_for(len as u64)).unwrap_or_default();
        let data = pattern(len, 5);
        s.write(&pages, 0, &data);
        let mut out = vec![0u8; len];
        s.read(&pages, 0, &mut out);
        assert_eq!(out, data);
        let mut mid = vec![0u8; 1000];
        s.read(&pages, PAGE - 500, &mut mid);
        assert_eq!(mid, data[PAGE as usize - 500..PAGE as usize + 500]);
    }

    #[test]
    fn non_contiguous_pages_round_trip() {
        let s = store();
        let a = s.alloc(4).unwrap_or_default();
        s.free(&[a[0], a[2]]);
        let b = s.alloc(3).unwrap_or_default();
        assert_eq!(b, vec![3, 1, 5]);
        let data = pattern(2 * PAGE as usize + 3, 9);
        s.write(&b, 0, &data);
        let mut out = vec![0u8; data.len()];
        s.read(&b, 0, &mut out);
        assert_eq!(out, data);
        let mut other = vec![0u8; 8];
        s.read(&[a[1]], 0, &mut other);
        assert_eq!(other, [0; 8]);
    }

    #[test]
    fn chunked_writes_match_single_write() {
        let s = store();
        let len = 5 * PAGE as usize / 2;
        let pages = s.alloc(pages_for(len as u64)).unwrap_or_default();
        let data = pattern(len, 1);
        for (i, c) in data.chunks(40_000).enumerate() {
            s.write(&pages, i as u64 * 40_000, c);
        }
        let mut out = vec![0u8; len];
        s.read(&pages, 0, &mut out);
        assert_eq!(out, data);
    }

    #[test]
    fn sha256_matches_direct_hash() {
        let s = store();
        for len in [
            1usize,
            PAGE as usize,
            PAGE as usize + 1,
            3 * PAGE as usize - 2,
        ] {
            let pages = s.alloc(pages_for(len as u64)).unwrap_or_default();
            let data = pattern(len, len as u8);
            s.write(&pages, 0, &data);
            let want: [u8; 32] = Sha256::digest(&data).into();
            assert_eq!(s.sha256(&pages, len as u64), want);
            s.free(&pages);
        }
    }

    #[test]
    fn sha256_ignores_tail_of_last_page() {
        let s = store();
        let pages = s.alloc(1).unwrap_or_default();
        s.write(&pages, 0, &[1, 2, 3, 4]);
        let want: [u8; 32] = Sha256::digest([1u8, 2]).into();
        assert_eq!(s.sha256(&pages, 2), want);
    }

    #[test]
    fn reinit_keeps_state() {
        let mem = VectorMemory::default();
        let s = PageStore::init(mem.clone());
        let a = s.alloc(2).unwrap_or_default();
        s.write(&a, 0, b"hello");
        s.free(&a[1..]);
        let s2 = PageStore::init(mem);
        assert_eq!(s2.free_pages(), 1);
        let mut out = [0u8; 5];
        s2.read(&a, 0, &mut out);
        assert_eq!(&out, b"hello");
        assert_eq!(s2.alloc(1), Ok(vec![a[1]]));
    }

    #[test]
    fn alloc_fails_cleanly_when_memory_cannot_grow() {
        let mem = ic_stable_structures::memory_manager::MemoryManager::init_with_bucket_size(
            VectorMemory::default(),
            1,
        );
        let s = PageStore::init(mem.get(ic_stable_structures::memory_manager::MemoryId::new(0)));
        assert_eq!(s.alloc(70_000), Err(OutOfMemory));
        assert_eq!(s.free_pages(), 0);
    }

    #[test]
    fn deterministic_allocation_order() {
        let run = || {
            let s = store();
            let a = s.alloc(5).unwrap_or_default();
            s.free(&[a[4], a[1]]);
            s.alloc(3).unwrap_or_default()
        };
        assert_eq!(run(), run());
    }
}
