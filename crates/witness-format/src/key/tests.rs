use super::*;
use crate::cbor::{from_slice, from_slice_canonical, to_vec};
use crate::testutil::Rng;
use alloc::vec;

const DAY_NS: u64 = 86_400_000_000_000;

fn ch(status: KeyStatus, since_ns: u64) -> StatusChange {
    StatusChange { status, since_ns }
}

fn history() -> KeyHistory {
    KeyHistory::from_changes(vec![
        ch(KeyStatus::Active, 100),
        ch(KeyStatus::Retired, 200),
        ch(KeyStatus::Suspended, 300),
        ch(KeyStatus::Active, 400),
        ch(KeyStatus::Revoked, 500),
    ])
    .unwrap()
}

fn info() -> KeyInfo {
    let pubkey = vec![0x30, 0x59, 0x01, 0x02];
    KeyInfo {
        key_id: KeyId::from_spki(&pubkey),
        pubkey,
        security: SecurityLevel::StrongBox,
        provisioning: Provisioning::Rkp,
        freshness: Freshness::Fresh,
        patch: 202_510,
        enrolled_ns: 1000,
        active_until_ns: 1000 + 60 * DAY_NS,
        history: KeyHistory::new(1000),
        chain_serials: vec![String::from("1a2b"), String::from("0"), String::from("ff")],
    }
}

#[test]
fn status_at_binary_search() {
    let h = history();
    assert_eq!(h.status_at(0), None);
    assert_eq!(h.status_at(99), None);
    assert_eq!(h.status_at(100), Some(KeyStatus::Active));
    assert_eq!(h.status_at(199), Some(KeyStatus::Active));
    assert_eq!(h.status_at(200), Some(KeyStatus::Retired));
    assert_eq!(h.status_at(350), Some(KeyStatus::Suspended));
    assert_eq!(h.status_at(450), Some(KeyStatus::Active));
    assert_eq!(h.status_at(500), Some(KeyStatus::Revoked));
    assert_eq!(h.status_at(u64::MAX), Some(KeyStatus::Revoked));
    assert_eq!(h.current(), KeyStatus::Revoked);
}

#[test]
fn active_at_treats_retired_as_trusted() {
    let h = history();
    assert!(!h.active_at(50));
    assert!(h.active_at(100));
    assert!(h.active_at(250));
    assert!(!h.active_at(300));
    assert!(!h.active_at(399));
    assert!(h.active_at(400));
    assert!(!h.active_at(500));
    assert!(!h.active_at(u64::MAX));
}

#[test]
fn revoked_after_semantics() {
    let h = history();
    assert!(h.revoked_after(0));
    assert!(h.revoked_after(299));
    assert!(h.revoked_after(300));
    assert!(h.revoked_after(499));
    assert!(!h.revoked_after(500));
    assert!(!h.revoked_after(u64::MAX));
    assert_eq!(
        h.first_distrusted_after(150),
        Some(ch(KeyStatus::Suspended, 300))
    );
    assert_eq!(
        h.first_distrusted_after(300),
        Some(ch(KeyStatus::Revoked, 500))
    );
    assert_eq!(h.first_distrusted_after(500), None);
    let clean = KeyHistory::new(5);
    assert!(!clean.revoked_after(0));
    assert!(clean.active_at(5));
    assert!(!clean.active_at(4));
}

#[test]
fn history_construction_rules() {
    assert_eq!(KeyHistory::from_changes(vec![]), Err(Error::Length));
    assert_eq!(
        KeyHistory::from_changes(vec![ch(KeyStatus::Active, 5), ch(KeyStatus::Retired, 5)]),
        Err(Error::Invalid)
    );
    assert_eq!(
        KeyHistory::from_changes(vec![ch(KeyStatus::Active, 5), ch(KeyStatus::Retired, 4)]),
        Err(Error::Invalid)
    );
    assert_eq!(
        KeyHistory::from_changes(vec![ch(KeyStatus::Active, 5), ch(KeyStatus::Active, 6)]),
        Err(Error::Invalid)
    );
    assert_eq!(
        KeyHistory::from_changes(vec![ch(KeyStatus::Revoked, 5), ch(KeyStatus::Active, 6)]),
        Err(Error::Invalid)
    );
    let too_long: Vec<StatusChange> = (0..=MAX_HISTORY as u64)
        .map(|i| {
            ch(
                if i % 2 == 0 {
                    KeyStatus::Active
                } else {
                    KeyStatus::Suspended
                },
                i,
            )
        })
        .collect();
    assert_eq!(KeyHistory::from_changes(too_long), Err(Error::Length));
    assert!(KeyHistory::from_changes(vec![ch(KeyStatus::Revoked, 0)]).is_ok());
}

