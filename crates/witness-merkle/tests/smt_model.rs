mod common;

use common::Rng;
use std::collections::BTreeMap;
use witness_format::KeyId;
use witness_merkle::hash::{smt_leaf, smt_node};
use witness_merkle::smt::empty;
use witness_merkle::{DEPTH, Hash, Smt, SmtProof, ZERO, key_of};

fn empties() -> Vec<Hash> {
    let mut v = vec![ZERO];
    for _ in 0..DEPTH {
        let e = v[v.len() - 1];
        v.push(smt_node(&e, &e));
    }
    v
}

fn reference(entries: &[(u128, Hash)], height: u32) -> Hash {
    reference_with(&empties(), entries, height)
}

fn reference_with(empties: &[Hash], entries: &[(u128, Hash)], height: u32) -> Hash {
    if entries.is_empty() {
        return empties[height as usize];
    }
    if height == 0 {
        return smt_leaf(entries[0].0, &entries[0].1);
    }
    let bit = 1u128 << (height - 1);
    let split = entries.partition_point(|(k, _)| k & bit == 0);
    let (l, r) = entries.split_at(split);
    smt_node(
        &reference_with(empties, l, height - 1),
        &reference_with(empties, r, height - 1),
    )
}

fn reference_root(model: &BTreeMap<u128, Hash>) -> Hash {
    let v: Vec<(u128, Hash)> = model.iter().map(|(k, h)| (*k, *h)).collect();
    reference(&v, u32::from(DEPTH))
}

#[test]
fn random_insert_update_remove_sequences_match_reference() {
    for seed in 0..8u64 {
        let mut rng = Rng::new(seed * 7919);
        let mut smt = Smt::new(BTreeMap::new());
        let mut model = BTreeMap::new();
        let clustered = rng.u128();
        let pool: Vec<u128> = (0..40)
            .map(|i| {
                if i % 2 == 0 {
                    rng.u128()
                } else {
                    clustered ^ u128::from(rng.below(64))
                }
            })
            .collect();
        for step in 0..120 {
            let key = pool[rng.below(pool.len() as u64) as usize];
            let root = match rng.below(5) {
                0 => {
                    model.remove(&key);
                    smt.remove(key)
                }
                _ => {
                    let v = rng.hash();
                    model.insert(key, v);
                    smt.insert(key, &v)
                }
            };
            if step % 10 == 0 {
                assert_eq!(root, reference_root(&model), "seed={seed} step={step}");
            }
            let probe = pool[rng.below(pool.len() as u64) as usize];
            let p = smt.prove(probe);
            assert!(p.verify(&root, probe, model.get(&probe)));
            let decoded = SmtProof::from_bytes(&p.to_bytes()).unwrap();
            assert!(decoded.verify(&root, probe, model.get(&probe)));
        }
        assert_eq!(smt.root(), reference_root(&model));
    }
}

#[test]
fn empty_table_matches_reference() {
    for h in [0u8, 1, 7, 64, 127, 128] {
        assert_eq!(*empty(h), reference(&[], u32::from(h)));
    }
}

#[test]
fn key_ids_map_into_tree() {
    let mut smt = Smt::new(BTreeMap::new());
    let ids: Vec<KeyId> = (0..20u8).map(|i| KeyId::from_spki(&[i; 91])).collect();
    for (i, id) in ids.iter().enumerate() {
        smt.insert(key_of(id), &[i as u8; 32]);
    }
    let root = smt.root();
    for (i, id) in ids.iter().enumerate() {
        assert!(
            smt.prove(key_of(id))
                .verify(&root, key_of(id), Some(&[i as u8; 32]))
        );
    }
    let absent = KeyId::from_spki(b"absent");
    assert!(
        smt.prove(key_of(&absent))
            .verify(&root, key_of(&absent), None)
    );
}

#[test]
fn proofs_are_compact_for_sparse_trees() {
    let mut rng = Rng::new(77);
    let mut smt = Smt::new(BTreeMap::new());
    for _ in 0..1000 {
        smt.insert(rng.u128(), &rng.hash());
    }
    for _ in 0..100 {
        let p = smt.prove(rng.u128());
        assert!(p.siblings.len() <= 20);
        assert!(p.to_bytes().len() <= 16 + 32 * 20);
    }
}
