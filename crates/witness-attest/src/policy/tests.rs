use super::*;
use crate::revocation::{Revocation, RevocationReason, RevocationStatus};
use crate::serial::Serial;
use crate::testkit::{self as kit, DAY, Fixture, HOUR, Rng, RootAlg, TestKey, kd as k};

fn fx() -> Fixture {
    Fixture::new(RootAlg::P384)
}

fn err(f: &Fixture) -> Error {
    f.run().expect_err("expected rejection")
}

fn now_ms(f: &Fixture) -> u64 {
    f.now_ns / 1_000_000
}

#[test]
fn accepts_rkp_chain_under_ec_root() {
    let f = fx();
    let chain = f.build();
    let a = f.run_chain(&chain).unwrap();
    assert_eq!(a.provisioning, Provisioning::Rkp);
    assert_eq!(a.freshness, Freshness::Fresh);
    assert_eq!(a.security, SecurityLevel::Tee);
    assert_eq!(a.boot, BootState::Verified);
    assert!(a.device_locked);
    assert_eq!(a.patch, 202_611);
    assert_eq!(a.attestation_version, 300);
    assert_eq!(a.keymint_version, 300);
    assert_eq!(a.pubkey, f.leaf_spki());
    assert_eq!(a.key_id, KeyId::from_spki(&f.leaf_spki()));
    assert_eq!(a.quota_key, sha256(&chain[1]));
    assert_eq!(a.chain_serials, ["10ab00", "11ab01", "12ab02", "13ab03"]);
    assert_eq!(a.created_ms, Some((f.now() as u64 - 60) * 1000));
}

#[test]
fn accepts_factory_chain_under_rsa_root() {
    let f = Fixture::new(RootAlg::Rsa);
    let a = f.run().unwrap();
    assert_eq!(a.provisioning, Provisioning::Factory);
}

#[test]
fn provisioning_info_marks_rsa_chain_as_rkp() {
    let mut f = Fixture::new(RootAlg::Rsa);
    f.certs[1].extensions.push(kit::provisioning_info_ext());
    assert_eq!(f.run().unwrap().provisioning, Provisioning::Rkp);
}

#[test]
fn strongbox_is_reported() {
    let mut f = fx();
    f.kd.attestation_security = 2;
    f.kd.keymint_security = 2;
    assert_eq!(f.run().unwrap().security, SecurityLevel::StrongBox);
    let mut f = fx();
    f.kd.attestation_security = 1;
    f.kd.keymint_security = 2;
    assert_eq!(f.run().unwrap().security, SecurityLevel::StrongBox);
}

#[test]
fn key_info_is_valid_and_windowed() {
    let f = fx();
    let a = f.run().unwrap();
    let info = a.key_info(f.now_ns, f.policy.active_ns);
    info.validate().unwrap();
    assert_eq!(info.key_id, a.key_id);
    assert_eq!(
        info.active_until_ns,
        f.now_ns + 60 * DAY as u64 * 1_000_000_000
    );
    assert_eq!(info.history.current(), witness_format::KeyStatus::Active);
    assert_eq!(info.chain_serials, a.chain_serials);
    assert_eq!(info.provisioning, Provisioning::Rkp);
    assert_eq!(a.key_info(u64::MAX - 1, 10).active_until_ns, u64::MAX);
}

#[test]
fn output_is_deterministic() {
    let f = fx();
    let chain = f.build();
    assert_eq!(f.run_chain(&chain), f.run_chain(&chain));
    assert_eq!(f.build(), chain);
}

#[test]
fn witness_policy_defaults() {
    let p = Policy::witness(&[[1; 32]]);
    assert_eq!(p.anchors, ANCHORS);
    assert_eq!(p.package, b"app.witness");
    assert_eq!(p.fresh_ttl_ns, HOUR_NS);
    assert_eq!(p.late_ttl_ns, 7 * DAY_NS);
    assert_eq!(p.active_ns, 60 * DAY_NS);
    assert_eq!(p.min_patch, DEFAULT_MIN_PATCH);
    assert_eq!(p.min_version, DEFAULT_MIN_VERSION);
}

