use ic_certification::{HashTree, empty, fork, fork_hash, pruned};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{Memory, StableBTreeMap, StableCell, Storable};
use std::borrow::Cow;

pub const KEY_LEN: usize = 16;
pub const KEY_BITS: u16 = (KEY_LEN * 8) as u16;
pub type Key = [u8; KEY_LEN];
pub type Hash = [u8; 32];
type NodeId = [u8; KEY_LEN + 2];

const CHILD_LEN: usize = 2 + KEY_LEN + 32;
const NODE_LEN: usize = 2 * CHILD_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Child {
    crit: u16,
    rep: Key,
    hash: Hash,
}

impl Child {
    pub fn leaf(key: Key, hash: Hash) -> Self {
        Self {
            crit: KEY_BITS,
            rep: key,
            hash,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.crit >= KEY_BITS
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    fn id(&self) -> NodeId {
        node_id(self.crit, &self.rep)
    }

    fn write(&self, out: &mut [u8]) {
        out[..2].copy_from_slice(&self.crit.to_be_bytes());
        out[2..2 + KEY_LEN].copy_from_slice(&self.rep);
        out[2 + KEY_LEN..CHILD_LEN].copy_from_slice(&self.hash);
    }

    fn read(b: &[u8]) -> Self {
        let mut rep = [0u8; KEY_LEN];
        let mut hash = [0u8; 32];
        rep.copy_from_slice(&b[2..2 + KEY_LEN]);
        hash.copy_from_slice(&b[2 + KEY_LEN..CHILD_LEN]);
        Self {
            crit: u16::from_be_bytes([b[0], b[1]]),
            rep,
            hash,
        }
    }
}

pub fn bit(key: &Key, i: u16) -> usize {
    usize::from((key[usize::from(i / 8)] >> (7 - i % 8)) & 1)
}

pub fn first_diff(a: &Key, b: &Key) -> Option<u16> {
    a.iter()
        .zip(b)
        .position(|(x, y)| x != y)
        .map(|i| (i * 8) as u16 + (a[i] ^ b[i]).leading_zeros() as u16)
}

fn node_id(crit: u16, rep: &Key) -> NodeId {
    let mut id = [0u8; KEY_LEN + 2];
    id[..2].copy_from_slice(&crit.to_be_bytes());
    let full = usize::from(crit / 8).min(KEY_LEN);
    id[2..2 + full].copy_from_slice(&rep[..full]);
    let rem = crit % 8;
    if rem > 0 && full < KEY_LEN {
        id[2 + full] = rep[full] & (0xffu8 << (8 - rem));
    }
    id
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Node([Child; 2]);

impl Node {
    fn hash(&self) -> Hash {
        fork_hash(&self.0[0].hash, &self.0[1].hash)
    }
}

impl Storable for Node {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut out = vec![0u8; NODE_LEN];
        self.0[0].write(&mut out[..CHILD_LEN]);
        self.0[1].write(&mut out[CHILD_LEN..]);
        out
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self([
            Child::read(&bytes[..CHILD_LEN]),
            Child::read(&bytes[CHILD_LEN..NODE_LEN]),
        ])
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: NODE_LEN as u32,
        is_fixed_size: true,
    };
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Root(Option<Child>);

impl Storable for Root {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        match self.0 {
            None => Vec::new(),
            Some(c) => {
                let mut out = vec![0u8; CHILD_LEN];
                c.write(&mut out);
                out
            }
        }
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self((bytes.len() == CHILD_LEN).then(|| Child::read(&bytes)))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: CHILD_LEN as u32,
        is_fixed_size: false,
    };
}

struct Step {
    at: Child,
    node: Node,
    dir: usize,
}

pub struct CritBit<M: Memory> {
    nodes: StableBTreeMap<NodeId, Node, M>,
    root: StableCell<Root, M>,
}

impl<M: Memory> CritBit<M> {
    pub fn init(nodes: M, root: M) -> Self {
        Self {
            nodes: StableBTreeMap::init(nodes),
            root: StableCell::init(root, Root::default()),
        }
    }

    pub fn root_hash(&self) -> Option<Hash> {
        self.root.get().0.map(|c| c.hash)
    }

    pub fn node_count(&self) -> u64 {
        self.nodes.len()
    }

    fn node(&self, c: &Child) -> Node {
        self.nodes.get(&c.id()).expect("crit-bit node missing")
    }

    fn descend(&self, root: Child, key: &Key) -> (Vec<Step>, Child) {
        let mut path = Vec::new();
        let mut cur = root;
        while !cur.is_leaf() {
            let node = self.node(&cur);
            let dir = bit(key, cur.crit);
            let next = node.0[dir];
            path.push(Step { at: cur, node, dir });
            cur = next;
        }
        (path, cur)
    }

    fn rebuild(&mut self, path: Vec<Step>, mut child: Child) {
        for step in path.into_iter().rev() {
            let mut node = step.node;
            node.0[step.dir] = child;
            child = Child {
                crit: step.at.crit,
                rep: step.at.rep,
                hash: node.hash(),
            };
            self.nodes.insert(step.at.id(), node);
        }
        self.root.set(Root(Some(child)));
    }

    pub fn insert(&mut self, key: Key, hash: Hash) -> bool {
        let leaf = Child::leaf(key, hash);
        let Some(root) = self.root.get().0 else {
            self.root.set(Root(Some(leaf)));
            return true;
        };
        let (mut path, found) = self.descend(root, &key);
        let Some(d) = first_diff(&key, &found.rep) else {
            if found.hash != hash {
                self.rebuild(path, leaf);
            }
            return false;
        };
        let cut = path
            .iter()
            .position(|s| s.at.crit > d)
            .unwrap_or(path.len());
        let slot = path.get(cut).map_or(found, |s| s.at);
        path.truncate(cut);
        let mut kids = [slot; 2];
        kids[bit(&key, d)] = leaf;
        let node = Node(kids);
        let at = Child {
            crit: d,
            rep: key,
            hash: node.hash(),
        };
        self.nodes.insert(at.id(), node);
        self.rebuild(path, at);
        true
    }

    pub fn remove(&mut self, key: &Key) -> bool {
        let Some(root) = self.root.get().0 else {
            return false;
        };
        let (mut path, found) = self.descend(root, key);
        if found.rep != *key {
            return false;
        }
        match path.pop() {
            None => {
                self.root.set(Root(None));
            }
            Some(parent) => {
                self.nodes.remove(&parent.at.id());
                self.rebuild(path, parent.node.0[1 - parent.dir]);
            }
        }
        true
    }

    pub fn witness(
        &self,
        targets: &[(Key, bool)],
        reveal: &dyn Fn(&Key, bool) -> HashTree,
    ) -> HashTree {
        match self.root.get().0 {
            None => empty(),
            Some(root) => self.prove(&root, targets, reveal),
        }
    }

    fn prove(
        &self,
        c: &Child,
        targets: &[(Key, bool)],
        reveal: &dyn Fn(&Key, bool) -> HashTree,
    ) -> HashTree {
        if targets.is_empty() {
            return pruned(c.hash);
        }
        if c.is_leaf() {
            return reveal(&c.rep, targets.iter().any(|t| t.1));
        }
        let node = self.node(c);
        let split = targets.partition_point(|t| bit(&t.0, c.crit) == 0);
        fork(
            self.prove(&node.0[0], &targets[..split], reveal),
            self.prove(&node.0[1], &targets[split..], reveal),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_certification::{LookupResult, labeled, labeled_hash, leaf};
    use ic_stable_structures::VectorMemory;
    use std::collections::BTreeMap;

    type Tree = CritBit<VectorMemory>;

    fn tree() -> Tree {
        CritBit::init(VectorMemory::default(), VectorMemory::default())
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

        fn key(&mut self) -> Key {
            let mut k = [0u8; KEY_LEN];
            k.iter_mut().for_each(|b| *b = self.next() as u8);
            k
        }
    }

    fn h(n: u8) -> Hash {
        [n; 32]
    }

    fn reference(entries: &[(Key, Hash)]) -> Option<Hash> {
        match entries.len() {
            0 => None,
            1 => Some(entries[0].1),
            n => {
                let d = first_diff(&entries[0].0, &entries[n - 1].0).unwrap();
                let split = entries.partition_point(|e| bit(&e.0, d) == 0);
                Some(fork_hash(
                    &reference(&entries[..split]).unwrap(),
                    &reference(&entries[split..]).unwrap(),
                ))
            }
        }
    }

    fn reference_of(map: &BTreeMap<Key, Hash>) -> Option<Hash> {
        reference(&map.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>())
    }

    fn reveal_leaf(key: &Key, full: bool) -> HashTree {
        if full {
            labeled(key.to_vec(), leaf(b"v".to_vec()))
        } else {
            labeled(key.to_vec(), pruned(leaf_hash_of_v()))
        }
    }

    fn leaf_hash_of_v() -> Hash {
        leaf(b"v".to_vec()).digest()
    }

    fn leaf_value(key: &Key) -> Hash {
        labeled_hash(key, &leaf_hash_of_v())
    }

    #[test]
    fn bit_reads_msb_first() {
        let mut k = [0u8; KEY_LEN];
        k[0] = 0b1000_0001;
        k[15] = 1;
        assert_eq!(bit(&k, 0), 1);
        assert_eq!(bit(&k, 1), 0);
        assert_eq!(bit(&k, 7), 1);
        assert_eq!(bit(&k, 127), 1);
        assert_eq!(bit(&k, 126), 0);
    }

    #[test]
    fn first_diff_finds_leading_bit() {
        let a = [0u8; KEY_LEN];
        let mut b = a;
        assert_eq!(first_diff(&a, &b), None);
        b[0] = 0x80;
        assert_eq!(first_diff(&a, &b), Some(0));
        b[0] = 0;
        b[15] = 1;
        assert_eq!(first_diff(&a, &b), Some(127));
        b[3] = 0x10;
        assert_eq!(first_diff(&a, &b), Some(27));
    }

    #[test]
    fn node_id_masks_bits_past_crit() {
        let rep = [0xffu8; KEY_LEN];
        let id = node_id(12, &rep);
        assert_eq!(&id[..2], &12u16.to_be_bytes());
        assert_eq!(id[2], 0xff);
        assert_eq!(id[3], 0xf0);
        assert!(id[4..].iter().all(|b| *b == 0));
        assert_eq!(node_id(0, &rep)[2..], [0u8; KEY_LEN]);
        assert_eq!(node_id(KEY_BITS, &rep)[2..], rep);
    }

    #[test]
    fn node_id_ignores_rep_suffix() {
        let mut a = [0x5au8; KEY_LEN];
        let b = a;
        a[9] ^= 0xff;
        assert_eq!(node_id(64, &a), node_id(64, &b));
        assert_ne!(node_id(80, &a), node_id(80, &b));
    }

    #[test]
    fn node_and_root_roundtrip() {
        let mut r = Rng(1);
        let c0 = Child {
            crit: 9,
            rep: r.key(),
            hash: h(1),
        };
        let c1 = Child::leaf(r.key(), h(2));
        let node = Node([c0, c1]);
        assert_eq!(Node::from_bytes(node.to_bytes()), node);
        assert_eq!(node.to_bytes().len(), NODE_LEN);
        for root in [Root(None), Root(Some(c0)), Root(Some(c1))] {
            assert_eq!(Root::from_bytes(root.to_bytes()), root);
        }
        assert_eq!(Root::from_bytes(Cow::Borrowed(&[1, 2, 3])), Root(None));
    }

    #[test]
    fn empty_tree_has_no_root_and_empty_witness() {
        let t = tree();
        assert_eq!(t.root_hash(), None);
        assert_eq!(t.node_count(), 0);
        assert_eq!(t.witness(&[], &reveal_leaf), empty());
    }

    #[test]
    fn single_insert_makes_leaf_root() {
        let mut t = tree();
        let k = [7u8; KEY_LEN];
        assert!(t.insert(k, h(3)));
        assert_eq!(t.root_hash(), Some(h(3)));
        assert_eq!(t.node_count(), 0);
    }

    #[test]
    fn reinsert_updates_hash_without_new_nodes() {
        let mut t = tree();
        let mut r = Rng(2);
        let keys: Vec<Key> = (0..10).map(|_| r.key()).collect();
        keys.iter().for_each(|k| assert!(t.insert(*k, h(1))));
        let before = t.root_hash();
        assert!(!t.insert(keys[4], h(1)));
        assert_eq!(t.root_hash(), before);
        assert!(!t.insert(keys[4], h(9)));
        assert_ne!(t.root_hash(), before);
        assert_eq!(t.node_count(), 9);
        let mut map: BTreeMap<Key, Hash> = keys.iter().map(|k| (*k, h(1))).collect();
        map.insert(keys[4], h(9));
        assert_eq!(t.root_hash(), reference_of(&map));
    }

    #[test]
    fn remove_missing_is_noop() {
        let mut t = tree();
        assert!(!t.remove(&[0u8; KEY_LEN]));
        t.insert([1u8; KEY_LEN], h(1));
        assert!(!t.remove(&[2u8; KEY_LEN]));
        assert_eq!(t.root_hash(), Some(h(1)));
    }

    #[test]
    fn remove_last_empties_tree() {
        let mut t = tree();
        let mut r = Rng(3);
        let keys: Vec<Key> = (0..50).map(|_| r.key()).collect();
        keys.iter().for_each(|k| {
            t.insert(*k, h(2));
        });
        keys.iter().for_each(|k| assert!(t.remove(k)));
        assert_eq!(t.root_hash(), None);
        assert_eq!(t.node_count(), 0);
    }

    #[test]
    fn extreme_and_adjacent_keys() {
        let mut t = tree();
        let lo = [0u8; KEY_LEN];
        let hi = [0xffu8; KEY_LEN];
        let mut lo1 = lo;
        lo1[15] = 1;
        let mut hi1 = hi;
        hi1[15] = 0xfe;
        let mut map = BTreeMap::new();
        for (i, k) in [hi, lo, lo1, hi1].into_iter().enumerate() {
            t.insert(k, h(i as u8));
            map.insert(k, h(i as u8));
            assert_eq!(t.root_hash(), reference_of(&map));
        }
        assert_eq!(t.node_count(), 3);
        t.remove(&lo);
        map.remove(&lo);
        assert_eq!(t.root_hash(), reference_of(&map));
    }

    #[test]
    fn randomized_ops_match_reference() {
        let mut r = Rng(42);
        let pool: Vec<Key> = (0..300u32)
            .map(|i| {
                let mut k = r.key();
                if i.is_multiple_of(3) {
                    k[..12].copy_from_slice(&[0xab; 12]);
                }
                k
            })
            .collect();
        let mut t = tree();
        let mut map: BTreeMap<Key, Hash> = BTreeMap::new();
        for step in 0..4000u32 {
            let k = pool[(r.next() % pool.len() as u64) as usize];
            match r.next() % 3 {
                0 | 1 => {
                    let v = h((step % 251) as u8);
                    assert_eq!(t.insert(k, v), map.insert(k, v).is_none());
                }
                _ => assert_eq!(t.remove(&k), map.remove(&k).is_some()),
            }
            assert_eq!(t.root_hash(), reference_of(&map), "step {step}");
            assert_eq!(t.node_count(), (map.len() as u64).saturating_sub(1));
        }
    }

    #[test]
    fn insertion_order_does_not_change_root() {
        let mut r = Rng(5);
        let keys: Vec<Key> = (0..200).map(|_| r.key()).collect();
        let mut a = tree();
        let mut b = tree();
        keys.iter().for_each(|k| {
            a.insert(*k, leaf_value(k));
        });
        keys.iter().rev().for_each(|k| {
            b.insert(*k, leaf_value(k));
        });
        assert_eq!(a.root_hash(), b.root_hash());
    }

    #[test]
    fn full_witness_digest_equals_root_and_finds_every_key() {
        let mut r = Rng(6);
        let mut t = tree();
        let mut keys: Vec<Key> = (0..64).map(|_| r.key()).collect();
        keys.iter().for_each(|k| {
            t.insert(*k, leaf_value(k));
        });
        keys.sort();
        let targets: Vec<(Key, bool)> = keys.iter().map(|k| (*k, true)).collect();
        let w = t.witness(&targets, &reveal_leaf);
        assert_eq!(w.digest(), t.root_hash().unwrap());
        for k in &keys {
            assert_eq!(w.lookup_path([k.as_slice()]), LookupResult::Found(b"v"));
        }
    }

    #[test]
    fn partial_witness_proves_absence_between_neighbours() {
        let mut r = Rng(7);
        let mut t = tree();
        let mut keys: Vec<Key> = (0..500).map(|_| r.key()).collect();
        keys.iter().for_each(|k| {
            t.insert(*k, leaf_value(k));
        });
        keys.sort();
        for i in [0usize, 1, 100, 250, 498] {
            let (a, b) = (keys[i], keys[i + 1]);
            let mut mid = a;
            mid[15] = mid[15].wrapping_add(1);
            if mid == b || mid < a {
                continue;
            }
            let w = t.witness(&[(a, false), (b, false)], &reveal_leaf);
            assert_eq!(w.digest(), t.root_hash().unwrap());
            assert_eq!(w.lookup_path([mid.as_slice()]), LookupResult::Absent);
            assert_eq!(w.lookup_path([a.as_slice()]), LookupResult::Unknown);
        }
        let low = t.witness(&[(keys[0], false)], &reveal_leaf);
        assert_eq!(low.lookup_path([[0u8; 3].as_slice()]), LookupResult::Absent);
        let high = t.witness(&[(keys[499], false)], &reveal_leaf);
        assert_eq!(
            high.lookup_path([[0xffu8; 17].as_slice()]),
            LookupResult::Absent
        );
    }

    #[test]
    fn single_target_witness_prunes_everything_else() {
        let mut r = Rng(8);
        let mut t = tree();
        let mut keys: Vec<Key> = (0..1024).map(|_| r.key()).collect();
        keys.iter().for_each(|k| {
            t.insert(*k, leaf_value(k));
        });
        keys.sort();
        let w = t.witness(&[(keys[17], true)], &reveal_leaf);
        assert_eq!(w.digest(), t.root_hash().unwrap());
        assert_eq!(
            w.lookup_path([keys[17].as_slice()]),
            LookupResult::Found(b"v")
        );
        assert_eq!(w.lookup_path([keys[18].as_slice()]), LookupResult::Unknown);
        let size = serde_cbor::to_vec(&w).unwrap().len();
        assert!(size < 40 * 32, "witness too large: {size}");
    }

    #[test]
    fn witness_is_deterministic() {
        let mut r = Rng(9);
        let mut t = tree();
        let keys: Vec<Key> = (0..100).map(|_| r.key()).collect();
        keys.iter().for_each(|k| {
            t.insert(*k, leaf_value(k));
        });
        let target = [(keys[3], true)];
        let a = serde_cbor::to_vec(&t.witness(&target, &reveal_leaf)).unwrap();
        let b = serde_cbor::to_vec(&t.witness(&target, &reveal_leaf)).unwrap();
        assert_eq!(a, b);
    }
}
