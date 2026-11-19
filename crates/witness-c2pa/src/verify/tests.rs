use super::*;
use crate::app11::{MAX_CHUNK, locate};
use crate::assertions::{ACTIONS, Action, actions_cbor, cbor_assertion_box};
use crate::claim::{Generator, HashedUri, write_claim};
use crate::jumbf;
use crate::manifest::write_store;
use crate::seal::{ManifestSpec, seal_jpeg_chunked};
use crate::testkit::*;
use alloc::vec;

fn find(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len())
        .position(|w| w == needle)
        .expect("needle present")
}

fn witness(r: &Report) -> &WitnessManifest {
    r.witness().expect("witness credentials")
}

#[test]
fn sealed_jpeg_verifies_as_witness() {
    let j = jpeg(1);
    let s = sealed(&j);
    let r = verify_jpeg(&s.jpeg, &anchor(), Some(NOW)).unwrap();
    assert_eq!(r.data_hash, s.data_hash);
    assert!(r.binding_intact());
    let w = witness(&r);
    assert_eq!(w.label, s.manifest_label);
    assert_eq!(w.c2pa_claim, s.c2pa_claim);
    assert_eq!(w.cose_sign1, s.cose_sign1);
    assert_eq!(w.witness_claim_cbor, claim_cbor_for(&j));
    assert_eq!(w.declared_hash, s.data_hash);
    assert_eq!(w.seal.signer_spki, spki(&device_key()));
    assert_eq!(w.seal.cert_serial, LEAF_SERIAL.to_be_bytes()[3..].to_vec());
    assert_eq!(w.seal.claim.asset_hash, s.data_hash);
}

#[test]
fn verification_without_time_skips_validity() {
    let s = sealed(&jpeg(2));
    assert!(
        verify_jpeg(&s.jpeg, &anchor(), None)
            .unwrap()
            .binding_intact()
    );
}

#[test]
fn plain_jpeg_is_absent() {
    let j = jpeg(3);
    let r = verify_jpeg(&j, &anchor(), Some(NOW)).unwrap();
    assert_eq!(r.credentials, Credentials::Absent);
    assert_eq!(r.data_hash, crate::data_hash(&j).unwrap());
    assert!(!r.binding_intact());
    assert!(r.witness().is_none());
}

#[test]
fn tampered_pixels_break_the_binding_only() {
    let j = jpeg(4);
    let s = sealed(&j);
    let mut t = s.jpeg.clone();
    let at = t.len() - 10;
    t[at] = if t[at] == 0x10 { 0x11 } else { 0x10 };
    let r = verify_jpeg(&t, &anchor(), Some(NOW)).unwrap();
    assert!(r.witness().is_some());
    assert!(!r.binding_intact());
    assert_ne!(r.data_hash, s.data_hash);
    assert_eq!(witness(&r).declared_hash, s.data_hash);
}

#[test]
fn tampered_claim_fails_signature() {
    let s = sealed(&jpeg(5));
    let mut t = s.jpeg.clone();
    let at = find(&t, b"Witness");
    t[at] = b'w';
    assert_eq!(verify_jpeg(&t, &anchor(), Some(NOW)), Err(Error::Signature));
}

#[test]
fn tampered_signature_fails() {
    let s = sealed(&jpeg(6));
    let mut t = s.jpeg.clone();
    let at = find(&t, &s.cose_sign1) + s.cose_sign1.len() - 1;
    t[at] ^= 0x01;
    assert_eq!(verify_jpeg(&t, &anchor(), Some(NOW)), Err(Error::Signature));
}

#[test]
fn tampered_witness_assertion_fails_hash() {
    let j = jpeg(7);
    let s = sealed(&j);
    let claim = claim_cbor_for(&j);
    let mut t = s.jpeg.clone();
    let at = find(&t, &claim) + claim.len() - 1;
    t[at] ^= 0x01;
    assert_eq!(
        verify_jpeg(&t, &anchor(), Some(NOW)),
        Err(Error::AssertionHash)
    );
}

#[test]
fn tampered_data_hash_assertion_fails_hash() {
    let s = sealed(&jpeg(8));
    let mut t = s.jpeg.clone();
    let at = find(&t, &s.data_hash);
    t[at] ^= 0x01;
    assert_eq!(
        verify_jpeg(&t, &anchor(), Some(NOW)),
        Err(Error::AssertionHash)
    );
}