#[test]
fn production_policy_rejects_synthetic_roots() {
    let f = fx();
    let chain = f.build();
    let ctx = Context {
        now_ns: f.now_ns,
        challenge_secret: &f.secret,
        revocations: &f.revocations,
    };
    let e = Enrollment {
        chain: &chain,
        key_id: None,
        late: None,
    };
    assert_eq!(
        verify(&e, &Policy::witness(kit::RELEASE_DIGESTS), &ctx),
        Err(Error::UntrustedRoot)
    );
}

#[test]
fn chain_length_limits() {
    let mut f = fx();
    f.remove_intermediate(2);
    assert!(f.run().is_ok());
    f.remove_intermediate(1);
    assert_eq!(err(&f), Error::ChainLength);
    let f = fx();
    let chain = f.build();
    let long: std::vec::Vec<_> = (0..9).map(|_| chain[0].clone()).collect();
    assert_eq!(f.run_chain(&long), Err(Error::ChainLength));
    let empty: [&[u8]; 0] = [];
    let ctx = Context {
        now_ns: 0,
        challenge_secret: &f.secret,
        revocations: &f.revocations,
    };
    assert_eq!(
        verify(
            &Enrollment {
                chain: &empty,
                key_id: None,
                late: None
            },
            &f.policy,
            &ctx
        ),
        Err(Error::ChainLength)
    );
}

#[test]
fn eight_certificate_chain_is_accepted() {
    let mut f = fx();
    for i in 0..4u8 {
        let mut spec = f.certs[2].clone();
        spec.serial = std::vec![0x20 + i];
        spec.subject = kit::name(&std::format!("extra {i}"));
        f.certs.insert(2, spec);
        f.keys.insert(2, TestKey::p256(700 + u64::from(i)));
    }
    assert_eq!(f.len(), 8);
    assert_eq!(f.run().unwrap().chain_serials.len(), 8);
}

#[test]
fn rejects_unparseable_certificate() {
    let f = fx();
    let mut chain = f.build();
    chain[2].truncate(10);
    assert_eq!(f.run_chain(&chain), Err(Error::Der));
}

#[test]
fn rejects_untrusted_root() {
    let f = fx();
    let mut chain = f.build();
    let mut other = f.clone();
    other.keys[3] = TestKey::p384(12345);
    chain[3] = other.build()[3].clone();
    assert_eq!(f.run_chain(&chain), Err(Error::UntrustedRoot));
}

#[test]
fn key_id_must_match_leaf() {
    let mut f = fx();
    f.key_id = Some(KeyId::from_spki(&f.leaf_spki()));
    assert!(f.run().is_ok());
    f.key_id = Some(KeyId::from_spki(&TestKey::p256(5).spki()));
    assert_eq!(err(&f), Error::KeyMismatch);
}

#[test]
fn rejects_non_ca_issuer() {
    let mut f = fx();
    f.certs[1].extensions = std::vec![kit::key_usage(true)];
    assert_eq!(err(&f), Error::NotCa);
    let mut f = fx();
    f.certs[2].extensions = std::vec![kit::basic_constraints(true, Some(0)), kit::key_usage(true)];
    assert_eq!(err(&f), Error::PathLen);
    let mut f = fx();
    f.certs[3].extensions = std::vec![kit::basic_constraints(true, None), kit::key_usage(false)];
    assert_eq!(err(&f), Error::KeyUsage);
}

#[test]
fn key_description_must_exist() {
    let mut f = fx();
    f.leaf_kd = false;
    assert_eq!(err(&f), Error::MissingKeyDescription);
}

#[test]
fn rejects_second_key_description_above_leaf() {
    for i in 1..4 {
        let mut f = fx();
        f.certs[i]
            .extensions
            .push(kit::key_description_ext(&f.kd.der()));
        assert_eq!(err(&f), Error::ExtensionPlacement, "{i}");
    }
}

#[test]
fn rejects_key_description_only_above_leaf() {
    let mut f = fx();
    f.leaf_kd = false;
    f.certs[1]
        .extensions
        .push(kit::key_description_ext(&f.kd.der()));
    assert_eq!(err(&f), Error::ExtensionPlacement);
}

