use super::*;
use crate::testkit::*;
use crate::x509::{TrustAnchor, verify_leaf};
use alloc::string::ToString;
use ed25519_dalek::Signer as _;
use x509_cert::Certificate;

fn ext(c: &Certificate, id: ObjectIdentifier) -> &Extension {
    c.tbs_certificate
        .extensions
        .as_ref()
        .unwrap()
        .iter()
        .find(|e| e.extn_id == id)
        .unwrap()
}

#[test]
fn root_has_ca_profile() {
    let c = Certificate::from_der(&root_der()).unwrap();
    let t = &c.tbs_certificate;
    assert_eq!(t.version, Version::V3);
    assert_eq!(t.issuer, t.subject);
    assert_eq!(t.serial_number.as_bytes(), &[1]);
    assert_eq!(
        t.validity.not_before.to_unix_duration().as_secs(),
        NOT_BEFORE
    );
    assert_eq!(t.validity.not_after.to_unix_duration().as_secs(), NOT_AFTER);
    let bc = ext(&c, oid::BASIC_CONSTRAINTS);
    assert!(bc.critical);
    let bc = BasicConstraints::from_der(bc.extn_value.as_bytes()).unwrap();
    assert!(bc.ca);
    assert_eq!(bc.path_len_constraint, Some(0));
    let ku = ext(&c, oid::KEY_USAGE);
    assert!(ku.critical);
    let ku = KeyUsage::from_der(ku.extn_value.as_bytes()).unwrap();
    assert!(ku.key_cert_sign() && ku.crl_sign() && !ku.digital_signature());
    assert!(!ext(&c, oid::SUBJECT_KEY_ID).critical);
}

#[test]
fn leaf_has_c2pa_signing_profile() {
    let c = Certificate::from_der(&leaf_der()).unwrap();
    let t = &c.tbs_certificate;
    assert_eq!(t.issuer, *anchor().subject());
    assert_eq!(t.subject.to_string(), "CN=Witness Device Key,O=Witness");
    let ku = ext(&c, oid::KEY_USAGE);
    assert!(ku.critical);
    let ku = KeyUsage::from_der(ku.extn_value.as_bytes()).unwrap();
    assert!(ku.digital_signature() && !ku.key_cert_sign());
    let eku =
        ExtendedKeyUsage::from_der(ext(&c, oid::EXT_KEY_USAGE).extn_value.as_bytes()).unwrap();
    assert_eq!(eku.0, alloc::vec![oid::KP_DOCUMENT_SIGNING]);
    let bc =
        BasicConstraints::from_der(ext(&c, oid::BASIC_CONSTRAINTS).extn_value.as_bytes()).unwrap();
    assert!(!bc.ca);
    let aki =
        AuthorityKeyIdentifier::from_der(ext(&c, oid::AUTHORITY_KEY_ID).extn_value.as_bytes())
            .unwrap();
    assert_eq!(aki.key_identifier.unwrap().as_bytes(), anchor().key_id());
    let ski =
        SubjectKeyIdentifier::from_der(ext(&c, oid::SUBJECT_KEY_ID).extn_value.as_bytes()).unwrap();
    assert_eq!(
        ski.0.as_bytes(),
        key_identifier(t.subject_public_key_info.subject_public_key.raw_bytes())
    );
    assert_eq!(
        t.subject_public_key_info.to_der().unwrap(),
        spki(&device_key())
    );
}

#[test]
fn build_is_deterministic_and_two_phase_equivalent() {
    assert_eq!(root_der(), root_der());
    let tbs = leaf_tbs(&leaf_params(), &spki(&device_key()), &anchor()).unwrap();
    let sig = root_key().sign(&tbs).to_bytes();
    assert_eq!(assemble(&tbs, &sig).unwrap(), leaf_der());
}

#[test]
fn assembled_cert_embeds_exact_tbs() {
    let tbs = leaf_tbs(&leaf_params(), &spki(&device_key()), &anchor()).unwrap();
    let der = assemble(&tbs, &[9; 64]).unwrap();
    assert!(der.windows(tbs.len()).any(|w| w == tbs.as_slice()));
    assert!(der.ends_with(&[9; 64]));
    assert!(Certificate::from_der(&der).is_ok());
}

