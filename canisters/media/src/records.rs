use crate::certtree::Hash;
use crate::paths::{BlobPath, MAX_LABEL};
use candid::Principal;
use ic_stable_structures::Storable;
use ic_stable_structures::storable::Bound;
use std::borrow::Cow;
use witness_canister_common::limits::MAX_CHUNK_BYTES;

pub type KeyBytes = [u8; 16];
pub type RecordBytes = [u8; 10];
pub type CaseBytes = [u8; 16];
pub type PathKey = [u8; MAX_LABEL + 1];
pub type QuotaKey = [u8; 21];

const NO_OWNER: u8 = u8::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub path: BlobPath,
    pub size: u64,
    pub sha256: Hash,
    pub key_id: KeyBytes,
    pub uploader: Principal,
    pub owner: Option<Principal>,
    pub created_ns: u64,
    pub expires_ns: u64,
    pub committed: bool,
    pub received: Vec<u8>,
    pub pages: Vec<u32>,
    pub touched_ns: u64,
}

impl Meta {
    pub fn held_for(&self, caller: &Principal, now_ns: u64, stale_ns: u64) -> Option<u64> {
        let free_at = self.touched_ns.saturating_add(stale_ns);
        (self.uploader != *caller && free_at > now_ns).then(|| free_at - now_ns)
    }

    pub fn chunk_count(&self) -> u32 {
        self.size.div_ceil(MAX_CHUNK_BYTES as u64) as u32
    }

    pub fn chunk_len(&self, ix: u32) -> Option<usize> {
        let start = ix as u64 * MAX_CHUNK_BYTES as u64;
        (ix < self.chunk_count()).then(|| (self.size - start).min(MAX_CHUNK_BYTES as u64) as usize)
    }

    pub fn has(&self, ix: u32) -> bool {
        self.committed
            || self
                .received
                .get((ix / 8) as usize)
                .is_some_and(|b| b & (1 << (ix % 8)) != 0)
    }

    pub fn mark(&mut self, ix: u32) {
        let n = self.chunk_count().div_ceil(8) as usize;
        if self.received.len() < n {
            self.received.resize(n, 0);
        }
        self.received[(ix / 8) as usize] |= 1 << (ix % 8);
    }

    pub fn missing(&self) -> Vec<u32> {
        (0..self.chunk_count()).filter(|i| !self.has(*i)).collect()
    }
}

struct Cursor<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> &'a [u8] {
        let s = &self.b[self.at..self.at + n];
        self.at += n;
        s
    }

    fn array<const N: usize>(&mut self) -> [u8; N] {
        let mut a = [0u8; N];
        a.copy_from_slice(self.take(N));
        a
    }

    fn remaining(&self) -> usize {
        self.b.len() - self.at
    }

    fn u8(&mut self) -> u8 {
        self.take(1)[0]
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.array())
    }

    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.array())
    }

    fn principal(&mut self) -> Option<Principal> {
        match self.u8() {
            NO_OWNER => None,
            n => Some(Principal::from_slice(self.take(n as usize))),
        }
    }
}

fn put_principal(out: &mut Vec<u8>, p: Option<&Principal>) {
    match p {
        Some(p) => {
            out.push(p.as_slice().len() as u8);
            out.extend_from_slice(p.as_slice());
        }
        None => out.push(NO_OWNER),
    }
}