#[test]
fn provisioning_info_must_sit_directly_above_attestation() {
    for (i, ok) in [(0usize, false), (1, true), (2, false), (3, false)] {
        let mut f = fx();
        f.certs[i].extensions.push(kit::provisioning_info_ext());
        assert_eq!(f.run().is_ok(), ok, "{i}");
        if !ok {
            assert_eq!(err(&f), Error::ExtensionPlacement, "{i}");
        }
    }
    let mut f = fx();
    f.certs[1].extensions.push(kit::provisioning_info_ext());
    f.certs[2].extensions.push(kit::provisioning_info_ext());
    assert_eq!(err(&f), Error::ExtensionPlacement);
}

#[test]
fn dev_root_may_carry_provisioning_info_but_google_roots_may_not() {
    let mut f = fx();
    f.root_kind = RootKind::Dev;
    f.certs[1].extensions.push(kit::provisioning_info_ext());
    f.certs[3].extensions.push(kit::provisioning_info_ext());
    assert_eq!(f.run().unwrap().provisioning, Provisioning::Dev);

    let mut g = f.clone();
    g.certs[2].extensions.push(kit::provisioning_info_ext());
    assert_eq!(err(&g), Error::ExtensionPlacement);

    let mut g = f.clone();
    g.certs[3]
        .extensions
        .push(kit::key_description_ext(&g.kd.der()));
    assert_eq!(err(&g), Error::ExtensionPlacement);

    let mut g = fx();
    g.certs[1].extensions.push(kit::provisioning_info_ext());
    g.certs[3].extensions.push(kit::provisioning_info_ext());
    assert_eq!(err(&g), Error::ExtensionPlacement);
}

#[test]
fn malformed_key_description_is_rejected() {
    let mut f = fx();
    f.kd.keymint_security = 9;
    assert_eq!(err(&f), Error::KeyDescription);
}

#[test]
fn security_level_rules() {
    for (att, km) in [(0, 0), (0, 1), (1, 0), (2, 0), (0, 2)] {
        let mut f = fx();
        f.kd.attestation_security = att;
        f.kd.keymint_security = km;
        assert_eq!(err(&f), Error::SecurityLevel, "{att} {km}");
    }
}

#[test]
fn boot_state_rules() {
    for state in 1..=3 {
        let mut f = fx();
        k::Kd::replace(
            &mut f.kd.hardware,
            k::ROOT_OF_TRUST,
            k::root_of_trust(true, state),
        );
        assert_eq!(err(&f), Error::BootState, "{state}");
    }
    let mut f = fx();
    k::Kd::replace(
        &mut f.kd.hardware,
        k::ROOT_OF_TRUST,
        k::root_of_trust(false, 0),
    );
    assert_eq!(err(&f), Error::DeviceLocked);
    let mut f = fx();
    k::Kd::remove(&mut f.kd.hardware, k::ROOT_OF_TRUST);
    assert_eq!(err(&f), Error::BootState);
    f.kd.software.push(k::root_of_trust(true, 0));
    assert_eq!(err(&f), Error::BootState);
}

#[test]
fn key_property_rules() {
    let cases: [(u32, Option<std::vec::Vec<u8>>, Error); 9] = [
        (k::ORIGIN, Some(k::int_field(k::ORIGIN, 1)), Error::Origin),
        (k::ORIGIN, Some(k::int_field(k::ORIGIN, 2)), Error::Origin),
        (k::ORIGIN, None, Error::Origin),
        (k::PURPOSE, Some(k::purpose(&[3])), Error::Purpose),
        (k::PURPOSE, None, Error::Purpose),
        (
            k::ALGORITHM,
            Some(k::int_field(k::ALGORITHM, 1)),
            Error::Algorithm,
        ),
        (k::ALGORITHM, None, Error::Algorithm),
        (
            k::EC_CURVE,
            Some(k::int_field(k::EC_CURVE, 2)),
            Error::Curve,
        ),
        (k::EC_CURVE, None, Error::Curve),
    ];
    for (tag, field, want) in cases {
        let mut f = fx();
        k::Kd::remove(&mut f.kd.hardware, tag);
        if let Some(field) = field {
            f.kd.hardware.push(field);
        }
        assert_eq!(err(&f), want, "{tag}");
    }
}

