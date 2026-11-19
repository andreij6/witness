use super::*;
use crate::cbor::{from_slice, from_slice_canonical, to_vec};
use crate::crypto::sha256;
use crate::testutil::Rng;
use alloc::vec::Vec;

fn checks() -> Checks {
    Checks {
        key_security: SecurityLevel::StrongBox,
        boot: BootState::Verified,
        device_locked: true,
        mock_location: false,
        gnss: GnssChecks::RAW_AVAILABLE | GnssChecks::CN0_CONSISTENT,
        recapture: 12,
    }
}

#[test]
fn time_golden_and_optional_gnss() {
    let t = TimeEvidence {
        wall_ms: 1000,
        gnss_ms: None,
        elapsed_ns: 5,
        boot_count: 2,
    };
    assert_eq!(
        to_vec(&t),
        [0xa3, 0x00, 0x19, 0x03, 0xe8, 0x02, 0x05, 0x03, 0x02]
    );
    assert_eq!(from_slice_canonical::<TimeEvidence>(&to_vec(&t)), Ok(t));
    let g = TimeEvidence {
        gnss_ms: Some(1001),
        ..t
    };
    assert_eq!(
        to_vec(&g),
        [
            0xa4, 0x00, 0x19, 0x03, 0xe8, 0x01, 0x19, 0x03, 0xe9, 0x02, 0x05, 0x03, 0x02
        ]
    );
    assert_eq!(from_slice_canonical::<TimeEvidence>(&to_vec(&g)), Ok(g));
}

#[test]
fn time_clock_skew() {
    let t = TimeEvidence {
        wall_ms: 1000,
        gnss_ms: None,
        elapsed_ns: 0,
        boot_count: 0,
    };
    assert_eq!(t.clock_skew_ms(), None);
    assert_eq!(
        TimeEvidence {
            gnss_ms: Some(400),
            ..t
        }
        .clock_skew_ms(),
        Some(600)
    );
    assert_eq!(
        TimeEvidence {
            gnss_ms: Some(1600),
            ..t
        }
        .clock_skew_ms(),
        Some(600)
    );
    assert_eq!(
        TimeEvidence {
            wall_ms: 0,
            gnss_ms: Some(u64::MAX),
            ..t
        }
        .clock_skew_ms(),
        Some(u64::MAX)
    );
}

#[test]
fn time_rejects_malformed() {
    assert_eq!(
        from_slice::<TimeEvidence>(&[0xa2, 0x00, 0x01, 0x02, 0x01]),
        Err(Error::MissingKey(3))
    );
    assert_eq!(
        from_slice::<TimeEvidence>(&[0xa4, 0x00, 0x01, 0x02, 0x01, 0x03, 0x01, 0x04, 0x00]),
        Err(Error::UnknownKey(4))
    );
    assert_eq!(
        from_slice::<TimeEvidence>(&[
            0xa3, 0x00, 0x01, 0x02, 0x01, 0x03, 0x1a, 0xff, 0xff, 0xff, 0xff
        ]),
        Ok(TimeEvidence {
            wall_ms: 1,
            gnss_ms: None,
            elapsed_ns: 1,
            boot_count: u32::MAX
        })
    );
    assert_eq!(
        from_slice::<TimeEvidence>(&[
            0xa3, 0x00, 0x01, 0x02, 0x01, 0x03, 0x1b, 0, 0, 0, 1, 0, 0, 0, 0
        ]),
        Err(Error::Range)
    );
}

#[test]
fn location_commit_golden_matches_formula() {
    let salt = [7u8; 32];
    let mut pre = Vec::new();
    pre.extend_from_slice(b"witness/loc/v1");
    pre.extend_from_slice(&salt);
    pre.extend_from_slice(&[0x1d, 0xcd, 0x65, 0x00]);
    pre.extend_from_slice(&[0xf8, 0xa4, 0x32, 0xeb]);
    pre.extend_from_slice(&[0x00, 0x00, 0x01, 0xf4]);
    assert_eq!(
        location_commit(&salt, 500_000_000, -123_456_789, 500),
        sha256(&pre)
    );
}

