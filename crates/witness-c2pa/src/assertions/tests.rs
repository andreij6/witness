use super::*;
use crate::jumbf::SuperBox;
use crate::testkit::Rng;
use alloc::vec;

fn enc(f: impl FnOnce(&mut Encoder<'_>)) -> Vec<u8> {
    let mut v = Vec::new();
    f(&mut Encoder::new(&mut v));
    v
}

fn read_actions(bytes: &[u8]) -> Vec<(alloc::string::String, Option<alloc::string::String>)> {
    let mut d = Decoder::new(bytes);
    assert_eq!(d.map().unwrap(), Some(1));
    assert_eq!(d.str().unwrap(), "actions");
    let n = d.array().unwrap().unwrap();
    (0..n)
        .map(|_| {
            let m = d.map().unwrap().unwrap();
            let mut action = None;
            let mut dst = None;
            for _ in 0..m {
                match d.str().unwrap() {
                    "action" => action = Some(d.str().unwrap().into()),
                    "digitalSourceType" => dst = Some(d.str().unwrap().into()),
                    k => panic!("unexpected key {k}"),
                }
            }
            (action.unwrap(), dst)
        })
        .collect()
}

#[test]
fn captured_action_encodes_created_with_source_type() {
    let a = read_actions(&actions_cbor(&[Action::CAPTURED]).unwrap());
    assert_eq!(a, vec![(CREATED.into(), Some(DIGITAL_CAPTURE.into()))]);
}

#[test]
fn opened_followed_by_edits_is_accepted() {
    let edited = Action {
        action: "c2pa.edited",
        digital_source_type: None,
    };
    let opened = Action {
        action: OPENED,
        digital_source_type: None,
    };
    let a = read_actions(&actions_cbor(&[opened, edited, edited]).unwrap());
    assert_eq!(a.len(), 3);
    assert_eq!(a[1], ("c2pa.edited".into(), None));
}

#[test]
fn actions_must_start_with_single_created_or_opened() {
    let edited = Action {
        action: "c2pa.edited",
        digital_source_type: None,
    };
    for bad in [
        &[][..],
        &[edited][..],
        &[Action::CAPTURED, Action::CAPTURED][..],
        &[
            Action::CAPTURED,
            Action {
                action: OPENED,
                digital_source_type: None,
            },
        ][..],
    ] {
        assert_eq!(actions_cbor(bad), Err(Error::MalformedAssertion));
    }
}

#[test]
fn data_hash_round_trips() {
    let mut rng = Rng::new(1);
    for _ in 0..500 {
        let e = Exclusion {
            start: rng.next() >> rng.below(64),
            length: rng.next() >> rng.below(64),
        };
        let h: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let cbor = data_hash_cbor(e, &h);
        let d = DataHash::parse(&cbor).unwrap();
        assert_eq!(d.exclusions, vec![e]);
        assert_eq!(d.alg, Some("sha256"));
        assert_eq!(d.hash, &h);
    }
}

#[test]
fn data_hash_matches_reference_encoding() {
    let cbor = data_hash_cbor(
        Exclusion {
            start: 20,
            length: 300,
        },
        &[0xab; 32],
    );
    let expected = enc(|e| {
        e.map(5);
        e.text("exclusions");
        e.array(1);
        e.map(2);
        e.text("start");
        e.u64(20);
        e.text("length");
        e.u64(300);
        e.text("name");
        e.text("jumbf manifest");
        e.text("alg");
        e.text("sha256");
        e.text("hash");
        e.bytes(&[0xab; 32]);
        e.text("pad");
        e.bytes(&[]);
    });
    assert_eq!(cbor, expected);
}

#[test]
fn data_hash_parses_minimal_and_padded_forms() {
    let minimal = enc(|e| {
        e.map(2);
        e.text("hash");
        e.bytes(&[1; 32]);
        e.text("pad");
        e.bytes(&[0; 3]);
    });
    let d = DataHash::parse(&minimal).unwrap();
    assert!(d.exclusions.is_empty() && d.alg.is_none());
    let padded = enc(|e| {
        e.map(3);
        e.text("pad2");
        e.bytes(&[0; 2]);
        e.text("hash");
        e.bytes(&[1; 32]);
        e.text("exclusions");
        e.array(2);
        for (s, l) in [(1, 2), (3, 4)] {
            e.map(2);
            e.text("length");
            e.u64(l);
            e.text("start");
            e.u64(s);
        }
    });
    let d = DataHash::parse(&padded).unwrap();
    assert_eq!(
        d.exclusions,
        vec![
            Exclusion {
                start: 1,
                length: 2
            },
            Exclusion {
                start: 3,
                length: 4
            }
        ]
    );
}

#[test]
fn data_hash_rejects_malformed() {
    let cases = [
        enc(|e| e.map(0)),
        enc(|e| {
            e.map(1);
            e.text("hash");
            e.text("not bytes");
        }),
        enc(|e| {
            e.map(2);
            e.text("hash");
            e.bytes(&[1]);
            e.text("hash");
            e.bytes(&[1]);
        }),
        enc(|e| {
            e.map(2);
            e.text("hash");
            e.bytes(&[1]);
            e.text("surprise");
            e.u64(1);
        }),
        enc(|e| {
            e.map(2);
            e.text("hash");
            e.bytes(&[1]);
            e.text("exclusions");
            e.array(1);
            e.map(1);
            e.text("start");
            e.u64(1);
        }),
        enc(|e| {
            e.map(2);
            e.text("hash");
            e.bytes(&[1]);
            e.text("exclusions");
            e.array(1);
            e.map(3);
            e.text("start");
            e.u64(1);
            e.text("length");
            e.u64(1);
            e.text("extra");
            e.u64(1);
        }),
        enc(|e| {
            e.map(2);
            e.text("hash");
            e.bytes(&[1]);
            e.text("exclusions");
            e.array(5000);
        }),
        enc(|e| {
            e.map(1);
            e.text("hash");
            e.bytes(&[1]);
            e.u64(0);
        }),
        vec![0xbf, 0xff],
        vec![],
    ];
    for (i, c) in cases.iter().enumerate() {
        assert!(DataHash::parse(c).is_err(), "case {i}");
    }
}

#[test]
fn assertion_box_layout_matches_c2pa() {
    let b = cbor_assertion_box(ACTIONS, &[0xa0]).unwrap();
    let sb = SuperBox::from_bytes(&b).unwrap();
    assert_eq!(sb.desc.uuid, uuid::CBOR);
    assert_eq!(sb.desc.toggles, 0x03);
    assert_eq!(sb.label(), Some(ACTIONS));
    assert_eq!(sb.only_child(CBOR), Ok(&[0xa0][..]));
    assert_eq!(b.len(), 8 + 8 + 16 + 1 + ACTIONS.len() + 1 + 8 + 1);
}

#[test]
fn assertion_hash_covers_payload_without_superbox_header() {
    let cbor = [0xa1, 0x61, b'a', 0x01];
    let b = cbor_assertion_box(WITNESS_CLAIM, &cbor).unwrap();
    let expected: [u8; 32] = Sha256::digest(&b[8..]).into();
    assert_eq!(box_hash(&b), expected);
    assert_eq!(cbor_assertion_hash(WITNESS_CLAIM, &cbor), Ok(expected));
    assert_ne!(cbor_assertion_hash(DATA_HASH, &cbor).unwrap(), expected);
    assert_ne!(
        cbor_assertion_hash(WITNESS_CLAIM, &[0xa0]).unwrap(),
        expected
    );
}

#[test]
fn random_bytes_never_panic_data_hash_parser() {
    let mut rng = Rng::new(2);
    let valid = data_hash_cbor(
        Exclusion {
            start: 1,
            length: 2,
        },
        &[3; 32],
    );
    for _ in 0..3000 {
        let mut d = valid.clone();
        let i = rng.below(d.len());
        d[i] = rng.next() as u8;
        let _ = DataHash::parse(&d);
        let n = rng.below(40);
        let _ = DataHash::parse(&rng.bytes(n));
    }
}