#[test]
fn software_listed_key_properties_do_not_count() {
    for tag in [
        k::ORIGIN,
        k::PURPOSE,
        k::ALGORITHM,
        k::EC_CURVE,
        k::OS_PATCH_LEVEL,
    ] {
        let mut f = fx();
        let moved: std::vec::Vec<_> =
            f.kd.hardware
                .iter()
                .filter(|x| x.starts_with(&kit::tag_bytes(2, true, tag)))
                .cloned()
                .collect();
        k::Kd::remove(&mut f.kd.hardware, tag);
        f.kd.software.extend(moved);
        assert!(f.run().is_err(), "{tag}");
    }
}

#[test]
fn leaf_key_must_be_p256() {
    let mut f = fx();
    f.keys[0] = TestKey::p384(77);
    assert_eq!(err(&f), Error::Algorithm);
}

#[test]
fn patch_level_rules() {
    let mut f = fx();
    k::Kd::replace(
        &mut f.kd.hardware,
        k::OS_PATCH_LEVEL,
        k::int_field(k::OS_PATCH_LEVEL, 202_512),
    );
    assert_eq!(err(&f), Error::PatchLevel);
    k::Kd::replace(
        &mut f.kd.hardware,
        k::OS_PATCH_LEVEL,
        k::int_field(k::OS_PATCH_LEVEL, 202_601),
    );
    assert_eq!(f.run().unwrap().patch, 202_601);
    k::Kd::remove(&mut f.kd.hardware, k::OS_PATCH_LEVEL);
    assert_eq!(err(&f), Error::PatchLevel);
}

#[test]
fn app_identity_rules() {
    let d = kit::RELEASE_DIGEST;
    let cases: [(std::vec::Vec<u8>, Result<(), Error>); 8] = [
        (k::app_id(&[(b"app.witness", 40)], &[&d]), Ok(())),
        (
            k::app_id(&[(b"app.witness", 39)], &[&d]),
            Err(Error::AppVersion),
        ),
        (
            k::app_id(&[(b"app.witnesss", 99)], &[&d]),
            Err(Error::Package),
        ),
        (k::app_id(&[(b"com.evil", 99)], &[&d]), Err(Error::Package)),
        (k::app_id(&[], &[&d]), Err(Error::Package)),
        (
            k::app_id(&[(b"app.witness", 50)], &[&[0xd3; 32]]),
            Err(Error::SignerDigest),
        ),
        (
            k::app_id(&[(b"app.witness", 50)], &[]),
            Err(Error::SignerDigest),
        ),
        (
            k::app_id(&[(b"com.shared", 1), (b"app.witness", 50)], &[&[0; 32], &d]),
            Ok(()),
        ),
    ];
    for (field, want) in cases {
        let mut f = fx();
        k::Kd::replace(&mut f.kd.software, k::APP_ID, field);
        assert_eq!(f.run().map(|_| ()), want);
    }
    let mut f = fx();
    k::Kd::remove(&mut f.kd.software, k::APP_ID);
    assert_eq!(err(&f), Error::Package);
    f.kd.hardware
        .push(k::app_id(&[(b"app.witness", 42)], &[&kit::OTHER_DIGEST]));
    assert!(f.run().is_ok());
    let mut f = fx();
    k::Kd::replace(
        &mut f.kd.software,
        k::APP_ID,
        kit::explicit(k::APP_ID, &kit::octets(&[0x30, 0x00])),
    );
    assert_eq!(err(&f), Error::KeyDescription);
}

#[test]
fn fresh_challenge_window() {
    let base = fx();
    let issue = |age_ns: u64| crate::issue_fresh(&base.secret, base.now_ns - age_ns).to_vec();
    for (age, want) in [
        (0u64, Ok(())),
        (HOUR_NS - 1, Ok(())),
        (HOUR_NS, Ok(())),
        (HOUR_NS + 1, Err(Error::ChallengeExpired)),
        (30 * DAY_NS, Err(Error::ChallengeExpired)),
    ] {
        let mut f = base.clone();
        f.kd.challenge = issue(age);
        assert_eq!(
            f.run().map(|a| a.freshness),
            want.map(|_| Freshness::Fresh),
            "{age}"
        );
    }
    let mut f = base.clone();
    f.kd.challenge = crate::issue_fresh(&f.secret, f.now_ns + 1).to_vec();
    assert_eq!(err(&f), Error::Challenge);
}