#[test]
fn history_push_rules() {
    let mut h = KeyHistory::new(10);
    assert_eq!(h.push(ch(KeyStatus::Retired, 10)), Err(Error::Invalid));
    assert_eq!(h.push(ch(KeyStatus::Active, 11)), Err(Error::Invalid));
    assert_eq!(h.push(ch(KeyStatus::Retired, 11)), Ok(()));
    assert_eq!(h.push(ch(KeyStatus::Revoked, 12)), Ok(()));
    assert_eq!(h.push(ch(KeyStatus::Active, 13)), Err(Error::Invalid));
    assert_eq!(h.changes().len(), 3);
    let mut full = KeyHistory::new(0);
    for i in 1..MAX_HISTORY as u64 {
        let s = if i % 2 == 1 {
            KeyStatus::Suspended
        } else {
            KeyStatus::Active
        };
        full.push(ch(s, i)).unwrap();
    }
    assert_eq!(full.changes().len(), MAX_HISTORY);
    assert_eq!(
        full.push(ch(KeyStatus::Revoked, 1 << 40)),
        Err(Error::Length)
    );
}

#[test]
fn history_cbor() {
    let h = KeyHistory::new(1);
    assert_eq!(to_vec(&h), [0x81, 0x82, 0x00, 0x01]);
    assert_eq!(
        from_slice_canonical::<KeyHistory>(&to_vec(&history())),
        Ok(history())
    );
    assert_eq!(from_slice::<KeyHistory>(&[0x80]), Err(Error::Length));
    assert_eq!(
        from_slice::<KeyHistory>(&[0x82, 0x82, 0x00, 0x02, 0x82, 0x01, 0x01]),
        Err(Error::Invalid)
    );
    assert_eq!(
        from_slice::<KeyHistory>(&[0x81, 0x82, 0x04, 0x01]),
        Err(Error::Range)
    );
    assert_eq!(
        from_slice::<KeyHistory>(&[0x81, 0x83, 0x00, 0x01, 0x00]),
        Err(Error::Length)
    );
}

#[test]
fn status_classification() {
    assert!(KeyStatus::Active.is_trusted());
    assert!(KeyStatus::Retired.is_trusted());
    assert!(!KeyStatus::Suspended.is_trusted());
    assert!(!KeyStatus::Revoked.is_trusted());
    for s in KeyStatus::ALL {
        assert_ne!(s.is_trusted(), s.is_distrusted());
    }
}

#[test]
fn serial_hex_normalization() {
    assert_eq!(normalize_serial_hex(&[]), "0");
    assert_eq!(normalize_serial_hex(&[0, 0]), "0");
    assert_eq!(normalize_serial_hex(&[0, 0x0a, 0xbc]), "abc");
    assert_eq!(normalize_serial_hex(&[0x10, 0x00]), "1000");
    assert_eq!(normalize_serial_hex(&[0xf9, 0x20, 0x09, 0xe8]), "f92009e8");
    for s in ["0", "1", "abc", "f92009e853b6b045"] {
        assert!(is_normalized_serial_hex(s), "{s}");
    }
    for s in ["", "00", "0a", "ABC", "g1", "-1", " 1", &"f".repeat(65)] {
        assert!(!is_normalized_serial_hex(s), "{s}");
    }
    let mut rng = Rng::new(70);
    for _ in 0..1000 {
        let mut b = vec![0u8; rng.below(20) as usize];
        rng.fill(&mut b);
        if rng.bool() && !b.is_empty() {
            b[0] = 0;
        }
        assert!(is_normalized_serial_hex(&normalize_serial_hex(&b)));
    }
}

