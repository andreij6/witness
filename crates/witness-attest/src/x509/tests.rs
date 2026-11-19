use super::*;
use crate::testkit::{self as kit, Alg, CertSpec, Rng, TestKey, oids};
use std::vec::Vec;

fn spec(extensions: Vec<Vec<u8>>) -> CertSpec {
    CertSpec {
        version: 2,
        serial: std::vec![0x01, 0x02, 0x03],
        subject: kit::name("subject"),
        not_before: 1_700_000_000,
        not_after: 1_800_000_000,
        extensions,
        alg: None,
        tbs_alg_override: None,
    }
}

fn issue(spec: &CertSpec) -> Vec<u8> {
    let key = TestKey::p256(1);
    kit::build_cert(spec, &key, &kit::name("issuer"), &TestKey::p256(2))
}

#[test]
fn parses_fields() {
    let kd = kit::seq(&[&kit::int(1)]);
    let der = issue(&spec(std::vec![
        kit::basic_constraints(true, Some(3)),
        kit::key_usage(true),
        kit::key_description_ext(&kd),
        kit::provisioning_info_ext(),
        kit::extension(oids::SUBJECT_KEY_ID, false, &kit::octets(&[1; 20])),
    ]));
    let c = Certificate::parse(&der).unwrap();
    assert_eq!(c.der, der.as_slice());
    assert_eq!(c.sig_alg, SigAlg::EcdsaSha256);
    assert_eq!(c.serial.to_hex(), "10203");
    assert_eq!(c.not_before, 1_700_000_000);
    assert_eq!(c.not_after, 1_800_000_000);
    assert_eq!(c.spki, TestKey::p256(1).spki().as_slice());
    assert_eq!(
        c.subject,
        der::single(&kit::name("subject"), Tag::SEQUENCE).unwrap()
    );
    assert_eq!(
        c.issuer,
        der::single(&kit::name("issuer"), Tag::SEQUENCE).unwrap()
    );
    assert_eq!(
        c.basic_constraints,
        Some(BasicConstraints {
            ca: true,
            path_len: Some(3)
        })
    );
    assert!(c.is_ca());
    assert_eq!(c.key_cert_sign, Some(true));
    assert_eq!(c.key_description, Some(kd.as_slice()));
    assert!(c.provisioning_info);
    assert!(c.public_key().unwrap().is_p256());
    c.verify_signed_by(&PublicKey::from_spki(&TestKey::p256(2).spki()).unwrap())
        .unwrap();
    assert_eq!(
        c.verify_signed_by(&PublicKey::from_spki(&TestKey::p256(3).spki()).unwrap()),
        Err(Error::Signature)
    );
}

#[test]
fn defaults_without_extensions() {
    let der = issue(&spec(Vec::new()));
    let c = Certificate::parse(&der).unwrap();
    assert_eq!(c.basic_constraints, None);
    assert!(!c.is_ca());
    assert_eq!(c.key_cert_sign, None);
    assert_eq!(c.key_description, None);
    assert!(!c.provisioning_info);
}

#[test]
fn leaf_key_usage_and_non_ca_constraints() {
    let der = issue(&spec(std::vec![
        kit::basic_constraints(false, None),
        kit::key_usage(false)
    ]));
    let c = Certificate::parse(&der).unwrap();
    assert_eq!(
        c.basic_constraints,
        Some(BasicConstraints {
            ca: false,
            path_len: None
        })
    );
    assert!(!c.is_ca());
    assert_eq!(c.key_cert_sign, Some(false));
}

#[test]
fn validity_window_is_inclusive() {
    let c_der = issue(&spec(Vec::new()));
    let c = Certificate::parse(&c_der).unwrap();
    assert!(c.valid_at(1_700_000_000));
    assert!(c.valid_at(1_800_000_000));
    assert!(!c.valid_at(1_699_999_999));
    assert!(!c.valid_at(1_800_000_001));
}

#[test]
fn generalized_time_after_2049() {
    let mut s = spec(Vec::new());
    s.not_after = kit::now_secs() + 100 * 365 * 86_400;
    let c_der = issue(&s);
    assert_eq!(Certificate::parse(&c_der).unwrap().not_after, s.not_after);
}

#[test]
fn serial_normalization_from_der() {
    let cases: &[(&[u8], &str)] = &[
        (&[0x00], "0"),
        (&[0x7f], "7f"),
        (&[0x00, 0x80], "80"),
        (&[0x80], "80"),
        (&[0x00, 0x00, 0x01], "1"),
        (
            &[0xf9, 0x20, 0x09, 0xe8, 0x53, 0xb6, 0xb0, 0x45],
            "f92009e853b6b045",
        ),
        (
            &[
                0x00, 0x84, 0xa9, 0xd0, 0x29, 0x7b, 0x0e, 0xb5, 0x8a, 0xe7, 0xff, 0x0e, 0x80, 0xde,
                0x76, 0x06, 0x05,
            ],
            "84a9d0297b0eb58ae7ff0e80de760605",
        ),
    ];
    for (content, hex) in cases {
        let mut s = spec(Vec::new());
        s.serial = content.to_vec();
        let der = issue(&s);
        assert_eq!(
            Certificate::parse(&der).unwrap().serial.to_hex(),
            *hex,
            "{content:x?}"
        );
    }
}