#[test]
fn time_switches_encoding_at_2050() {
    assert!(matches!(time(0).unwrap(), Time::UtcTime(_)));
    assert!(matches!(time(UTC_TIME_END - 1).unwrap(), Time::UtcTime(_)));
    assert!(matches!(time(UTC_TIME_END).unwrap(), Time::GeneralTime(_)));
    for s in [0, 86_399, UTC_TIME_END - 1, UTC_TIME_END, 4_102_444_800] {
        assert_eq!(time(s).unwrap().to_unix_duration().as_secs(), s);
    }
    assert!(time(u64::MAX).is_err());
}

#[test]
fn ten_year_leaf_crossing_2050_verifies() {
    let p = CertParams {
        not_before: 2_500_000_000,
        not_after: 2_815_000_000,
        ..leaf_params()
    };
    let root = root_key();
    let d = build_leaf(&p, &spki(&device_key()), &anchor(), |m| {
        root.sign(m).to_bytes()
    })
    .unwrap();
    assert!(verify_leaf(&d, &anchor(), Some(2_600_000_000)).is_err());
    let long_root = CertParams {
        not_after: 3_000_000_000,
        ..root_params()
    };
    let rd = build_root(&long_root, &root.verifying_key().to_bytes(), |m| {
        root.sign(m).to_bytes()
    })
    .unwrap();
    let a = TrustAnchor::from_der(&rd).unwrap();
    let d = build_leaf(&p, &spki(&device_key()), &a, |m| root.sign(m).to_bytes()).unwrap();
    assert!(verify_leaf(&d, &a, Some(2_600_000_000)).is_ok());
}

#[test]
fn invalid_parameters_are_rejected() {
    let a = anchor();
    let s = spki(&device_key());
    let backwards = CertParams {
        not_before: NOT_AFTER,
        not_after: NOT_BEFORE,
        ..leaf_params()
    };
    assert_eq!(
        leaf_tbs(&backwards, &s, &a),
        Err(Error::MalformedCertificate)
    );
    let equal = CertParams {
        not_after: NOT_BEFORE,
        ..leaf_params()
    };
    assert_eq!(leaf_tbs(&equal, &s, &a), Err(Error::MalformedCertificate));
    let bad_name = CertParams {
        subject: "not a dn",
        ..leaf_params()
    };
    assert_eq!(
        leaf_tbs(&bad_name, &s, &a),
        Err(Error::MalformedCertificate)
    );
    assert_eq!(
        leaf_tbs(&leaf_params(), &[0x30, 0x00], &a),
        Err(Error::MalformedCertificate)
    );
    let ed_spki = {
        let c = Certificate::from_der(&root_der()).unwrap();
        c.tbs_certificate.subject_public_key_info.to_der().unwrap()
    };
    assert_eq!(
        leaf_tbs(&leaf_params(), &ed_spki, &a),
        Err(Error::MalformedCertificate)
    );
    assert_eq!(
        assemble(&[0x30, 0x00], &[0; 64]),
        Err(Error::MalformedCertificate)
    );
}

#[test]
fn serial_edge_cases() {
    let a = anchor();
    let s = spki(&device_key());
    let max_safe = CertSerial::new(&[0x7f; 20]).unwrap();
    let p = CertParams {
        serial: max_safe,
        ..leaf_params()
    };
    let root = root_key();
    let d = build_leaf(&p, &s, &a, |m| root.sign(m).to_bytes()).unwrap();
    assert_eq!(
        verify_leaf(&d, &a, None).unwrap().serial,
        alloc::vec![0x7f; 20]
    );
    let high = CertParams {
        serial: CertSerial::new(&[0x80; 19]).unwrap(),
        ..leaf_params()
    };
    let d = build_leaf(&high, &s, &a, |m| root.sign(m).to_bytes()).unwrap();
    assert_eq!(
        verify_leaf(&d, &a, None).unwrap().serial,
        alloc::vec![0x80; 19]
    );
    let overlong = CertParams {
        serial: CertSerial::new(&[0x80; 20]).unwrap(),
        ..leaf_params()
    };
    assert_eq!(
        leaf_tbs(&overlong, &s, &a),
        Err(Error::MalformedCertificate)
    );
}
