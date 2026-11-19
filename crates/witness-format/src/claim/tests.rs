use super::*;
use crate::cbor::{from_slice, to_vec};
use crate::crypto::sha256;
use crate::edit::{BlurClass, Rect};
use crate::evidence::{BootState, Confidence, GnssChecks, LocationReasons, SecurityLevel};
use crate::testutil::Rng;
use alloc::string::ToString;
use alloc::vec;

fn hex(s: &str) -> Vec<u8> {
    let s: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    s.chunks(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

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

fn original() -> WitnessClaim {
    WitnessClaim {
        capture_id: CaptureId::new(0x0002_0304_0506_0708, 1).unwrap(),
        kind: Kind::Original,
        asset_hash: [0xaa; 32],
        pixel_hash: [0xbb; 32],
        fp_commit: [0xcc; 32],
        dims: Dims {
            width: 4032,
            height: 3024,
        },
        time: TimeEvidence {
            wall_ms: 1_700_000_000_000,
            gnss_ms: None,
            elapsed_ns: 5_000_000_000,
            boot_count: 3,
        },
        loc: LocationEvidence {
            commit: Some([0x11; 32]),
            confidence: Confidence::High,
            reasons: LocationReasons::RAW_GNSS_PASSED,
        },
        checks: checks(),
        key: KeyRef {
            key_id: KeyId::from_bytes([0x22; 16]),
            counter: 42,
        },
        cert_serial: CertSerial::from_u64(0x1234).unwrap(),
        coarse: None,
    }
}

fn derivative() -> WitnessClaim {
    WitnessClaim {
        kind: Kind::Derivative {
            parent: RecordId::from_bytes([0x44; 10]),
            root: CaptureId::new(7, 0).unwrap(),
            edits: vec![
                Edit::Resize {
                    w: 1080,
                    h: 1350,
                    filter: 1,
                },
                Edit::Blur {
                    rect: Rect {
                        x: 1,
                        y: 2,
                        w: 3,
                        h: 4,
                    },
                    class: BlurClass::Face,
                    sigma_e2: 300,
                },
            ],
        },
        loc: LocationEvidence {
            commit: None,
            confidence: Confidence::Medium,
            reasons: LocationReasons::FUSED_FIX,
        },
        coarse: Geohash5::new("9q8yy"),
        ..original()
    }
}

fn original_golden() -> Vec<u8> {
    let mut v = hex("ac 0002 01 1b0102030405060708 02 a10000 03 5820");
    v.extend([0xaa; 32]);
    v.extend(hex("04 5820"));
    v.extend([0xbb; 32]);
    v.extend(hex("05 5820"));
    v.extend([0xcc; 32]);
    v.extend(hex("06 82 190fc0 190bd0"));
    v.extend(hex(
        "07 a3 00 1b0000018bcfe56800 02 1b000000012a05f200 03 03",
    ));
    v.extend(hex("08 a3 00 5820"));
    v.extend([0x11; 32]);
    v.extend(hex("01 00 02 01"));
    v.extend(hex("09 1a000c0310"));
    v.extend(hex("0a 82 50"));
    v.extend([0x22; 16]);
    v.extend(hex("182a"));
    v.extend(hex("0b 421234"));
    v
}

#[test]
fn original_golden_bytes() {
    let c = original();
    assert_eq!(to_vec(&c), original_golden());
    assert_eq!(c.to_cbor(), Ok(original_golden()));
    assert_eq!(WitnessClaim::from_cbor(&original_golden()), Ok(c));
}

#[test]
fn original_golden_record_id() {
    let c = original();
    let id = c.record_id();
    assert_eq!(id.as_bytes()[..], sha256(&original_golden())[..10]);
    assert_eq!(id, RecordId::from_claim_cbor(&original_golden()));
    assert_eq!(id.to_string(), "SYXJWEC36ZHC9FGC");
}

#[test]
fn derivative_golden_kind_and_tail() {
    let d = derivative();
    let enc = to_vec(&d);
    assert_eq!(enc[0], 0xad);
    let kind = to_vec(&d.kind);
    let mut want = hex("a4 0001 01 4a");
    want.extend([0x44; 10]);
    want.extend(hex("02 07 03 82"));
    want.extend(hex("a2 0000 01 a3 00190438 01190546 0201"));
    want.extend(hex(
        "a2 0007 01 a3 00 a4 0001 0102 0203 0304 01 00 02 19012c",
    ));
    assert_eq!(kind, want);
    assert!(enc.ends_with(&hex("0c 65 3971387979")));
    assert_eq!(WitnessClaim::from_cbor(&enc), Ok(d));
}

#[test]
fn kind_helpers() {
    assert!(Kind::Original.is_original());
    assert!(Kind::Original.edits().is_empty());
    assert!(Kind::Original.presentation_only());
    let d = derivative().kind;
    assert!(!d.is_original());
    assert_eq!(d.edits().len(), 2);
    assert!(!d.presentation_only());
    let p = Kind::Derivative {
        parent: RecordId::from_bytes([0; 10]),
        root: CaptureId::new(0, 0).unwrap(),
        edits: vec![Edit::Orient { orientation: 1 }],
    };
    assert!(p.presentation_only());
}

#[test]
fn kind_rejects_malformed() {
    assert_eq!(from_slice::<Kind>(&hex("a1 0002")), Err(Error::Range));
    assert_eq!(
        from_slice::<Kind>(&hex("a2 0000 01 00")),
        Err(Error::UnknownKey(1))
    );
    assert_eq!(from_slice::<Kind>(&hex("a0")), Err(Error::MissingKey(0)));
    let mut no_edits = hex("a3 0001 01 4a");
    no_edits.extend([0; 10]);
    no_edits.extend(hex("02 00"));
    assert_eq!(from_slice::<Kind>(&no_edits), Err(Error::MissingKey(3)));
    let mut big_root = hex("a4 0001 01 4a");
    big_root.extend([0; 10]);
    big_root.extend(hex("02 1b2000000000000000 03 80"));
    assert_eq!(from_slice::<Kind>(&big_root), Err(Error::Range));
    let mut too_many = hex("a4 0001 01 4a");
    too_many.extend([0; 10]);
    too_many.extend(hex("02 00 03 98 41"));
    for _ in 0..65 {
        too_many.extend(hex("a2 0002 01 a1 0001"));
    }
    assert_eq!(from_slice::<Kind>(&too_many), Err(Error::Length));
}

#[test]
fn claim_semantic_validation() {
    let mut c = original();
    c.coarse = Geohash5::new("u4pru");
    assert_eq!(c.validate(), Err(Error::Invalid));
    assert_eq!(c.to_cbor(), Err(Error::Invalid));
    assert_eq!(from_slice::<WitnessClaim>(&to_vec(&c)), Err(Error::Invalid));
    let mut d = derivative();
    d.loc.commit = Some([0; 32]);
    assert_eq!(d.validate(), Err(Error::Invalid));
    assert_eq!(from_slice::<WitnessClaim>(&to_vec(&d)), Err(Error::Invalid));
    let mut z = original();
    z.dims.width = 0;
    assert_eq!(z.validate(), Err(Error::Invalid));
    assert_eq!(from_slice::<WitnessClaim>(&to_vec(&z)), Err(Error::Invalid));
    let mut many = derivative();
    if let Kind::Derivative { edits, .. } = &mut many.kind {
        edits.resize(MAX_EDITS + 1, Edit::Orient { orientation: 0 });
    }
    assert_eq!(many.validate(), Err(Error::Length));
    let mut nocoarse = derivative();
    nocoarse.coarse = None;
    assert_eq!(nocoarse.validate(), Ok(()));
}

#[test]
fn claim_rejects_structural_errors() {
    let g = original_golden();
    let mut version = g.clone();
    version[2] = 0x01;
    assert_eq!(WitnessClaim::from_cbor(&version), Err(Error::Version(1)));
    let mut extra = g.clone();
    extra[0] = 0xad;
    extra.extend(hex("0d 00"));
    assert_eq!(WitnessClaim::from_cbor(&extra), Err(Error::UnknownKey(13)));
    let mut trailing = g.clone();
    trailing.push(0xf6);
    assert_eq!(WitnessClaim::from_cbor(&trailing), Err(Error::Trailing));
    let mut missing = g.clone();
    missing[0] = 0xab;
    let cut = missing.len() - 4;
    missing.truncate(cut);
    assert_eq!(
        WitnessClaim::from_cbor(&missing),
        Err(Error::MissingKey(11))
    );
    let mut float = g;
    let at = float.iter().position(|&b| b == 0x1a).unwrap();
    float[at] = 0xfa;
    assert_eq!(WitnessClaim::from_cbor(&float), Err(Error::Float));
    for cut in 0..original_golden().len() {
        assert!(WitnessClaim::from_cbor(&original_golden()[..cut]).is_err());
    }
}

#[test]
fn record_id_depends_on_every_field() {
    let base = original().record_id();
    let variants = [
        WitnessClaim {
            capture_id: CaptureId::new(1, 1).unwrap(),
            ..original()
        },
        WitnessClaim {
            asset_hash: [0; 32],
            ..original()
        },
        WitnessClaim {
            pixel_hash: [0; 32],
            ..original()
        },
        WitnessClaim {
            fp_commit: [0; 32],
            ..original()
        },
        WitnessClaim {
            dims: Dims {
                width: 3024,
                height: 4032,
            },
            ..original()
        },
        WitnessClaim {
            time: TimeEvidence {
                boot_count: 4,
                ..original().time
            },
            ..original()
        },
        WitnessClaim {
            loc: LocationEvidence {
                commit: None,
                ..original().loc
            },
            ..original()
        },
        WitnessClaim {
            checks: Checks {
                recapture: 13,
                ..checks()
            },
            ..original()
        },
        WitnessClaim {
            key: KeyRef {
                counter: 43,
                ..original().key
            },
            ..original()
        },
        WitnessClaim {
            cert_serial: CertSerial::from_u64(1).unwrap(),
            ..original()
        },
        derivative(),
    ];
    for v in variants {
        assert_ne!(v.record_id(), base);
    }
}

#[test]
fn cert_serial_rules() {
    assert_eq!(CertSerial::new(&[]), None);
    assert_eq!(CertSerial::new(&[0]), None);
    assert_eq!(CertSerial::new(&[0, 1]), None);
    assert_eq!(CertSerial::new(&[1; 21]), None);
    assert!(CertSerial::new(&[0x80; 20]).is_some());
    assert_eq!(CertSerial::from_u64(0), None);
    assert_eq!(CertSerial::from_u64(1).unwrap().as_bytes(), &[1]);
    assert_eq!(CertSerial::from_u64(0x100).unwrap().as_bytes(), &[1, 0]);
    assert_eq!(
        CertSerial::from_u64(u64::MAX).unwrap().as_bytes(),
        &[0xff; 8]
    );
    assert_eq!(
        alloc::format!("{:?}", CertSerial::from_u64(0x0abc).unwrap()),
        "CertSerial(0abc)"
    );
    assert_eq!(
        from_slice::<CertSerial>(&hex("420012")),
        Err(Error::Invalid)
    );
    assert_eq!(from_slice::<CertSerial>(&hex("40")), Err(Error::Invalid));
    let mut long = hex("55");
    long.extend([1; 21]);
    assert_eq!(from_slice::<CertSerial>(&long), Err(Error::Invalid));
}

#[test]
fn geohash_rules() {
    let g = Geohash5::new("9q8yy").unwrap();
    assert_eq!(g.as_str(), "9q8yy");
    assert_eq!(alloc::format!("{g}"), "9q8yy");
    assert_eq!(alloc::format!("{g:?}"), "Geohash5(9q8yy)");
    for bad in [
        "9q8ya",
        "9q8y",
        "9q8yyy",
        "9Q8YY",
        "9q8yi",
        "9q8yl",
        "9q8yo",
        "",
        "9q8y\u{e9}",
    ] {
        assert_eq!(Geohash5::new(bad), None, "{bad}");
    }
    assert_eq!(
        from_slice::<Geohash5>(&hex("65 3971387961")),
        Err(Error::Invalid)
    );
    assert_eq!(from_slice::<Geohash5>(&to_vec(&g)), Ok(g));
}

#[test]
fn dims_and_keyref_codecs() {
    assert_eq!(from_slice::<Dims>(&hex("82 00 01")), Err(Error::Invalid));
    assert_eq!(from_slice::<Dims>(&hex("82 01 00")), Err(Error::Invalid));
    assert_eq!(from_slice::<Dims>(&hex("83 01 01 01")), Err(Error::Length));
    assert_eq!(
        from_slice::<Dims>(&hex("82 01 01")),
        Ok(Dims {
            width: 1,
            height: 1
        })
    );
    let k = KeyRef {
        key_id: KeyId::from_bytes([1; 16]),
        counter: u64::MAX,
    };
    assert_eq!(from_slice::<KeyRef>(&to_vec(&k)), Ok(k));
    assert_eq!(from_slice::<KeyRef>(&hex("82 4100 00")), Err(Error::Length));
}

fn submission() -> Submission {
    Submission {
        claim_cbor: original_golden(),
        c2pa_claim_cbor: vec![0xa0],
        cose: vec![0x84, 0x40, 0xa0, 0xf6, 0x40],
    }
}

#[test]
fn submission_roundtrip_and_accessors() {
    let s = submission();
    let enc = to_vec(&s);
    assert_eq!(&enc[..5], &hex("a4 0001 01 58"));
    assert_eq!(usize::from(enc[5]), original_golden().len());
    assert_eq!(
        crate::cbor::from_slice_canonical::<Submission>(&enc),
        Ok(s.clone())
    );
    assert_eq!(s.record_id(), original().record_id());
    assert_eq!(s.claim(), Ok(original()));
    assert!(s.within_limits());
}

#[test]
fn submission_limits() {
    let mut empty = submission();
    empty.cose.clear();
    assert!(!empty.within_limits());
    assert_eq!(
        from_slice::<Submission>(&to_vec(&empty)),
        Err(Error::Length)
    );
    let mut big = submission();
    big.c2pa_claim_cbor = vec![0; Submission::MAX_PART + 1];
    assert!(!big.within_limits());
    assert_eq!(from_slice::<Submission>(&to_vec(&big)), Err(Error::Length));
    let mut edge = submission();
    edge.c2pa_claim_cbor = vec![0; Submission::MAX_PART];
    assert!(edge.within_limits());
    let mut bad_claim = submission();
    bad_claim.claim_cbor = vec![0xa0];
    assert_eq!(bad_claim.claim(), Err(Error::MissingKey(0)));
    let mut version = to_vec(&submission());
    version[2] = 0x02;
    assert_eq!(from_slice::<Submission>(&version), Err(Error::Version(2)));
}

fn random_claim(rng: &mut Rng) -> WitnessClaim {
    let derivative = rng.bool();
    let kind = if derivative {
        let n = rng.below(5);
        Kind::Derivative {
            parent: RecordId::from_bytes(rng.bytes()),
            root: CaptureId::from_u64(rng.next() >> 3).unwrap(),
            edits: (0..n)
                .map(|_| match rng.below(3) {
                    0 => Edit::Orient {
                        orientation: rng.below(8) as u8,
                    },
                    1 => Edit::Tone {
                        exposure_e2: rng.next() as i16,
                        contrast_e2: rng.next() as i16,
                        saturation_e2: rng.next() as i16,
                    },
                    _ => Edit::Crop(Rect {
                        x: rng.below(100) as u32,
                        y: rng.below(100) as u32,
                        w: 1 + rng.below(100) as u32,
                        h: 1 + rng.below(100) as u32,
                    }),
                })
                .collect(),
        }
    } else {
        Kind::Original
    };
    WitnessClaim {
        capture_id: CaptureId::from_u64(rng.next() >> 3).unwrap(),
        kind,
        asset_hash: rng.bytes(),
        pixel_hash: rng.bytes(),
        fp_commit: rng.bytes(),
        dims: Dims {
            width: 1 + rng.below(10_000) as u32,
            height: 1 + rng.below(10_000) as u32,
        },
        time: TimeEvidence {
            wall_ms: rng.next() >> rng.below(64),
            gnss_ms: rng.bool().then(|| rng.next() >> rng.below(64)),
            elapsed_ns: rng.next(),
            boot_count: rng.next() as u32,
        },
        loc: LocationEvidence {
            commit: (!derivative && rng.bool()).then(|| rng.bytes()),
            confidence: Confidence::ALL[rng.below(3) as usize],
            reasons: LocationReasons::from_bits_truncate(rng.next() as u16),
        },
        checks: Checks {
            recapture: rng.next() as u8,
            ..checks()
        },
        key: KeyRef {
            key_id: KeyId::from_bytes(rng.bytes()),
            counter: rng.next() >> rng.below(64),
        },
        cert_serial: CertSerial::from_u64(1 + (rng.next() >> 1)).unwrap(),
        coarse: (derivative && rng.bool()).then(|| Geohash5::new("gcpvj").unwrap()),
    }
}

#[test]
fn property_random_claims_roundtrip() {
    let mut rng = Rng::new(60);
    for _ in 0..1000 {
        let c = random_claim(&mut rng);
        let enc = c.to_cbor().unwrap();
        let back = WitnessClaim::from_cbor(&enc).unwrap();
        assert_eq!(back, c);
        assert_eq!(back.record_id(), RecordId::from_claim_cbor(&enc));
    }
}

#[test]
fn property_mutated_claims_never_panic() {
    let mut rng = Rng::new(61);
    let encs: Vec<Vec<u8>> = (0..20).map(|_| to_vec(&random_claim(&mut rng))).collect();
    for i in 0..20_000 {
        let mut m = encs[i % encs.len()].clone();
        for _ in 0..1 + rng.below(3) {
            let at = rng.below(m.len() as u64) as usize;
            m[at] = rng.next() as u8;
        }
        if let Ok(c) = from_slice::<WitnessClaim>(&m) {
            assert_eq!(to_vec(&c), m);
            assert_eq!(c.validate(), Ok(()));
        }
    }
}
