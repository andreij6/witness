mod common;

use common::{
    AOSP_EC_INTERMEDIATE, AOSP_EC_ROOT, AOSP_RSA_INTERMEDIATE, AOSP_RSA_ROOT, CertSpec, Fixture,
    GOOGLE_ROOTS, RELEASE_DIGESTS, RootAlg, SECRET, TestKey,
};
use witness_attest::policy::attestation_index;
use witness_attest::roots::{
    self, AOSP_SOFT_EC_SPKI, AOSP_SOFT_RSA_SPKI, DROID_UNREGISTERED_SPKI, GOOGLE_EC_CA1_SPKI,
    GOOGLE_RSA_SPKI, RootKind,
};
use witness_attest::{
    ANCHORS, Certificate, Context, Enrollment, Error, Policy, PublicKey, RevocationList, SigAlg,
    verify,
};
use witness_format::{BootState, Freshness, Provisioning, SecurityLevel};

#[test]
fn google_roots_share_pinned_keys_and_self_verify() {
    let serials = [
        "f1c172a699eaf51d",
        "84a9d0297b0eb58ae7ff0e80de760605",
        "e8fa196314d2fa18",
        "d50ff25ba3f2d6b3",
        "c36b7c44b9ae1831",
    ];
    for (der, serial) in GOOGLE_ROOTS.iter().zip(serials) {
        let c = Certificate::parse(der).unwrap();
        assert_eq!(c.serial.to_hex(), serial);
        assert!(c.is_ca());
        let expected = if c.sig_alg == SigAlg::EcdsaSha384 {
            GOOGLE_EC_CA1_SPKI
        } else {
            GOOGLE_RSA_SPKI
        };
        assert_eq!(c.spki, expected, "{serial}");
        c.verify_signed_by(&c.public_key().unwrap()).unwrap();
        let kind = roots::find(ANCHORS, c.spki).unwrap().kind;
        assert_eq!(
            kind,
            if expected == GOOGLE_RSA_SPKI {
                RootKind::GoogleRsa
            } else {
                RootKind::GoogleEc
            }
        );
    }
}

#[test]
fn google_ec_root_validity_matches_published_dates() {
    let c = Certificate::parse(GOOGLE_ROOTS[1]).unwrap();
    assert_eq!(c.not_before, 1_752_791_538);
    assert_eq!(c.not_after, 2_068_151_538);
}

#[test]
fn tampered_google_root_fails_self_verification() {
    let mut der = GOOGLE_ROOTS[0].to_vec();
    let c = Certificate::parse(GOOGLE_ROOTS[0]).unwrap();
    let key = c.public_key().unwrap();
    let pos = c.subject.as_ptr() as usize - GOOGLE_ROOTS[0].as_ptr() as usize + 6;
    der[pos] ^= 1;
    let t = Certificate::parse(&der).unwrap();
    assert_eq!(t.verify_signed_by(&key), Err(Error::Signature));
}

#[test]
fn aosp_soft_attestation_hierarchy_verifies() {
    for (root, intermediate, spki) in [
        (AOSP_EC_ROOT, AOSP_EC_INTERMEDIATE, AOSP_SOFT_EC_SPKI),
        (AOSP_RSA_ROOT, AOSP_RSA_INTERMEDIATE, AOSP_SOFT_RSA_SPKI),
    ] {
        let r = Certificate::parse(root).unwrap();
        let i = Certificate::parse(intermediate).unwrap();
        assert_eq!(r.spki, spki);
        let root_key = PublicKey::from_spki(spki).unwrap();
        r.verify_signed_by(&root_key).unwrap();
        i.verify_signed_by(&root_key).unwrap();
        assert!(i.is_ca());
        assert_eq!(i.basic_constraints.unwrap().path_len, Some(0));
        assert_eq!(i.key_cert_sign, Some(true));
    }
}

#[test]
fn emulator_rkp_test_root_is_pinned_and_self_signed() {
    let r = Certificate::parse(include_bytes!("data/droid_unregistered_root.der")).unwrap();
    assert_eq!(r.spki, DROID_UNREGISTERED_SPKI);
    assert!(r.is_ca());
    r.verify_signed_by(&PublicKey::from_spki(DROID_UNREGISTERED_SPKI).unwrap())
        .unwrap();
    assert_eq!(
        roots::find(ANCHORS, DROID_UNREGISTERED_SPKI).map(|a| a.kind),
        cfg!(feature = "dev-attestation").then_some(RootKind::Dev)
    );
}

