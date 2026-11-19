use crate::paths::{Kind, MAX_LABEL};
use ic_certification::{
    HashTree, empty, fork, fork_hash, labeled, labeled_hash, leaf, leaf_hash, pruned,
};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{Memory, StableVec, Storable};
use std::borrow::Cow;

pub type Hash = [u8; 32];

const NONE: u32 = u32::MAX;
const HEADER: u64 = 0;
const NODE_BYTES: usize = 80;
pub const EXPR_PREFIX: &[u8] = b"http_expr";
pub const EXACT: &[u8] = b"<$>";
pub const WILDCARD: &[u8] = b"<*>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Node {
    Header {
        roots: [u32; 4],
        free: u32,
    },
    Leaf {
        len: u8,
        label: [u8; MAX_LABEL],
        resp: Hash,
    },
    Fork {
        bit: u32,
        left: u32,
        right: u32,
        hash: Hash,
    },
    Free {
        next: u32,
    },
}

fn put_u32(out: &mut [u8], at: usize, v: u32) {
    out[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn get_u32(b: &[u8], at: usize) -> u32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&b[at..at + 4]);
    u32::from_le_bytes(a)
}

fn get_hash(b: &[u8], at: usize) -> Hash {
    let mut h = [0u8; 32];
    h.copy_from_slice(&b[at..at + 32]);
    h
}

impl Storable for Node {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut b = vec![0u8; NODE_BYTES];
        match self {
            Self::Header { roots, free } => {
                b[0] = 0;
                for (i, r) in roots.iter().enumerate() {
                    put_u32(&mut b, 1 + 4 * i, *r);
                }
                put_u32(&mut b, 17, free);
            }
            Self::Leaf { len, label, resp } => {
                b[0] = 1;
                b[1] = len;
                b[2..2 + MAX_LABEL].copy_from_slice(&label);
                b[2 + MAX_LABEL..2 + MAX_LABEL + 32].copy_from_slice(&resp);
            }
            Self::Fork {
                bit,
                left,
                right,
                hash,
            } => {
                b[0] = 2;
                put_u32(&mut b, 1, bit);
                put_u32(&mut b, 5, left);
                put_u32(&mut b, 9, right);
                b[13..45].copy_from_slice(&hash);
            }
            Self::Free { next } => {
                b[0] = 3;
                put_u32(&mut b, 1, next);
            }
        }
        b
    }

    fn from_bytes(b: Cow<[u8]>) -> Self {
        match b[0] {
            0 => Self::Header {
                roots: [
                    get_u32(&b, 1),
                    get_u32(&b, 5),
                    get_u32(&b, 9),
                    get_u32(&b, 13),
                ],
                free: get_u32(&b, 17),
            },
            1 => {
                let mut label = [0u8; MAX_LABEL];
                label.copy_from_slice(&b[2..2 + MAX_LABEL]);
                Self::Leaf {
                    len: b[1],
                    label,
                    resp: get_hash(&b, 2 + MAX_LABEL),
                }
            }
            2 => Self::Fork {
                bit: get_u32(&b, 1),
                left: get_u32(&b, 5),
                right: get_u32(&b, 9),
                hash: get_hash(&b, 13),
            },
            _ => Self::Free {
                next: get_u32(&b, 1),
            },
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: NODE_BYTES as u32,
        is_fixed_size: true,
    };
}

pub fn bit_at(label: &[u8], bit: u32) -> u8 {
    let byte = label.get((bit / 8) as usize).copied().unwrap_or(0);
    (byte >> (7 - bit % 8)) & 1
}

pub fn crit_bit(a: &[u8], b: &[u8]) -> Option<u32> {
    (0..a.len().max(b.len())).find_map(|i| {
        let x = a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
        (x != 0).then(|| i as u32 * 8 + x.leading_zeros())
    })
}

pub fn entry_tree(label: &[u8], expr_hash: &Hash, resp: &Hash) -> HashTree {
    labeled(
        label.to_vec(),
        labeled(
            EXACT,
            labeled(
                expr_hash.to_vec(),
                labeled(b"".to_vec(), labeled(resp.to_vec(), leaf(b"".to_vec()))),
            ),
        ),
    )
}