#[test]
fn fresh_challenge_must_be_authentic() {
    let mut f = fx();
    f.kd.challenge = crate::issue_fresh(&[0x99; 32], f.now_ns).to_vec();
    assert_eq!(err(&f), Error::Challenge);
    let mut f = fx();
    f.kd.challenge[39] ^= 1;
    assert_eq!(err(&f), Error::Challenge);
    let mut f = fx();
    f.kd.challenge[0] ^= 1;
    assert_eq!(err(&f), Error::Challenge);
    let mut f = fx();
    f.secret = [0x11; 32];
    assert_eq!(err(&f), Error::Challenge);
    for len in [0usize, 16, 31, 33, 39, 41, 64] {
        let mut f = fx();
        f.kd.challenge = std::vec![7; len];
        assert_eq!(err(&f), Error::Challenge, "{len}");
    }
}

#[test]
fn late_challenge_accepted_within_seven_days() {
    let mut f = fx();
    let created = now_ms(&f) - 6 * DAY as u64 * 1000;
    f.make_late(b"install-nonce-0123456789abcdef", created);
    k::Kd::replace(
        &mut f.kd.software,
        k::CREATION_DATETIME,
        k::int_field(k::CREATION_DATETIME, created),
    );
    let a = f.run().unwrap();
    assert_eq!(a.freshness, Freshness::Late);
    assert_eq!(a.created_ms, Some(created));
}

#[test]
fn late_challenge_window_edges() {
    let week_ms = 7 * DAY as u64 * 1000;
    let skew_ms = 10 * 60 * 1000;
    for (offset, ok) in [
        (week_ms as i64, true),
        (week_ms as i64 + 1, false),
        (0, true),
        (-(skew_ms as i64), true),
        (-(skew_ms as i64) - 1, false),
    ] {
        let mut f = fx();
        let created = (now_ms(&f) as i64 - offset) as u64;
        f.make_late(b"n", now_ms(&f) - 1000);
        k::Kd::replace(
            &mut f.kd.software,
            k::CREATION_DATETIME,
            k::int_field(k::CREATION_DATETIME, created),
        );
        let got = f.run().map(|a| a.freshness);
        assert_eq!(
            got,
            if ok {
                Ok(Freshness::Late)
            } else {
                Err(Error::CreationTime)
            },
            "{offset}"
        );
    }
}

#[test]
fn late_wall_clock_must_also_be_recent() {
    let mut f = fx();
    f.make_late(b"n", now_ms(&f) - 8 * DAY as u64 * 1000);
    assert_eq!(err(&f), Error::CreationTime);
    let mut f = fx();
    f.make_late(b"n", now_ms(&f) + HOUR as u64 * 1000);
    assert_eq!(err(&f), Error::CreationTime);
}

#[test]
fn late_creation_falls_back_to_leaf_not_before() {
    let mut f = fx();
    f.make_late(b"n", now_ms(&f));
    k::Kd::remove(&mut f.kd.software, k::CREATION_DATETIME);
    f.certs[0].not_before = f.now() - 2 * DAY;
    assert_eq!(
        f.run().unwrap().created_ms,
        Some((f.now() - 2 * DAY) as u64 * 1000)
    );
    f.certs[0].not_before = f.now() - 8 * DAY;
    assert_eq!(err(&f), Error::CreationTime);
    f.certs[0].not_before = -5;
    assert_eq!(err(&f), Error::CreationTime);
}

#[test]
fn late_creation_prefers_hardware_value() {
    let mut f = fx();
    f.make_late(b"n", now_ms(&f));
    f.kd.hardware.push(k::int_field(
        k::CREATION_DATETIME,
        now_ms(&f) - 9 * DAY as u64 * 1000,
    ));
    assert_eq!(err(&f), Error::CreationTime);
}

