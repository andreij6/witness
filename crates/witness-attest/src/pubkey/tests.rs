use super::*;
use crate::testkit::{self as kit, Alg, Rng, TestKey, oids};

fn alg_of(a: Alg) -> SigAlg {
    match a {
        Alg::EcdsaSha256 => SigAlg::EcdsaSha256,
        Alg::EcdsaSha384 => SigAlg::EcdsaSha384,
        Alg::EcdsaSha512 => SigAlg::EcdsaSha512,
        Alg::RsaSha256 => SigAlg::RsaSha256,
        Alg::RsaSha384 => SigAlg::RsaSha384,
        Alg::RsaSha512 => SigAlg::RsaSha512,
    }
}

#[test]
fn algorithm_identifiers_parse() {
    for a in [
        Alg::EcdsaSha256,
        Alg::EcdsaSha384,
        Alg::EcdsaSha512,
        Alg::RsaSha256,
        Alg::RsaSha384,
        Alg::RsaSha512,
    ] {
        let id = a.identifier();
        let value = der::single(&id, Tag::SEQUENCE).unwrap();
        assert_eq!(SigAlg::from_algorithm_identifier(value), Ok(alg_of(a)));
    }
    let absent_null = kit::oid(oids::SHA256_RSA);
    assert_eq!(
        SigAlg::from_algorithm_identifier(&absent_null),
        Ok(SigAlg::RsaSha256)
    );
}

#[test]
fn algorithm_identifiers_reject() {
    let cases: [(std::vec::Vec<u8>, Error); 6] = [
        (
            kit::cat(&[&kit::oid(oids::SHA1_RSA), &kit::null()]),
            Error::UnsupportedAlgorithm,
        ),
        (kit::oid(oids::EC_PUBLIC_KEY), Error::UnsupportedAlgorithm),
        (
            kit::cat(&[&kit::oid(oids::ECDSA_SHA256), &kit::null()]),
            Error::Der,
        ),
        (
            kit::cat(&[&kit::oid(oids::SHA256_RSA), &kit::tlv(&[0x05], &[0])]),
            Error::Der,
        ),
        (
            kit::cat(&[&kit::oid(oids::SHA256_RSA), &kit::null(), &kit::null()]),
            Error::Der,
        ),
        (std::vec::Vec::new(), Error::Der),
    ];
    for (value, err) in cases {
        assert_eq!(
            SigAlg::from_algorithm_identifier(&value),
            Err(err),
            "{value:x?}"
        );
    }
}

#[test]
fn spki_parses_each_key_type() {
    assert!(matches!(
        PublicKey::from_spki(&TestKey::p256(1).spki()),
        Ok(PublicKey::P256(_))
    ));
    assert!(matches!(
        PublicKey::from_spki(&TestKey::p384(1).spki()),
        Ok(PublicKey::P384(_))
    ));
    assert!(matches!(
        PublicKey::from_spki(&TestKey::aosp_rsa().spki()),
        Ok(PublicKey::Rsa(_))
    ));
    assert!(
        PublicKey::from_spki(&TestKey::p256(1).spki())
            .unwrap()
            .is_p256()
    );
    assert!(
        !PublicKey::from_spki(&TestKey::p384(1).spki())
            .unwrap()
            .is_p256()
    );
}

#[test]
fn spki_accepts_compressed_points() {
    let TestKey::P256(k) = TestKey::p256(3) else {
        unreachable!()
    };
    let point = k.verifying_key().to_encoded_point(true);
    let spki = kit::seq(&[
        &kit::seq(&[&kit::oid(oids::EC_PUBLIC_KEY), &kit::oid(oids::PRIME256V1)]),
        &kit::bits(point.as_bytes()),
    ]);
    assert_eq!(
        PublicKey::from_spki(&spki),
        Ok(PublicKey::P256(*k.verifying_key()))
    );
}

