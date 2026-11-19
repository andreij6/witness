use crate::cert;
use crate::critbit::{CritBit, Hash, KEY_LEN, Key};
use ic_certification::HashTree;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{Memory, StableBTreeMap, Storable};
use std::borrow::Cow;
use std::ops::Bound as Range;
use witness_canister_common::Removal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    Shell,
    DevShell,
    Gone(Removal),
}

impl PageKind {
    pub const fn code(self) -> u8 {
        match self {
            Self::Shell => 0,
            Self::DevShell => 1,
            Self::Gone(Removal::Erased) => 2,
            Self::Gone(Removal::Takedown) => 3,
            Self::Gone(Removal::Revoked) => 4,
        }
    }

    pub const fn from_code(c: u8) -> Option<Self> {
        match c {
            0 => Some(Self::Shell),
            1 => Some(Self::DevShell),
            2 => Some(Self::Gone(Removal::Erased)),
            3 => Some(Self::Gone(Removal::Takedown)),
            4 => Some(Self::Gone(Removal::Revoked)),
            _ => None,
        }
    }

    pub const fn is_gone(self) -> bool {
        matches!(self, Self::Gone(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub kind: PageKind,
    pub resp_hash: Hash,
    pub body: Vec<u8>,
}

impl Storable for Page {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut out = Vec::with_capacity(33 + self.body.len());
        out.push(self.kind.code());
        out.extend_from_slice(&self.resp_hash);
        out.extend_from_slice(&self.body);
        Cow::Owned(out)
    }

    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let mut resp_hash = [0u8; 32];
        resp_hash.copy_from_slice(&bytes[1..33]);
        Self {
            kind: PageKind::from_code(bytes[0]).expect("page kind"),
            resp_hash,
            body: bytes[33..].to_vec(),
        }
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookup {
    Found(Key),
    Absent(Option<Key>, Option<Key>),
}

pub struct Store<M: Memory> {
    pages: StableBTreeMap<Key, Page, M>,
    tree: CritBit<M>,
}

impl<M: Memory> Store<M> {
    pub fn init(pages: M, nodes: M, root: M) -> Self {
        Self {
            pages: StableBTreeMap::init(pages),
            tree: CritBit::init(nodes, root),
        }
    }

    pub fn len(&self) -> u64 {
        self.pages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    pub fn node_count(&self) -> u64 {
        self.tree.node_count()
    }

    pub fn get(&self, key: &Key) -> Option<Page> {
        self.pages.get(key)
    }

    pub fn put(&mut self, key: Key, page: Page) {
        self.tree
            .insert(key, cert::record_leaf_hash(&key, &page.resp_hash));
        self.pages.insert(key, page);
    }

    pub fn next_after(&self, after: Option<&Key>) -> Option<(Key, Page)> {
        let lo = after.map_or(Range::Unbounded, |k| Range::Excluded(*k));
        self.pages
            .range((lo, Range::Unbounded))
            .next()
            .map(|e| e.into_pair())
    }

    pub fn root_hash(&self) -> Option<Hash> {
        self.tree.root_hash()
    }

    pub fn lookup(&self, label: &[u8]) -> Lookup {
        let mut lo = [0u8; KEY_LEN];
        let n = label.len().min(KEY_LEN);
        lo[..n].copy_from_slice(&label[..n]);
        let exists = label.len() >= KEY_LEN && self.pages.contains_key(&lo);
        if label.len() == KEY_LEN && exists {
            return Lookup::Found(lo);
        }
        let pred = if label.len() > KEY_LEN && exists {
            Some(lo)
        } else {
            self.pages.keys_range(..lo).next_back()
        };
        let succ = if label.len() < KEY_LEN {
            self.pages.keys_range(lo..).next()
        } else {
            self.pages
                .keys_range((Range::Excluded(lo), Range::Unbounded))
                .next()
        };
        Lookup::Absent(pred, succ)
    }

    pub fn witness(&self, labels: &[&[u8]]) -> HashTree {
        let mut targets: Vec<(Key, bool)> = Vec::with_capacity(labels.len() * 2);
        for label in labels {
            match self.lookup(label) {
                Lookup::Found(k) => targets.push((k, true)),
                Lookup::Absent(pred, succ) => {
                    targets.extend(pred.into_iter().chain(succ).map(|k| (k, false)))
                }
            }
        }
        targets.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        targets.dedup_by(|a, b| a.0 == b.0);
        let reveal = |key: &Key, full: bool| {
            let page = self.pages.get(key).expect("tree key without page");
            cert::record_leaf(key, &page.resp_hash, full)
        };
        self.tree.witness(&targets, &reveal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_certification::{SubtreeLookupResult, labeled};
    use ic_stable_structures::VectorMemory;
    use std::collections::BTreeSet;

    fn store() -> Store<VectorMemory> {
        Store::init(
            VectorMemory::default(),
            VectorMemory::default(),
            VectorMemory::default(),
        )
    }

    fn page(n: u8) -> Page {
        Page {
            kind: PageKind::Shell,
            resp_hash: [n; 32],
            body: vec![n; 3],
        }
    }

    fn key(s: &str) -> Key {
        s.as_bytes().try_into().unwrap()
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        fn id(&mut self) -> Key {
            const A: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
            let mut k = [0u8; KEY_LEN];
            k.iter_mut()
                .for_each(|b| *b = A[(self.next() % 32) as usize]);
            k
        }

        fn label(&mut self) -> Vec<u8> {
            const A: &[u8] = b"0123<>*$9ABZ";
            let len = (self.next() % 20) as usize;
            (0..len)
                .map(|_| A[(self.next() % A.len() as u64) as usize])
                .collect()
        }
    }

    #[test]
    fn page_kind_codes_roundtrip() {
        for c in 0..5u8 {
            assert_eq!(PageKind::from_code(c).unwrap().code(), c);
        }
        assert_eq!(PageKind::from_code(5), None);
        assert!(PageKind::Gone(Removal::Revoked).is_gone());
        assert!(!PageKind::DevShell.is_gone());
    }

    #[test]
    fn page_storable_roundtrip() {
        for p in [
            page(1),
            Page {
                kind: PageKind::Gone(Removal::Takedown),
                resp_hash: [7; 32],
                body: Vec::new(),
            },
        ] {
            assert_eq!(Page::from_bytes(p.to_bytes()), p);
            assert_eq!(Page::from_bytes(Cow::Owned(p.clone().into_bytes())), p);
        }
    }

    #[test]
    #[should_panic(expected = "page kind")]
    fn page_rejects_unknown_kind() {
        let mut b = page(1).to_bytes().into_owned();
        b[0] = 99;
        Page::from_bytes(Cow::Owned(b));
    }

    #[test]
    fn next_after_walks_every_page_in_key_order() {
        let mut s = store();
        assert_eq!(s.next_after(None), None);
        for (i, k) in ["CCCCCCCCCCCCCCCC", "AAAAAAAAAAAAAAAA", "BBBBBBBBBBBBBBBB"]
            .iter()
            .enumerate()
        {
            s.put(key(k), page(i as u8));
        }
        let mut seen = Vec::new();
        let mut at = None;
        while let Some((k, p)) = s.next_after(at.as_ref()) {
            seen.push((k, p.resp_hash[0]));
            at = Some(k);
        }
        assert_eq!(
            seen,
            [
                (key("AAAAAAAAAAAAAAAA"), 1),
                (key("BBBBBBBBBBBBBBBB"), 2),
                (key("CCCCCCCCCCCCCCCC"), 0)
            ]
        );
    }

    #[test]
    fn put_get_and_overwrite() {
        let mut s = store();
        assert!(s.is_empty());
        s.put(key("AAAAAAAAAAAAAAAA"), page(1));
        let r1 = s.root_hash();
        s.put(key("AAAAAAAAAAAAAAAA"), page(2));
        assert_eq!(s.get(&key("AAAAAAAAAAAAAAAA")), Some(page(2)));
        assert_ne!(s.root_hash(), r1);
        assert_eq!(s.len(), 1);
        assert_eq!(s.get(&key("BBBBBBBBBBBBBBBB")), None);
    }

    #[test]
    fn lookup_handles_every_label_length() {
        let mut s = store();
        for k in ["1111111111111111", "5555555555555555", "AAAAAAAAAAAAAAAA"] {
            s.put(key(k), page(1));
        }
        assert_eq!(
            s.lookup(b"5555555555555555"),
            Lookup::Found(key("5555555555555555"))
        );
        assert_eq!(
            s.lookup(b"<*>"),
            Lookup::Absent(Some(key("5555555555555555")), Some(key("AAAAAAAAAAAAAAAA")))
        );
        assert_eq!(
            s.lookup(b""),
            Lookup::Absent(None, Some(key("1111111111111111")))
        );
        assert_eq!(
            s.lookup(b"55555555555555551"),
            Lookup::Absent(Some(key("5555555555555555")), Some(key("AAAAAAAAAAAAAAAA")))
        );
        assert_eq!(
            s.lookup(b"555555555555555"),
            Lookup::Absent(Some(key("1111111111111111")), Some(key("5555555555555555")))
        );
        assert_eq!(
            s.lookup(b"ZZZZ"),
            Lookup::Absent(Some(key("AAAAAAAAAAAAAAAA")), None)
        );
        assert_eq!(store().lookup(b"x"), Lookup::Absent(None, None));
    }

    #[test]
    fn randomized_lookup_matches_btreeset() {
        let mut r = Rng(11);
        let mut s = store();
        let mut set: BTreeSet<Vec<u8>> = BTreeSet::new();
        for _ in 0..300 {
            let k = r.id();
            s.put(k, page(1));
            set.insert(k.to_vec());
        }
        for i in 0..2000u32 {
            let label = if i.is_multiple_of(4) {
                let mut l = set.iter().nth((r.next() % 300) as usize).unwrap().clone();
                l.truncate((r.next() % 18) as usize);
                if r.next().is_multiple_of(2) {
                    l.push(b'0');
                }
                l
            } else {
                r.label()
            };
            let expected = if set.contains(&label) {
                Lookup::Found(label.as_slice().try_into().unwrap())
            } else {
                let to = |v: &Vec<u8>| -> Key { v.as_slice().try_into().unwrap() };
                Lookup::Absent(
                    set.range(..label.clone()).next_back().map(to),
                    set.range(label.clone()..).next().map(to),
                )
            };
            assert_eq!(s.lookup(&label), expected, "label {label:?}");
        }
    }

    #[test]
    fn witness_proves_found_and_absent_labels() {
        let mut r = Rng(12);
        let mut s = store();
        let ids: Vec<Key> = (0..400).map(|_| r.id()).collect();
        ids.iter().for_each(|k| s.put(*k, page(k[0])));
        let missing = r.id();
        let labels: [&[u8]; 4] = [&ids[7], &missing, b"", b"<*>"];
        let w = s.witness(&labels);
        assert_eq!(w.digest(), s.root_hash().unwrap());
        let top = labeled(b"p".to_vec(), w);
        assert!(matches!(
            top.lookup_subtree([b"p".as_slice(), &ids[7], cert::EXACT]),
            SubtreeLookupResult::Found(_)
        ));
        for absent in [&missing[..], b"", b"<*>"] {
            assert_eq!(
                top.lookup_subtree([b"p".as_slice(), absent]),
                SubtreeLookupResult::Absent,
                "{absent:?}"
            );
        }
        assert_eq!(
            top.lookup_subtree([b"p".as_slice(), &ids[7], cert::WILDCARD]),
            SubtreeLookupResult::Absent
        );
    }

    #[test]
    fn witness_on_empty_store_is_empty_and_absent() {
        let s = store();
        let w = s.witness(&[b"ABC"]);
        let top = labeled(b"p".to_vec(), w);
        assert_eq!(
            top.lookup_subtree([b"p".as_slice(), b"ABC"]),
            SubtreeLookupResult::Absent
        );
    }

    #[test]
    fn witness_with_duplicate_labels_is_stable() {
        let mut s = store();
        let k = key("7777777777777777");
        s.put(k, page(1));
        s.put(key("8888888888888888"), page(2));
        let a = s.witness(&[&k, &k, b"7"]);
        let b = s.witness(&[b"7", &k]);
        assert_eq!(a, b);
        assert_eq!(a.digest(), s.root_hash().unwrap());
    }
}
