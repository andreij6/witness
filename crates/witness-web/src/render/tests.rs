use super::*;
use crate::testkit::{World, replay};
use witness_core::format::{CaptureId, Geohash5, RecordId, Rect};
use witness_core::verdict::{Anchor, Reason, Reasons, Region, VERDICT_V};

#[test]
fn outcome_names_are_distinct_and_screaming_snake() {
    let names: Vec<_> = Outcome::ALL.iter().map(|o| outcome_name(*o)).collect();
    assert_eq!(
        names,
        [
            "ORIGINAL",
            "MATCHES_WITH_CHANGES",
            "ALTERED",
            "NO_RECORD",
            "SEALED_UNANCHORED"
        ]
    );
}

#[test]
fn flag_names_follow_bit_order() {
    assert_eq!(FLAG_NAMES.len(), VerdictFlags::FLAGS.len());
    assert_eq!(flag_names(VerdictFlags::EMPTY), Vec::<&str>::new());
    assert_eq!(flag_names(VerdictFlags::ALL), FLAG_NAMES);
    assert_eq!(
        flag_names(VerdictFlags::COULDNT_COMPARE | VerdictFlags::KEY_REVOKED_AFTER_ANCHOR),
        ["KEY_REVOKED_AFTER_ANCHOR", "COULDNT_COMPARE"]
    );
    for (i, f) in VerdictFlags::FLAGS.iter().enumerate() {
        assert_eq!(f.bits(), 1 << i);
    }
}

#[test]
fn every_edit_kind_renders() {
    let r = Rect {
        x: 1,
        y: 2,
        w: 3,
        h: 4,
    };
    let edits = [
        (
            Edit::Resize {
                w: 10,
                h: 20,
                filter: 1,
            },
            "resize",
        ),
        (
            Edit::Recompress {
                format: ImageFormat::Webp,
                quality: 80,
            },
            "recompress",
        ),
        (Edit::Orient { orientation: 6 }, "orient"),
        (Edit::Crop(r), "crop"),
        (Edit::Rotate { deg_e2: -250 }, "rotate"),
        (
            Edit::Tone {
                exposure_e2: 1,
                contrast_e2: -2,
                saturation_e2: 3,
            },
            "tone",
        ),
        (
            Edit::Filter {
                id: 7,
                strength_e2: 50,
            },
            "filter",
        ),
        (
            Edit::Blur {
                rect: r,
                class: BlurClass::HouseNo,
                sigma_e2: 300,
            },
            "blur",
        ),
    ];
    for (e, kind) in edits {
        assert_eq!(edit(&e)["kind"], kind);
    }
    assert_eq!(edit(&edits[1].0)["format"], "webp");
    assert_eq!(edit(&edits[4].0)["degE2"], -250);
    assert_eq!(edit(&edits[7].0)["class"], "house_no");
    assert_eq!(edit(&edits[3].0)["h"], 4);
    for f in ImageFormat::ALL {
        assert!(!format_name(*f).is_empty());
    }
    for c in BlurClass::ALL {
        assert!(!blur_name(*c).is_empty());
    }
}

#[test]
fn claims_render_ids_as_strings() {
    let w = World::new();
    let mut c = w.dev.claim(u64::from(u32::MAX) << 20, Kind::Original, 3);
    let v = claim(&c);
    assert_eq!(v["kind"]["type"], "original");
    assert_eq!(v["captureId"], c.capture_id.to_u64().to_string());
    assert_eq!(v["counter"], "3");
    assert!(v["coarse"].is_null());
    c.kind = Kind::Derivative {
        parent: RecordId::from_bytes([1; 10]),
        root: CaptureId::from_u64(77).unwrap(),
        edits: vec![Edit::Orient { orientation: 1 }],
    };
    c.coarse = Geohash5::new("9q8yy");
    let v = claim(&c);
    assert_eq!(v["kind"]["type"], "derivative");
    assert_eq!(v["kind"]["root"], "77");
    assert_eq!(v["kind"]["edits"][0]["kind"], "orient");
    assert_eq!(v["coarse"], "9q8yy");
}

#[test]
fn verdict_renders_reference_regions_and_reasons() {
    let w = World::new();
    let c = w.dev.claim(5, Kind::Original, 1);
    let v = Verdict {
        version: VERDICT_V,
        outcome: Outcome::Altered,
        flags: VerdictFlags::DEV_ATTESTATION,
        reasons: [Reason::SealVerified, Reason::Unconfirmed]
            .into_iter()
            .collect::<Reasons>(),
        declared: vec![],
        regions: vec![Region {
            x: 1,
            y: 2,
            w: 3,
            h: 4,
        }],
        reference: Some(Reference {
            record_id: c.record_id(),
            claim: c.clone(),
            anchor: Some(Anchor {
                record_ix: u64::MAX,
                anchored_ns: 9,
            }),
            evidence: Evidence::Pixels,
            confidence_q16: 1,
            compared_q16: 2,
            changed_q16: 3,
        }),
    };
    let j = verdict(&v);
    assert_eq!(j["outcome"], "ALTERED");
    assert_eq!(j["outcomeCode"], 2);
    assert_eq!(j["flagNames"][0], "DEV_ATTESTATION");
    assert_eq!(j["reasons"], json!(["SealVerified", "Unconfirmed"]));
    assert_eq!(j["regions"][0]["w"], 3);
    assert_eq!(j["recordId"], c.record_id().to_string());
    assert_eq!(j["reference"]["anchor"]["recordIx"], u64::MAX.to_string());
    assert_eq!(j["reference"]["evidence"], "pixels");
    for e in [Evidence::Seal, Evidence::ExactHash, Evidence::Grid] {
        assert!(!evidence_name(e).is_empty());
    }
}

