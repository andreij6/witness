#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootKind {
    GoogleRsa,
    GoogleEc,
    Dev,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustAnchor<'a> {
    pub spki: &'a [u8],
    pub kind: RootKind,
}

pub const GOOGLE_RSA_SPKI: &[u8] = include_bytes!("../roots/google_rsa.spki.der");
pub const GOOGLE_EC_CA1_SPKI: &[u8] = include_bytes!("../roots/google_ec_ca1.spki.der");
pub const AOSP_SOFT_EC_SPKI: &[u8] = include_bytes!("../roots/aosp_soft_ec.spki.der");
pub const AOSP_SOFT_RSA_SPKI: &[u8] = include_bytes!("../roots/aosp_soft_rsa.spki.der");
pub const DROID_UNREGISTERED_SPKI: &[u8] =
    include_bytes!("../roots/droid_unregistered_ec.spki.der");

pub const GOOGLE_RSA: TrustAnchor<'static> = TrustAnchor {
    spki: GOOGLE_RSA_SPKI,
    kind: RootKind::GoogleRsa,
};
pub const GOOGLE_EC_CA1: TrustAnchor<'static> = TrustAnchor {
    spki: GOOGLE_EC_CA1_SPKI,
    kind: RootKind::GoogleEc,
};
pub const AOSP_SOFT_EC: TrustAnchor<'static> = TrustAnchor {
    spki: AOSP_SOFT_EC_SPKI,
    kind: RootKind::Dev,
};
pub const AOSP_SOFT_RSA: TrustAnchor<'static> = TrustAnchor {
    spki: AOSP_SOFT_RSA_SPKI,
    kind: RootKind::Dev,
};
pub const DROID_UNREGISTERED: TrustAnchor<'static> = TrustAnchor {
    spki: DROID_UNREGISTERED_SPKI,
    kind: RootKind::Dev,
};

#[cfg(not(feature = "dev-attestation"))]
pub const ANCHORS: &[TrustAnchor<'static>] = &[GOOGLE_RSA, GOOGLE_EC_CA1];

#[cfg(feature = "dev-attestation")]
pub const ANCHORS: &[TrustAnchor<'static>] = &[
    GOOGLE_RSA,
    GOOGLE_EC_CA1,
    AOSP_SOFT_EC,
    AOSP_SOFT_RSA,
    DROID_UNREGISTERED,
];

pub fn find<'a>(anchors: &[TrustAnchor<'a>], spki: &[u8]) -> Option<TrustAnchor<'a>> {
    anchors.iter().find(|a| a.spki == spki).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pubkey::PublicKey;
    use witness_format::crypto::sha256;

    fn hex(b: &[u8]) -> alloc::string::String {
        b.iter().map(|x| alloc::format!("{x:02x}")).collect()
    }

    #[test]
    fn pinned_spki_fingerprints() {
        assert_eq!(
            hex(&sha256(GOOGLE_RSA_SPKI)),
            "feb2ea7551ee316ed4bb443c8293b884dbfdea40b603ee3e4f4a897e4580fbae"
        );
        assert_eq!(
            hex(&sha256(GOOGLE_EC_CA1_SPKI)),
            "3ee44512a1af2beb39c889490c60ea3f82e43f5d5a5532f5ab9419f676cd07ec"
        );
        assert_eq!(
            hex(&sha256(AOSP_SOFT_EC_SPKI)),
            "d5100c7942ef2e8310dc30ef82729680cf48d690735c3f68179a33c7c370f286"
        );
        assert_eq!(
            hex(&sha256(AOSP_SOFT_RSA_SPKI)),
            "f2c4746f545946c100e72297f8f946344d7052f03a2f694221f9c893b0e6f711"
        );
        assert_eq!(
            hex(&sha256(DROID_UNREGISTERED_SPKI)),
            "ff7a4c045501b8b8db811ee7d49867e8751356b644534e17658740d3a1a14e20"
        );
    }

    #[test]
    fn pinned_keys_parse_with_expected_types() {
        assert!(matches!(
            PublicKey::from_spki(GOOGLE_RSA_SPKI),
            Ok(PublicKey::Rsa(_))
        ));
        assert!(matches!(
            PublicKey::from_spki(GOOGLE_EC_CA1_SPKI),
            Ok(PublicKey::P384(_))
        ));
        assert!(matches!(
            PublicKey::from_spki(AOSP_SOFT_EC_SPKI),
            Ok(PublicKey::P256(_))
        ));
        assert!(matches!(
            PublicKey::from_spki(AOSP_SOFT_RSA_SPKI),
            Ok(PublicKey::Rsa(_))
        ));
        assert!(matches!(
            PublicKey::from_spki(DROID_UNREGISTERED_SPKI),
            Ok(PublicKey::P256(_))
        ));
    }

    #[test]
    fn anchors_follow_feature() {
        assert!(ANCHORS.contains(&GOOGLE_RSA));
        assert!(ANCHORS.contains(&GOOGLE_EC_CA1));
        let dev = ANCHORS.iter().filter(|a| a.kind == RootKind::Dev).count();
        assert_eq!(
            dev,
            if cfg!(feature = "dev-attestation") {
                3
            } else {
                0
            }
        );
    }

    #[test]
    fn find_matches_exact_bytes_only() {
        assert_eq!(find(ANCHORS, GOOGLE_RSA_SPKI), Some(GOOGLE_RSA));
        assert_eq!(
            find(ANCHORS, GOOGLE_EC_CA1_SPKI).map(|a| a.kind),
            Some(RootKind::GoogleEc)
        );
        assert_eq!(
            find(ANCHORS, &GOOGLE_RSA_SPKI[..GOOGLE_RSA_SPKI.len() - 1]),
            None
        );
        let mut flipped = GOOGLE_EC_CA1_SPKI.to_vec();
        flipped[40] ^= 1;
        assert_eq!(find(ANCHORS, &flipped), None);
        assert_eq!(find(ANCHORS, &[]), None);
        assert_eq!(find(&[], GOOGLE_RSA_SPKI), None);
        assert_eq!(
            find(ANCHORS, AOSP_SOFT_EC_SPKI).is_some(),
            cfg!(feature = "dev-attestation")
        );
        assert_eq!(
            find(ANCHORS, DROID_UNREGISTERED_SPKI).map(|a| a.kind),
            cfg!(feature = "dev-attestation").then_some(RootKind::Dev)
        );
    }
}
