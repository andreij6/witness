use ic_stable_structures::{Memory, StableBTreeMap, Storable};
use std::ops::RangeInclusive;

pub struct Multimap<K, M>
where
    K: Storable + Ord + Copy,
    M: Memory,
{
    map: StableBTreeMap<(K, u64), (), M>,
}

impl<K, M> Multimap<K, M>
where
    K: Storable + Ord + Copy,
    M: Memory,
{
    pub fn init(memory: M) -> Self {
        Self {
            map: StableBTreeMap::init(memory),
        }
    }

    fn span(key: K) -> RangeInclusive<(K, u64)> {
        (key, 0)..=(key, u64::MAX)
    }

    pub fn len(&self) -> u64 {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn insert(&mut self, key: K, ix: u64, cap: usize) -> bool {
        if cap == 0 {
            return false;
        }
        if self.map.contains_key(&(key, ix)) {
            return true;
        }
        let mut held = 0usize;
        let mut largest = None;
        for entry in self.map.keys_range(Self::span(key)).take(cap) {
            held += 1;
            largest = Some(entry.1);
        }
        if held == cap {
            match largest {
                Some(last) if ix > last => return false,
                Some(last) => {
                    self.map.remove(&(key, last));
                }
                None => {}
            }
        }
        self.map.insert((key, ix), ());
        true
    }

    pub fn extend_into(&self, key: K, limit: usize, out: &mut Vec<u64>) {
        out.extend(
            self.map
                .keys_range(Self::span(key))
                .take(limit)
                .map(|(_, ix)| ix),
        );
    }

    pub fn get(&self, key: K, limit: usize) -> Vec<u64> {
        let mut out = Vec::new();
        self.extend_into(key, limit, &mut out);
        out
    }

    pub fn remove(&mut self, key: K, ix: u64) -> bool {
        self.map.remove(&(key, ix)).is_some()
    }

    pub fn remove_all(&mut self, key: K) -> usize {
        let doomed: Vec<(K, u64)> = self.map.keys_range(Self::span(key)).collect();
        for k in &doomed {
            self.map.remove(k);
        }
        doomed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{Mem, mem};

    fn ids() -> Multimap<u64, Mem> {
        Multimap::init(mem())
    }

    #[test]
    fn insert_and_get_earliest_first() {
        let mut m = ids();
        for ix in [9, 3, 7, 1] {
            assert!(m.insert(5, ix, 32));
        }
        assert_eq!(m.get(5, 32), [1, 3, 7, 9]);
        assert_eq!(m.get(5, 2), [1, 3]);
        assert!(m.get(4, 32).is_empty());
        assert!(m.get(6, 32).is_empty());
    }

    #[test]
    fn keys_at_extremes_are_isolated() {
        let mut m = ids();
        m.insert(0, u64::MAX, 4);
        m.insert(u64::MAX, 0, 4);
        m.insert(1, 0, 4);
        assert_eq!(m.get(0, 4), [u64::MAX]);
        assert_eq!(m.get(u64::MAX, 4), [0]);
        assert_eq!(m.get(1, 4), [0]);
    }

    #[test]
    fn duplicate_insert_is_idempotent() {
        let mut m = ids();
        assert!(m.insert(1, 4, 2));
        assert!(m.insert(1, 4, 2));
        assert_eq!(m.get(1, 8), [4]);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn cap_keeps_earliest_and_rejects_later_flood() {
        let mut m = ids();
        for ix in 100..132 {
            assert!(m.insert(7, ix, 32));
        }
        for ix in 1000..1100 {
            assert!(!m.insert(7, ix, 32));
        }
        assert_eq!(m.get(7, 64), (100..132).collect::<Vec<_>>());
        assert!(m.insert(7, 5, 32));
        let got = m.get(7, 64);
        assert_eq!(got.len(), 32);
        assert_eq!(got[0], 5);
        assert_eq!(*got.last().unwrap(), 130);
    }

    #[test]
    fn cap_zero_stores_nothing() {
        let mut m = ids();
        assert!(!m.insert(1, 1, 0));
        assert!(m.is_empty());
    }

    #[test]
    fn existing_member_is_kept_when_full() {
        let mut m = ids();
        for ix in 0..8 {
            m.insert(2, ix, 8);
        }
        assert!(m.insert(2, 7, 8));
        assert_eq!(m.get(2, 8), (0..8).collect::<Vec<_>>());
    }

    #[test]
    fn remove_and_remove_all() {
        let mut m = ids();
        for ix in 0..5 {
            m.insert(3, ix, usize::MAX);
            m.insert(4, ix, usize::MAX);
        }
        assert!(m.remove(3, 2));
        assert!(!m.remove(3, 2));
        assert_eq!(m.get(3, 10), [0, 1, 3, 4]);
        assert_eq!(m.remove_all(3), 4);
        assert_eq!(m.remove_all(3), 0);
        assert!(m.get(3, 10).is_empty());
        assert_eq!(m.get(4, 10), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn byte_array_keys_order_and_isolate() {
        let mut m: Multimap<[u8; 16], Mem> = Multimap::init(mem());
        let a = [1u8; 16];
        let mut b = [1u8; 16];
        b[15] = 2;
        m.insert(b, 9, 8);
        m.insert(a, 4, 8);
        m.insert(a, 2, 8);
        assert_eq!(m.get(a, 8), [2, 4]);
        assert_eq!(m.get(b, 8), [9]);
        let mut out = vec![77];
        m.extend_into(b, 8, &mut out);
        assert_eq!(out, [77, 9]);
    }

    #[test]
    fn deterministic_under_insert_order() {
        let mut a = ids();
        let mut b = ids();
        let xs = [5u64, 1, 9, 3, 7, 2, 8];
        for &x in &xs {
            a.insert(1, x, 4);
        }
        for &x in xs.iter().rev() {
            b.insert(1, x, 4);
        }
        assert_eq!(a.get(1, 8), [1, 2, 3, 5]);
        assert_eq!(a.get(1, 8), b.get(1, 8));
    }
}
