use ic_stable_structures::Storable;
use ic_stable_structures::storable::Bound;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Trace {
    pub local_ix: Option<u32>,
    pub wm_root: u64,
    pub asset: Option<[u8; 16]>,
    pub pixel: Option<[u8; 16]>,
    pub root_ix: Option<u64>,
    pub erased: bool,
}

const LOCAL: u8 = 1;
const ASSET: u8 = 2;
const PIXEL: u8 = 4;
const ROOT: u8 = 8;
const ERASED: u8 = 16;
pub const TRACE_BYTES: usize = 1 + 4 + 8 + 16 + 16 + 8;

impl Trace {
    pub const ERASED: Self = Self {
        local_ix: None,
        wm_root: 0,
        asset: None,
        pixel: None,
        root_ix: None,
        erased: true,
    };

    fn encode(&self) -> [u8; TRACE_BYTES] {
        let mut out = [0u8; TRACE_BYTES];
        let flag = |present: bool, bit: u8| if present { bit } else { 0 };
        out[0] = flag(self.local_ix.is_some(), LOCAL)
            | flag(self.asset.is_some(), ASSET)
            | flag(self.pixel.is_some(), PIXEL)
            | flag(self.root_ix.is_some(), ROOT)
            | flag(self.erased, ERASED);
        out[1..5].copy_from_slice(&self.local_ix.unwrap_or(0).to_be_bytes());
        out[5..13].copy_from_slice(&self.wm_root.to_be_bytes());
        out[13..29].copy_from_slice(&self.asset.unwrap_or_default());
        out[29..45].copy_from_slice(&self.pixel.unwrap_or_default());
        out[45..53].copy_from_slice(&self.root_ix.unwrap_or(0).to_be_bytes());
        out
    }

    fn decode(b: &[u8]) -> Self {
        assert_eq!(b.len(), TRACE_BYTES, "trace length");
        let flags = b[0];
        let has = |bit: u8| flags & bit != 0;
        let u64_at = |at: usize| u64::from_be_bytes(b[at..at + 8].try_into().expect("8 bytes"));
        let key_at = |at: usize| -> [u8; 16] { b[at..at + 16].try_into().expect("16 bytes") };
        Self {
            local_ix: has(LOCAL).then(|| u32::from_be_bytes(b[1..5].try_into().expect("4 bytes"))),
            wm_root: u64_at(5),
            asset: has(ASSET).then(|| key_at(13)),
            pixel: has(PIXEL).then(|| key_at(29)),
            root_ix: has(ROOT).then(|| u64_at(45)),
            erased: has(ERASED),
        }
    }
}

impl Storable for Trace {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.encode().to_vec())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.encode().to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self::decode(&bytes)
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: TRACE_BYTES as u32,
        is_fixed_size: true,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> Trace {
        Trace {
            local_ix: Some(0xDEAD_BEEF),
            wm_root: u64::MAX - 3,
            asset: Some([7; 16]),
            pixel: Some([9; 16]),
            root_ix: Some(12),
            erased: false,
        }
    }

    #[test]
    fn round_trip_full_and_sparse() {
        for t in [full(), Trace::default(), Trace::ERASED] {
            assert_eq!(Trace::from_bytes(t.to_bytes()), t);
            assert_eq!(t.to_bytes().len(), TRACE_BYTES);
        }
    }

    #[test]
    fn zero_values_survive_presence_flags() {
        let t = Trace {
            local_ix: Some(0),
            asset: Some([0; 16]),
            root_ix: Some(0),
            ..Trace::default()
        };
        let back = Trace::from_bytes(t.to_bytes());
        assert_eq!(back.local_ix, Some(0));
        assert_eq!(back.asset, Some([0; 16]));
        assert_eq!(back.root_ix, Some(0));
        assert_eq!(back.pixel, None);
    }

    #[test]
    fn erased_tombstone_holds_no_data() {
        let bytes = Trace::ERASED.to_bytes();
        assert_eq!(bytes[0], ERASED);
        assert!(bytes[1..].iter().all(|&b| b == 0));
    }

    #[test]
    fn unknown_flag_bits_ignored() {
        let mut bytes = Trace::default().encode();
        bytes[0] = 0xE0;
        assert_eq!(
            Trace::from_bytes(Cow::Owned(bytes.to_vec())),
            Trace::default()
        );
    }

    #[test]
    #[should_panic(expected = "trace length")]
    fn wrong_length_rejected() {
        Trace::from_bytes(Cow::Owned(vec![0; TRACE_BYTES + 1]));
    }

    #[test]
    fn deterministic_encoding() {
        assert_eq!(full().to_bytes(), full().to_bytes());
        assert!(Trace::BOUND.is_fixed_size());
    }
}
