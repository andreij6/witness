use super::*;
use crate::error::Error;
use crate::testkit::{
    LEDGER_CANISTER as CANISTER, LEDGER_NOW as NOW, Ledger, TestSubnet, ledger, shard_list,
};
use crate::wire::{Dec, Hex, ShardsWire};

fn with<T>(l: &Ledger, f: impl FnOnce(&Trust<'_>) -> T) -> T {
    let key = l.subnet.public_der();
    f(&Trust {
        root_key: &key,
        canister: &CANISTER,
        now_ns: NOW,
        max_age_ns: witness_proof::DEFAULT_MAX_AGE_NS,
    })
}

#[test]
fn every_record_proof_reaches_the_certified_root() {
    let l = ledger(5);
    let roots: Vec<Hash> = l.records.iter().map(|r| record_root(r).unwrap()).collect();
    assert!(roots.windows(2).all(|w| w[0] == w[1]));
    for r in &l.records {
        let v = with(&l, |t| verify_record(r, t)).unwrap();
        assert_eq!(v["recordId"], r.record_id);
        assert_eq!(v["leafCount"], "5");
        assert_eq!(v["certTimeNs"], NOW.to_string());
        assert_eq!(v["certifiedData"], hex::encode(roots[0]));
        assert_eq!(v["mediaShard"], 0);
    }
}

#[test]
fn media_shard_is_a_routing_hint_outside_the_proof() {
    let l = ledger(3);
    let r = RecordWire {
        media_shard: 9,
        ..l.records[1].clone()
    };
    assert_eq!(record_root(&r), record_root(&l.records[1]));
    let v = with(&l, |t| verify_record(&r, t)).unwrap();
    assert_eq!(v["mediaShard"], 9);
}

#[test]
fn tampered_records_are_rejected() {
    let l = ledger(4);
    let r = &l.records[2];
    let mut cases = vec![
        RecordWire {
            anchored_ns: Dec(1),
            ..r.clone()
        },
        RecordWire {
            record_ix: Dec(3),
            ..r.clone()
        },
        RecordWire {
            leaf_count: Dec(5),
            ..r.clone()
        },
        RecordWire {
            cose: Hex(vec![1]),
            ..r.clone()
        },
        RecordWire {
            c2pa_claim: Hex(vec![1]),
            ..r.clone()
        },
    ];
    let mut smt = r.clone();
    *smt.mmr_proof.last_mut().unwrap() = Hex(vec![8; 32]);
    cases.push(smt);
    let mut sibling = r.clone();
    sibling.mmr_proof[0].0[0] ^= 1;
    cases.push(sibling);
    for c in &cases {
        assert!(with(&l, |t| verify_record(c, t)).is_err());
    }
    let other_claim = RecordWire {
        record_id: l.records[1].record_id.clone(),
        ..r.clone()
    };
    assert_eq!(
        with(&l, |t| verify_record(&other_claim, t)).unwrap_err(),
        Error::Malformed("record id")
    );
    let swapped = RecordWire {
        claim: l.records[1].claim.clone(),
        record_id: l.records[1].record_id.clone(),
        ..r.clone()
    };
    assert_eq!(
        with(&l, |t| verify_record(&swapped, t)).unwrap_err(),
        Error::Proof("certified data mismatch")
    );
}

#[test]
fn malformed_record_proofs_fail_cleanly() {
    let l = ledger(2);
    let mut empty = l.records[0].clone();
    empty.mmr_proof.clear();
    assert_eq!(record_root(&empty), Err(Error::Proof("record")));
    let mut short = l.records[0].clone();
    short.mmr_proof[0].0.pop();
    assert_eq!(record_root(&short), Err(Error::Malformed("record proof")));
    let mut hostile = l.records[0].clone();
    hostile.record_ix = Dec(u64::MAX);
    hostile.leaf_count = Dec(u64::MAX);
    hostile.mmr_proof = vec![Hex(vec![0; 32]); 70];
    let _ = record_root(&hostile);
}

#[test]
fn key_proof_reaches_the_certified_root() {
    let l = ledger(3);
    let v = with(&l, |t| verify_key(&l.key, t)).unwrap();
    assert_eq!(v["keyId"], hex::encode(&l.key.key.key_id.0));
    assert_eq!(
        v["certifiedData"],
        hex::encode(record_root(&l.records[0]).unwrap())
    );
}

#[test]
fn tampered_keys_are_rejected() {
    let l = ledger(1);
    let mut patch = l.key.clone();
    patch.key.patch += 1;
    assert_eq!(
        with(&l, |t| verify_key(&patch, t)).unwrap_err(),
        Error::Proof("certified data mismatch")
    );
    let mut revoked = l.key.clone();
    revoked.key.history.clear();
    assert!(with(&l, |t| verify_key(&revoked, t)).is_err());
    let mut cut = l.key.clone();
    cut.smt_proof.0.pop();
    assert!(with(&l, |t| verify_key(&cut, t)).is_err());
    let mut tiny = l.key.clone();
    tiny.smt_proof.0.truncate(10);
    assert_eq!(key_root(&tiny), Err(Error::Proof("smt bundle")));
    let mut leaf_count = l.key.clone();
    let n = leaf_count.smt_proof.0.len();
    leaf_count.smt_proof.0[n - 1] ^= 1;
    assert!(with(&l, |t| verify_key(&leaf_count, t)).is_err());
}

#[test]
fn shard_map_proof_reaches_the_certified_root() {
    let l = ledger(2);
    let v = with(&l, |t| verify_shards(&l.shards, t)).unwrap();
    assert_eq!(
        (v["index"].clone(), v["media"].clone()),
        (2.into(), 1.into())
    );
    let mut closed = l.shards.clone();
    closed.index[0].open = false;
    assert!(with(&l, |t| verify_shards(&closed, t)).is_err());
    let mut moved = l.shards.clone();
    moved.media[0].canister = Hex(vec![9; 10]);
    assert!(with(&l, |t| verify_shards(&moved, t)).is_err());
    let mut swapped = l.shards.clone();
    core::mem::swap(&mut swapped.index, &mut swapped.media);
    assert!(with(&l, |t| verify_shards(&swapped, t)).is_err());
}

#[test]
fn malformed_shard_bundles_fail_cleanly() {
    let l = ledger(1);
    let mut short = l.shards.clone();
    short.bundle.0.truncate(3);
    assert_eq!(
        shards_root(&short).unwrap_err(),
        Error::Proof("shards bundle")
    );
    let mut long = l.shards.clone();
    long.bundle.0[..4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(shards_root(&long).is_err());
    let mut wide = l.shards.clone();
    wide.index[0].canister = Hex(vec![1; 30]);
    assert_eq!(shard_map_hash(&wide), Err(Error::Malformed("canister id")));
}

#[test]
fn shard_hash_is_deterministic_and_order_sensitive() {
    let (index, media) = shard_list();
    let s = ShardsWire {
        index,
        media,
        bundle: Hex::default(),
    };
    assert_eq!(shard_map_hash(&s), shard_map_hash(&s.clone()));
    let mut reordered = s.clone();
    reordered.index.reverse();
    assert_ne!(shard_map_hash(&s), shard_map_hash(&reordered));
    assert_ne!(shard_map_hash(&ShardsWire::default()), shard_map_hash(&s));
}

#[test]
fn wrong_trust_inputs_are_rejected() {
    let l = ledger(1);
    let other = TestSubnet::new(12).public_der();
    let t = Trust {
        root_key: &other,
        canister: &CANISTER,
        now_ns: NOW,
        max_age_ns: 0,
    };
    assert_eq!(
        verify_record(&l.records[0], &t).unwrap_err(),
        Error::Certificate("signature")
    );
    let key = l.subnet.public_der();
    let t = Trust {
        root_key: &key,
        canister: &[1; 10],
        now_ns: NOW,
        max_age_ns: 0,
    };
    assert_eq!(
        verify_record(&l.records[0], &t).unwrap_err(),
        Error::Certificate("certified data")
    );
    let t = Trust {
        root_key: &key,
        canister: &CANISTER,
        now_ns: NOW * 2,
        max_age_ns: 1,
    };
    assert_eq!(
        verify_key(&l.key, &t).unwrap_err(),
        Error::Certificate("stale")
    );
}

#[test]
fn json_entry_points_parse_and_verify() {
    let l = ledger(2);
    let rec = serde_json::to_string(&l.records[1]).unwrap();
    let key = serde_json::to_string(&l.key).unwrap();
    let shards = serde_json::to_string(&l.shards).unwrap();
    let out = with(&l, |t| verify_record_json(&rec, t)).unwrap();
    assert!(out.contains(&l.records[1].record_id));
    assert!(with(&l, |t| verify_key_json(&key, t)).is_ok());
    assert!(with(&l, |t| verify_shards_json(&shards, t)).is_ok());
    assert!(matches!(
        with(&l, |t| verify_record_json("{}", t)),
        Err(Error::Json(_))
    ));
    assert!(with(&l, |t| verify_key_json("[", t)).is_err());
    assert!(with(&l, |t| verify_shards_json("1", t)).is_err());
}
