use crate::serial::Serial;
use alloc::vec::Vec;
use witness_format::cbor::{Decode, Decoder, Encode, Encoder, Error as CborError};
use witness_format::{KeyStatus, cbor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RevocationStatus {
    Suspended = 0,
    Revoked = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RevocationReason {
    Unspecified = 0,
    KeyCompromise = 1,
    CaCompromise = 2,
    Superseded = 3,
    SoftwareFlaw = 4,
}

impl RevocationStatus {
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Suspended),
            1 => Some(Self::Revoked),
            _ => None,
        }
    }

    pub fn from_google(text: &str) -> Self {
        if text == "REVOKED" {
            Self::Revoked
        } else {
            Self::Suspended
        }
    }

    pub const fn key_status(self) -> KeyStatus {
        match self {
            Self::Suspended => KeyStatus::Suspended,
            Self::Revoked => KeyStatus::Revoked,
        }
    }
}

impl RevocationReason {
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Unspecified),
            1 => Some(Self::KeyCompromise),
            2 => Some(Self::CaCompromise),
            3 => Some(Self::Superseded),
            4 => Some(Self::SoftwareFlaw),
            _ => None,
        }
    }

    pub fn from_google(text: &str) -> Self {
        match text {
            "KEY_COMPROMISE" => Self::KeyCompromise,
            "CA_COMPROMISE" => Self::CaCompromise,
            "SUPERSEDED" => Self::Superseded,
            "SOFTWARE_FLAW" => Self::SoftwareFlaw,
            _ => Self::Unspecified,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Revocation {
    pub serial: Serial,
    pub status: RevocationStatus,
    pub reason: RevocationReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RevocationList(Vec<Revocation>);

impl RevocationList {
    pub const fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn new(mut entries: Vec<Revocation>) -> Self {
        entries.sort_unstable_by(|a, b| {
            a.serial
                .cmp(&b.serial)
                .then(b.status.cmp(&a.status))
                .then(b.reason.cmp(&a.reason))
        });
        entries.dedup_by_key(|e| e.serial);
        Self(entries)
    }

    pub fn entries(&self) -> &[Revocation] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, serial: &Serial) -> Option<&Revocation> {
        self.0
            .binary_search_by(|e| e.serial.cmp(serial))
            .ok()
            .map(|i| &self.0[i])
    }

    pub fn status(&self, serial: &Serial) -> Option<RevocationStatus> {
        self.get(serial).map(|e| e.status)
    }

    pub fn to_cbor(&self) -> Vec<u8> {
        cbor::to_vec(self)
    }

    pub fn from_cbor(bytes: &[u8]) -> Result<Self, CborError> {
        cbor::from_slice_canonical(bytes)
    }
}

impl Encode for Revocation {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.array(3);
        e.bytes(self.serial.magnitude());
        e.u64(u64::from(self.status as u8));
        e.u64(u64::from(self.reason as u8));
    }
}

fn small_code(d: &mut Decoder<'_>) -> Result<u8, CborError> {
    u8::try_from(d.u64()?).map_err(|_| CborError::Range)
}

impl<'a> Decode<'a> for Revocation {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, CborError> {
        d.array_of(3)?;
        let magnitude = d.bytes()?;
        if magnitude.first() == Some(&0) {
            return Err(CborError::Invalid);
        }
        let serial = Serial::from_be_bytes(magnitude).ok_or(CborError::Length)?;
        let status = RevocationStatus::from_code(small_code(d)?).ok_or(CborError::Range)?;
        let reason = RevocationReason::from_code(small_code(d)?).ok_or(CborError::Range)?;
        Ok(Self {
            serial,
            status,
            reason,
        })
    }
}

impl Encode for RevocationList {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.put(&self.0);
    }
}

