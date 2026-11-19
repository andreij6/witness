use super::*;
use crate::testkit::{World, record_wire};
use witness_core::format::{KeyHistory, VerdictFlags};
use witness_verdict::testkit::{ENROLLED_NS, TestCa};

fn key_wire() -> KeyWire {
    let ca = TestCa::new(3);
    KeyWire::from_info(&ca.device(5, 6).key_info(KeyHistory::new(ENROLLED_NS)))
}

#[test]
fn hex_roundtrips_and_rejects_garbage() {
    let h = Hex(vec![0, 1, 0xab, 0xff]);
    let s = serde_json::to_string(&h).unwrap();
    assert_eq!(s, "\"0001abff\"");
    assert_eq!(serde_json::from_str::<Hex>(&s).unwrap(), h);
    assert_eq!(
        serde_json::from_str::<Hex>("\"ABFF\"").unwrap().0,
        [0xab, 0xff]
    );
    assert_eq!(
        serde_json::from_str::<Hex>("\"\"").unwrap().0,
        Vec::<u8>::new()
    );
    for bad in ["\"abc\"", "\"zz\"", "12", "null", "[1]"] {
        assert!(serde_json::from_str::<Hex>(bad).is_err(), "{bad}");
    }
}

#[test]
fn hex_array_checks_length() {
    assert_eq!(Hex(vec![7; 4]).array::<4>("x"), Ok([7; 4]));
    assert_eq!(Hex(vec![7; 3]).array::<4>("x"), Err(Error::Malformed("x")));
    assert_eq!(Hex(vec![7; 5]).array::<4>("x"), Err(Error::Malformed("x")));
}

#[test]
fn b64_accepts_padded_and_unpadded() {
    let b = B64(vec![1, 2, 3, 4]);
    let s = serde_json::to_string(&b).unwrap();
    assert_eq!(s, "\"AQIDBA==\"");
    assert_eq!(serde_json::from_str::<B64>(&s).unwrap(), b);
    assert_eq!(serde_json::from_str::<B64>("\"AQIDBA\"").unwrap(), b);
    assert!(serde_json::from_str::<B64>("\"!!\"").is_err());
    assert!(serde_json::from_str::<B64>("\"AQ-_\"").is_err());
}

#[test]
fn dec_accepts_numbers_and_strings_and_serializes_as_string() {
    assert_eq!(serde_json::from_str::<Dec>("42").unwrap(), Dec(42));
    assert_eq!(serde_json::from_str::<Dec>("\"42\"").unwrap(), Dec(42));
    let max = format!("\"{}\"", u64::MAX);
    assert_eq!(serde_json::from_str::<Dec>(&max).unwrap(), Dec(u64::MAX));
    assert_eq!(serde_json::to_string(&Dec(u64::MAX)).unwrap(), max);
    for bad in [
        "-1",
        "1.5",
        "\"\"",
        "\"+1\"",
        "\" 1\"",
        "\"18446744073709551616\"",
        "null",
        "\"0x1\"",
    ] {
        assert!(serde_json::from_str::<Dec>(bad).is_err(), "{bad}");
    }
}

#[test]
fn dec_and_hex_work_as_map_keys() {
    let mut m = BTreeMap::new();
    m.insert(Dec(9), Hex(vec![1]));
    m.insert(Dec(10), Hex(vec![2]));
    let s = serde_json::to_string(&m).unwrap();
    assert_eq!(s, "{\"9\":\"01\",\"10\":\"02\"}");
    assert_eq!(serde_json::from_str::<BTreeMap<Dec, Hex>>(&s).unwrap(), m);
    let k: BTreeMap<Hex, u8> = serde_json::from_str("{\"0aff\":1}").unwrap();
    assert_eq!(k.keys().next().unwrap().0, [0x0a, 0xff]);
    assert!(serde_json::from_str::<BTreeMap<Dec, u8>>("{\"x\":1}").is_err());
}

#[test]
fn key_wire_roundtrips_through_key_info() {
    let w = key_wire();
    let info = w.to_info().unwrap();
    assert_eq!(KeyWire::from_info(&info), w);
    let json = serde_json::to_string(&w).unwrap();
    assert!(json.contains("\"security\":\"Tee\""));
    assert!(json.contains("\"enrolledNs\":\""));
    assert_eq!(serde_json::from_str::<KeyWire>(&json).unwrap(), w);
}