#[test]
fn wrong_anchor_rejects_chain() {
    let s = sealed(&jpeg(9));
    let impostor = TrustAnchor::from_der(&root_der_with(&other_root_key())).unwrap();
    assert_eq!(
        verify_jpeg(&s.jpeg, &impostor, Some(NOW)),
        Err(Error::Chain(crate::ChainError::Anchor))
    );
}

#[test]
fn expired_or_premature_time_rejects_chain() {
    let s = sealed(&jpeg(10));
    for t in [NOT_BEFORE - 1, NOT_AFTER + 1] {
        assert_eq!(
            verify_jpeg(&s.jpeg, &anchor(), Some(t)),
            Err(Error::Chain(crate::ChainError::Validity))
        );
    }
    assert!(verify_jpeg(&s.jpeg, &anchor(), Some(NOT_AFTER)).is_ok());
    assert!(verify_jpeg(&s.jpeg, &anchor(), Some(NOT_BEFORE)).is_ok());
}

fn foreign_store(label: &str, with_witness: Option<&[u8]>) -> Vec<u8> {
    let actions = cbor_assertion_box(ACTIONS, &actions_cbor(&[Action::CAPTURED]).unwrap()).unwrap();
    let mut boxes = vec![actions];
    if let Some(w) = with_witness {
        boxes.push(cbor_assertion_box(WITNESS_CLAIM, w).unwrap());
    }
    let mut claim = Vec::new();
    write_claim(
        &mut claim,
        "xmp:iid:1",
        &Generator {
            name: "Other",
            version: "1",
        },
        &claim::signature_uri(label),
        &[HashedUri {
            url: &claim::assertion_uri(label, ACTIONS),
            alg: None,
            hash: &[0; 32],
        }],
    );
    let refs: Vec<&[u8]> = boxes.iter().map(Vec::as_slice).collect();
    let mut store = Vec::new();
    write_store(&mut store, label, &refs, &claim, &[0xd2, 0x80]).unwrap();
    store
}

fn with_store(j: &[u8], store: &[u8]) -> Vec<u8> {
    let at = locate(j).unwrap().insert_at;
    let mut out = j[..at].to_vec();
    crate::app11::write_packets(&mut out, store, 1, MAX_CHUNK).unwrap();
    out.extend_from_slice(&j[at..]);
    out
}

#[test]
fn foreign_manifest_is_other_credentials() {
    let j = jpeg(11);
    let f = with_store(&j, &foreign_store("urn:c2pa:foreign", None));
    let r = verify_jpeg(&f, &anchor(), Some(NOW)).unwrap();
    assert_eq!(r.credentials, Credentials::Other);
    assert_eq!(r.data_hash, crate::data_hash(&j).unwrap());
}

#[test]
fn foreign_signature_with_witness_label_is_an_error() {
    let j = jpeg(12);
    let f = with_store(&j, &foreign_store("urn:c2pa:x", Some(&claim_cbor_for(&j))));
    assert!(verify_jpeg(&f, &anchor(), Some(NOW)).is_err());
}

#[test]
fn update_manifest_as_active_is_other() {
    let mut store = Vec::new();
    jumbf::write_superbox(&mut store, &jumbf::uuid::STORE, "c2pa", |o| {
        jumbf::write_superbox(o, &jumbf::uuid::UPDATE_MANIFEST, "urn:c2pa:u", |_| Ok(()))?;
        Ok(())
    })
    .unwrap();
    let f = with_store(&jpeg(13), &store);
    assert_eq!(
        verify_jpeg(&f, &anchor(), Some(NOW)).unwrap().credentials,
        Credentials::Other
    );
}

#[test]
fn malformed_store_is_an_error() {
    let mut store = Vec::new();
    jumbf::write_superbox(&mut store, &jumbf::uuid::STORE, "c2pa", |_| Ok(())).unwrap();
    let f = with_store(&jpeg(14), &store);
    assert_eq!(
        verify_jpeg(&f, &anchor(), Some(NOW)),
        Err(Error::MissingPart(Part::Manifest))
    );
}

#[test]
fn verify_seal_accepts_submitted_parts() {
    let j = jpeg(15);
    let s = sealed(&j);
    let claim = claim_cbor_for(&j);
    let seal = verify_seal(&claim, &s.c2pa_claim, &s.cose_sign1, &anchor(), Some(NOW)).unwrap();
    assert_eq!(seal.claim.asset_hash, s.data_hash);
    assert_eq!(seal.signer_spki, spki(&device_key()));
}