#[test]
fn rejects_bad_serials() {
    for content in [std::vec![], std::vec![1u8; 34], std::vec![0x7f; 33]] {
        let mut s = spec(Vec::new());
        s.serial = content;
        assert_eq!(Certificate::parse(&issue(&s)), Err(Error::Serial));
    }
    let mut s = spec(Vec::new());
    let mut padded = std::vec![0u8];
    padded.extend([0x80; 32]);
    s.serial = padded;
    assert!(Certificate::parse(&issue(&s)).is_ok());
}

#[test]
fn rejects_non_v3() {
    for v in [0u64, 1, 3] {
        let mut s = spec(Vec::new());
        s.version = v;
        assert_eq!(Certificate::parse(&issue(&s)), Err(Error::Version), "{v}");
    }
}

#[test]
fn rejects_mismatched_inner_algorithm() {
    let mut s = spec(Vec::new());
    s.tbs_alg_override = Some(Alg::EcdsaSha384.identifier());
    assert_eq!(
        Certificate::parse(&issue(&s)),
        Err(Error::AlgorithmMismatch)
    );
}

#[test]
fn rejects_duplicate_and_unknown_critical_extensions() {
    let dup_cases = [
        std::vec![
            kit::basic_constraints(true, None),
            kit::basic_constraints(true, None)
        ],
        std::vec![kit::key_usage(true), kit::key_usage(true)],
        std::vec![
            kit::key_description_ext(&[0x30, 0x00]),
            kit::key_description_ext(&[0x30, 0x00])
        ],
        std::vec![kit::provisioning_info_ext(), kit::provisioning_info_ext()],
    ];
    for exts in dup_cases {
        assert_eq!(
            Certificate::parse(&issue(&spec(exts))),
            Err(Error::DuplicateExtension)
        );
    }
    let critical = kit::extension(oids::NAME_CONSTRAINTS, true, &kit::seq(&[]));
    assert_eq!(
        Certificate::parse(&issue(&spec(std::vec![critical]))),
        Err(Error::UnknownCritical)
    );
    let non_critical = kit::extension(oids::NAME_CONSTRAINTS, false, &kit::seq(&[]));
    assert!(Certificate::parse(&issue(&spec(std::vec![non_critical]))).is_ok());
}

#[test]
fn rejects_malformed_extension_bodies() {
    let explicit_false = kit::seq(&[
        &kit::oid(oids::SUBJECT_KEY_ID),
        &kit::boolean(false),
        &kit::octets(&[4, 0]),
    ]);
    assert!(Certificate::parse(&issue(&spec(std::vec![explicit_false]))).is_ok());
    let cases = [
        kit::extension(oids::BASIC_CONSTRAINTS, true, &kit::octets(&[])),
        kit::extension(
            oids::BASIC_CONSTRAINTS,
            true,
            &kit::seq(&[&kit::boolean(true), &kit::int_raw(&[0x80])]),
        ),
        kit::extension(
            oids::BASIC_CONSTRAINTS,
            true,
            &kit::seq(&[&kit::boolean(true), &kit::int(1 << 40)]),
        ),
        kit::extension(
            oids::BASIC_CONSTRAINTS,
            true,
            &kit::seq(&[&kit::boolean(true), &kit::int(1), &kit::int(1)]),
        ),
        kit::extension(
            oids::BASIC_CONSTRAINTS,
            true,
            &kit::seq(&[&kit::tlv(&[0x01], &[0x01])]),
        ),
        kit::extension(oids::KEY_USAGE, true, &kit::octets(&[1])),
        kit::extension(oids::KEY_USAGE, true, &kit::tlv(&[0x03], &[0x09, 0x00])),
        kit::seq(&[&kit::oid(oids::KEY_USAGE)]),
        kit::seq(&[
            &kit::oid(oids::KEY_USAGE),
            &kit::tlv(&[0x01], &[0x01]),
            &kit::octets(&[]),
        ]),
        kit::seq(&[&kit::oid(oids::KEY_USAGE), &kit::octets(&[]), &kit::null()]),
    ];
    for ext in cases {
        assert_eq!(
            Certificate::parse(&issue(&spec(std::vec![ext.clone()]))),
            Err(Error::Der),
            "{ext:x?}"
        );
    }
}

