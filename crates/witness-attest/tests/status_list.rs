use witness_attest::{
    RevocationList, RevocationReason, RevocationStatus, Serial, parse_status_list,
};

const EXCERPT: &[u8] = br#"{
  "entries": {
    "6681152659205225093" : {
      "status": "REVOKED",
      "reason": "KEY_COMPROMISE"
    },
    "8350192447815228107" : {
      "status": "REVOKED",
      "reason": "KEY_COMPROMISE"
    },
    "2c8cdddfd5e03bfc": {
      "status": "REVOKED",
      "expires": "2020-11-13",
      "reason": "KEY_COMPROMISE",
      "comment": "Key stored on unsecure system"
    },
    "c8966fcb2fbb0d7a": {
      "status": "SUSPENDED",
      "reason": "SOFTWARE_FLAW",
      "comment": "Bug in keystore causes this key malfunction b/555555"
    },
    "e29c7a6b8a0e3f4d5c6b7a8f9e0d1c2b": {
      "status": "REVOKED",
      "reason": "SOFTWARE_FLAW"
    }
  }
}"#;

#[test]
fn excerpt_parses_to_canonical_list() {
    let list = parse_status_list(EXCERPT).unwrap();
    assert_eq!(list.len(), 7);
    let hit = |hex: &str| {
        list.get(&Serial::from_hex(hex).unwrap())
            .map(|e| (e.status, e.reason))
    };
    assert_eq!(
        hit("5cb838f1fe157a85"),
        Some((RevocationStatus::Revoked, RevocationReason::KeyCompromise))
    );
    assert_eq!(
        hit("6681152659205225093"),
        Some((RevocationStatus::Revoked, RevocationReason::KeyCompromise))
    );
    assert_eq!(
        hit("73e1d7ca422d1acb"),
        Some((RevocationStatus::Revoked, RevocationReason::KeyCompromise))
    );
    assert_eq!(
        hit("2c8cdddfd5e03bfc"),
        Some((RevocationStatus::Revoked, RevocationReason::KeyCompromise))
    );
    assert_eq!(
        hit("C8966FCB2FBB0D7A"),
        Some((RevocationStatus::Suspended, RevocationReason::SoftwareFlaw))
    );
    assert_eq!(
        hit("e29c7a6b8a0e3f4d5c6b7a8f9e0d1c2b"),
        Some((RevocationStatus::Revoked, RevocationReason::SoftwareFlaw))
    );
    assert_eq!(hit("1"), None);
    assert!(list.entries().windows(2).all(|w| w[0].serial < w[1].serial));
}

#[test]
fn canonical_bytes_round_trip() {
    let list = parse_status_list(EXCERPT).unwrap();
    let bytes = list.to_cbor();
    let back = RevocationList::from_cbor(&bytes).unwrap();
    assert_eq!(back, list);
    assert_eq!(back.to_cbor(), bytes);
}

#[test]
fn scales_to_production_size() {
    let mut json = String::from(r#"{"entries":{"#);
    let mut x: u64 = 0x1234_5678;
    for i in 0..4000u32 {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        if i > 0 {
            json.push(',');
        }
        if i % 2 == 0 {
            json.push_str(&format!(
                r#""{x}":{{"status":"REVOKED","reason":"KEY_COMPROMISE"}}"#
            ));
        } else {
            json.push_str(&format!(r#""{x:032x}":{{"status":"SUSPENDED"}}"#));
        }
    }
    json.push_str("}}");
    let list = parse_status_list(json.as_bytes()).unwrap();
    assert!((6000..=8000).contains(&list.len()), "{}", list.len());
    assert!(list.entries().windows(2).all(|w| w[0].serial < w[1].serial));
    let again = parse_status_list(json.as_bytes()).unwrap();
    assert_eq!(again.to_cbor(), list.to_cbor());
}