pub fn entry_hash(label: &[u8], expr_hash: &Hash, resp: &Hash) -> Hash {
    let h = labeled_hash(resp, &leaf_hash(b""));
    let h = labeled_hash(b"", &h);
    let h = labeled_hash(expr_hash, &h);
    let h = labeled_hash(EXACT, &h);
    labeled_hash(label, &h)
}

pub fn fallback_tree(expr_hash: &Hash, resp: &Hash) -> HashTree {
    labeled(
        WILDCARD,
        labeled(
            expr_hash.to_vec(),
            labeled(b"".to_vec(), labeled(resp.to_vec(), leaf(b"".to_vec()))),
        ),
    )
}

pub struct CertTree<M: Memory> {
    nodes: StableVec<Node, M>,
    expr_hash: Hash,
    fallback: Option<Hash>,
}

struct Header {
    roots: [u32; 4],
    free: u32,
}

impl<M: Memory> CertTree<M> {
    pub fn init(mem: M, expr_hash: Hash, fallback: Option<Hash>) -> Self {
        let nodes = StableVec::init(mem);
        if nodes.is_empty() {
            nodes.push(&Node::Header {
                roots: [NONE; 4],
                free: NONE,
            });
        }
        Self {
            nodes,
            expr_hash,
            fallback,
        }
    }

    fn node(&self, id: u32) -> Node {
        self.nodes
            .get(id as u64)
            .unwrap_or_else(|| ic_cdk::trap("cert tree corrupt"))
    }

    fn header(&self) -> Header {
        match self.node(HEADER as u32) {
            Node::Header { roots, free } => Header { roots, free },
            _ => ic_cdk::trap("cert tree header corrupt"),
        }
    }

    fn save_header(&self, h: &Header) {
        self.nodes.set(
            HEADER,
            &Node::Header {
                roots: h.roots,
                free: h.free,
            },
        );
    }

    fn alloc(&self, h: &mut Header, node: Node) -> u32 {
        if h.free != NONE {
            let id = h.free;
            if let Node::Free { next } = self.node(id) {
                h.free = next;
            }
            self.nodes.set(id as u64, &node);
            id
        } else {
            self.nodes.push(&node);
            (self.nodes.len() - 1) as u32
        }
    }

    fn release(&self, h: &mut Header, id: u32) {
        self.nodes.set(id as u64, &Node::Free { next: h.free });
        h.free = id;
    }

    fn hash_of(&self, id: u32) -> Hash {
        match self.node(id) {
            Node::Fork { hash, .. } => hash,
            Node::Leaf { len, label, resp } => {
                entry_hash(&label[..len as usize], &self.expr_hash, &resp)
            }
            _ => ic_cdk::trap("cert tree node corrupt"),
        }
    }

    fn rehash(&self, id: u32) {
        if let Node::Fork {
            bit, left, right, ..
        } = self.node(id)
        {
            let hash = fork_hash(&self.hash_of(left), &self.hash_of(right));
            self.nodes.set(
                id as u64,
                &Node::Fork {
                    bit,
                    left,
                    right,
                    hash,
                },
            );
        }
    }

    fn set_child(&self, parent: u32, dir: u8, child: u32) {
        if let Node::Fork {
            bit,
            left,
            right,
            hash,
        } = self.node(parent)
        {
            let (left, right) = if dir == 0 {
                (child, right)
            } else {
                (left, child)
            };
            self.nodes.set(
                parent as u64,
                &Node::Fork {
                    bit,
                    left,
                    right,
                    hash,
                },
            );
        }
    }

    fn descend(&self, root: u32, label: &[u8], stop_bit: u32) -> (Vec<(u32, u8)>, u32) {
        let mut path = Vec::new();
        let mut cur = root;
        while let Node::Fork {
            bit, left, right, ..
        } = self.node(cur)
        {
            if bit >= stop_bit {
                break;
            }
            let dir = bit_at(label, bit);
            path.push((cur, dir));
            cur = if dir == 0 { left } else { right };
        }
        (path, cur)
    }

    fn rehash_path(&self, path: &[(u32, u8)]) {
        for (id, _) in path.iter().rev() {
            self.rehash(*id);
        }
    }

