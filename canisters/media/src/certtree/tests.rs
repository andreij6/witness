use super::*;
use ic_certification::LookupResult;
use ic_http_certification::{
    HttpCertification, HttpCertificationPath, HttpCertificationTree, HttpCertificationTreeEntry,
};
use ic_stable_structures::VectorMemory;

const EXPR: Hash = [0xEE; 32];

fn tree() -> CertTree<VectorMemory> {
    CertTree::init(VectorMemory::default(), EXPR, None)
}

fn label(i: u32) -> Vec<u8> {
    let mut x = i.wrapping_mul(2_654_435_761) ^ 0x5bd1_e995;
    (0..16)
        .map(|_| {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            b"0123456789ABCDEFGHJKMNPQRSTVWXYZ"[(x >> 16) as usize % 32]
        })
        .collect()
}

fn resp(i: u32) -> Hash {
    let mut h = [0u8; 32];
    h[..4].copy_from_slice(&i.to_be_bytes());
    h
}

fn path_of<'a>(kind: Kind, l: &'a [u8], r: &'a Hash) -> [&'a [u8]; 7] {
    [
        EXPR_PREFIX,
        kind.segment().as_bytes(),
        l,
        EXACT,
        &EXPR,
        b"",
        r,
    ]
}

fn found_leaf(t: &HashTree, kind: Kind, l: &[u8], r: &Hash) -> bool {
    matches!(t.lookup_path(path_of(kind, l, r)), LookupResult::Found(v) if v.is_empty())
}

#[test]
fn bit_helpers() {
    assert_eq!(bit_at(&[0b1000_0000], 0), 1);
    assert_eq!(bit_at(&[0b1000_0000], 1), 0);
    assert_eq!(bit_at(&[0, 1], 15), 1);
    assert_eq!(bit_at(&[0xFF], 8), 0);
    assert_eq!(crit_bit(b"ab", b"ab"), None);
    assert_eq!(crit_bit(b"a", b"c"), Some(6));
    assert_eq!(crit_bit(b"ab", b"abc"), Some(17));
    assert_eq!(crit_bit(b"", b"\x80"), Some(0));
    assert_eq!(crit_bit(b"", b""), None);
}

#[test]
fn empty_root_matches_library() {
    let t = tree();
    assert_eq!(t.root_hash(), HttpCertificationTree::default().root_hash());
    assert!(t.witness(Kind::Preview, b"X").is_none());
}

#[test]
fn single_entry_root_matches_library() {
    for kind in Kind::ALL {
        let mut t = tree();
        let l = label(1);
        t.insert(kind, &l, resp(1));
        let mut lib = HttpCertificationTree::default();
        let url = format!(
            "/{}/{}",
            kind.segment(),
            core::str::from_utf8(&l).unwrap_or("")
        );
        let path = HttpCertificationPath::exact(url);
        let cert = HttpCertification::response_only_prehashed(EXPR, resp(1));
        lib.insert(&HttpCertificationTreeEntry::new(&path, cert));
        assert_eq!(t.root_hash(), lib.root_hash());
        let w = t.witness(kind, &l).unwrap_or_else(empty);
        assert_eq!(w.digest(), lib.root_hash());
    }
}

#[test]
fn witnesses_prove_every_entry() {
    let mut t = tree();
    let n = 200;
    for i in 0..n {
        t.insert(Kind::ALL[i as usize % 4], &label(i), resp(i));
    }
    let root = t.root_hash();
    for i in 0..n {
        let kind = Kind::ALL[i as usize % 4];
        let w = t.witness(kind, &label(i)).unwrap_or_else(empty);
        assert_eq!(w.digest(), root);
        assert!(found_leaf(&w, kind, &label(i), &resp(i)));
        assert!(!found_leaf(&w, kind, &label(i), &resp(i + 1)));
    }
}

