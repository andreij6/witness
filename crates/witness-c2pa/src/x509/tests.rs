use super::*;
use crate::cert::{assemble, leaf_tbs};
use crate::testkit::*;
use alloc::string::ToString;
use alloc::vec;
use ed25519_dalek::Signer as _;
use x509_cert::TbsCertificate;
use x509_cert::der::asn1::{BitString, OctetString};

fn leaf_with(mutate: impl FnOnce(&mut TbsCertificate)) -> Vec<u8> {
    let tbs = leaf_tbs(&leaf_params(), &spki(&device_key()), &anchor()).unwrap();
    let mut t = TbsCertificate::from_der(&tbs).unwrap();
    mutate(&mut t);
    let der = t.to_der().unwrap();
    assemble(&der, &root_key().sign(&der).to_bytes()).unwrap()
}

fn ext_mut(t: &mut TbsCertificate, id: ObjectIdentifier) -> &mut Extension {
    t.extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e.extn_id == id)
        .unwrap()
}

fn drop_ext(t: &mut TbsCertificate, id: ObjectIdentifier) {
    t.extensions.as_mut().unwrap().retain(|e| e.extn_id != id);
}

fn set_ext(t: &mut TbsCertificate, id: ObjectIdentifier, critical: bool, value: &impl Encode) {
    let e = ext_mut(t, id);
    e.critical = critical;
    e.extn_value = OctetString::new(value.to_der().unwrap()).unwrap();
}

fn check(der: &[u8]) -> Result<Leaf, ChainError> {
    verify_leaf(der, &anchor(), Some(NOW))
}

#[test]
fn anchor_parses_witness_root() {
    let a = anchor();
    assert_eq!(a.public_key(), &root_key().verifying_key().to_bytes());
    assert_eq!(a.key_id(), key_identifier(a.public_key()).as_slice());
    assert_eq!(a.der(), root_der().as_slice());
    assert_eq!(a.subject().to_string(), "CN=Witness Root CA,O=Witness");
}

#[test]
fn anchor_rejects_non_roots_and_tampering() {
    assert_eq!(TrustAnchor::from_der(&leaf_der()), Err(ChainError::Issuer));
    assert_eq!(TrustAnchor::from_der(&[]), Err(ChainError::Malformed));
    assert_eq!(
        TrustAnchor::from_der(&[0x30, 0x00]),
        Err(ChainError::Malformed)
    );
    let mut r = root_der();
    let last = r.len() - 1;
    r[last] ^= 1;
    assert_eq!(TrustAnchor::from_der(&r), Err(ChainError::Signature));
}

#[test]
fn leaf_verifies_and_exposes_identity() {
    let l = check(&leaf_der()).unwrap();
    assert_eq!(l.spki_der, spki(&device_key()));
    assert_eq!(l.serial, LEAF_SERIAL.to_be_bytes()[3..].to_vec());
    assert_eq!(
        l.key,
        witness_format::crypto::Es256Key::from_spki(&l.spki_der).unwrap()
    );
}

#[test]
fn validity_window_is_inclusive() {
    let d = leaf_der();
    for t in [NOT_BEFORE, NOW, NOT_AFTER] {
        assert!(verify_leaf(&d, &anchor(), Some(t)).is_ok());
    }
    for t in [0, NOT_BEFORE - 1, NOT_AFTER + 1, u64::MAX] {
        assert_eq!(
            verify_leaf(&d, &anchor(), Some(t)),
            Err(ChainError::Validity)
        );
    }
    assert!(verify_leaf(&d, &anchor(), None).is_ok());
}

#[test]
fn impostor_root_with_same_name_fails_signature() {
    let impostor = TrustAnchor::from_der(&root_der_with(&other_root_key())).unwrap();
    assert_eq!(
        verify_leaf(&leaf_der(), &impostor, None),
        Err(ChainError::Signature)
    );
}

#[test]
fn chain_shapes() {
    let (leaf, root) = (leaf_der(), root_der());
    let a = anchor();
    assert!(verify_chain(&[&leaf], &a, None).is_ok());
    assert!(verify_chain(&[&leaf, &root], &a, None).is_ok());
    let other = root_der_with(&other_root_key());
    assert_eq!(
        verify_chain(&[&leaf, &other], &a, None),
        Err(ChainError::Anchor)
    );
    assert_eq!(verify_chain(&[], &a, None), Err(ChainError::Length));
    assert_eq!(
        verify_chain(&[&leaf, &root, &root], &a, None),
        Err(ChainError::Length)
    );
}