impl Storable for Meta {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.clone().into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(168 + self.received.len() + 4 * self.pages.len());
        out.extend_from_slice(&self.path.key());
        out.extend_from_slice(&self.size.to_le_bytes());
        out.extend_from_slice(&self.sha256);
        out.extend_from_slice(&self.key_id);
        put_principal(&mut out, Some(&self.uploader));
        put_principal(&mut out, self.owner.as_ref());
        out.extend_from_slice(&self.created_ns.to_le_bytes());
        out.extend_from_slice(&self.expires_ns.to_le_bytes());
        out.push(self.committed as u8);
        out.extend_from_slice(&(self.received.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.received);
        out.extend_from_slice(&(self.pages.len() as u32).to_le_bytes());
        for p in &self.pages {
            out.extend_from_slice(&p.to_le_bytes());
        }
        out.extend_from_slice(&self.touched_ns.to_le_bytes());
        out
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let mut c = Cursor { b: &bytes, at: 0 };
        let path = BlobPath::from_key(&c.array()).unwrap_or_else(|| ic_cdk::trap("meta corrupt"));
        let size = c.u64();
        let sha256 = c.array();
        let key_id = c.array();
        let uploader = c.principal().unwrap_or_else(Principal::anonymous);
        let owner = c.principal();
        let created_ns = c.u64();
        let expires_ns = c.u64();
        let committed = c.u8() != 0;
        let n = c.u32() as usize;
        let received = c.take(n).to_vec();
        let n = c.u32() as usize;
        let pages = (0..n).map(|_| c.u32()).collect();
        let touched_ns = if c.remaining() >= 8 {
            c.u64()
        } else {
            created_ns
        };
        Self {
            path,
            size,
            sha256,
            key_id,
            uploader,
            owner,
            created_ns,
            expires_ns,
            committed,
            received,
            pages,
            touched_ns,
        }
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CaseMeta {
    pub key_id: KeyBytes,
    pub count: u32,
    pub reserved: u64,
    pub live: u32,
    pub expires_ns: u64,
}

impl Storable for CaseMeta {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(40);
        out.extend_from_slice(&self.key_id);
        out.extend_from_slice(&self.count.to_le_bytes());
        out.extend_from_slice(&self.reserved.to_le_bytes());
        out.extend_from_slice(&self.live.to_le_bytes());
        out.extend_from_slice(&self.expires_ns.to_le_bytes());
        out
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let mut c = Cursor { b: &bytes, at: 0 };
        Self {
            key_id: c.array(),
            count: c.u32(),
            reserved: c.u64(),
            live: c.u32(),
            expires_ns: c.u64(),
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 40,
        is_fixed_size: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    pub used: u64,
    pub expires_ns: u64,
}

impl Storable for Usage {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16);
        out.extend_from_slice(&self.used.to_le_bytes());
        out.extend_from_slice(&self.expires_ns.to_le_bytes());
        out
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let mut c = Cursor { b: &bytes, at: 0 };
        Self {
            used: c.u64(),
            expires_ns: c.u64(),
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 16,
        is_fixed_size: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    pub next_id: u64,
    pub check_window: u64,
    pub check_used: u32,
}

impl Storable for State {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(20);
        out.extend_from_slice(&self.next_id.to_le_bytes());
        out.extend_from_slice(&self.check_window.to_le_bytes());
        out.extend_from_slice(&self.check_used.to_le_bytes());
        out
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let mut c = Cursor { b: &bytes, at: 0 };
        Self {
            next_id: c.u64(),
            check_window: c.u64(),
            check_used: c.u32(),
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 20,
        is_fixed_size: true,
    };
}

pub fn quota_key(day: u32, kind_code: u8, key_id: &KeyBytes) -> QuotaKey {
    let mut k = [0u8; 21];
    k[..4].copy_from_slice(&day.to_be_bytes());
    k[4] = kind_code;
    k[5..].copy_from_slice(key_id);
    k
}

pub fn quota_day(k: &QuotaKey) -> u32 {
    u32::from_be_bytes([k[0], k[1], k[2], k[3]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Kind;
    use witness_format::{CaseId, RecordId};

    fn meta() -> Meta {
        Meta {
            path: BlobPath::case(&CaseId::new(1, [2; 15]), 3),
            size: 3 * MAX_CHUNK_BYTES as u64 + 5,
            sha256: [9; 32],
            key_id: [4; 16],
            uploader: Principal::from_slice(&[1; 29]),
            owner: Some(Principal::from_slice(&[2; 10])),
            created_ns: 11,
            expires_ns: 12,
            committed: false,
            received: vec![],
            pages: vec![1, 5, 3, u32::MAX - 1],
            touched_ns: 13,
        }
    }

    fn rt<T: Storable + PartialEq + core::fmt::Debug + Clone>(v: &T) {
        let b = v.to_bytes().into_owned();
        assert_eq!(v.clone().into_bytes(), b);
        assert_eq!(&T::from_bytes(Cow::Owned(b)), v);
    }

    #[test]
    fn meta_round_trips() {
        let mut m = meta();
        rt(&m);
        m.owner = None;
        m.uploader = Principal::anonymous();
        m.committed = true;
        m.mark(2);
        m.path = BlobPath::record(Kind::Full, &RecordId::from_bytes([7; 10]));
        m.pages.clear();
        rt(&m);
        m.uploader = Principal::management_canister();
        rt(&m);
    }

    #[test]
    fn meta_written_before_touched_ns_decodes_with_created_time() {
        let m = meta();
        let mut old = m.to_bytes().into_owned();
        old.truncate(old.len() - 8);
        let decoded = Meta::from_bytes(Cow::Owned(old));
        assert_eq!(decoded.touched_ns, m.created_ns);
        assert_eq!(
            decoded,
            Meta {
                touched_ns: m.created_ns,
                ..m
            }
        );
    }

    #[test]
    fn upload_is_held_for_others_until_stale() {
        let m = meta();
        let owner = m.uploader;
        let other = Principal::from_slice(&[9; 29]);
        assert_eq!(m.held_for(&owner, 13, 100), None);
        assert_eq!(m.held_for(&other, 13, 100), Some(100));
        assert_eq!(m.held_for(&other, 112, 100), Some(1));
        assert_eq!(m.held_for(&other, 113, 100), None);
        assert_eq!(m.held_for(&other, u64::MAX, 100), None);
        let m = Meta {
            touched_ns: u64::MAX - 1,
            ..m
        };
        assert_eq!(m.held_for(&other, 0, 100), Some(u64::MAX));
        assert_eq!(
            m.held_for(&Principal::anonymous(), 0, 0),
            Some(u64::MAX - 1)
        );
    }

    #[test]
    fn chunk_accounting() {
        let mut m = meta();
        assert_eq!(m.chunk_count(), 4);
        assert_eq!(m.chunk_len(0), Some(MAX_CHUNK_BYTES));
        assert_eq!(m.chunk_len(3), Some(5));
        assert_eq!(m.chunk_len(4), None);
        assert_eq!(m.chunk_len(u32::MAX), None);
        assert_eq!(m.missing(), vec![0, 1, 2, 3]);
        m.mark(1);
        m.mark(1);
        m.mark(3);
        assert_eq!(m.missing(), vec![0, 2]);
        assert!(m.has(3) && !m.has(0) && !m.has(99));
        m.committed = true;
        assert!(m.missing().is_empty());
    }

    #[test]
    fn exact_chunk_multiple() {
        let mut m = meta();
        m.size = 2 * MAX_CHUNK_BYTES as u64;
        assert_eq!(m.chunk_count(), 2);
        assert_eq!(m.chunk_len(1), Some(MAX_CHUNK_BYTES));
        m.size = 1;
        assert_eq!(m.chunk_count(), 1);
        assert_eq!(m.chunk_len(0), Some(1));
    }

    #[test]
    fn many_chunks_bitset() {
        let mut m = meta();
        m.size = 64 * MAX_CHUNK_BYTES as u64;
        for i in (0..64).step_by(2) {
            m.mark(i);
        }
        assert_eq!(m.received.len(), 8);
        assert_eq!(m.missing(), (1..64).step_by(2).collect::<Vec<u32>>());
        rt(&m);
    }

    #[test]
    fn fixed_records_round_trip() {
        rt(&CaseMeta {
            key_id: [1; 16],
            count: 3,
            reserved: u64::MAX,
            live: 2,
            expires_ns: 99,
        });
        rt(&CaseMeta::default());
        rt(&Usage {
            used: 5,
            expires_ns: u64::MAX,
        });
        rt(&State {
            next_id: 1,
            check_window: 2,
            check_used: 3,
        });
    }

    #[test]
    fn quota_keys_sort_by_day_first() {
        let a = quota_key(1, 9, &[0xFF; 16]);
        let b = quota_key(2, 0, &[0; 16]);
        assert!(a < b);
        assert_eq!(quota_day(&a), 1);
        assert_eq!(quota_day(&quota_key(u32::MAX, 0, &[0; 16])), u32::MAX);
        assert_ne!(quota_key(1, 0, &[1; 16]), quota_key(1, 1, &[1; 16]));
    }
}
