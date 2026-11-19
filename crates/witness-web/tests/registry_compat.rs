#![cfg(not(target_arch = "wasm32"))]

use candid::Principal;
use std::collections::BTreeMap;
use witness_canister_common::{CertifiedKey, CertifiedRecord, KeyInfo, ShardInfo, Shards};
use witness_core::format::{KeyHistory, Kind};
use witness_core::merkle::{Hash, Mmr, Smt, key_of};
use witness_registry::proof::{Roots, SHARDS_KEY, record_path, shards_bundle, smt_bundle};
use witness_registry::record::leaf_data as registry_leaf_data;
use witness_verdict::testkit::ENROLLED_NS;
use witness_web::proof::{
    SHARDS_DOMAIN, SHARDS_KEY as WEB_SHARDS_KEY, key_root, leaf_data, record_root, shard_map_hash,
    shards_root,
};
use witness_web::testkit::{World, jpeg_of};
use witness_web::wire::{
    CertifiedKeyWire, Dec, Hex, KeyWire, RecordWire, ShardInfoWire, ShardsWire,
};

fn record_wire(r: &CertifiedRecord) -> RecordWire {
    RecordWire {
        record_ix: Dec(r.record_ix),
        record_id: r.record_id.clone(),
        claim: Hex(r.claim.clone()),
        c2pa_claim: Hex(r.c2pa_claim.clone()),
        cose: Hex(r.cose.clone()),
        anchored_ns: Dec(r.anchored_ns),
        mmr_proof: r.mmr_proof.iter().map(|b| Hex(b.to_vec())).collect(),
        leaf_count: Dec(r.leaf_count),
        certificate: Hex(r.certificate.clone()),
        media_shard: r.media_shard,
    }
}

fn shards_wire(s: &Shards, bundle: Vec<u8>) -> ShardsWire {
    let info = |i: &ShardInfo| ShardInfoWire {
        shard: i.shard,
        canister: Hex(i.canister.as_slice().to_vec()),
        open: i.open,
    };
    ShardsWire {
        index: s.index.iter().map(info).collect(),
        media: s.media.iter().map(info).collect(),
        bundle: Hex(bundle),
    }
}

fn key_wire(k: &KeyInfo) -> KeyWire {
    serde_json::from_value(serde_json::json!({
        "keyId": hex::encode(&k.key_id),
        "pubkey": hex::encode(&k.pubkey),
        "security": format!("{:?}", k.security),
        "provisioning": format!("{:?}", k.provisioning),
        "freshness": format!("{:?}", k.freshness),
        "patch": k.patch,
        "enrolledNs": k.enrolled_ns.to_string(),
        "activeUntilNs": k.active_until_ns.to_string(),
        "history": k.history.iter().map(|c| serde_json::json!({
            "status": format!("{:?}", c.status),
            "sinceNs": c.since_ns.to_string(),
        })).collect::<Vec<_>>(),
        "chainSerials": k.chain_serials,
    }))
    .unwrap()
}

fn shards() -> Shards {
    Shards {
        index: vec![
            ShardInfo {
                shard: 0,
                canister: Principal::from_slice(&[1; 10]),
                open: false,
            },
            ShardInfo {
                shard: 1,
                canister: Principal::from_slice(&[2; 29]),
                open: true,
            },
        ],
        media: vec![ShardInfo {
            shard: 0,
            canister: Principal::from_slice(&[3; 10]),
            open: true,
        }],
    }
}

#[test]
fn constants_and_leaf_layout_match_the_registry() {
    assert_eq!(WEB_SHARDS_KEY, SHARDS_KEY);
    assert_eq!(SHARDS_DOMAIN, witness_registry::proof::SHARDS_DOMAIN);
    let mut seed = 7u64;
    for len in [0usize, 1, 31, 32, 33, 500] {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let a: Vec<u8> = (0..len).map(|i| (seed >> (i % 56)) as u8).collect();
        let b: Vec<u8> = a.iter().rev().copied().collect();
        assert_eq!(
            leaf_data(&a, &b, &a, seed),
            registry_leaf_data(&a, &b, &a, seed)
        );
    }
}

#[test]
fn unpinned_anchor_is_shared_with_android() {
    assert_eq!(
        witness_web::check::UNPINNED_ANCHOR_DER,
        witness_canister_common::UNPINNED_ANCHOR_DER
    );
}