#[test]
fn key_usage_rules() {
    let du = |t: &mut TbsCertificate| drop_ext(t, oid::KEY_USAGE);
    assert_eq!(check(&leaf_with(du)), Err(ChainError::KeyUsage));
    let not_critical = |t: &mut TbsCertificate| ext_mut(t, oid::KEY_USAGE).critical = false;
    assert_eq!(check(&leaf_with(not_critical)), Err(ChainError::KeyUsage));
    for bad in [
        KeyUsages::KeyCertSign | KeyUsages::DigitalSignature,
        KeyUsages::CRLSign | KeyUsages::DigitalSignature,
        KeyUsages::NonRepudiation.into(),
    ] {
        let ku = KeyUsage(bad);
        assert_eq!(
            check(&leaf_with(|t| set_ext(t, oid::KEY_USAGE, true, &ku))),
            Err(ChainError::KeyUsage)
        );
    }
    let ok = KeyUsage(KeyUsages::DigitalSignature | KeyUsages::NonRepudiation);
    assert!(check(&leaf_with(|t| set_ext(t, oid::KEY_USAGE, true, &ok))).is_ok());
}

use x509_cert::ext::pkix::KeyUsages;

#[test]
fn extended_key_usage_rules() {
    assert_eq!(
        check(&leaf_with(|t| drop_ext(t, oid::EXT_KEY_USAGE))),
        Err(ChainError::ExtendedKeyUsage)
    );
    for (oids, ok) in [
        (vec![oid::KP_C2PA_CLAIM_SIGNING], true),
        (
            vec![oid::KP_DOCUMENT_SIGNING, oid::KP_C2PA_CLAIM_SIGNING],
            true,
        ),
        (
            vec![oid::KP_DOCUMENT_SIGNING, oid::ANY_EXT_KEY_USAGE],
            false,
        ),
        (
            vec![ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.1")],
            false,
        ),
        (vec![], false),
    ] {
        let eku = ExtendedKeyUsage(oids);
        let r = check(&leaf_with(|t| set_ext(t, oid::EXT_KEY_USAGE, false, &eku)));
        assert_eq!(r.is_ok(), ok, "{eku:?} -> {r:?}");
    }
}

#[test]
fn basic_constraints_rules() {
    let ca = BasicConstraints {
        ca: true,
        path_len_constraint: None,
    };
    assert_eq!(
        check(&leaf_with(|t| set_ext(
            t,
            oid::BASIC_CONSTRAINTS,
            true,
            &ca
        ))),
        Err(ChainError::BasicConstraints)
    );
    assert!(check(&leaf_with(|t| drop_ext(t, oid::BASIC_CONSTRAINTS))).is_ok());
}

#[test]
fn authority_key_identifier_rules() {
    assert_eq!(
        check(&leaf_with(|t| drop_ext(t, oid::AUTHORITY_KEY_ID))),
        Err(ChainError::KeyIdentifier)
    );
    let wrong = AuthorityKeyIdentifier {
        key_identifier: Some(OctetString::new(vec![1; 20]).unwrap()),
        ..Default::default()
    };
    assert_eq!(
        check(&leaf_with(|t| set_ext(
            t,
            oid::AUTHORITY_KEY_ID,
            false,
            &wrong
        ))),
        Err(ChainError::KeyIdentifier)
    );
    let empty = AuthorityKeyIdentifier::default();
    assert_eq!(
        check(&leaf_with(|t| set_ext(
            t,
            oid::AUTHORITY_KEY_ID,
            false,
            &empty
        ))),
        Err(ChainError::KeyIdentifier)
    );
}

#[test]
fn unknown_extensions() {
    let unknown = |critical| {
        move |t: &mut TbsCertificate| {
            t.extensions.as_mut().unwrap().push(Extension {
                extn_id: ObjectIdentifier::new_unwrap("1.2.3.4.5"),
                critical,
                extn_value: OctetString::new(vec![0x05, 0x00]).unwrap(),
            })
        }
    };
    assert!(check(&leaf_with(unknown(false))).is_ok());
    assert_eq!(
        check(&leaf_with(unknown(true))),
        Err(ChainError::CriticalExtension)
    );
    let dup = |t: &mut TbsCertificate| {
        let e = ext_mut(t, oid::EXT_KEY_USAGE).clone();
        t.extensions.as_mut().unwrap().push(e);
    };
    assert_eq!(check(&leaf_with(dup)), Err(ChainError::DuplicateExtension));
    let garbage = |t: &mut TbsCertificate| {
        ext_mut(t, oid::KEY_USAGE).extn_value = OctetString::new(vec![0xff]).unwrap();
    };
    assert_eq!(check(&leaf_with(garbage)), Err(ChainError::Malformed));
    let flood = |t: &mut TbsCertificate| {
        let list = t.extensions.as_mut().unwrap();
        for i in 0..70u32 {
            list.push(Extension {
                extn_id: ObjectIdentifier::new_unwrap("1.2.3").push_arc(i).unwrap(),
                critical: false,
                extn_value: OctetString::new(vec![0x05, 0x00]).unwrap(),
            });
        }
    };
    assert_eq!(check(&leaf_with(flood)), Err(ChainError::Malformed));
}

#[test]
fn structural_profile_rules() {
    let v1 = |t: &mut TbsCertificate| t.version = Version::V1;
    assert_eq!(check(&leaf_with(v1)), Err(ChainError::Version));
    let uid =
        |t: &mut TbsCertificate| t.issuer_unique_id = Some(BitString::from_bytes(&[1]).unwrap());
    assert_eq!(check(&leaf_with(uid)), Err(ChainError::Malformed));
    let alg = |t: &mut TbsCertificate| {
        t.signature = AlgorithmIdentifierOwned {
            oid: ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.2"),
            parameters: None,
        }
    };
    assert_eq!(check(&leaf_with(alg)), Err(ChainError::Algorithm));
    let issuer = |t: &mut TbsCertificate| t.issuer = t.subject.clone();
    assert_eq!(check(&leaf_with(issuer)), Err(ChainError::Issuer));
    let ed_key = |t: &mut TbsCertificate| {
        t.subject_public_key_info = x509_cert::spki::SubjectPublicKeyInfoOwned {
            algorithm: ed25519_alg(),
            subject_public_key: BitString::from_bytes(&[1; 32]).unwrap(),
        }
    };
    assert_eq!(check(&leaf_with(ed_key)), Err(ChainError::PublicKey));
    let backwards = |t: &mut TbsCertificate| {
        core::mem::swap(&mut t.validity.not_before, &mut t.validity.not_after)
    };
    assert_eq!(
        verify_leaf(&leaf_with(backwards), &anchor(), None),
        Err(ChainError::Validity)
    );
}

#[test]
fn positive_serial_normalizes() {
    assert_eq!(positive_serial(&[]), Err(ChainError::Serial));
    assert_eq!(positive_serial(&[0]), Err(ChainError::Serial));
    assert_eq!(positive_serial(&[0, 0]), Err(ChainError::Serial));
    assert_eq!(positive_serial(&[0x80]), Err(ChainError::Serial));
    assert_eq!(positive_serial(&[0, 0x80]), Ok(vec![0x80]));
    assert_eq!(positive_serial(&[1, 2]), Ok(vec![1, 2]));
}

#[test]
fn key_identifier_is_truncated_sha256() {
    let id = key_identifier(b"abc");
    assert_eq!(id[..], witness_format::crypto::sha256(b"abc")[..20]);
}

#[test]
fn every_byte_flip_of_leaf_is_rejected() {
    let d = leaf_der();
    let a = anchor();
    let mut rng = Rng::new(1);
    let mut t = d.clone();
    for i in 0..t.len() {
        let mask = 1 << rng.below(8);
        t[i] ^= mask;
        assert!(verify_leaf(&t, &a, None).is_err(), "byte {i}");
        t[i] ^= mask;
    }
}

#[test]
fn truncations_are_rejected() {
    let d = leaf_der();
    for n in 0..d.len() {
        assert!(verify_leaf(&d[..n], &anchor(), None).is_err());
    }
}