#[test]
fn verify_seal_rejects_substituted_claim() {
    let j = jpeg(16);
    let s = sealed(&j);
    let other = claim_cbor_for(&jpeg(17));
    assert_eq!(
        verify_seal(&other, &s.c2pa_claim, &s.cose_sign1, &anchor(), Some(NOW)),
        Err(Error::AssertionHash)
    );
}

#[test]
fn verify_seal_rejects_tampered_parts() {
    let j = jpeg(18);
    let s = sealed(&j);
    let claim = claim_cbor_for(&j);
    let mut c2pa = s.c2pa_claim.clone();
    let last = c2pa.len() - 1;
    c2pa[last] ^= 1;
    assert_eq!(
        verify_seal(&claim, &c2pa, &s.cose_sign1, &anchor(), Some(NOW)),
        Err(Error::Signature)
    );
    let mut cose = s.cose_sign1.clone();
    let last = cose.len() - 1;
    cose[last] ^= 1;
    assert_eq!(
        verify_seal(&claim, &s.c2pa_claim, &cose, &anchor(), Some(NOW)),
        Err(Error::Signature)
    );
    assert_eq!(
        verify_seal(&claim, &s.c2pa_claim, &[], &anchor(), Some(NOW)),
        Err(Error::MalformedCbor)
    );
}

#[test]
fn multi_segment_seal_verifies() {
    let j = jpeg(19);
    let claim = claim_cbor_for(&j);
    for chunk in [32, 33, 100, 511, 1000] {
        let s = seal_jpeg_chunked(&j, &spec(&claim), &TestSigner::new(false), &anchor(), chunk)
            .unwrap();
        let r = verify_jpeg(&s.jpeg, &anchor(), Some(NOW)).unwrap();
        assert!(r.binding_intact(), "chunk {chunk}");
        assert!(locate(&s.jpeg).unwrap().excluded.len() > 1);
    }
}

#[test]
fn single_byte_flips_are_detected_or_harmless() {
    let j = jpeg(20);
    let s = seal_jpeg_chunked(
        &j,
        &spec(&claim_cbor_for(&j)),
        &TestSigner::new(true),
        &anchor(),
        400,
    )
    .unwrap();
    let excluded = locate(&s.jpeg).unwrap().excluded;
    let original = verify_jpeg(&s.jpeg, &anchor(), Some(NOW)).unwrap();
    let w0 = witness(&original).clone();
    let mut rng = Rng::new(21);
    let mut t = s.jpeg.clone();
    let positions: Vec<usize> = excluded
        .iter()
        .flat_map(|r| r.clone())
        .chain((0..300).map(|_| rng.below(t.len())))
        .collect();
    for i in positions {
        let mask = 1u8 << rng.below(8);
        t[i] ^= mask;
        let inside = excluded.iter().any(|r| r.contains(&i));
        if let Ok(r) = verify_jpeg(&t, &anchor(), Some(NOW)) {
            if !inside {
                assert!(!r.binding_intact(), "outside byte {i} flipped yet intact");
            }
            if let Some(w) = r.witness() {
                assert_eq!(w.c2pa_claim, w0.c2pa_claim, "byte {i}");
                assert_eq!(w.witness_claim_cbor, w0.witness_claim_cbor, "byte {i}");
                assert_eq!(w.declared_hash, w0.declared_hash, "byte {i}");
                assert_eq!(w.seal, w0.seal, "byte {i}");
            }
        }
        t[i] ^= mask;
    }
}

#[test]
fn spec_without_witness_claim_label_variant() {
    let j = jpeg(22);
    let claim = claim_cbor_for(&j);
    let spec = ManifestSpec {
        id: [1; 16],
        generator: Generator {
            name: "Witness",
            version: "9",
        },
        actions: &[
            Action::CAPTURED,
            Action {
                action: "c2pa.edited",
                digital_source_type: None,
            },
        ],
        witness_claim: &claim,
    };
    let s = seal_jpeg_chunked(&j, &spec, &TestSigner::new(true), &anchor(), MAX_CHUNK).unwrap();
    assert!(
        verify_jpeg(&s.jpeg, &anchor(), Some(NOW))
            .unwrap()
            .binding_intact()
    );
}