#[test]
fn key_usage_bit_positions() {
    let ku = |bits: &[u8]| kit::extension(oids::KEY_USAGE, true, &kit::tlv(&[0x03], bits));
    let cases: [(&[u8], Option<bool>); 5] = [
        (&[0x02, 0x04], Some(true)),
        (&[0x07, 0x80], Some(false)),
        (&[0x00], Some(false)),
        (&[0x00, 0xfb, 0xff], Some(false)),
        (&[0x00, 0x04, 0x00], Some(true)),
    ];
    for (bits, want) in cases {
        let c_der = issue(&spec(std::vec![ku(bits)]));
        assert_eq!(
            Certificate::parse(&c_der).unwrap().key_cert_sign,
            want,
            "{bits:x?}"
        );
    }
}

#[test]
fn rejects_empty_extension_list_and_trailing_data() {
    let mut der = issue(&spec(Vec::new()));
    assert!(Certificate::parse(&der).is_ok());
    der.push(0);
    assert_eq!(Certificate::parse(&der), Err(Error::Der));
    let empty = TestKey::p256(1);
    let tbs = kit::seq(&[
        &kit::explicit(0, &kit::int(2)),
        &kit::int(1),
        &Alg::EcdsaSha256.identifier(),
        &kit::name("i"),
        &kit::seq(&[&kit::time(0), &kit::time(1)]),
        &kit::name("s"),
        &empty.spki(),
        &kit::explicit(3, &kit::seq(&[])),
    ]);
    let cert = kit::seq(&[
        &tbs,
        &Alg::EcdsaSha256.identifier(),
        &kit::bits(&[0x30, 0x00]),
    ]);
    assert_eq!(Certificate::parse(&cert), Err(Error::Der));
}

#[test]
fn accepts_unique_ids() {
    let k = TestKey::p256(1);
    let tbs = kit::seq(&[
        &kit::explicit(0, &kit::int(2)),
        &kit::int(1),
        &Alg::EcdsaSha256.identifier(),
        &kit::name("i"),
        &kit::seq(&[&kit::time(0), &kit::time(1)]),
        &kit::name("s"),
        &k.spki(),
        &kit::tlv(&[0x81], &[0x00, 0xaa]),
        &kit::tlv(&[0x82], &[0x00, 0xbb]),
    ]);
    let cert = kit::seq(&[
        &tbs,
        &Alg::EcdsaSha256.identifier(),
        &kit::bits(&[0x30, 0x00]),
    ]);
    assert!(Certificate::parse(&cert).is_ok());
}

#[test]
fn rejects_bad_validity() {
    let k = TestKey::p256(1);
    let make = |validity: Vec<u8>| {
        let tbs = kit::seq(&[
            &kit::explicit(0, &kit::int(2)),
            &kit::int(1),
            &Alg::EcdsaSha256.identifier(),
            &kit::name("i"),
            &validity,
            &kit::name("s"),
            &k.spki(),
        ]);
        kit::seq(&[
            &tbs,
            &Alg::EcdsaSha256.identifier(),
            &kit::bits(&[0x30, 0x00]),
        ])
    };
    assert_eq!(
        Certificate::parse(&make(kit::seq(&[&kit::time(0)]))),
        Err(Error::Der)
    );
    assert_eq!(
        Certificate::parse(&make(kit::seq(&[
            &kit::time(0),
            &kit::time(1),
            &kit::time(2)
        ]))),
        Err(Error::Der)
    );
    assert_eq!(
        Certificate::parse(&make(kit::seq(&[
            &kit::tlv(&[0x17], b"991399000000Z"),
            &kit::time(1)
        ]))),
        Err(Error::Time)
    );
    assert_eq!(
        Certificate::parse(&make(kit::seq(&[
            &kit::octets(b"700101000000Z"),
            &kit::time(1)
        ]))),
        Err(Error::Time)
    );
}

#[test]
fn rejects_unaligned_signature_bits() {
    let mut der = issue(&spec(Vec::new()));
    let sig_start = der.len() - {
        let c = Certificate::parse(&der).unwrap();
        c.signature.len() + 1
    };
    der[sig_start] = 1;
    assert!(Certificate::parse(&der).is_err());
}

#[test]
fn every_truncation_and_bit_flip_is_handled() {
    let der = issue(&spec(std::vec![
        kit::basic_constraints(true, Some(1)),
        kit::key_usage(true)
    ]));
    for cut in 0..der.len() {
        assert!(Certificate::parse(&der[..cut]).is_err(), "cut {cut}");
    }
    let mut rng = Rng::new(17);
    let signer = PublicKey::from_spki(&TestKey::p256(2).spki()).unwrap();
    for _ in 0..3000 {
        let mut m = der.clone();
        let i = rng.below(m.len() as u64) as usize;
        m[i] ^= 1 << rng.below(8);
        if let Ok(c) = Certificate::parse(&m) {
            let _ = c.verify_signed_by(&signer);
        }
    }
}