#[test]
fn late_challenge_requires_matching_opening() {
    let mut f = fx();
    f.make_late(b"nonce", now_ms(&f));
    f.late = None;
    assert_eq!(err(&f), Error::Challenge);
    let mut f = fx();
    f.make_late(b"nonce", now_ms(&f));
    f.late = Some((b"other".to_vec(), now_ms(&f)));
    assert_eq!(err(&f), Error::Challenge);
    let mut f = fx();
    f.make_late(b"nonce", now_ms(&f));
    f.late = Some((b"nonce".to_vec(), now_ms(&f) - 1));
    assert_eq!(err(&f), Error::Challenge);
    let mut f = fx();
    f.make_late(&[], now_ms(&f));
    assert_eq!(err(&f), Error::Challenge);
    let mut f = fx();
    f.make_late(&[1; MAX_INSTALL_NONCE + 1], now_ms(&f));
    assert_eq!(err(&f), Error::Challenge);
    let mut f = fx();
    f.make_late(&[1; MAX_INSTALL_NONCE], now_ms(&f));
    assert!(f.run().is_ok());
}

#[test]
fn fresh_challenge_ignores_late_opening() {
    let mut f = fx();
    f.late = Some((b"n".to_vec(), 0));
    assert_eq!(f.run().unwrap().freshness, Freshness::Fresh);
}

#[test]
fn leaf_validity_is_always_enforced() {
    for root in [RootAlg::P384, RootAlg::Rsa] {
        let mut f = Fixture::new(root);
        f.certs[0].not_after = f.now() - 1;
        assert_eq!(err(&f), Error::Validity);
        let mut f = Fixture::new(root);
        f.certs[0].not_before = f.now() + 1;
        assert_eq!(err(&f), Error::Validity);
    }
    let mut f = fx();
    f.root_kind = RootKind::Dev;
    f.certs[0].not_after = f.now() - 1;
    assert_eq!(err(&f), Error::Validity);
}

#[test]
fn issuer_validity_depends_on_provisioning() {
    for i in 1..=2 {
        let mut f = fx();
        f.certs[i].not_after = f.now() - 1;
        assert_eq!(err(&f), Error::Validity, "rkp ec {i}");

        let mut f = Fixture::new(RootAlg::Rsa);
        f.certs[i].not_after = f.now() - 1;
        assert!(f.run().is_ok(), "factory {i}");

        let mut f = Fixture::new(RootAlg::Rsa);
        f.certs[1].extensions.push(kit::provisioning_info_ext());
        f.certs[i].not_after = f.now() - 1;
        assert_eq!(err(&f), Error::Validity, "rkp rsa {i}");

        let mut f = fx();
        f.root_kind = RootKind::Dev;
        f.certs[i].not_before = f.now() + 1;
        assert!(f.run().is_ok(), "dev {i}");
    }
}

#[test]
fn root_validity_is_not_enforced() {
    for root in [RootAlg::P384, RootAlg::Rsa] {
        let mut f = Fixture::new(root);
        f.certs[3].not_after = f.now() - 1;
        assert!(f.run().is_ok());
    }
}

fn revoke(f: &mut Fixture, hex: &str, status: RevocationStatus) {
    f.revocations = RevocationList::new(std::vec![
        Revocation {
            serial: Serial::from_hex("deadbeef").unwrap(),
            status: RevocationStatus::Revoked,
            reason: RevocationReason::KeyCompromise,
        },
        Revocation {
            serial: Serial::from_hex(hex).unwrap(),
            status,
            reason: RevocationReason::Unspecified,
        },
    ]);
}

#[test]
fn every_chain_serial_is_checked_against_revocations() {
    let f = fx();
    for hex in ["10ab00", "11ab01", "12ab02", "13ab03"] {
        for status in [RevocationStatus::Revoked, RevocationStatus::Suspended] {
            let mut g = f.clone();
            revoke(&mut g, hex, status);
            assert_eq!(err(&g), Error::Revoked(status), "{hex}");
        }
    }
    let mut g = f.clone();
    revoke(&mut g, "10ab01", RevocationStatus::Revoked);
    assert!(g.run().is_ok());
}