#[test]
fn key_wire_rejects_inconsistent_keys() {
    let mut w = key_wire();
    w.key_id = Hex(vec![0; 16]);
    assert_eq!(w.to_info(), Err(Error::Malformed("key")));
    let mut w = key_wire();
    w.key_id = Hex(vec![0; 3]);
    assert!(w.to_info().is_err());
    let mut w = key_wire();
    w.history.clear();
    assert!(w.to_info().is_err());
    let mut w = key_wire();
    w.active_until_ns = Dec(0);
    assert!(w.to_info().is_err());
    let mut w = key_wire();
    w.chain_serials = vec!["0x1".into()];
    assert!(w.to_info().is_err());
    assert!(serde_json::from_str::<KeyWire>("{\"keyId\":\"00\"}").is_err());
}

#[test]
fn record_wire_checks_id_against_claim() {
    let mut w = World::new();
    let s = w.original(1, false);
    let rec = w.oracle.records()[0].clone();
    let wire = record_wire(&rec);
    assert_eq!(wire.record_id(), Ok(s.record_id));
    assert_eq!(wire.to_record().unwrap(), rec);
    let grouped = RecordWire {
        record_id: s.record_id.grouped().to_lowercase(),
        ..wire.clone()
    };
    assert_eq!(grouped.record_id(), Ok(s.record_id));
    let other = RecordWire {
        record_id: RecordId::from_bytes([1; 10]).to_string(),
        ..wire.clone()
    };
    assert_eq!(other.to_record(), Err(Error::Malformed("record id")));
    let garbage = RecordWire {
        claim: Hex(vec![0xa0]),
        record_id: RecordId::from_claim_cbor(&[0xa0]).to_string(),
        ..wire
    };
    assert_eq!(garbage.to_record(), Err(Error::Malformed("record claim")));
}

#[test]
fn record_wire_derives_clock_flags_when_read() {
    let mut w = World::new();
    w.original(3, false);
    let rec = w.oracle.records()[0].clone();
    assert_eq!(rec.flags, VerdictFlags::EMPTY);
    let late = RecordWire {
        anchored_ns: Dec(rec.claim.time.wall_ms * 1_000_000 + 25 * 3_600_000_000_000),
        ..record_wire(&rec)
    };
    assert_eq!(late.to_record().unwrap().flags, VerdictFlags::LATE_ANCHOR);
}

#[test]
fn record_wire_parses_bridge_shaped_json() {
    let mut w = World::new();
    w.original(2, false);
    let rec = w.oracle.records()[0].clone();
    let full = RecordWire {
        mmr_proof: vec![Hex(vec![1; 32])],
        leaf_count: Dec(5),
        certificate: Hex(vec![9]),
        ..record_wire(&rec)
    };
    let json = serde_json::to_string(&full).unwrap();
    for field in [
        "recordIx",
        "recordId",
        "c2paClaim",
        "anchoredNs",
        "mmrProof",
        "leafCount",
    ] {
        assert!(json.contains(field), "{field}");
    }
    assert_eq!(serde_json::from_str::<RecordWire>(&json).unwrap(), full);
    let minimal = format!(
        "{{\"recordIx\":\"0\",\"recordId\":\"{}\",\"claim\":\"{}\",\"anchoredNs\":7}}",
        rec.record_id,
        hex::encode(rec.claim.to_cbor().unwrap())
    );
    let parsed: RecordWire = serde_json::from_str(&minimal).unwrap();
    assert_eq!(parsed.to_record().unwrap().anchored_ns, 7);
}

