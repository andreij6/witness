code_enum! {
    pub enum Outcome {
        Original = 0,
        MatchesWithChanges = 1,
        Altered = 2,
        NoRecord = 3,
        SealedUnanchored = 4,
    }
}

bitset! {
    pub struct VerdictFlags: u16 {
        const KEY_REVOKED_AFTER_ANCHOR = 0;
        const LATE_ANCHOR = 1;
        const CLOCK_MISMATCH = 2;
        const COULDNT_COMPARE = 3;
        const NO_PUBLIC_REFERENCE = 4;
        const DEV_ATTESTATION = 5;
        const TRUNCATED_CANDIDATES = 6;
    }
}

impl Outcome {
    pub const fn is_sealed_match(self) -> bool {
        matches!(
            self,
            Self::Original | Self::MatchesWithChanges | Self::SealedUnanchored
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cbor::{Error, from_slice, to_vec};

    #[test]
    fn outcome_codes_are_stable() {
        assert_eq!(to_vec(&Outcome::Original), [0x00]);
        assert_eq!(to_vec(&Outcome::MatchesWithChanges), [0x01]);
        assert_eq!(to_vec(&Outcome::Altered), [0x02]);
        assert_eq!(to_vec(&Outcome::NoRecord), [0x03]);
        assert_eq!(to_vec(&Outcome::SealedUnanchored), [0x04]);
        assert_eq!(Outcome::ALL.len(), 5);
    }

    #[test]
    fn outcome_roundtrip_and_rejects_unknown() {
        for o in Outcome::ALL {
            assert_eq!(from_slice::<Outcome>(&to_vec(o)), Ok(*o));
            assert_eq!(Outcome::from_code(o.code()), Some(*o));
        }
        assert_eq!(from_slice::<Outcome>(&[0x05]), Err(Error::Range));
        assert_eq!(
            from_slice::<Outcome>(&[0x19, 0x01, 0x00]),
            Err(Error::Range)
        );
        assert_eq!(from_slice::<Outcome>(&[0x20]), Err(Error::Type));
    }

    #[test]
    fn sealed_match_classification() {
        assert!(Outcome::Original.is_sealed_match());
        assert!(Outcome::MatchesWithChanges.is_sealed_match());
        assert!(Outcome::SealedUnanchored.is_sealed_match());
        assert!(!Outcome::Altered.is_sealed_match());
        assert!(!Outcome::NoRecord.is_sealed_match());
    }

    #[test]
    fn flag_bits_are_stable() {
        assert_eq!(VerdictFlags::KEY_REVOKED_AFTER_ANCHOR.bits(), 1);
        assert_eq!(VerdictFlags::LATE_ANCHOR.bits(), 2);
        assert_eq!(VerdictFlags::CLOCK_MISMATCH.bits(), 4);
        assert_eq!(VerdictFlags::COULDNT_COMPARE.bits(), 8);
        assert_eq!(VerdictFlags::NO_PUBLIC_REFERENCE.bits(), 16);
        assert_eq!(VerdictFlags::DEV_ATTESTATION.bits(), 32);
        assert_eq!(VerdictFlags::TRUNCATED_CANDIDATES.bits(), 64);
        assert_eq!(VerdictFlags::ALL.bits(), 0x7f);
    }

    #[test]
    fn flag_set_operations() {
        let mut f = VerdictFlags::EMPTY;
        assert!(f.is_empty());
        f.insert(VerdictFlags::LATE_ANCHOR);
        f |= VerdictFlags::DEV_ATTESTATION;
        assert!(f.contains(VerdictFlags::LATE_ANCHOR | VerdictFlags::DEV_ATTESTATION));
        assert!(!f.contains(VerdictFlags::CLOCK_MISMATCH));
        assert!(f.intersects(VerdictFlags::DEV_ATTESTATION | VerdictFlags::CLOCK_MISMATCH));
        assert_eq!(f.len(), 2);
        f.remove(VerdictFlags::LATE_ANCHOR);
        assert_eq!(f, VerdictFlags::DEV_ATTESTATION);
        f.set(VerdictFlags::COULDNT_COMPARE, true);
        f.set(VerdictFlags::DEV_ATTESTATION, false);
        assert_eq!(f, VerdictFlags::COULDNT_COMPARE);
        assert_eq!(
            (VerdictFlags::ALL & VerdictFlags::LATE_ANCHOR),
            VerdictFlags::LATE_ANCHOR
        );
        assert_eq!(
            VerdictFlags::ALL.difference(VerdictFlags::ALL),
            VerdictFlags::EMPTY
        );
        let listed: alloc::vec::Vec<_> = VerdictFlags::ALL.iter().collect();
        assert_eq!(listed.as_slice(), VerdictFlags::FLAGS);
    }

    #[test]
    fn flags_reject_unknown_bits() {
        assert_eq!(VerdictFlags::from_bits(0x80), None);
        assert_eq!(VerdictFlags::from_bits_truncate(0xffff), VerdictFlags::ALL);
        assert_eq!(from_slice::<VerdictFlags>(&[0x18, 0x80]), Err(Error::Range));
        assert_eq!(
            from_slice::<VerdictFlags>(&[0x1a, 0, 1, 0, 0]),
            Err(Error::Range)
        );
        assert_eq!(
            from_slice::<VerdictFlags>(&[0x18, 0x7f]),
            Ok(VerdictFlags::ALL)
        );
    }

    #[test]
    fn every_flag_subset_roundtrips() {
        for bits in 0..=VerdictFlags::ALL.bits() {
            let f = VerdictFlags::from_bits(bits).unwrap();
            assert_eq!(from_slice::<VerdictFlags>(&to_vec(&f)), Ok(f));
        }
    }
}
