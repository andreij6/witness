#![cfg(target_arch = "wasm32")]

use serde_json::Value;
use wasm_bindgen_test::wasm_bindgen_test;
use witness_core::format::CaseId;
use witness_core::format::crypto::sha256;
use witness_core::verdict::judge as native_judge;
use witness_web::bundle::{CaseBundle, CaseItem};
use witness_web::oracle::Need;
use witness_web::testkit::{
    LEDGER_CANISTER, LEDGER_NOW, World, answer, jpeg_of, ledger, recompress, seal_case,
};
use witness_web::wire::Fetched;
use witness_web::{
    JsCaseOpener as CaseOpener, decode_claim, judge, parse_case_bundle, query_plan, render,
    verify_certified_key, verify_certified_record, verify_shards, version,
};

fn drive(input: &[u8], w: &World) -> Value {
    let options = serde_json::to_string(&w.options()).unwrap();
    let mut step: Value = serde_json::from_str(&query_plan(input, &options).unwrap()).unwrap();
    let mut fetched = Fetched::default();
    for _ in 0..12 {
        if step["done"] == true {
            return step;
        }
        let need: Need = serde_json::from_value(step["need"].clone()).unwrap();
        answer(&need, &w.oracle, &mut fetched);
        let f = serde_json::to_string(&fetched).unwrap();
        step = serde_json::from_str(&judge(input, &options, &f).unwrap()).unwrap();
    }
    panic!("did not converge");
}

fn native(input: &[u8], w: &World) -> Value {
    let o = w.options();
    let anchor = o.anchor().unwrap();
    render::verdict(&native_judge(input, &w.oracle, &o.options(&anchor)))
}

#[wasm_bindgen_test]
fn version_is_the_crate_version() {
    assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}

#[wasm_bindgen_test]
fn json_driven_check_equals_native_judge() {
    let mut w = World::new();
    let s = w.original(1, true);
    for input in [s.jpeg.clone(), recompress(&s.jpeg, 70), jpeg_of(9, 85)] {
        let done = drive(&input, &w);
        assert_eq!(done["verdict"], native(&input, &w));
    }
    assert_eq!(drive(&s.jpeg, &w)["verdict"]["outcome"], "ORIGINAL");
}

#[wasm_bindgen_test]
fn malformed_calls_throw() {
    let w = World::new();
    let options = serde_json::to_string(&w.options()).unwrap();
    assert!(query_plan(&[1, 2, 3], "{").is_err());
    assert!(query_plan(&[1, 2, 3], "{\"anchor\":\"00\"}").is_err());
    assert!(judge(&[1, 2, 3], &options, "[").is_err());
    assert!(judge(&[1, 2, 3], &options, "{}").is_ok());
}

#[wasm_bindgen_test]
fn certified_reads_verify_through_the_exports() {
    let l = ledger(2);
    let root = l.subnet.public_der();
    let rec = serde_json::to_string(&l.records[1]).unwrap();
    let out: Value = serde_json::from_str(
        &verify_certified_record(&rec, &root, &LEDGER_CANISTER, LEDGER_NOW, 300_000_000_000)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(out["recordId"], l.records[1].record_id);
    let key = serde_json::to_string(&l.key).unwrap();
    assert!(verify_certified_key(&key, &root, &LEDGER_CANISTER, LEDGER_NOW, 0).is_ok());
    let shards = serde_json::to_string(&l.shards).unwrap();
    assert!(verify_shards(&shards, &root, &LEDGER_CANISTER, LEDGER_NOW, 0).is_ok());
    assert!(verify_certified_record(&rec, &root, &[1; 10], LEDGER_NOW, 0).is_err());
    assert!(verify_certified_record(&rec, &root[1..], &LEDGER_CANISTER, LEDGER_NOW, 0).is_err());
    assert!(verify_certified_record(&rec, &root, &LEDGER_CANISTER, LEDGER_NOW * 2, 1).is_err());
    assert!(verify_certified_key("{}", &root, &LEDGER_CANISTER, 0, 0).is_err());
    assert!(verify_shards("{}", &root, &LEDGER_CANISTER, 0, 0).is_err());
}

#[wasm_bindgen_test]
fn case_link_opens_and_parses() {
    let jpeg = jpeg_of(3, 80);
    let item = CaseItem {
        record_id: witness_core::format::RecordId::from_bytes([4; 10]),
        jpeg: &jpeg,
        submission: None,
    };
    let bundle = CaseBundle {
        title: "Case",
        summary: Some("s"),
        items: vec![item],
    }
    .to_cbor();
    let key = [0x11u8; 32];
    let case_id = CaseId::new(1, [2; 15]);
    let chunks = seal_case(&key, &case_id, &bundle, 9);
    let fragment = format!("#k={}", base64_url(&key));
    let mut opener = CaseOpener::new(
        &fragment,
        &case_id.to_string(),
        chunks.len() as u32,
        &sha256(&bundle),
    )
    .unwrap();
    assert_eq!(opener.missing().unwrap(), vec![0]);
    let mut bad = chunks[0].clone();
    bad[13] ^= 1;
    assert!(opener.add(0, &bad).is_err());
    opener.add(0, &chunks[0]).unwrap();
    let opened = opener.finish().unwrap();
    assert_eq!(opened, bundle);
    assert!(opener.finish().is_err());
    assert!(opener.add(0, &chunks[0]).is_err());
    let parsed: Value = serde_json::from_str(&parse_case_bundle(&opened).unwrap()).unwrap();
    let off = parsed["items"][0]["jpeg"]["offset"].as_u64().unwrap() as usize;
    assert_eq!(&opened[off..off + jpeg.len()], jpeg.as_slice());
    assert!(parse_case_bundle(&opened[1..]).is_err());
    assert!(CaseOpener::new("#k=", &case_id.to_string(), 1, &[0; 32]).is_err());
}

fn base64_url(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[wasm_bindgen_test]
fn decode_claim_renders_the_full_claim() {
    let mut w = World::new();
    let s = w.original(3, true);
    let cbor = s.claim.to_cbor().unwrap();
    let v: Value = serde_json::from_str(&decode_claim(&cbor).unwrap()).unwrap();
    assert_eq!(v, render::claim_detail(&s.claim));
    assert!(decode_claim(&[0xa0]).is_err());
}
