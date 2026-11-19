use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::Signer;
use witness_format::cbor::{self, from_slice};
use witness_format::crypto::{
    Es256Key, ed25519_verify, es256_normalize_low_s, es256_raw_to_der, es256_verify_low_s, sha256,
};
use witness_format::key::normalize_serial_hex;
use witness_format::*;

const SPKI_P256_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

fn device_key() -> (SigningKey, Vec<u8>) {
    let sk = SigningKey::from_slice(&[0x42; 32]).unwrap();
    let mut spki = SPKI_P256_PREFIX.to_vec();
    spki.extend_from_slice(sk.verifying_key().to_encoded_point(false).as_bytes());
    (sk, spki)
}

fn checks() -> Checks {
    Checks {
        key_security: SecurityLevel::Tee,
        boot: BootState::Verified,
        device_locked: true,
        mock_location: false,
        gnss: GnssChecks::ALL,
        recapture: 0,
    }
}

fn sealed_original(key_id: KeyId, counter: u64, opening: &LocationOpening) -> WitnessClaim {
    let bundle = FingerprintBundle::new(
        vec![
            PdqShape {
                tag: 0,
                hash: [1; 32],
            },
            PdqShape {
                tag: 4,
                hash: [2; 32],
            },
        ],
        90,
        [0x0f0f; 192],
        false,
    )
    .unwrap();
    WitnessClaim {
        capture_id: CaptureId::from_random_bytes([9, 8, 7, 6, 5, 4, 3], 0).unwrap(),
        kind: Kind::Original,
        asset_hash: sha256(b"jpeg without c2pa segments"),
        pixel_hash: sha256(b"decoded rgb8"),
        fp_commit: bundle.fp_commit(),
        dims: Dims {
            width: 3024,
            height: 4032,
        },
        time: TimeEvidence {
            wall_ms: 1_763_510_400_000,
            gnss_ms: Some(1_763_510_400_120),
            elapsed_ns: 77,
            boot_count: 12,
        },
        loc: LocationEvidence {
            commit: Some(opening.commit()),
            confidence: Confidence::High,
            reasons: LocationReasons::RAW_GNSS_PASSED,
        },
        checks: checks(),
        key: KeyRef { key_id, counter },
        cert_serial: CertSerial::from_u64(0x0102_0304).unwrap(),
        coarse: None,
    }
}

#[test]
fn end_to_end_seal_submit_verify() {
    let (sk, spki) = device_key();
    let key_id = KeyId::from_spki(&spki);
    let opening = LocationOpening {
        salt: [5; 32],
        lat_e7: 377_749_000,
        lon_e7: -1_224_194_000,
        acc_cm: 450,
    };
    let claim = sealed_original(key_id, 1, &opening);
    let claim_cbor = claim.to_cbor().unwrap();
    let sig: p256::ecdsa::Signature = sk.sign(&claim_cbor);
    let raw: [u8; 64] = sig.to_bytes().into();
    let low = es256_normalize_low_s(&raw).unwrap();
    let submission = Submission {
        claim_cbor: claim_cbor.clone(),
        c2pa_claim_cbor: vec![0xa0],
        cose: low.to_vec(),
    };
    let wire = cbor::to_vec(&submission);
    let received: Submission = cbor::from_slice_canonical(&wire).unwrap();
    let parsed = received.claim().unwrap();
    assert_eq!(parsed, claim);
    assert_eq!(cbor::to_vec(&parsed), received.claim_cbor);
    assert_eq!(received.record_id(), claim.record_id());
    let key = KeyInfo {
        key_id,
        pubkey: spki.clone(),
        security: SecurityLevel::Tee,
        provisioning: Provisioning::Rkp,
        freshness: Freshness::Fresh,
        patch: 20_251_105,
        enrolled_ns: 1_763_000_000_000_000_000,
        active_until_ns: 1_763_000_000_000_000_000 + 60 * 86_400_000_000_000,
        history: KeyHistory::new(1_763_000_000_000_000_000),
        chain_serials: vec![
            normalize_serial_hex(&[0x00, 0xf9, 0x20]),
            normalize_serial_hex(&[1]),
        ],
    };
    let key: KeyInfo = from_slice(&cbor::to_vec(&key)).unwrap();
    assert_eq!(parsed.key.key_id, key.key_id);
    assert!(key.accepts_original_at_ms(parsed.time.wall_ms));
    assert_eq!(
        es256_verify_low_s(&key.pubkey, &received.claim_cbor, &received.cose),
        Ok(())
    );
    let der = es256_raw_to_der(&low).unwrap();
    assert_eq!(
        Es256Key::parse(&key.pubkey)
            .unwrap()
            .verify(&received.claim_cbor, der.as_bytes()),
        Ok(())
    );
    assert!(opening.verify(&parsed.loc.commit.unwrap()));
    assert!(key.history.active_at(key.enrolled_ns + 1));
    assert!(!key.history.revoked_after(key.enrolled_ns));
    let rid = received.record_id();
    assert_eq!(rid.grouped().parse::<RecordId>(), Ok(rid));
}

