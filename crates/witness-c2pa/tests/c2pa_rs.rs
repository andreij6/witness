#![cfg(not(target_arch = "wasm32"))]

#[path = "../src/testkit.rs"]
mod testkit;

use c2pa::{Builder, CallbackSigner, Context, Reader, SigningAlg, ValidationState};
use p256::ecdsa::signature::Signer as _;
use std::io::Cursor;
use testkit::*;
use witness_c2pa::assertions::{DATA_HASH, DataHash, WITNESS_CLAIM};
use witness_c2pa::manifest::{active_manifest, parse_manifest};
use witness_c2pa::{Credentials, app11, verify_jpeg};

const NO_TRUST: &str = r#"{"verify": {"verify_trust": false}}"#;

fn real_jpeg(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let (w, h) = (64u16, 48u16);
    let pixels = rng.bytes(usize::from(w) * usize::from(h) * 3);
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, 85)
        .encode(&pixels, w, h, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    out
}

fn base64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in data.chunks(3) {
        let n = c.iter().fold(0u32, |acc, &b| (acc << 8) | u32::from(b)) << (8 * (3 - c.len()));
        for i in 0..4 {
            if i <= c.len() {
                s.push(char::from(A[(n >> (18 - 6 * i)) as usize & 63]));
            } else {
                s.push('=');
            }
        }
    }
    s
}

fn pem(ders: &[Vec<u8>]) -> String {
    ders.iter()
        .map(|d| {
            let b = base64(d);
            let lines: Vec<&str> = b
                .as_bytes()
                .chunks(64)
                .map(|l| std::str::from_utf8(l).unwrap())
                .collect();
            format!(
                "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
                lines.join("\n")
            )
        })
        .collect()
}

fn read(bytes: &[u8], settings: &str) -> Reader {
    Reader::from_context(Context::new().with_settings(settings).unwrap())
        .with_stream("image/jpeg", Cursor::new(bytes.to_vec()))
        .unwrap()
}