#[test]
fn location_commit_matches_the_kotlin_preimage_vector() {
    let pre = "7769746e6573732f6c6f632f763107070707070707070707070707070707070707070707070707070707070707071dcd6500f8a432eb000001f4";
    let bytes: Vec<u8> = (0..pre.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&pre[i..i + 2], 16).unwrap())
        .collect();
    let commit = location_commit(&[7; 32], 500_000_000, -123_456_789, 500);
    assert_eq!(commit, sha256(&bytes));
    let hex: alloc::string::String = commit.iter().map(|b| alloc::format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        "8ac44dbe7f6cf58dc115e45ae9f2f733ef0b835c7ff27b39b47bc905715649c7"
    );
}

#[test]
fn location_commit_binds_every_field() {
    let base = LocationOpening {
        salt: [1; 32],
        lat_e7: 10,
        lon_e7: 20,
        acc_cm: 30,
    };
    let c = base.commit();
    assert!(base.verify(&c));
    let mut other_salt = base;
    other_salt.salt[31] ^= 1;
    for o in [
        other_salt,
        LocationOpening { lat_e7: 11, ..base },
        LocationOpening { lon_e7: 21, ..base },
        LocationOpening { acc_cm: 31, ..base },
        LocationOpening {
            lat_e7: 20,
            lon_e7: 10,
            ..base
        },
    ] {
        assert!(!o.verify(&c));
        assert_ne!(o.commit(), c);
    }
    let mut flipped = c;
    flipped[0] ^= 0x80;
    assert!(!base.verify(&flipped));
}

#[test]
fn location_opening_range_checks() {
    let ok = LocationOpening {
        salt: [0; 32],
        lat_e7: LAT_E7_MAX,
        lon_e7: -LON_E7_MAX,
        acc_cm: u32::MAX,
    };
    assert!(ok.in_range());
    assert!(ok.verify(&ok.commit()));
    for bad in [
        LocationOpening {
            lat_e7: LAT_E7_MAX + 1,
            ..ok
        },
        LocationOpening {
            lat_e7: -LAT_E7_MAX - 1,
            ..ok
        },
        LocationOpening {
            lon_e7: LON_E7_MAX + 1,
            ..ok
        },
        LocationOpening {
            lon_e7: i32::MIN,
            ..ok
        },
    ] {
        assert!(!bad.in_range());
        assert!(!bad.verify(&bad.commit()));
        assert_eq!(
            from_slice::<LocationOpening>(&to_vec(&bad)),
            Err(Error::Range)
        );
    }
}

#[test]
fn location_opening_roundtrip_and_debug_hides_secret() {
    let o = LocationOpening {
        salt: [9; 32],
        lat_e7: -1,
        lon_e7: 1,
        acc_cm: 0,
    };
    let b = to_vec(&o);
    assert_eq!(&b[..4], &[0xa5, 0x00, 0x01, 0x01]);
    assert_eq!(from_slice_canonical::<LocationOpening>(&b), Ok(o));
    let dbg = alloc::format!("{o:?}");
    assert!(!dbg.contains('9'));
    assert!(!dbg.contains("lat"));
    let mut wrong_version = b.clone();
    wrong_version[2] = 0x02;
    assert_eq!(
        from_slice::<LocationOpening>(&wrong_version),
        Err(Error::Version(2))
    );
}

#[test]
fn location_evidence_golden_and_roundtrip() {
    let l = LocationEvidence {
        commit: None,
        confidence: Confidence::Medium,
        reasons: LocationReasons::FUSED_FIX | LocationReasons::INDOOR,
    };
    assert_eq!(to_vec(&l), [0xa2, 0x01, 0x01, 0x02, 0x19, 0x04, 0x02]);
    assert_eq!(from_slice_canonical::<LocationEvidence>(&to_vec(&l)), Ok(l));
    let c = LocationEvidence {
        commit: Some([0xab; 32]),
        confidence: Confidence::High,
        reasons: LocationReasons::RAW_GNSS_PASSED,
    };
    let b = to_vec(&c);
    assert_eq!(&b[..4], &[0xa3, 0x00, 0x58, 0x20]);
    assert_eq!(from_slice_canonical::<LocationEvidence>(&b), Ok(c));
}