#[test]
fn spki_rejects_malformed() {
    let p256 = TestKey::p256(1);
    let TestKey::P256(k) = &p256 else {
        unreachable!()
    };
    let point = k.verifying_key().to_encoded_point(false);
    let ec = |curve: &[u8], pt: &[u8]| {
        kit::seq(&[
            &kit::seq(&[&kit::oid(oids::EC_PUBLIC_KEY), &kit::oid(curve)]),
            &kit::bits(pt),
        ])
    };
    let mut off_curve = point.as_bytes().to_vec();
    off_curve[64] ^= 1;
    let rsa = |n: &[u8], e: &[u8]| {
        kit::seq(&[
            &kit::seq(&[&kit::oid(oids::RSA_ENCRYPTION), &kit::null()]),
            &kit::bits(&kit::seq(&[&kit::int_raw(n), &kit::int_raw(e)])),
        ])
    };
    let mut small_n = std::vec![0x00];
    small_n.extend([0xc3; 64]);
    let mut big_n = std::vec![0x00];
    big_n.extend([0xc3; 128]);
    let cases: std::vec::Vec<(std::vec::Vec<u8>, Error)> = std::vec![
        (
            ec(oids::SECP521R1, point.as_bytes()),
            Error::UnsupportedAlgorithm
        ),
        (ec(oids::PRIME256V1, &off_curve), Error::Key),
        (ec(oids::PRIME256V1, &point.as_bytes()[..64]), Error::Key),
        (ec(oids::SECP384R1, point.as_bytes()), Error::Key),
        (ec(oids::PRIME256V1, &[]), Error::Key),
        (rsa(&small_n, &[1, 0, 1]), Error::Key),
        (rsa(&big_n, &[0x80]), Error::Der),
        (rsa(&big_n, &[0x01]), Error::Key),
        (
            kit::seq(&[
                &kit::seq(&[&kit::oid(oids::RSA_ENCRYPTION)]),
                &kit::bits(&[1])
            ]),
            Error::Der
        ),
        (
            kit::seq(&[
                &kit::seq(&[&kit::oid(oids::ECDSA_SHA256)]),
                &kit::bits(&[1])
            ]),
            Error::UnsupportedAlgorithm
        ),
        (
            kit::seq(&[
                &kit::seq(&[&kit::oid(oids::EC_PUBLIC_KEY)]),
                &kit::bits(&[1])
            ]),
            Error::Der
        ),
        (
            kit::seq(&[&kit::seq(&[
                &kit::oid(oids::EC_PUBLIC_KEY),
                &kit::oid(oids::PRIME256V1)
            ])]),
            Error::Der
        ),
        (kit::cat(&[&p256.spki(), &[0]]), Error::Der),
        (std::vec![], Error::Der),
    ];
    for (spki, err) in cases {
        assert_eq!(
            PublicKey::from_spki(&spki).map(|_| ()),
            Err(err),
            "{spki:x?}"
        );
    }
}

#[test]
fn verify_round_trips_every_algorithm() {
    let msg = b"tbs certificate bytes";
    let keys = [
        (
            TestKey::p256(5),
            [Alg::EcdsaSha256, Alg::EcdsaSha384, Alg::EcdsaSha512],
        ),
        (
            TestKey::p384(5),
            [Alg::EcdsaSha256, Alg::EcdsaSha384, Alg::EcdsaSha512],
        ),
        (
            TestKey::aosp_rsa(),
            [Alg::RsaSha256, Alg::RsaSha384, Alg::RsaSha512],
        ),
    ];
    for (key, algs) in keys {
        let pk = PublicKey::from_spki(&key.spki()).unwrap();
        for a in algs {
            let sig = key.sign(a, msg);
            assert_eq!(pk.verify(alg_of(a), msg, &sig), Ok(()));
            assert_eq!(
                pk.verify(alg_of(a), b"other message", &sig),
                Err(Error::Signature)
            );
            let mut bad = sig.clone();
            let last = bad.len() - 1;
            bad[last] ^= 1;
            assert_eq!(pk.verify(alg_of(a), msg, &bad), Err(Error::Signature));
        }
    }
}

#[test]
fn verify_rejects_wrong_digest_and_key_family() {
    let msg = b"m";
    let ec = TestKey::p256(8);
    let ec_pk = PublicKey::from_spki(&ec.spki()).unwrap();
    let sig = ec.sign(Alg::EcdsaSha256, msg);
    assert_eq!(
        ec_pk.verify(SigAlg::EcdsaSha384, msg, &sig),
        Err(Error::Signature)
    );
    assert_eq!(
        ec_pk.verify(SigAlg::RsaSha256, msg, &sig),
        Err(Error::AlgorithmMismatch)
    );
    let rsa = TestKey::aosp_rsa();
    let rsa_pk = PublicKey::from_spki(&rsa.spki()).unwrap();
    let rsig = rsa.sign(Alg::RsaSha256, msg);
    assert_eq!(
        rsa_pk.verify(SigAlg::RsaSha384, msg, &rsig),
        Err(Error::Signature)
    );
    assert_eq!(
        rsa_pk.verify(SigAlg::EcdsaSha256, msg, &rsig),
        Err(Error::AlgorithmMismatch)
    );
    let other = PublicKey::from_spki(&TestKey::p256(9).spki()).unwrap();
    assert_eq!(
        other.verify(SigAlg::EcdsaSha256, msg, &sig),
        Err(Error::Signature)
    );
}

#[test]
fn verify_rejects_garbage_signatures() {
    let mut rng = Rng::new(4);
    let ec = PublicKey::from_spki(&TestKey::p384(2).spki()).unwrap();
    let rsa = PublicKey::from_spki(&TestKey::aosp_rsa().spki()).unwrap();
    for _ in 0..300 {
        let len = rng.below(140) as usize;
        let mut sig = std::vec![0u8; len];
        rng.fill(&mut sig);
        assert_eq!(
            ec.verify(SigAlg::EcdsaSha384, b"x", &sig),
            Err(Error::Signature)
        );
        assert_eq!(
            rsa.verify(SigAlg::RsaSha256, b"x", &sig),
            Err(Error::Signature)
        );
    }
}
