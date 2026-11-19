use super::*;
use crate::testkit::{self as kit, Fixture, Rng, RootAlg, kd as k};

fn good() -> k::Kd {
    Fixture::new(RootAlg::P384).kd
}

#[test]
fn parses_full_description() {
    let src = good();
    let der = src.der();
    let d = KeyDescription::parse(&der).unwrap();
    assert_eq!(d.attestation_version, 300);
    assert_eq!(d.keymint_version, 300);
    assert_eq!(d.attestation_security, KmSecurity::Tee);
    assert_eq!(d.keymint_security, KmSecurity::Tee);
    assert_eq!(d.challenge, src.challenge.as_slice());
    assert_eq!(d.unique_id, &[] as &[u8]);
    let hw = d.hardware;
    assert!(hw.has_purpose(PURPOSE_SIGN));
    assert!(hw.has_purpose(3));
    assert!(!hw.has_purpose(0));
    assert!(!hw.has_purpose(64));
    assert_eq!(hw.algorithm, Some(ALGORITHM_EC));
    assert_eq!(hw.key_size, Some(256));
    assert_eq!(hw.ec_curve, Some(CURVE_P256));
    assert_eq!(hw.origin, Some(ORIGIN_GENERATED));
    assert_eq!(hw.os_patch_level, Some(202_611));
    assert_eq!(hw.vendor_patch_level, Some(20_261_105));
    assert_eq!(hw.boot_patch_level, Some(20_261_105));
    let rot = hw.root_of_trust.unwrap();
    assert!(rot.device_locked);
    assert_eq!(rot.state, VerifiedBoot::Verified);
    assert_eq!(rot.verified_boot_key, &[0x11; 32]);
    assert_eq!(rot.verified_boot_hash, Some(&[0x22u8; 32][..]));
    assert_eq!(hw.creation_ms, None);
    assert_eq!(hw.attestation_application_id, None);
    let sw = d.software;
    assert!(sw.creation_ms.is_some());
    let app = AppId::parse(sw.attestation_application_id.unwrap()).unwrap();
    assert_eq!(app.packages, std::vec![(&b"app.witness"[..], 42)]);
    assert_eq!(app.digests, std::vec![&[0xd1u8; 32][..]]);
}

#[test]
fn security_levels_and_boot_states() {
    for (code, want) in [
        (0, KmSecurity::Software),
        (1, KmSecurity::Tee),
        (2, KmSecurity::StrongBox),
    ] {
        let mut kd = good();
        kd.attestation_security = code;
        kd.keymint_security = code;
        let der = kd.der();
        let d = KeyDescription::parse(&der).unwrap();
        assert_eq!((d.attestation_security, d.keymint_security), (want, want));
    }
    for (code, want) in [
        (0, VerifiedBoot::Verified),
        (1, VerifiedBoot::SelfSigned),
        (2, VerifiedBoot::Unverified),
        (3, VerifiedBoot::Failed),
    ] {
        let mut kd = good();
        k::Kd::replace(
            &mut kd.hardware,
            k::ROOT_OF_TRUST,
            k::root_of_trust(false, code),
        );
        let der = kd.der();
        let rot = KeyDescription::parse(&der)
            .unwrap()
            .hardware
            .root_of_trust
            .unwrap();
        assert_eq!(rot.state, want);
        assert!(!rot.device_locked);
    }
}

#[test]
fn rejects_out_of_range_enums() {
    let mut kd = good();
    kd.attestation_security = 3;
    assert_eq!(KeyDescription::parse(&kd.der()), Err(Error::KeyDescription));
    let mut kd = good();
    kd.keymint_security = 100;
    assert_eq!(KeyDescription::parse(&kd.der()), Err(Error::KeyDescription));
    let mut kd = good();
    k::Kd::replace(
        &mut kd.hardware,
        k::ROOT_OF_TRUST,
        k::root_of_trust(true, 4),
    );
    assert_eq!(KeyDescription::parse(&kd.der()), Err(Error::KeyDescription));
}