#[test]
fn location_evidence_rejects_bad_fields() {
    assert_eq!(
        from_slice::<LocationEvidence>(&[0xa2, 0x01, 0x03, 0x02, 0x00]),
        Err(Error::Range)
    );
    assert_eq!(
        from_slice::<LocationEvidence>(&[0xa2, 0x01, 0x00, 0x02, 0x19, 0x08, 0x00]),
        Err(Error::Range)
    );
    assert_eq!(
        from_slice::<LocationEvidence>(&[0xa3, 0x00, 0x41, 0x00, 0x01, 0x00, 0x02, 0x00]),
        Err(Error::Length)
    );
    assert_eq!(
        from_slice::<LocationEvidence>(&[0xa1, 0x02, 0x00]),
        Err(Error::MissingKey(1))
    );
}

#[test]
fn checks_golden_bits() {
    let c = checks();
    assert_eq!(c.to_bits(), 0x000c_0310);
    assert_eq!(to_vec(&c), [0x1a, 0x00, 0x0c, 0x03, 0x10]);
    assert_eq!(Checks::from_bits(0x000c_0310), Some(c));
    let all = Checks {
        key_security: SecurityLevel::Dev,
        boot: BootState::Failed,
        device_locked: true,
        mock_location: true,
        gnss: GnssChecks::ALL,
        recapture: 255,
    };
    assert_eq!(all.to_bits(), 0x00ff_7f3e);
    assert_eq!(Checks::from_bits(all.to_bits()), Some(all));
}

#[test]
fn checks_reject_reserved_and_unknown_codes() {
    let base = checks().to_bits();
    for bit in [6u32, 7, 15, 24, 25, 31] {
        assert_eq!(Checks::from_bits(base | (1 << bit)), None, "bit {bit}");
    }
    assert_eq!(Checks::from_bits(base | 3), None);
    assert_eq!(
        from_slice::<Checks>(&to_vec(&u64::from(base | 1 << 31))),
        Err(Error::Range)
    );
    assert_eq!(
        from_slice::<Checks>(&to_vec(&(1u64 << 32))),
        Err(Error::Range)
    );
}

#[test]
fn checks_boot_trust() {
    assert!(checks().boot_trusted());
    assert!(
        !Checks {
            device_locked: false,
            ..checks()
        }
        .boot_trusted()
    );
    assert!(
        !Checks {
            boot: BootState::Unverified,
            ..checks()
        }
        .boot_trusted()
    );
    assert!(
        !Checks {
            boot: BootState::SelfSigned,
            ..checks()
        }
        .boot_trusted()
    );
}

#[test]
fn property_checks_exhaustive_valid_space() {
    let mut rng = Rng::new(40);
    for _ in 0..5000 {
        let c = Checks {
            key_security: SecurityLevel::ALL[rng.below(3) as usize],
            boot: BootState::ALL[rng.below(4) as usize],
            device_locked: rng.bool(),
            mock_location: rng.bool(),
            gnss: GnssChecks::from_bits_truncate(rng.next() as u8),
            recapture: rng.next() as u8,
        };
        assert_eq!(Checks::from_bits(c.to_bits()), Some(c));
        assert_eq!(from_slice_canonical::<Checks>(&to_vec(&c)), Ok(c));
    }
    for _ in 0..20_000 {
        let bits = rng.next() as u32;
        if let Some(c) = Checks::from_bits(bits) {
            assert_eq!(c.to_bits(), bits);
        }
    }
}

#[test]
fn reasons_bitset_bounds() {
    assert_eq!(LocationReasons::ALL.bits(), 0x07ff);
    assert_eq!(LocationReasons::from_bits(0x0800), None);
    assert_eq!(GnssChecks::ALL.bits(), 0x7f);
    assert_eq!(GnssChecks::from_bits(0x80), None);
    assert_eq!(Confidence::from_code(3), None);
    assert_eq!(SecurityLevel::from_code(3), None);
    assert_eq!(BootState::from_code(4), None);
}