#[test]
fn key_info_roundtrip_and_leaf_hash() {
    let k = info();
    assert_eq!(k.validate(), Ok(()));
    let enc = to_vec(&k);
    assert_eq!(&enc[..4], &[0xab, 0x00, 0x01, 0x01]);
    assert_eq!(from_slice_canonical::<KeyInfo>(&enc), Ok(k.clone()));
    assert_eq!(k.leaf_hash(), crate::crypto::sha256(&enc));
    let mut revoked = k.clone();
    revoked.history.push(ch(KeyStatus::Revoked, 2000)).unwrap();
    assert_ne!(revoked.leaf_hash(), k.leaf_hash());
}

#[test]
fn key_info_validation() {
    let mut wrong_id = info();
    wrong_id.key_id = KeyId::from_bytes([0; 16]);
    assert_eq!(wrong_id.validate(), Err(Error::Invalid));
    assert_eq!(
        from_slice::<KeyInfo>(&to_vec(&wrong_id)),
        Err(Error::Invalid)
    );
    let mut empty = info();
    empty.pubkey.clear();
    empty.key_id = KeyId::from_spki(&[]);
    assert_eq!(empty.validate(), Err(Error::Length));
    let mut window = info();
    window.active_until_ns = window.enrolled_ns - 1;
    assert_eq!(window.validate(), Err(Error::Range));
    let mut serial = info();
    serial.chain_serials.push(String::from("00ff"));
    assert_eq!(serial.validate(), Err(Error::Invalid));
    assert_eq!(from_slice::<KeyInfo>(&to_vec(&serial)), Err(Error::Invalid));
    let mut chain = info();
    chain.chain_serials = vec![String::from("1"); MAX_CHAIN + 1];
    assert_eq!(chain.validate(), Err(Error::Length));
    let mut big = info();
    big.pubkey = vec![1; MAX_PUBKEY + 1];
    big.key_id = KeyId::from_spki(&big.pubkey);
    assert_eq!(big.validate(), Err(Error::Length));
}

#[test]
fn key_info_rejects_structural() {
    let enc = to_vec(&info());
    let mut version = enc.clone();
    version[2] = 0x02;
    assert_eq!(from_slice::<KeyInfo>(&version), Err(Error::Version(2)));
    for cut in 0..enc.len() {
        assert!(from_slice::<KeyInfo>(&enc[..cut]).is_err());
    }
    let mut bad_security = enc.clone();
    assert_eq!(enc[27], 0x03);
    bad_security[28] = 0x03;
    assert_eq!(from_slice::<KeyInfo>(&bad_security), Err(Error::Range));
}

#[test]
fn dev_and_window_helpers() {
    let k = info();
    assert!(!k.is_dev());
    assert!(
        KeyInfo {
            provisioning: Provisioning::Dev,
            ..info()
        }
        .is_dev()
    );
    assert!(
        KeyInfo {
            security: SecurityLevel::Dev,
            ..info()
        }
        .is_dev()
    );
    let until_ms = k.active_until_ns / 1_000_000;
    assert!(k.accepts_original_at_ms(until_ms));
    assert!(!k.accepts_original_at_ms(until_ms + 1));
    assert!(k.accepts_original_at_ms(0));
    assert!(!k.accepts_original_at_ms(u64::MAX));
    let max = KeyInfo {
        active_until_ns: u64::MAX,
        ..info()
    };
    assert!(max.accepts_original_at_ms(u64::MAX / 1_000_000));
    assert!(!max.accepts_original_at_ms(u64::MAX / 1_000_000 + 1));
}

#[test]
fn property_history_queries_match_linear_scan() {
    let mut rng = Rng::new(71);
    for _ in 0..300 {
        let mut h = KeyHistory::new(rng.below(1000));
        let mut t = h.changes()[0].since_ns;
        for _ in 0..rng.below(10) {
            t += 1 + rng.below(1000);
            let s = KeyStatus::ALL[rng.below(4) as usize];
            let _ = h.push(ch(s, t));
        }
        for _ in 0..50 {
            let q = rng.below(t + 2000);
            let linear = h
                .changes()
                .iter()
                .rev()
                .find(|c| c.since_ns <= q)
                .map(|c| c.status);
            assert_eq!(h.status_at(q), linear);
            let later = h
                .changes()
                .iter()
                .any(|c| c.since_ns > q && c.status.is_distrusted());
            assert_eq!(h.revoked_after(q), later);
        }
        assert_eq!(from_slice::<KeyHistory>(&to_vec(&h)), Ok(h));
    }
}