impl<'a> Decode<'a> for RevocationList {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, CborError> {
        let entries: Vec<Revocation> = Vec::decode(d)?;
        if entries.windows(2).all(|w| w[0].serial < w[1].serial) {
            Ok(Self(entries))
        } else {
            Err(CborError::Invalid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Rng;

    fn rev(hex: &str, status: RevocationStatus, reason: RevocationReason) -> Revocation {
        Revocation {
            serial: Serial::from_hex(hex).unwrap(),
            status,
            reason,
        }
    }

    #[test]
    fn new_sorts_and_dedups_keeping_most_severe() {
        use RevocationReason::*;
        use RevocationStatus::*;
        let list = RevocationList::new(alloc::vec![
            rev("ff", Suspended, SoftwareFlaw),
            rev("0a", Suspended, Unspecified),
            rev("a", Revoked, KeyCompromise),
            rev("1", Suspended, Superseded),
            rev("a", Suspended, SoftwareFlaw),
            rev("1", Suspended, SoftwareFlaw),
        ]);
        let got: Vec<_> = list
            .entries()
            .iter()
            .map(|e| (e.serial.to_hex(), e.status, e.reason))
            .collect();
        assert_eq!(
            got,
            alloc::vec![
                ("1".into(), Suspended, SoftwareFlaw),
                ("a".into(), Revoked, KeyCompromise),
                ("ff".into(), Suspended, SoftwareFlaw),
            ]
        );
    }

    #[test]
    fn lookup_hits_and_misses() {
        let list = RevocationList::new(alloc::vec![
            rev(
                "2c8cdddfd5e03bfc",
                RevocationStatus::Revoked,
                RevocationReason::KeyCompromise
            ),
            rev(
                "c8966fcb2fbb0d7a",
                RevocationStatus::Suspended,
                RevocationReason::SoftwareFlaw
            ),
        ]);
        let s = |h| Serial::from_hex(h).unwrap();
        assert_eq!(
            list.status(&s("2c8cdddfd5e03bfc")),
            Some(RevocationStatus::Revoked)
        );
        assert_eq!(
            list.status(&s("002C8CDDDFD5E03BFC")),
            Some(RevocationStatus::Revoked)
        );
        assert_eq!(
            list.status(&s("c8966fcb2fbb0d7a")),
            Some(RevocationStatus::Suspended)
        );
        assert_eq!(list.status(&s("c8966fcb2fbb0d7b")), None);
        assert_eq!(list.status(&Serial::ZERO), None);
        assert_eq!(RevocationList::empty().status(&s("1")), None);
        assert!(RevocationList::empty().is_empty());
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn status_mapping() {
        assert_eq!(RevocationStatus::Revoked.key_status(), KeyStatus::Revoked);
        assert_eq!(
            RevocationStatus::Suspended.key_status(),
            KeyStatus::Suspended
        );
        assert_eq!(
            RevocationStatus::from_google("REVOKED"),
            RevocationStatus::Revoked
        );
        assert_eq!(
            RevocationStatus::from_google("SUSPENDED"),
            RevocationStatus::Suspended
        );
        assert_eq!(
            RevocationStatus::from_google("revoked"),
            RevocationStatus::Suspended
        );
        assert_eq!(
            RevocationStatus::from_google("NEW_STATE"),
            RevocationStatus::Suspended
        );
        assert_eq!(
            RevocationReason::from_google("KEY_COMPROMISE"),
            RevocationReason::KeyCompromise
        );
        assert_eq!(
            RevocationReason::from_google("CA_COMPROMISE"),
            RevocationReason::CaCompromise
        );
        assert_eq!(
            RevocationReason::from_google("SUPERSEDED"),
            RevocationReason::Superseded
        );
        assert_eq!(
            RevocationReason::from_google("SOFTWARE_FLAW"),
            RevocationReason::SoftwareFlaw
        );
        assert_eq!(
            RevocationReason::from_google("UNSPECIFIED"),
            RevocationReason::Unspecified
        );
        assert_eq!(
            RevocationReason::from_google("other"),
            RevocationReason::Unspecified
        );
        for c in 0..=255u8 {
            if let Some(s) = RevocationStatus::from_code(c) {
                assert_eq!(s as u8, c);
            }
            if let Some(r) = RevocationReason::from_code(c) {
                assert_eq!(r as u8, c);
            }
        }
        assert!(RevocationStatus::from_code(2).is_none());
        assert!(RevocationReason::from_code(5).is_none());
    }

    fn random_list(rng: &mut Rng, n: usize) -> RevocationList {
        RevocationList::new(
            (0..n)
                .map(|_| {
                    let mut b = [0u8; 16];
                    rng.fill(&mut b);
                    let len = rng.below(17) as usize;
                    Revocation {
                        serial: Serial::from_be_bytes(&b[..len]).unwrap(),
                        status: RevocationStatus::from_code(rng.below(2) as u8).unwrap(),
                        reason: RevocationReason::from_code(rng.below(5) as u8).unwrap(),
                    }
                })
                .collect(),
        )
    }

    #[test]
    fn cbor_round_trips_and_is_canonical() {
        let mut rng = Rng::new(42);
        for n in [0usize, 1, 2, 17, 300] {
            let list = random_list(&mut rng, n);
            let bytes = list.to_cbor();
            let back = RevocationList::from_cbor(&bytes).unwrap();
            assert_eq!(back, list);
            assert_eq!(back.to_cbor(), bytes);
            for e in list.entries() {
                assert_eq!(list.get(&e.serial), Some(e));
            }
        }
    }

    #[test]
    fn construction_is_order_independent() {
        let mut rng = Rng::new(9);
        let list = random_list(&mut rng, 200);
        let mut shuffled = list.entries().to_vec();
        for i in (1..shuffled.len()).rev() {
            shuffled.swap(i, rng.below(i as u64 + 1) as usize);
        }
        assert_eq!(RevocationList::new(shuffled).to_cbor(), list.to_cbor());
    }

    #[test]
    fn cbor_rejects_unsorted_duplicate_and_malformed() {
        let a = rev(
            "1",
            RevocationStatus::Revoked,
            RevocationReason::Unspecified,
        );
        let b = rev(
            "2",
            RevocationStatus::Revoked,
            RevocationReason::Unspecified,
        );
        let unsorted = cbor::to_vec(&alloc::vec![b, a]);
        assert!(RevocationList::from_cbor(&unsorted).is_err());
        let dup = cbor::to_vec(&alloc::vec![a, a]);
        assert!(RevocationList::from_cbor(&dup).is_err());
        let mut buf = Vec::new();
        let mut e = Encoder::new(&mut buf);
        e.array(1);
        e.array(3);
        e.bytes(&[0, 1]);
        e.u64(1);
        e.u64(0);
        assert!(RevocationList::from_cbor(&buf).is_err());
        for (status, reason) in [(2u64, 0u64), (0, 5), (300, 0)] {
            let mut buf = Vec::new();
            let mut e = Encoder::new(&mut buf);
            e.array(1);
            e.array(3);
            e.bytes(&[1]);
            e.u64(status);
            e.u64(reason);
            assert!(RevocationList::from_cbor(&buf).is_err());
        }
        let mut buf = Vec::new();
        let mut e = Encoder::new(&mut buf);
        e.array(1);
        e.array(3);
        e.bytes(&[1; 33]);
        e.u64(0);
        e.u64(0);
        assert!(RevocationList::from_cbor(&buf).is_err());
        assert!(RevocationList::from_cbor(&[0x80, 0x00]).is_err());
        assert!(RevocationList::from_cbor(&[]).is_err());
    }
}