#[test]
fn emulator_rkp_chain_places_extensions_for_a_dev_root_only() {
    let ders: [&[u8]; 3] = [
        include_bytes!("data/emulator_rkp_leaf.der"),
        include_bytes!("data/emulator_rkp_intermediate.der"),
        include_bytes!("data/droid_unregistered_root.der"),
    ];
    let certs: Vec<Certificate<'_>> = ders
        .iter()
        .map(|d| Certificate::parse(d).unwrap())
        .collect();
    assert!(certs[0].key_description.is_some());
    assert!(certs[1].provisioning_info && certs[2].provisioning_info);
    assert_eq!(attestation_index(&certs, RootKind::Dev), Ok(0));
    for kind in [RootKind::GoogleEc, RootKind::GoogleRsa] {
        assert_eq!(
            attestation_index(&certs, kind),
            Err(Error::ExtensionPlacement)
        );
    }
    let want = if cfg!(feature = "dev-attestation") {
        Ok(RootKind::Dev)
    } else {
        Err(Error::UntrustedRoot)
    };
    assert_eq!(witness_attest::chain::root_kind(&certs, ANCHORS), want);
    witness_attest::chain::check_issuers(&certs).unwrap();
    witness_attest::chain::verify_signatures(&certs).unwrap();
}

#[test]
fn aosp_intermediate_keys_match_their_certificates() {
    let ec = Certificate::parse(AOSP_EC_INTERMEDIATE).unwrap();
    assert_eq!(ec.spki, TestKey::aosp_ec().spki().as_slice());
    let rsa = Certificate::parse(AOSP_RSA_INTERMEDIATE).unwrap();
    assert_eq!(
        PublicKey::from_spki(rsa.spki).unwrap(),
        PublicKey::from_spki(&TestKey::aosp_rsa().spki()).unwrap()
    );
}

fn emulator_chain(signer: TestKey, intermediate: &[u8], root: &[u8]) -> (Vec<Vec<u8>>, Fixture) {
    let mut f = Fixture::new(RootAlg::P384);
    f.kd.attestation_security = 0;
    f.kd.keymint_security = 0;
    common::kd::Kd::replace(
        &mut f.kd.hardware,
        common::kd::ROOT_OF_TRUST,
        common::kd::root_of_trust(false, 2),
    );
    let mut spec: CertSpec = f.certs[0].clone();
    spec.extensions
        .push(common::key_description_ext(&f.kd.der()));
    let issuer = Certificate::parse(intermediate).unwrap();
    let issuer_name = common::seq(&[issuer.subject]);
    let leaf = common::build_cert(&spec, &f.keys[0], &issuer_name, &signer);
    (vec![leaf, intermediate.to_vec(), root.to_vec()], f)
}

fn run_compiled(chain: &[Vec<u8>], f: &Fixture) -> Result<witness_attest::Attested, Error> {
    let revocations = RevocationList::empty();
    let ctx = Context {
        now_ns: f.now_ns,
        challenge_secret: &SECRET,
        revocations: &revocations,
    };
    verify(
        &Enrollment {
            chain,
            key_id: None,
            late: None,
        },
        &Policy::witness(RELEASE_DIGESTS),
        &ctx,
    )
}

#[test]
fn emulator_chains_follow_dev_feature() {
    for (signer, intermediate, root) in [
        (TestKey::aosp_ec(), AOSP_EC_INTERMEDIATE, AOSP_EC_ROOT),
        (TestKey::aosp_rsa(), AOSP_RSA_INTERMEDIATE, AOSP_RSA_ROOT),
    ] {
        let (chain, f) = emulator_chain(signer, intermediate, root);
        let got = run_compiled(&chain, &f);
        if cfg!(feature = "dev-attestation") {
            let a = got.unwrap();
            assert_eq!(a.provisioning, Provisioning::Dev);
            assert_eq!(a.security, SecurityLevel::Dev);
            assert_eq!(a.boot, BootState::Unverified);
            assert_eq!(a.freshness, Freshness::Fresh);
            assert!(a.key_info(f.now_ns, 1).is_dev());
        } else {
            assert_eq!(got, Err(Error::UntrustedRoot));
        }
    }
}

#[test]
fn emulator_chain_still_needs_a_valid_leaf_signature() {
    let (mut chain, f) = emulator_chain(TestKey::p256(4242), AOSP_EC_INTERMEDIATE, AOSP_EC_ROOT);
    let want = if cfg!(feature = "dev-attestation") {
        Error::Signature
    } else {
        Error::UntrustedRoot
    };
    assert_eq!(run_compiled(&chain, &f), Err(want));
    chain.swap(1, 2);
    assert!(run_compiled(&chain, &f).is_err());
}

#[test]
fn compiled_anchor_set() {
    let kinds: Vec<RootKind> = ANCHORS.iter().map(|a| a.kind).collect();
    assert!(kinds.contains(&RootKind::GoogleRsa));
    assert!(kinds.contains(&RootKind::GoogleEc));
    assert_eq!(
        kinds.contains(&RootKind::Dev),
        cfg!(feature = "dev-attestation")
    );
}