#[test]
fn derivative_presentation_rules_through_public_api() {
    let (_, spki) = device_key();
    let opening = LocationOpening {
        salt: [0; 32],
        lat_e7: 0,
        lon_e7: 0,
        acc_cm: 0,
    };
    let parent = sealed_original(KeyId::from_spki(&spki), 1, &opening);
    let web = WitnessClaim {
        kind: Kind::Derivative {
            parent: parent.record_id(),
            root: parent.capture_id,
            edits: vec![
                Edit::Resize {
                    w: 1512,
                    h: 2016,
                    filter: 0,
                },
                Edit::Recompress {
                    format: ImageFormat::Webp,
                    quality: 80,
                },
            ],
        },
        loc: LocationEvidence {
            commit: None,
            ..parent.loc
        },
        coarse: Geohash5::new("9q8yy"),
        key: KeyRef {
            counter: 2,
            ..parent.key
        },
        ..parent.clone()
    };
    assert!(web.kind.presentation_only());
    let enc = web.to_cbor().unwrap();
    let back = WitnessClaim::from_cbor(&enc).unwrap();
    assert!(presentation_only(back.kind.edits()));
    let cropped = WitnessClaim {
        kind: Kind::Derivative {
            parent: parent.record_id(),
            root: parent.capture_id,
            edits: vec![Edit::Crop(Rect {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            })],
        },
        ..web.clone()
    };
    assert!(!cropped.kind.presentation_only());
    assert_ne!(cropped.record_id(), web.record_id());
}

#[test]
fn verdict_vocabulary_wire_codes() {
    let flags = VerdictFlags::KEY_REVOKED_AFTER_ANCHOR | VerdictFlags::DEV_ATTESTATION;
    assert_eq!(
        cbor::to_vec(&(Outcome::MatchesWithChanges, flags)),
        [0x82, 0x01, 0x18, 0x21]
    );
    assert_eq!(
        from_slice::<(Outcome, VerdictFlags)>(&[0x82, 0x04, 0x00]),
        Ok((Outcome::SealedUnanchored, VerdictFlags::EMPTY))
    );
}

#[test]
fn ed25519_witness_root_vector() {
    let pk = [
        0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07,
        0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07,
        0x51, 0x1a,
    ];
    let sig = [
        0xe5, 0x56, 0x43, 0x00, 0xc3, 0x60, 0xac, 0x72, 0x90, 0x86, 0xe2, 0xcc, 0x80, 0x6e, 0x82,
        0x8a, 0x84, 0x87, 0x7f, 0x1e, 0xb8, 0xe5, 0xd9, 0x74, 0xd8, 0x73, 0xe0, 0x65, 0x22, 0x49,
        0x01, 0x55, 0x5f, 0xb8, 0x82, 0x15, 0x90, 0xa3, 0x3b, 0xac, 0xc6, 0x1e, 0x39, 0x70, 0x1c,
        0xf9, 0xb4, 0x6b, 0xd2, 0x5b, 0xf5, 0xf0, 0x59, 0x5b, 0xbe, 0x24, 0x65, 0x51, 0x41, 0x43,
        0x8e, 0x7a, 0x10, 0x0b,
    ];
    assert_eq!(ed25519_verify(&pk, b"", &sig), Ok(()));
}

#[test]
fn case_id_shard_routing_text() {
    let id = CaseId::new(7, [0x5a; 15]);
    let text = id.to_string();
    let back: CaseId = text.to_lowercase().parse().unwrap();
    assert_eq!(back.shard(), 7);
}