    pub fn insert(&mut self, kind: Kind, label: &[u8], resp: Hash) {
        assert!(label.len() <= MAX_LABEL && !label.contains(&0));
        let mut h = self.header();
        let mut buf = [0u8; MAX_LABEL];
        buf[..label.len()].copy_from_slice(label);
        let new_leaf = Node::Leaf {
            len: label.len() as u8,
            label: buf,
            resp,
        };
        let root = h.roots[kind.ix()];
        if root == NONE {
            h.roots[kind.ix()] = self.alloc(&mut h, new_leaf);
            self.save_header(&h);
            return;
        }
        let (path, found) = self.descend(root, label, u32::MAX);
        let Node::Leaf {
            len, label: other, ..
        } = self.node(found)
        else {
            ic_cdk::trap("cert tree corrupt")
        };
        let Some(crit) = crit_bit(label, &other[..len as usize]) else {
            self.nodes.set(found as u64, &new_leaf);
            self.rehash_path(&path);
            return;
        };
        let (path, at) = self.descend(root, label, crit);
        let leaf_id = self.alloc(&mut h, new_leaf);
        let (left, right) = if bit_at(label, crit) == 0 {
            (leaf_id, at)
        } else {
            (at, leaf_id)
        };
        let fork_id = self.alloc(
            &mut h,
            Node::Fork {
                bit: crit,
                left,
                right,
                hash: [0; 32],
            },
        );
        self.rehash(fork_id);
        match path.last() {
            Some((parent, dir)) => self.set_child(*parent, *dir, fork_id),
            None => h.roots[kind.ix()] = fork_id,
        }
        self.rehash_path(&path);
        self.save_header(&h);
    }

    pub fn remove(&mut self, kind: Kind, label: &[u8]) -> bool {
        let mut h = self.header();
        let root = h.roots[kind.ix()];
        if root == NONE {
            return false;
        }
        let (mut path, found) = self.descend(root, label, u32::MAX);
        match self.node(found) {
            Node::Leaf {
                len, label: other, ..
            } if &other[..len as usize] == label => {}
            _ => return false,
        }
        self.release(&mut h, found);
        match path.pop() {
            None => h.roots[kind.ix()] = NONE,
            Some((parent, dir)) => {
                let sibling = match self.node(parent) {
                    Node::Fork { left, right, .. } => {
                        if dir == 0 {
                            right
                        } else {
                            left
                        }
                    }
                    _ => ic_cdk::trap("cert tree corrupt"),
                };
                self.release(&mut h, parent);
                match path.last() {
                    Some((gp, gdir)) => self.set_child(*gp, *gdir, sibling),
                    None => h.roots[kind.ix()] = sibling,
                }
                self.rehash_path(&path);
            }
        }
        self.save_header(&h);
        true
    }

    pub fn contains(&self, kind: Kind, label: &[u8]) -> bool {
        let root = self.header().roots[kind.ix()];
        if root == NONE {
            return false;
        }
        let (_, found) = self.descend(root, label, u32::MAX);
        matches!(self.node(found), Node::Leaf { len, label: l, .. } if &l[..len as usize] == label)
    }

    fn kinds_tree(
        &self,
        roots: &[u32; 4],
        mut part: impl FnMut(Kind, u32) -> HashTree,
    ) -> HashTree {
        Kind::ALL
            .iter()
            .rev()
            .filter(|k| roots[k.ix()] != NONE)
            .fold(None, |acc: Option<HashTree>, k| {
                let t = part(*k, roots[k.ix()]);
                Some(match acc {
                    None => t,
                    Some(a) => fork(t, a),
                })
            })
            .unwrap_or_else(empty)
    }

    fn pruned_kind(&self, kind: Kind, root: u32) -> HashTree {
        labeled(
            kind.segment().as_bytes().to_vec(),
            pruned(self.hash_of(root)),
        )
    }

    fn with_fallback(&self, roots: &[u32; 4], kinds: HashTree) -> HashTree {
        let t = match self.fallback {
            None => kinds,
            Some(resp) if roots.iter().all(|r| *r == NONE) => fallback_tree(&self.expr_hash, &resp),
            Some(resp) => fork(fallback_tree(&self.expr_hash, &resp), kinds),
        };
        labeled(EXPR_PREFIX, t)
    }

    pub fn root_hash(&self) -> Hash {
        let h = self.header();
        let t = self.kinds_tree(&h.roots, |k, r| self.pruned_kind(k, r));
        self.with_fallback(&h.roots, t).digest()
    }

