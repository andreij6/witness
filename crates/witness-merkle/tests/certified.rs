mod common;

use common::{Rng, record};
use std::collections::BTreeMap;
use witness_merkle::{Mmr, Smt, certified_data};

#[test]
fn certified_data_tracks_both_structures() {
    let mut rng = Rng::new(5);
    let mut mmr = Mmr::new(Vec::new()).unwrap();
    let mut smt = Smt::new(BTreeMap::new());
    let mut seen = Vec::new();
    for i in 0..50 {
        if rng.below(2) == 0 {
            mmr.append(&record(i)).unwrap();
        } else {
            smt.insert(rng.u128(), &rng.hash());
        }
        let c = certified_data(&mmr.root().unwrap(), mmr.leaf_count(), &smt.root());
        assert!(!seen.contains(&c));
        seen.push(c);
    }
}

#[test]
fn record_and_key_proofs_against_one_certificate() {
    let mut mmr = Mmr::new(Vec::new()).unwrap();
    let mut smt = Smt::new(BTreeMap::new());
    for i in 0..33 {
        mmr.append(&record(i)).unwrap();
    }
    smt.insert(9, &[9; 32]);
    let mmr_root = mmr.root().unwrap();
    let smt_root = smt.root();
    let cert = certified_data(&mmr_root, mmr.leaf_count(), &smt_root);
    let rp = mmr.prove(17).unwrap();
    let kp = smt.prove(9);
    assert!(rp.verify(&mmr_root, &record(17)));
    assert!(kp.verify(&smt_root, 9, Some(&[9; 32])));
    assert_eq!(
        certified_data(
            &rp.compute_root(&witness_merkle::hash::mmr_leaf(&record(17)))
                .unwrap(),
            rp.leaf_count,
            &kp.compute_root(9, &witness_merkle::hash::smt_leaf(9, &[9; 32]))
                .unwrap()
        ),
        cert
    );
}
