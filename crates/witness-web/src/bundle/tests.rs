use super::*;
use crate::testkit::{World, jpeg_of};
use witness_core::format::Kind;

fn sample(w: &mut World) -> (Vec<u8>, Vec<u8>, Submission) {
    let a = jpeg_of(1, 80);
    let s = w.seal(&jpeg_of(2, 90), Kind::Original);
    (a, s.jpeg.clone(), s.submission.clone())
}

fn encode(title: &str, summary: Option<&str>, items: Vec<CaseItem<'_>>) -> Vec<u8> {
    CaseBundle {
        title,
        summary,
        items,
    }
    .to_cbor()
}

#[test]
fn bundles_roundtrip_with_and_without_optionals() {
    let mut w = World::new();
    let (a, b, sub) = sample(&mut w);
    let items = vec![
        CaseItem {
            record_id: RecordId::from_bytes([1; 10]),
            jpeg: &a,
            submission: None,
        },
        CaseItem {
            record_id: sub.record_id(),
            jpeg: &b,
            submission: Some(sub.clone()),
        },
    ];
    let bytes = encode("Street", Some("two photos"), items.clone());
    let parsed = CaseBundle::parse(&bytes).unwrap();
    assert_eq!(parsed.title, "Street");
    assert_eq!(parsed.summary, Some("two photos"));
    assert_eq!(parsed.items, items);
    let bare = encode("", None, items[..1].to_vec());
    assert_eq!(CaseBundle::parse(&bare).unwrap().summary, None);
}

#[test]
fn json_points_at_jpeg_bytes_without_copying() {
    let mut w = World::new();
    let (a, b, sub) = sample(&mut w);
    let bytes = encode(
        "t",
        None,
        vec![
            CaseItem {
                record_id: RecordId::from_bytes([1; 10]),
                jpeg: &a,
                submission: None,
            },
            CaseItem {
                record_id: sub.record_id(),
                jpeg: &b,
                submission: Some(sub.clone()),
            },
        ],
    );
    let v: Value = serde_json::from_str(&parse_json(&bytes).unwrap()).unwrap();
    for (i, jpeg) in [&a, &b].into_iter().enumerate() {
        let off = v["items"][i]["jpeg"]["offset"].as_u64().unwrap() as usize;
        let len = v["items"][i]["jpeg"]["len"].as_u64().unwrap() as usize;
        assert_eq!(&bytes[off..off + len], jpeg.as_slice());
    }
    assert!(v["items"][0]["submission"].is_null());
    assert_eq!(v["items"][1]["submission"]["cose"], hex::encode(&sub.cose));
    assert_eq!(v["items"][1]["recordId"], sub.record_id().to_string());
    assert_eq!(v["version"], 1);
}

#[test]
fn mismatched_submission_is_rejected() {
    let mut w = World::new();
    let (a, _, sub) = sample(&mut w);
    let bytes = encode(
        "t",
        None,
        vec![CaseItem {
            record_id: RecordId::from_bytes([2; 10]),
            jpeg: &a,
            submission: Some(sub),
        }],
    );
    assert_eq!(
        CaseBundle::parse(&bytes),
        Err(WebError::Malformed("case bundle"))
    );
}

#[test]
fn limits_are_enforced() {
    let jpeg = [0xff, 0xd8];
    let item = || CaseItem {
        record_id: RecordId::from_bytes([1; 10]),
        jpeg: &jpeg,
        submission: None,
    };
    let long_title = "x".repeat(MAX_TITLE + 1);
    let long_summary = "y".repeat(MAX_SUMMARY + 1);
    let edge_title = "x".repeat(MAX_TITLE);
    assert!(CaseBundle::parse(&encode(&edge_title, None, vec![item()])).is_ok());
    assert!(CaseBundle::parse(&encode(&long_title, None, vec![item()])).is_err());
    assert!(CaseBundle::parse(&encode("t", Some(&long_summary), vec![item()])).is_err());
    assert!(CaseBundle::parse(&encode("t", None, vec![])).is_err());
    assert!(
        CaseBundle::parse(&encode("t", None, (0..MAX_ITEMS).map(|_| item()).collect())).is_ok()
    );
    assert!(
        CaseBundle::parse(&encode(
            "t",
            None,
            (0..=MAX_ITEMS).map(|_| item()).collect()
        ))
        .is_err()
    );
    let empty = CaseItem {
        record_id: RecordId::from_bytes([1; 10]),
        jpeg: &[],
        submission: None,
    };
    assert!(CaseBundle::parse(&encode("t", None, vec![empty])).is_err());
}

#[test]
fn malformed_and_non_canonical_bytes_are_rejected() {
    let jpeg = [1u8, 2, 3];
    let good = encode(
        "t",
        None,
        vec![CaseItem {
            record_id: RecordId::from_bytes([1; 10]),
            jpeg: &jpeg,
            submission: None,
        }],
    );
    for cut in 0..good.len() {
        assert!(CaseBundle::parse(&good[..cut]).is_err(), "{cut}");
    }
    let mut trailing = good.clone();
    trailing.push(0);
    assert!(CaseBundle::parse(&trailing).is_err());
    let mut version = good.clone();
    version[2] = 2;
    assert!(CaseBundle::parse(&version).is_err());
    let mut wide = vec![0xb8, 0x03];
    wide.extend_from_slice(&good[1..]);
    assert!(CaseBundle::parse(&wide).is_err());
    assert!(CaseBundle::parse(&[]).is_err());
    assert!(parse_json(b"garbage").is_err());
    let mut seed = 0x9e37_79b9_u32;
    for _ in 0..500 {
        let mut b = good.clone();
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let i = seed as usize % b.len();
        b[i] ^= (seed >> 24) as u8 | 1;
        if let Ok(p) = CaseBundle::parse(&b) {
            assert_eq!(p.to_cbor(), b);
        }
    }
}

#[test]
fn encoding_is_deterministic() {
    let jpeg = [9u8; 10];
    let make = || {
        encode(
            "t",
            Some("s"),
            vec![CaseItem {
                record_id: RecordId::from_bytes([3; 10]),
                jpeg: &jpeg,
                submission: None,
            }],
        )
    };
    assert_eq!(make(), make());
    assert_eq!(make()[0], 0xa4);
}