#[test]
fn verdict_without_reference_has_null_record() {
    let w = World::new();
    let (step, _, _) = replay(&crate::testkit::jpeg_of(31, 90), &w.options(), &w.oracle);
    let j = verdict(&step.verdict);
    assert_eq!(j["outcome"], "NO_RECORD");
    assert!(j["recordId"].is_null() && j["reference"].is_null());
    assert_eq!(j.to_string(), verdict(&step.verdict).to_string());
}

fn keys_sorted(v: &Value) -> bool {
    match v {
        Value::Object(m) => {
            let keys: Vec<&String> = m.keys().collect();
            keys.windows(2).all(|w| w[0] < w[1]) && m.values().all(keys_sorted)
        }
        Value::Array(a) => a.iter().all(keys_sorted),
        _ => true,
    }
}

#[test]
fn rendered_json_has_canonical_key_order_under_any_feature_set() {
    let w = World::new();
    let c = w.dev.claim(5, Kind::Original, 1);
    let v = Verdict {
        version: VERDICT_V,
        outcome: Outcome::MatchesWithChanges,
        flags: VerdictFlags::EMPTY,
        reasons: [Reason::SealVerified].into_iter().collect::<Reasons>(),
        declared: vec![Edit::Rotate { deg_e2: 9 }],
        regions: vec![Region {
            x: 1,
            y: 2,
            w: 3,
            h: 4,
        }],
        reference: Some(Reference {
            record_id: c.record_id(),
            claim: c.clone(),
            anchor: None,
            evidence: Evidence::Seal,
            confidence_q16: 0,
            compared_q16: 0,
            changed_q16: 0,
        }),
    };
    let j = verdict(&v);
    assert!(keys_sorted(&j), "{j}");
    assert!(keys_sorted(&claim(&c)));
    assert!(keys_sorted(&edit(&Edit::Rotate { deg_e2: 1 })));
    assert!(j.to_string().starts_with("{\"declared\":"));
    assert_eq!(verdict(&v).to_string(), j.to_string());
}

#[test]
fn claim_detail_extends_the_lean_claim() {
    let w = World::new();
    let c = w.dev.claim(5, Kind::Original, 2);
    let lean = claim(&c);
    let full = claim_detail(&c);
    for (k, v) in lean.as_object().unwrap() {
        assert_eq!(&full[k], v, "{k}");
    }
    assert_eq!(full["fpCommit"], hex::encode(c.fp_commit));
    assert_eq!(full["elapsedNs"], c.time.elapsed_ns.to_string());
    assert_eq!(full["bootCount"], c.time.boot_count);
    assert_eq!(full["gnssMs"], json!(c.time.gnss_ms.map(|g| g.to_string())));
    assert_eq!(full["location"]["confidence"], c.loc.confidence.code());
    assert_eq!(full["location"]["reasons"], c.loc.reasons.bits());
    assert_eq!(
        full["location"]["commit"],
        json!(c.loc.commit.map(hex::encode))
    );
    assert_eq!(full["checks"]["keySecurity"], c.checks.key_security.code());
    assert_eq!(full["checks"]["boot"], c.checks.boot.code());
    assert_eq!(full["checks"]["deviceLocked"], c.checks.device_locked);
    assert_eq!(full["checks"]["mockLocation"], c.checks.mock_location);
    assert_eq!(full["checks"]["gnss"], c.checks.gnss.bits());
    assert_eq!(full["checks"]["recapture"], c.checks.recapture);
    assert_eq!(full["certSerial"], hex::encode(c.cert_serial.as_bytes()));
}

#[test]
fn claim_detail_renders_absent_gnss_and_commit_as_null() {
    let w = World::new();
    let mut c = w.dev.claim(6, Kind::Original, 1);
    c.time.gnss_ms = None;
    c.loc.commit = None;
    let full = claim_detail(&c);
    assert!(full["gnssMs"].is_null());
    assert!(full["location"]["commit"].is_null());
    c.time.gnss_ms = Some(u64::MAX);
    assert_eq!(claim_detail(&c)["gnssMs"], u64::MAX.to_string());
}

#[test]
fn claim_cbor_json_decodes_and_is_deterministic() {
    let w = World::new();
    let c = w.dev.claim(7, Kind::Original, 4);
    let cbor = c.to_cbor().unwrap();
    let text = claim_cbor_json(&cbor).unwrap();
    assert_eq!(text, claim_cbor_json(&cbor).unwrap());
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed, claim_detail(&c));
    let keys: Vec<_> = parsed.as_object().unwrap().keys().cloned().collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[test]
fn claim_cbor_json_rejects_malformed_input() {
    for bad in [&[][..], &[0xa0], &[0xff, 0x00], &[0x01]] {
        assert_eq!(
            claim_cbor_json(bad).unwrap_err().to_string(),
            "malformed claim"
        );
    }
    let w = World::new();
    let mut cbor = w.dev.claim(8, Kind::Original, 1).to_cbor().unwrap();
    cbor.truncate(cbor.len() - 1);
    assert!(claim_cbor_json(&cbor).is_err());
}