#[test]
fn root_of_trust_without_hash_is_v2_compatible() {
    let mut kd = good();
    let rot = kit::explicit(
        k::ROOT_OF_TRUST,
        &kit::seq(&[
            &kit::octets(&[1; 32]),
            &kit::boolean(true),
            &kit::enumerated(0),
        ]),
    );
    k::Kd::replace(&mut kd.hardware, k::ROOT_OF_TRUST, rot);
    let der = kd.der();
    let rot = KeyDescription::parse(&der)
        .unwrap()
        .hardware
        .root_of_trust
        .unwrap();
    assert_eq!(rot.verified_boot_hash, None);
}

#[test]
fn rejects_malformed_root_of_trust() {
    let bads = [
        kit::seq(&[&kit::octets(&[1]), &kit::boolean(true)]),
        kit::seq(&[
            &kit::octets(&[1]),
            &kit::tlv(&[0x01], &[0x01]),
            &kit::enumerated(0),
        ]),
        kit::seq(&[&kit::octets(&[1]), &kit::boolean(true), &kit::int(0)]),
        kit::seq(&[
            &kit::octets(&[1]),
            &kit::boolean(true),
            &kit::enumerated(0),
            &kit::octets(&[]),
            &kit::null(),
        ]),
        kit::octets(&[]),
    ];
    for bad in bads {
        let mut kd = good();
        k::Kd::replace(
            &mut kd.hardware,
            k::ROOT_OF_TRUST,
            kit::explicit(k::ROOT_OF_TRUST, &bad),
        );
        assert_eq!(
            KeyDescription::parse(&kd.der()),
            Err(Error::KeyDescription),
            "{bad:x?}"
        );
    }
}

#[test]
fn rejects_duplicate_known_tags_in_one_list() {
    for field in [
        k::int_field(k::ALGORITHM, 3),
        k::int_field(k::EC_CURVE, 1),
        k::int_field(k::ORIGIN, 0),
        k::int_field(k::OS_PATCH_LEVEL, 202_611),
        k::purpose(&[2]),
        k::root_of_trust(true, 0),
    ] {
        let mut kd = good();
        kd.hardware.push(field);
        assert_eq!(KeyDescription::parse(&kd.der()), Err(Error::KeyDescription));
    }
    let mut kd = good();
    kd.software.push(k::int_field(k::CREATION_DATETIME, 1));
    assert_eq!(KeyDescription::parse(&kd.der()), Err(Error::KeyDescription));
}

#[test]
fn unknown_tags_are_skipped_including_duplicates() {
    let mut kd = good();
    kd.hardware.push(k::int_field(9999, 1));
    kd.hardware.push(k::int_field(9999, 2));
    kd.hardware.push(k::null_field(k::NO_AUTH_REQUIRED));
    kd.software.push(kit::explicit(710, &kit::octets(b"brand")));
    let der = kd.der();
    let d = KeyDescription::parse(&der).unwrap();
    assert_eq!(d.hardware.algorithm, Some(ALGORITHM_EC));
}

#[test]
fn rejects_non_context_or_primitive_fields() {
    for bad in [
        kit::int(5),
        kit::tlv(&[0x82], &[0x02, 0x01, 0x03]),
        kit::tlv(&[0x62], &kit::int(3)),
    ] {
        let mut kd = good();
        kd.hardware.push(bad.clone());
        assert_eq!(
            KeyDescription::parse(&kd.der()),
            Err(Error::KeyDescription),
            "{bad:x?}"
        );
    }
}

#[test]
fn rejects_wrong_inner_types() {
    for (tag, inner) in [
        (k::ALGORITHM, kit::octets(&[3])),
        (k::ALGORITHM, kit::cat(&[&kit::int(3), &kit::int(3)])),
        (k::PURPOSE, kit::int(2)),
        (k::PURPOSE, kit::set(&[&kit::octets(&[2])])),
        (k::PURPOSE, kit::set(&[&kit::int_raw(&[0xff])])),
        (k::OS_PATCH_LEVEL, kit::int(1 << 33)),
        (k::OS_PATCH_LEVEL, kit::int_raw(&[0x80])),
        (k::APP_ID, kit::int(1)),
        (k::CREATION_DATETIME, kit::int_raw(&[0x00, 0x01])),
    ] {
        let mut kd = good();
        k::Kd::replace(&mut kd.hardware, tag, kit::explicit(tag, &inner));
        assert_eq!(
            KeyDescription::parse(&kd.der()),
            Err(Error::KeyDescription),
            "{tag} {inner:x?}"
        );
    }
}