#[test]
fn case_caps_match_the_media_contract() {
    use witness_canister_common::limits as l;
    use witness_web::case;
    assert_eq!(case::MAX_CASE_CHUNKS, l::MAX_CASE_CHUNKS);
    assert_eq!(case::MAX_PLUS_CASE_CHUNKS, l::MAX_PLUS_CASE_CHUNKS);
    assert_eq!(case::MAX_WIRE_CHUNK, l::MAX_CHUNK_BYTES);
    assert!(case::MAX_CASE_CHUNKS as u64 * case::MAX_WIRE_CHUNK as u64 <= l::MAX_CASE_BYTES);
    assert!(
        case::MAX_PLUS_CASE_CHUNKS as u64 * case::MAX_WIRE_CHUNK as u64 <= l::MAX_PLUS_CASE_BYTES
    );
}

#[test]
fn record_json_carries_the_media_shard_and_tolerates_old_payloads() {
    let w: RecordWire = serde_json::from_value(serde_json::json!({
        "recordIx": "1", "recordId": "X", "claim": "", "anchoredNs": "2", "mediaShard": 7
    }))
    .unwrap();
    assert_eq!(w.media_shard, 7);
    let old: RecordWire = serde_json::from_value(serde_json::json!({
        "recordIx": "1", "recordId": "X", "claim": "", "anchoredNs": "2"
    }))
    .unwrap();
    assert_eq!(old.media_shard, 0);
    assert_eq!(serde_json::to_value(&w).unwrap()["mediaShard"], 7);
}

#[test]
fn shard_map_hash_matches_the_registry() {
    let s = shards();
    assert_eq!(
        shard_map_hash(&shards_wire(&s, vec![])).unwrap(),
        witness_registry::proof::shard_map_hash(&s)
    );
    assert_eq!(
        shard_map_hash(&shards_wire(&Shards::default(), vec![])).unwrap(),
        witness_registry::proof::shard_map_hash(&Shards::default())
    );
}

#[test]
fn registry_built_proofs_verify_in_the_browser_crate() {
    let mut w = World::new();
    let key = w.dev.key_info(KeyHistory::new(ENROLLED_NS));
    let candid_key = KeyInfo::from(&key);
    let mut mmr = Mmr::new(Vec::<Hash>::new()).unwrap();
    let mut records = Vec::new();
    for i in 0..9u64 {
        let s = w.seal(&jpeg_of(i + 1, 90), Kind::Original);
        let r = CertifiedRecord {
            record_ix: i,
            record_id: s.record_id.to_string(),
            claim: s.submission.claim_cbor.clone(),
            c2pa_claim: s.submission.c2pa_claim_cbor.clone(),
            cose: s.submission.cose.clone(),
            anchored_ns: ENROLLED_NS + i,
            mmr_proof: vec![],
            leaf_count: 0,
            certificate: b"cert".to_vec(),
            media_shard: (i % 3) as u8,
        };
        mmr.append(&registry_leaf_data(
            &r.claim,
            &r.c2pa_claim,
            &r.cose,
            r.anchored_ns,
        ))
        .unwrap();
        records.push(r);
    }
    let s = shards();
    let mut smt = Smt::new(BTreeMap::new());
    smt.insert(key_of(&key.key_id), &key.leaf_hash());
    smt.insert(SHARDS_KEY, &witness_registry::proof::shard_map_hash(&s));
    let roots = Roots {
        mmr_root: mmr.root().unwrap(),
        leaf_count: 9,
        smt_root: smt.root(),
    };
    let expected = roots.certified();
    for mut r in records {
        r.leaf_count = 9;
        r.mmr_proof = record_path(&mmr.prove(r.record_ix).unwrap().path, &roots.smt_root);
        assert_eq!(
            witness_registry::proof::record_certified(&r),
            Some(expected)
        );
        assert_eq!(record_root(&record_wire(&r)), Ok(expected));
    }
    let ck = CertifiedKey {
        key: candid_key.clone(),
        smt_proof: smt_bundle(&smt.prove(key_of(&key.key_id)), &roots),
        certificate: vec![],
    };
    assert_eq!(witness_registry::proof::key_certified(&ck), Some(expected));
    let wire = CertifiedKeyWire {
        key: key_wire(&candid_key),
        smt_proof: Hex(ck.smt_proof.clone()),
        certificate: Hex(vec![]),
    };
    assert_eq!(wire.key, KeyWire::from_info(&key));
    assert_eq!(key_root(&wire), Ok(expected));
    let bundle = shards_bundle(b"cert", &smt.prove(SHARDS_KEY), &roots);
    let sw = shards_wire(&s, bundle);
    let (cert, root) = shards_root(&sw).unwrap();
    assert_eq!((cert, root), (&b"cert"[..], expected));
}