fn failures(r: &Reader) -> Vec<String> {
    r.validation_results()
        .and_then(|v| v.active_manifest())
        .map(|m| m.failure().iter().map(|s| s.code().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn base64_matches_reference() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn c2pa_rs_accepts_our_manifest() {
    let j = real_jpeg(1);
    let s = sealed(&j);
    let r = read(&s.jpeg, NO_TRUST);
    assert_eq!(failures(&r), Vec::<String>::new(), "{}", r.json());
    assert_ne!(
        r.validation_state(),
        ValidationState::Invalid,
        "{}",
        r.json()
    );
    let m = r.active_manifest().expect("active manifest");
    assert_eq!(r.active_label(), Some(s.manifest_label.as_str()));
    assert!(
        m.assertions().iter().any(|a| a.label() == WITNESS_CLAIM),
        "{}",
        r.json()
    );
}

#[test]
fn c2pa_rs_trusts_our_chain_with_witness_root_anchor() {
    let s = sealed(&real_jpeg(2));
    let settings = serde_json_settings(&pem(&[root_der()]));
    let r = read(&s.jpeg, &settings);
    assert_eq!(failures(&r), Vec::<String>::new(), "{}", r.json());
    assert_eq!(
        r.validation_state(),
        ValidationState::Trusted,
        "{}",
        r.json()
    );
}

fn serde_json_settings(anchors: &str) -> String {
    let escaped = anchors.replace('\n', "\\n");
    format!(r#"{{"trust": {{"user_anchors": "{escaped}"}}, "verify": {{"verify_trust": true}}}}"#)
}

#[test]
fn c2pa_rs_detects_tampered_pixels_in_our_manifest() {
    let s = sealed(&real_jpeg(3));
    let mut t = s.jpeg.clone();
    let at = t.len() - 20;
    t[at] = if t[at] == 0x10 { 0x11 } else { 0x10 };
    let r = read(&t, NO_TRUST);
    assert!(
        failures(&r).iter().any(|c| c.contains("dataHash")),
        "{}",
        r.json()
    );
    assert!(
        !verify_jpeg(&t, &anchor(), Some(NOW))
            .unwrap()
            .binding_intact()
    );
}

#[allow(clippy::result_large_err)]
fn callback_signer(der: bool) -> CallbackSigner {
    let key = device_key();
    CallbackSigner::new(
        move |_, data: &[u8]| {
            let sig: p256::ecdsa::Signature = key.sign(data);
            Ok(if der {
                sig.to_der().as_bytes().to_vec()
            } else {
                sig.to_bytes().to_vec()
            })
        },
        SigningAlg::Es256,
        pem(&[leaf_der(), root_der()]),
    )
}

fn c2pa_rs_sign(jpeg: &[u8], with_witness: Option<&[u8]>) -> Vec<u8> {
    let definition = r#"{
        "claim_generator_info": [{"name": "c2pa-rs interop", "version": "1"}],
        "assertions": [{"label": "c2pa.actions.v2", "data": {"actions": [
            {"action": "c2pa.created", "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture"}
        ]}}]
    }"#;
    let mut b = Builder::from_context(Context::new().with_settings(NO_TRUST).unwrap())
        .with_definition(definition)
        .unwrap();
    if let Some(w) = with_witness {
        let v: c2pa_cbor::Value = c2pa_cbor::from_slice(w).unwrap();
        b.add_assertion(WITNESS_CLAIM, &v).unwrap();
        if let Some(a) = b.definition.assertions.last_mut() {
            a.created = true;
        }
    }
    let signer = callback_signer(true);
    let mut dest = Cursor::new(Vec::new());
    b.sign(
        &signer,
        "image/jpeg",
        &mut Cursor::new(jpeg.to_vec()),
        &mut dest,
    )
    .unwrap();
    dest.into_inner()
}

fn c2pa_rs_hard_binding(signed: &[u8]) -> [u8; 32] {
    let layout = app11::locate(signed).unwrap();
    let store = layout.store.unwrap();
    let m = parse_manifest(&active_manifest(&store).unwrap()).unwrap();
    let dh = DataHash::parse(m.assertion(DATA_HASH).unwrap().cbor().unwrap()).unwrap();
    dh.hash.try_into().unwrap()
}

#[test]
fn our_data_hash_equals_c2pa_rs_hard_binding() {
    for seed in 10..14 {
        let j = real_jpeg(seed);
        let signed = c2pa_rs_sign(&j, None);
        assert_eq!(
            witness_c2pa::data_hash(&signed).unwrap(),
            c2pa_rs_hard_binding(&signed)
        );
        let r = read(&signed, NO_TRUST);
        assert_eq!(failures(&r), Vec::<String>::new(), "{}", r.json());
    }
}

#[test]
fn c2pa_rs_manifest_without_witness_claim_is_other_credentials() {
    let j = real_jpeg(20);
    let signed = c2pa_rs_sign(&j, None);
    let r = verify_jpeg(&signed, &anchor(), Some(NOW)).unwrap();
    assert_eq!(r.credentials, Credentials::Other);
    assert_eq!(r.data_hash, witness_c2pa::data_hash(&j).unwrap());
}

#[test]
fn c2pa_rs_signed_witness_claim_verifies_in_c2pa_lite() {
    let j = real_jpeg(21);
    let claim = claim_cbor_for(&j);
    let roundtrip =
        c2pa_cbor::to_vec(&c2pa_cbor::from_slice::<c2pa_cbor::Value>(&claim).unwrap()).unwrap();
    assert_eq!(roundtrip, claim);
    let signed = c2pa_rs_sign(&j, Some(&claim));
    let r = verify_jpeg(&signed, &anchor(), Some(NOW)).unwrap();
    let w = r.witness().expect("witness credentials");
    assert_eq!(w.witness_claim_cbor, claim);
    assert!(r.binding_intact());
}

#[test]
fn gathered_witness_claim_is_not_attested() {
    let j = real_jpeg(22);
    let claim = claim_cbor_for(&j);
    let mut b = Builder::from_context(Context::new().with_settings(NO_TRUST).unwrap());
    let v: c2pa_cbor::Value = c2pa_cbor::from_slice(&claim).unwrap();
    b.add_assertion(WITNESS_CLAIM, &v).unwrap();
    b.add_assertion("c2pa.actions.v2", &serde_json_actions())
        .unwrap();
    let signer = callback_signer(false);
    let mut dest = Cursor::new(Vec::new());
    b.sign(
        &signer,
        "image/jpeg",
        &mut Cursor::new(j.clone()),
        &mut dest,
    )
    .unwrap();
    assert_eq!(
        verify_jpeg(&dest.into_inner(), &anchor(), Some(NOW)),
        Err(witness_c2pa::Error::UnresolvedAssertion)
    );
}

fn serde_json_actions() -> c2pa_cbor::Value {
    let mut created = std::collections::BTreeMap::new();
    created.insert(
        c2pa_cbor::Value::Text("action".into()),
        c2pa_cbor::Value::Text("c2pa.created".into()),
    );
    created.insert(
        c2pa_cbor::Value::Text("digitalSourceType".into()),
        c2pa_cbor::Value::Text(witness_c2pa::assertions::DIGITAL_CAPTURE.into()),
    );
    let mut root = std::collections::BTreeMap::new();
    root.insert(
        c2pa_cbor::Value::Text("actions".into()),
        c2pa_cbor::Value::Array(vec![c2pa_cbor::Value::Map(created)]),
    );
    c2pa_cbor::Value::Map(root)
}