#[test]
fn fingerprints_rebuild_the_committed_bundle() {
    let mut w = World::new();
    let s = w.original(3, false);
    let wire = FingerprintsWire::from_bundle(&s.fingerprints);
    assert_eq!(wire.bundle_for(&s.claim), Some(s.fingerprints.clone()));
    let index_shaped = FingerprintsWire {
        quality: None,
        portrait: None,
        ..wire.clone()
    };
    assert_eq!(
        index_shaped.bundle_for(&s.claim),
        Some(s.fingerprints.clone())
    );
    let json = serde_json::to_string(&index_shaped).unwrap();
    assert!(!json.contains("quality"));
    assert_eq!(
        serde_json::from_str::<FingerprintsWire>(&json).unwrap(),
        index_shaped
    );
}

#[test]
fn fingerprints_reject_tampering_and_bad_shapes() {
    let mut w = World::new();
    let s = w.original(4, false);
    let wire = FingerprintsWire::from_bundle(&s.fingerprints);
    let mut grid = wire.clone();
    grid.grid.0[10] ^= 1;
    assert_eq!(grid.bundle_for(&s.claim), None);
    let mut shape = wire.clone();
    shape.shapes[0].1.0[0] ^= 1;
    assert_eq!(shape.bundle_for(&s.claim), None);
    let mut quality = wire.clone();
    quality.quality = Some(wire.quality.unwrap().wrapping_add(1) % 101);
    assert_eq!(quality.bundle_for(&s.claim), None);
    let mut short = wire.clone();
    short.grid.0.pop();
    assert_eq!(short.bundle_for(&s.claim), None);
    let mut hash = wire.clone();
    hash.shapes[0].1.0.pop();
    assert_eq!(hash.bundle_for(&s.claim), None);
    let mut dup = wire.clone();
    let first = dup.shapes[0].clone();
    dup.shapes.push(first);
    assert_eq!(dup.bundle_for(&s.claim), None);
    let mut flipped = wire;
    flipped.portrait = Some(!flipped.portrait.unwrap());
    assert_eq!(flipped.bundle_for(&s.claim), None);
}

#[test]
fn fetched_defaults_and_rejects_unknown_shapes() {
    let f: Fetched = serde_json::from_str("{}").unwrap();
    assert_eq!(f, Fetched::default());
    let f: Fetched =
        serde_json::from_str("{\"exact\":[],\"pdq\":{\"hits\":[]},\"indexPending\":true}").unwrap();
    assert_eq!(f.exact, Some(vec![]));
    assert_eq!(f.pdq, Some(PdqWire::default()));
    assert!(f.index_pending);
    assert!(serde_json::from_str::<Fetched>("{\"exact\":\"x\"}").is_err());
    assert!(serde_json::from_str::<Fetched>("\"x\"").is_err());
    assert!(serde_json::from_str::<Fetched>("[1]").is_err());
}

#[test]
fn check_options_parse() {
    let o: CheckOptions =
        serde_json::from_str("{\"anchor\":\"00ff\",\"atS\":5,\"topK\":2}").unwrap();
    assert_eq!(o.anchor.0, [0, 0xff]);
    assert_eq!((o.at_s, o.top_k, o.max_pixels), (Some(5), Some(2), None));
    assert!(!o.dev_anchor);
    let empty: CheckOptions = serde_json::from_str("{}").unwrap();
    assert!(empty.anchor.0.is_empty() && !empty.dev_anchor);
    assert!(serde_json::from_str::<CheckOptions>("{\"anchor\":\"zz\"}").is_err());
    assert!(serde_json::from_str::<CheckOptions>("{\"devAnchor\":1}").is_err());
}

#[test]
fn mirrored_enums_cover_every_variant() {
    for s in [
        SecurityLevel::StrongBox,
        SecurityLevel::Tee,
        SecurityLevel::Dev,
    ] {
        assert_eq!(SecurityLevel::from(SecurityWire::from(s)), s);
    }
    for p in [Provisioning::Rkp, Provisioning::Factory, Provisioning::Dev] {
        assert_eq!(Provisioning::from(ProvisioningWire::from(p)), p);
    }
    for f in [Freshness::Fresh, Freshness::Late] {
        assert_eq!(Freshness::from(FreshnessWire::from(f)), f);
    }
    for k in [
        KeyStatus::Active,
        KeyStatus::Retired,
        KeyStatus::Suspended,
        KeyStatus::Revoked,
    ] {
        assert_eq!(KeyStatus::from(StatusWire::from(k)), k);
    }
}