#[test]
fn large_purposes_are_ignored() {
    let mut kd = good();
    k::Kd::replace(&mut kd.hardware, k::PURPOSE, k::purpose(&[2, 64, 1000]));
    let der = kd.der();
    let d = KeyDescription::parse(&der).unwrap();
    assert_eq!(d.hardware.purposes, 1 << 2);
    let mut kd = good();
    k::Kd::replace(&mut kd.hardware, k::PURPOSE, k::purpose(&[]));
    let der = kd.der();
    assert!(
        !KeyDescription::parse(&der)
            .unwrap()
            .hardware
            .has_purpose(PURPOSE_SIGN)
    );
}

#[test]
fn trailing_fields_are_tolerated() {
    let mut kd = good();
    kd.trailing = kit::octets(b"future field");
    assert!(KeyDescription::parse(&kd.der()).is_ok());
}

#[test]
fn rejects_structural_damage() {
    let der = good().der();
    for cut in 0..der.len() {
        assert_eq!(
            KeyDescription::parse(&der[..cut]),
            Err(Error::KeyDescription),
            "{cut}"
        );
    }
    let mut extended = der.clone();
    extended.push(0);
    assert_eq!(KeyDescription::parse(&extended), Err(Error::KeyDescription));
    let mut kd = good();
    kd.attestation_version = 1 << 40;
    assert_eq!(KeyDescription::parse(&kd.der()), Err(Error::KeyDescription));
}

#[test]
fn random_mutations_never_panic() {
    let der = good().der();
    let mut rng = Rng::new(23);
    for _ in 0..20_000 {
        let mut m = der.clone();
        for _ in 0..=rng.below(3) {
            let i = rng.below(m.len() as u64) as usize;
            m[i] = rng.next() as u8;
        }
        if let Ok(d) = KeyDescription::parse(&m)
            && let Some(app) = d.software.attestation_application_id
        {
            let _ = AppId::parse(app);
        }
    }
}

#[test]
fn app_id_variants() {
    let der = k::app_id_der(
        &[(b"com.other", 7), (b"app.witness", 42), (b"app.witness", 3)],
        &[&[1u8; 32], &[2u8; 32]],
    );
    let app = AppId::parse(&der).unwrap();
    assert_eq!(app.packages.len(), 3);
    assert_eq!(app.packages[1], (&b"app.witness"[..], 42));
    assert_eq!(app.digests.len(), 2);
    let empty_der = k::app_id_der(&[], &[]);
    let empty = AppId::parse(&empty_der).unwrap();
    assert!(empty.packages.is_empty() && empty.digests.is_empty());
}

#[test]
fn app_id_rejects_malformed() {
    let bads = [
        kit::seq(&[&kit::set(&[])]),
        kit::seq(&[&kit::set(&[]), &kit::set(&[]), &kit::set(&[])]),
        kit::seq(&[
            &kit::set(&[&kit::seq(&[&kit::octets(b"a")])]),
            &kit::set(&[]),
        ]),
        kit::seq(&[
            &kit::set(&[&kit::seq(&[&kit::octets(b"a"), &kit::int(1), &kit::int(1)])]),
            &kit::set(&[]),
        ]),
        kit::seq(&[
            &kit::set(&[&kit::seq(&[&kit::octets(b"a"), &kit::int_raw(&[0xff])])]),
            &kit::set(&[]),
        ]),
        kit::seq(&[&kit::set(&[]), &kit::set(&[&kit::int(1)])]),
        kit::seq(&[&kit::seq(&[]), &kit::set(&[])]),
        kit::octets(&[]),
        std::vec![],
    ];
    for bad in bads {
        assert_eq!(AppId::parse(&bad), Err(Error::KeyDescription), "{bad:x?}");
    }
}