#[test]
fn revocation_uses_normalized_serials() {
    let mut f = fx();
    f.certs[1].serial = std::vec![0x00, 0xc8, 0x96, 0x6f, 0xcb, 0x2f, 0xbb, 0x0d, 0x7a];
    f.revocations =
        crate::parse_status_list(br#"{"entries":{"00C8966FCB2FBB0D7A":{"status":"SUSPENDED"}}}"#)
            .unwrap();
    assert_eq!(err(&f), Error::Revoked(RevocationStatus::Suspended));
    f.certs[1].serial = std::vec![0x5c, 0xb8, 0x38, 0xf1, 0xfe, 0x15, 0x7a, 0x85];
    f.revocations =
        crate::parse_status_list(br#"{"entries":{"6681152659205225093":{"status":"REVOKED"}}}"#)
            .unwrap();
    assert_eq!(err(&f), Error::Revoked(RevocationStatus::Revoked));
}

#[test]
fn signatures_are_verified_for_every_link() {
    let f = fx();
    let chain = f.build();
    for i in 0..3 {
        let mut other = f.clone();
        other.keys[i + 1] = TestKey::p256(5000 + i as u64);
        let mut forged = chain.clone();
        forged[i] = other.build()[i].clone();
        assert_eq!(f.run_chain(&forged), Err(Error::Signature), "{i}");
    }
}

#[test]
fn chain_order_matters() {
    let f = fx();
    let mut chain = f.build();
    chain.swap(1, 2);
    assert!(f.run_chain(&chain).is_err());
    let mut chain = f.build();
    chain.reverse();
    assert!(f.run_chain(&chain).is_err());
}

#[test]
fn dev_root_relaxes_device_state_only() {
    let mut f = fx();
    f.root_kind = RootKind::Dev;
    f.kd.attestation_security = 0;
    f.kd.keymint_security = 0;
    k::Kd::replace(
        &mut f.kd.hardware,
        k::ROOT_OF_TRUST,
        k::root_of_trust(false, 2),
    );
    let a = f.run().unwrap();
    assert_eq!(a.provisioning, Provisioning::Dev);
    assert_eq!(a.security, SecurityLevel::Dev);
    assert_eq!(a.boot, BootState::Unverified);
    assert!(!a.device_locked);
    assert!(a.key_info(0, 1).is_dev());

    let mut g = f.clone();
    k::Kd::remove(&mut g.kd.hardware, k::ROOT_OF_TRUST);
    assert_eq!(g.run().unwrap().boot, BootState::Unverified);

    let mut g = f.clone();
    g.kd.keymint_security = 1;
    assert_eq!(g.run().unwrap().security, SecurityLevel::Tee);

    let mut g = f.clone();
    g.kd.challenge = std::vec![0; 40];
    assert_eq!(g.run().map(|_| ()), Err(Error::Challenge));

    let mut g = f.clone();
    k::Kd::replace(
        &mut g.kd.software,
        k::APP_ID,
        k::app_id(&[(b"app.witness", 50)], &[&[9; 32]]),
    );
    assert_eq!(g.run().map(|_| ()), Err(Error::SignerDigest));

    let mut g = f.clone();
    k::Kd::replace(&mut g.kd.hardware, k::ORIGIN, k::int_field(k::ORIGIN, 1));
    assert_eq!(g.run().map(|_| ()), Err(Error::Origin));
}

#[test]
fn dev_root_reads_software_enforced_lists() {
    let mut f = fx();
    f.root_kind = RootKind::Dev;
    f.kd.attestation_security = 0;
    f.kd.keymint_security = 0;
    let hw = core::mem::take(&mut f.kd.hardware);
    f.kd.software.extend(hw);
    let a = f.run().unwrap();
    assert_eq!(a.patch, 202_611);
    assert!(a.device_locked);
    assert_eq!(a.boot, BootState::Verified);
}

#[test]
fn random_mutations_below_root_never_verify() {
    let f = fx();
    let chain = f.build();
    let good = f.run_chain(&chain).unwrap();
    let mut rng = Rng::new(31);
    for _ in 0..150 {
        let i = rng.below(chain.len() as u64) as usize;
        let mut m = chain.clone();
        let pos = rng.below(m[i].len() as u64) as usize;
        m[i][pos] ^= 1 << rng.below(8);
        if let Ok(a) = f.run_chain(&m) {
            assert_eq!(
                i,
                chain.len() - 1,
                "mutation in signed cert {i} at {pos} verified"
            );
            assert_eq!(a.key_id, good.key_id);
        }
    }
}