#[test]
fn full_tree_is_sorted_so_absence_is_provable() {
    let mut t = tree();
    for i in 0..300 {
        t.insert(Kind::Preview, &label(i), resp(i));
    }
    let full = t.full();
    assert_eq!(full.digest(), t.root_hash());
    for i in 0..300 {
        assert!(found_leaf(&full, Kind::Preview, &label(i), &resp(i)));
    }
    for i in 300..400 {
        let p = [EXPR_PREFIX, b"p", &label(i)];
        assert!(matches!(full.lookup_path(p), LookupResult::Absent));
    }
}

#[test]
fn variable_length_labels_sort_like_bytes() {
    let mut t = tree();
    let labels: Vec<Vec<u8>> = [
        "AB.1", "AB.10", "AB.2", "AB.0", "AB.100", "AC.0", "A.1", "B",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    for (i, l) in labels.iter().enumerate() {
        t.insert(Kind::Case, l, resp(i as u32));
    }
    let full = t.full();
    for (i, l) in labels.iter().enumerate() {
        assert!(found_leaf(&full, Kind::Case, l, &resp(i as u32)));
    }
    for absent in ["AB.11", "AB.", "AB", "A", "AC.00", "C"] {
        let p = [EXPR_PREFIX, b"c", absent.as_bytes()];
        assert!(
            matches!(full.lookup_path(p), LookupResult::Absent),
            "{absent}"
        );
    }
}

#[test]
fn root_is_independent_of_insertion_order() {
    let mut a = tree();
    let mut b = tree();
    for i in 0..64 {
        a.insert(Kind::Og, &label(i), resp(i));
    }
    for i in (0..64).rev() {
        b.insert(Kind::Og, &label(i), resp(i));
    }
    assert_eq!(a.root_hash(), b.root_hash());
}

#[test]
fn remove_restores_previous_root() {
    let mut t = tree();
    for i in 0..20 {
        t.insert(Kind::Preview, &label(i), resp(i));
    }
    let before = t.root_hash();
    t.insert(Kind::Preview, &label(99), resp(99));
    t.insert(Kind::Full, &label(98), resp(98));
    assert_ne!(t.root_hash(), before);
    assert!(t.remove(Kind::Preview, &label(99)));
    assert!(t.remove(Kind::Full, &label(98)));
    assert_eq!(t.root_hash(), before);
}

#[test]
fn remove_everything_gives_empty_root() {
    let mut t = tree();
    for i in 0..50 {
        t.insert(Kind::ALL[i as usize % 4], &label(i), resp(i));
    }
    for i in (0..50).step_by(3).chain((0..50).filter(|i| i % 3 != 0)) {
        assert!(t.remove(Kind::ALL[i as usize % 4], &label(i)));
        assert!(!t.contains(Kind::ALL[i as usize % 4], &label(i)));
    }
    assert_eq!(t.root_hash(), tree().root_hash());
}

#[test]
fn remove_missing_is_noop() {
    let mut t = tree();
    assert!(!t.remove(Kind::Preview, b"A"));
    t.insert(Kind::Preview, b"AB", resp(1));
    let root = t.root_hash();
    assert!(!t.remove(Kind::Preview, b"AC"));
    assert!(!t.remove(Kind::Preview, b"A"));
    assert!(!t.remove(Kind::Og, b"AB"));
    assert_eq!(t.root_hash(), root);
}

#[test]
fn reinsert_replaces_response() {
    let mut t = tree();
    for i in 0..10 {
        t.insert(Kind::Preview, &label(i), resp(i));
    }
    let nodes = t.node_count();
    t.insert(Kind::Preview, &label(3), resp(77));
    assert_eq!(t.node_count(), nodes);
    let w = t.witness(Kind::Preview, &label(3)).unwrap_or_else(empty);
    assert!(found_leaf(&w, Kind::Preview, &label(3), &resp(77)));
    assert_eq!(w.digest(), t.root_hash());
}

#[test]
fn freed_nodes_are_reused() {
    let mut t = tree();
    for i in 0..10 {
        t.insert(Kind::Preview, &label(i), resp(i));
    }
    let nodes = t.node_count();
    for i in 0..5 {
        t.remove(Kind::Preview, &label(i));
    }
    for i in 100..105 {
        t.insert(Kind::Preview, &label(i), resp(i));
    }
    assert_eq!(t.node_count(), nodes);
}

#[test]
fn witness_for_absent_label_is_none() {
    let mut t = tree();
    t.insert(Kind::Preview, &label(1), resp(1));
    assert!(t.witness(Kind::Preview, &label(2)).is_none());
    assert!(t.witness(Kind::Og, &label(1)).is_none());
}

#[test]
fn state_survives_reinit() {
    let mem = VectorMemory::default();
    let mut t = CertTree::init(mem.clone(), EXPR, None);
    for i in 0..30 {
        t.insert(Kind::Case, &label(i), resp(i));
    }
    let root = t.root_hash();
    let t2 = CertTree::init(mem, EXPR, None);
    assert_eq!(t2.root_hash(), root);
    assert!(t2.contains(Kind::Case, &label(7)));
}

#[test]
fn node_encoding_round_trips() {
    let nodes = [
        Node::Header {
            roots: [1, 2, 3, NONE],
            free: 9,
        },
        Node::Leaf {
            len: 3,
            label: [b'Z'; MAX_LABEL],
            resp: [4; 32],
        },
        Node::Fork {
            bit: 77,
            left: 5,
            right: 6,
            hash: [8; 32],
        },
        Node::Free { next: NONE },
    ];
    for n in nodes {
        let b = n.into_bytes();
        assert_eq!(b.len(), NODE_BYTES);
        assert_eq!(Node::from_bytes(Cow::Owned(b)), n);
    }
}

#[test]
#[should_panic]
fn rejects_nul_labels() {
    tree().insert(Kind::Preview, b"A\0", resp(0));
}

#[test]
#[should_panic]
fn rejects_long_labels() {
    tree().insert(Kind::Preview, &[b'A'; MAX_LABEL + 1], resp(0));
}

#[test]
fn entry_hash_matches_entry_tree() {
    let l = label(5);
    assert_eq!(
        entry_tree(&l, &EXPR, &resp(5)).digest(),
        entry_hash(&l, &EXPR, &resp(5))
    );
}

const FALLBACK: Hash = [0x44; 32];

fn fallback_tree_with(n: u32) -> CertTree<VectorMemory> {
    let mut t = CertTree::init(VectorMemory::default(), EXPR, Some(FALLBACK));
    for i in 0..n {
        t.insert(Kind::ALL[i as usize % 4], &label(i), resp(i));
    }
    t
}

fn proof_paths(url: &str) -> Vec<Vec<Vec<u8>>> {
    let req: Vec<Vec<u8>> = HttpCertificationPath::exact(url)
        .to_expr_path()
        .into_iter()
        .skip(1)
        .map(String::into_bytes)
        .collect();
    let mut proofs =
        ic_http_certification::utils::more_specific_wildcards_for(&req, &[WILDCARD.to_vec()]);
    proofs.push(req);
    proofs
}

fn assert_absent(t: &CertTree<VectorMemory>, url: &str) {
    let proofs = proof_paths(url);
    let w = t.witness_paths(&proofs);
    assert_eq!(w.digest(), t.root_hash(), "{url}");
    for p in &proofs {
        let mut full = vec![EXPR_PREFIX.to_vec()];
        full.extend(p.iter().cloned());
        assert_eq!(
            w.lookup_subtree(&full),
            ic_certification::SubtreeLookupResult::Absent,
            "{url} {p:?}"
        );
    }
    let fb = [EXPR_PREFIX, WILDCARD, &EXPR, b"", &FALLBACK];
    assert!(
        matches!(w.lookup_path(fb), LookupResult::Found(v) if v.is_empty()),
        "{url}"
    );
}

#[test]
fn fallback_root_matches_library() {
    let lib_fallback = HttpCertificationTreeEntry::new(
        HttpCertificationPath::wildcard(""),
        HttpCertification::response_only_prehashed(EXPR, FALLBACK),
    );
    let t = fallback_tree_with(0);
    let mut lib = HttpCertificationTree::default();
    lib.insert(&lib_fallback);
    assert_eq!(t.root_hash(), lib.root_hash());
    for kind in Kind::ALL {
        let mut t = CertTree::init(VectorMemory::default(), EXPR, Some(FALLBACK));
        let l = label(1);
        t.insert(kind, &l, resp(1));
        let mut lib = HttpCertificationTree::default();
        lib.insert(&lib_fallback);
        let url = format!("/{}/{}", kind.segment(), core::str::from_utf8(&l).unwrap());
        lib.insert(&HttpCertificationTreeEntry::new(
            HttpCertificationPath::exact(url),
            HttpCertification::response_only_prehashed(EXPR, resp(1)),
        ));
        assert_eq!(t.root_hash(), lib.root_hash(), "{kind:?}");
    }
}

#[test]
fn found_witnesses_still_prove_entries_with_a_fallback() {
    let t = fallback_tree_with(120);
    let root = t.root_hash();
    for i in 0..120 {
        let kind = Kind::ALL[i as usize % 4];
        let w = t.witness(kind, &label(i)).unwrap();
        assert_eq!(w.digest(), root);
        assert!(found_leaf(&w, kind, &label(i), &resp(i)));
    }
}

#[test]
fn absent_paths_get_absence_proofs_and_the_fallback() {
    let t = fallback_tree_with(200);
    let mut urls: Vec<String> = (200..260)
        .map(|i| {
            let kind = Kind::ALL[i as usize % 4];
            format!(
                "/{}/{}",
                kind.segment(),
                core::str::from_utf8(&label(i)).unwrap()
            )
        })
        .collect();
    urls.extend(
        [
            "/",
            "",
            "/p",
            "/p/",
            "/p/<*>",
            "/p/0",
            "/p/ZZZZZZZZZZZZZZZZZZZZ",
            "/x/abc",
            "/x",
            "/c/ABC.1/extra",
            "/f/a/b/c",
            "/o/~",
            "/p/%00",
            "/<*>",
            "/zz",
            "/a",
        ]
        .map(String::from),
    );
    let first = format!("/p/{}", core::str::from_utf8(&label(3)).unwrap());
    urls.push(format!("{first}/"));
    urls.push(format!("{first}x"));
    urls.push(first[..first.len() - 1].to_string());
    for url in &urls {
        assert_absent(&t, url);
    }
}

#[test]
fn absence_on_small_and_empty_trees() {
    let t = fallback_tree_with(0);
    for url in ["/p/ABC", "/", "/c/X.1"] {
        assert_absent(&t, url);
    }
    for n in 1..6 {
        let t = fallback_tree_with(n);
        for i in 0..12 {
            let kind = Kind::ALL[i as usize % 4];
            let url = format!(
                "/{}/{}",
                kind.segment(),
                core::str::from_utf8(&label(100 + i)).unwrap()
            );
            assert_absent(&t, &url);
        }
    }
}

#[test]
fn absence_proofs_bracket_every_gap() {
    let mut t = CertTree::init(VectorMemory::default(), EXPR, Some(FALLBACK));
    for l in ["B", "BB", "D", "DA", "F"] {
        t.insert(Kind::Preview, l.as_bytes(), resp(l.len() as u32));
    }
    for gap in ["A", "B0", "BA", "BC", "C", "D0", "DB", "E", "G", "Z", "BBB"] {
        assert_absent(&t, &format!("/p/{gap}"));
    }
}

#[test]
fn witness_paths_reveal_little() {
    let t = fallback_tree_with(2000);
    let url = "/p/0000000000000000";
    let w = t.witness_paths(&proof_paths(url));
    let bytes = serde_cbor_len(&w);
    assert!(bytes < 6 * 1024, "{bytes}");
}

fn serde_cbor_len(t: &HashTree) -> usize {
    ic_http_certification::utils::cbor_encode_to_base64(t).len()
}