    pub fn witness(&self, kind: Kind, label: &[u8]) -> Option<HashTree> {
        self.contains(kind, label).then(|| {
            self.witness_paths(&[vec![kind.segment().as_bytes().to_vec(), label.to_vec()]])
        })
    }

    pub fn witness_paths(&self, paths: &[Vec<Vec<u8>>]) -> HashTree {
        let h = self.header();
        let t = self.kinds_tree(&h.roots, |k, root| {
            let seg = k.segment().as_bytes();
            let mut targets = Vec::new();
            for p in paths.iter().filter(|p| p.len() >= 2 && p[0] == seg) {
                self.neighbors(root, &p[1], &mut targets);
            }
            if targets.is_empty() {
                return self.pruned_kind(k, root);
            }
            let refs: Vec<&[u8]> = targets.iter().map(Vec::as_slice).collect();
            labeled(seg.to_vec(), self.reveal(root, &refs))
        });
        self.with_fallback(&h.roots, t)
    }

    fn leaf_label(&self, id: u32) -> Vec<u8> {
        match self.node(id) {
            Node::Leaf { len, label, .. } => label[..len as usize].to_vec(),
            _ => ic_cdk::trap("cert tree corrupt"),
        }
    }

    fn edge(&self, mut id: u32, dir: u8) -> u32 {
        while let Node::Fork { left, right, .. } = self.node(id) {
            id = if dir == 0 { left } else { right };
        }
        id
    }

    fn child(&self, id: u32, dir: u8) -> u32 {
        match self.node(id) {
            Node::Fork { left, right, .. } => {
                if dir == 0 {
                    left
                } else {
                    right
                }
            }
            _ => ic_cdk::trap("cert tree corrupt"),
        }
    }

    fn neighbors(&self, root: u32, label: &[u8], out: &mut Vec<Vec<u8>>) {
        if label.contains(&0) {
            return;
        }
        let (_, found) = self.descend(root, label, u32::MAX);
        let Some(crit) = crit_bit(label, &self.leaf_label(found)) else {
            out.push(label.to_vec());
            return;
        };
        let (path, at) = self.descend(root, label, crit);
        let side = bit_at(label, crit);
        let near = self.edge(at, side);
        let far = path
            .iter()
            .rev()
            .find(|(_, d)| *d != side)
            .map(|(id, _)| self.edge(self.child(*id, side), 1 - side));
        out.push(self.leaf_label(near));
        out.extend(far.map(|id| self.leaf_label(id)));
    }

    fn reveal(&self, id: u32, targets: &[&[u8]]) -> HashTree {
        match self.node(id) {
            Node::Leaf { len, label, resp } => {
                let l = &label[..len as usize];
                if targets.contains(&l) {
                    entry_tree(l, &self.expr_hash, &resp)
                } else {
                    pruned(self.hash_of(id))
                }
            }
            Node::Fork {
                bit, left, right, ..
            } => {
                let (l, r): (Vec<&[u8]>, Vec<&[u8]>) =
                    targets.iter().partition(|t| bit_at(t, bit) == 0);
                let side = |child: u32, ts: &[&[u8]]| {
                    if ts.is_empty() {
                        pruned(self.hash_of(child))
                    } else {
                        self.reveal(child, ts)
                    }
                };
                fork(side(left, &l), side(right, &r))
            }
            _ => ic_cdk::trap("cert tree node corrupt"),
        }
    }

    pub fn node_count(&self) -> u64 {
        self.nodes.len()
    }

    #[cfg(test)]
    fn full(&self) -> HashTree {
        let h = self.header();
        let t = self.kinds_tree(&h.roots, |k, r| {
            labeled(k.segment().as_bytes().to_vec(), self.full_node(r))
        });
        self.with_fallback(&h.roots, t)
    }

    #[cfg(test)]
    fn full_node(&self, id: u32) -> HashTree {
        match self.node(id) {
            Node::Leaf { len, label, resp } => {
                entry_tree(&label[..len as usize], &self.expr_hash, &resp)
            }
            Node::Fork { left, right, .. } => fork(self.full_node(left), self.full_node(right)),
            _ => empty(),
        }
    }
}

#[cfg(test)]
mod tests;
