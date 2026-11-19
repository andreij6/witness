use super::*;
use crate::roots::{GOOGLE_EC_CA1, GOOGLE_RSA};
use crate::testkit::{self as kit, DAY, Fixture, RootAlg};
use std::vec::Vec;

fn parsed(ders: &[Vec<u8>]) -> Vec<Certificate<'_>> {
    ders.iter()
        .map(|d| Certificate::parse(d).unwrap())
        .collect()
}

#[test]
fn length_bounds() {
    for n in 0..=10 {
        assert_eq!(check_length(n).is_ok(), (3..=8).contains(&n), "{n}");
    }
    assert_eq!(check_length(2), Err(Error::ChainLength));
}

#[test]
fn root_kind_matches_anchor() {
    let f = Fixture::new(RootAlg::P384);
    let ders = f.build();
    let certs = parsed(&ders);
    let spki = f.root_spki();
    let anchors = [TrustAnchor {
        spki: &spki,
        kind: RootKind::Dev,
    }];
    assert_eq!(root_kind(&certs, &anchors), Ok(RootKind::Dev));
    assert_eq!(
        root_kind(&certs, &[GOOGLE_RSA, GOOGLE_EC_CA1]),
        Err(Error::UntrustedRoot)
    );
    assert_eq!(root_kind(&certs, &[]), Err(Error::UntrustedRoot));
    assert_eq!(root_kind(&[], &anchors), Err(Error::ChainLength));
    assert_eq!(root_kind(&certs[..3], &anchors), Err(Error::UntrustedRoot));
}

#[test]
fn issuers_must_be_cas() {
    let f = Fixture::new(RootAlg::P384);
    assert_eq!(check_issuers(&parsed(&f.build())), Ok(()));
    for i in 1..f.len() {
        let mut g = f.clone();
        g.certs[i].extensions = std::vec![kit::key_usage(true)];
        assert_eq!(check_issuers(&parsed(&g.build())), Err(Error::NotCa), "{i}");
        let mut g = f.clone();
        g.certs[i].extensions =
            std::vec![kit::basic_constraints(false, None), kit::key_usage(true)];
        assert_eq!(check_issuers(&parsed(&g.build())), Err(Error::NotCa), "{i}");
    }
}

#[test]
fn leaf_may_lack_constraints() {
    let mut f = Fixture::new(RootAlg::P384);
    f.certs[0].extensions.clear();
    assert_eq!(check_issuers(&parsed(&f.build())), Ok(()));
}

#[test]
fn key_usage_must_allow_cert_sign_when_present() {
    let f = Fixture::new(RootAlg::P384);
    for i in 1..f.len() {
        let mut g = f.clone();
        g.certs[i].extensions =
            std::vec![kit::basic_constraints(true, None), kit::key_usage(false)];
        assert_eq!(
            check_issuers(&parsed(&g.build())),
            Err(Error::KeyUsage),
            "{i}"
        );
        let mut g = f.clone();
        g.certs[i].extensions = std::vec![kit::basic_constraints(true, None)];
        assert_eq!(check_issuers(&parsed(&g.build())), Ok(()), "{i}");
    }
}

#[test]
fn path_len_counts_intermediates_below() {
    let f = Fixture::new(RootAlg::P384);
    let with = |i: usize, p: u64| {
        let mut g = f.clone();
        g.certs[i].extensions =
            std::vec![kit::basic_constraints(true, Some(p)), kit::key_usage(true)];
        check_issuers(&parsed(&g.build()))
    };
    assert_eq!(with(1, 0), Ok(()));
    assert_eq!(with(2, 0), Err(Error::PathLen));
    assert_eq!(with(2, 1), Ok(()));
    assert_eq!(with(3, 1), Err(Error::PathLen));
    assert_eq!(with(3, 2), Ok(()));
    assert_eq!(with(3, 100), Ok(()));
}

#[test]
fn validity_rules() {
    let f = Fixture::new(RootAlg::P384);
    let now = f.now();
    let ders = f.build();
    let certs = parsed(&ders);
    assert_eq!(check_validity(&certs, false, now), Ok(()));
    assert_eq!(check_validity(&certs, true, now), Ok(()));
    assert_eq!(
        check_validity(&certs, true, now - 3600),
        Err(Error::Validity)
    );
    assert_eq!(
        check_validity(&certs, false, now + 3650 * DAY + 1),
        Err(Error::Validity)
    );

    for i in 0..f.len() {
        let mut g = f.clone();
        g.certs[i].not_after = now - 1;
        let ders = g.build();
        let certs = parsed(&ders);
        let strict = check_validity(&certs, false, now);
        let lenient = check_validity(&certs, true, now);
        let root = i == f.len() - 1;
        assert_eq!(strict.is_ok(), root, "strict {i}");
        assert_eq!(lenient.is_ok(), i != 0, "lenient {i}");
    }
    let mut g = f.clone();
    g.certs[0].not_before = now + 1;
    let ders = g.build();
    assert_eq!(
        check_validity(&parsed(&ders), true, now),
        Err(Error::Validity)
    );
}

#[test]
fn signatures_verify_and_detect_tampering() {
    for alg in [RootAlg::P384, RootAlg::Rsa, RootAlg::P256] {
        let f = Fixture::new(alg);
        let ders = f.build();
        assert_eq!(verify_signatures(&parsed(&ders)), Ok(()), "{alg:?}");
        for i in 0..ders.len() - 1 {
            let mut bad = ders.clone();
            let c = Certificate::parse(&bad[i]).unwrap();
            let pos = c.subject.as_ptr() as usize - c.der.as_ptr() as usize + c.subject.len() - 1;
            bad[i][pos] ^= 0x01;
            assert_eq!(
                verify_signatures(&parsed(&bad)),
                Err(Error::Signature),
                "{alg:?} {i}"
            );
        }
    }
}

#[test]
fn signatures_fail_when_order_swapped() {
    let f = Fixture::new(RootAlg::P384);
    let mut ders = f.build();
    ders.swap(1, 2);
    assert!(verify_signatures(&parsed(&ders)).is_err());
}

#[test]
fn root_self_signature_is_not_required() {
    let mut f = Fixture::new(RootAlg::P384);
    let ders = f.build();
    f.keys.push(kit::TestKey::p384(999));
    let resigned = kit::build_cert(&f.certs[3], &f.keys[3], &kit::name("other"), &f.keys[4]);
    let chain = [ders[0].clone(), ders[1].clone(), ders[2].clone(), resigned];
    assert_eq!(verify_signatures(&parsed(&chain)), Ok(()));
}
