use crate::oracle::Record;
use witness_format::{Edit, Outcome, RecordId, VerdictFlags, WitnessClaim};

pub const VERDICT_V: u16 = 1;
pub const Q16_ONE: u32 = 1 << 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Reason {
    SealVerified,
    SealBroken,
    UnverifiedCredentials,
    OtherCredentials,
    KeyNotTrusted,
    KeyNotTrustedAtAnchor,
    ExactAssetHash,
    ExactPixelHash,
    WatermarkFound,
    WatermarkUnavailable,
    ForeignWatermark,
    Unconfirmed,
    NoCandidates,
    Undecodable,
    IndexPending,
}

impl Reason {
    pub const ALL: [Self; 15] = [
        Self::SealVerified,
        Self::SealBroken,
        Self::UnverifiedCredentials,
        Self::OtherCredentials,
        Self::KeyNotTrusted,
        Self::KeyNotTrustedAtAnchor,
        Self::ExactAssetHash,
        Self::ExactPixelHash,
        Self::WatermarkFound,
        Self::WatermarkUnavailable,
        Self::ForeignWatermark,
        Self::Unconfirmed,
        Self::NoCandidates,
        Self::Undecodable,
        Self::IndexPending,
    ];

    const fn bit(self) -> u32 {
        1 << self as u8
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Reasons(u32);

impl Reasons {
    pub const EMPTY: Self = Self(0);

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn from_bits(bits: u32) -> Option<Self> {
        if bits >> Reason::ALL.len() == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    pub const fn contains(self, reason: Reason) -> bool {
        self.0 & reason.bit() != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    pub fn insert(&mut self, reason: Reason) {
        self.0 |= reason.bit();
    }

    pub fn iter(self) -> impl Iterator<Item = Reason> {
        Reason::ALL.into_iter().filter(move |r| self.contains(*r))
    }
}

impl FromIterator<Reason> for Reasons {
    fn from_iter<I: IntoIterator<Item = Reason>>(iter: I) -> Self {
        let mut out = Self::EMPTY;
        iter.into_iter().for_each(|r| out.insert(r));
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Evidence {
    Seal,
    ExactHash,
    Pixels,
    Grid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Region {
    pub fn normalized(x: u32, y: u32, w: u32, h: u32, frame: (u32, u32)) -> Option<Self> {
        let (fw, fh) = (u64::from(frame.0), u64::from(frame.1));
        if fw == 0 || fh == 0 || w == 0 || h == 0 {
            return None;
        }
        let one = u64::from(Q16_ONE);
        let axis = |start: u32, len: u32, total: u64| {
            let lo = u64::from(start).min(total);
            let hi = (lo + u64::from(len)).min(total);
            let a = lo * one / total;
            let b = (hi * one).div_ceil(total);
            (a as u32, (b - a) as u32)
        };
        let (x, w) = axis(x, w, fw);
        let (y, h) = axis(y, h, fh);
        (w > 0 && h > 0).then_some(Self { x, y, w, h })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Anchor {
    pub record_ix: u64,
    pub anchored_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub record_id: RecordId,
    pub claim: WitnessClaim,
    pub anchor: Option<Anchor>,
    pub evidence: Evidence,
    pub confidence_q16: u32,
    pub compared_q16: u32,
    pub changed_q16: u32,
}

impl Reference {
    pub fn certain(record: &Record, evidence: Evidence) -> Self {
        Self {
            record_id: record.record_id,
            claim: record.claim.clone(),
            anchor: Some(Anchor {
                record_ix: record.record_ix,
                anchored_ns: record.anchored_ns,
            }),
            evidence,
            confidence_q16: Q16_ONE,
            compared_q16: Q16_ONE,
            changed_q16: 0,
        }
    }

    pub fn unanchored(record_id: RecordId, claim: WitnessClaim) -> Self {
        Self {
            record_id,
            claim,
            anchor: None,
            evidence: Evidence::Seal,
            confidence_q16: Q16_ONE,
            compared_q16: Q16_ONE,
            changed_q16: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub version: u16,
    pub outcome: Outcome,
    pub flags: VerdictFlags,
    pub reasons: Reasons,
    pub declared: Vec<Edit>,
    pub regions: Vec<Region>,
    pub reference: Option<Reference>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Notes {
    pub reasons: Reasons,
    pub flags: VerdictFlags,
}

impl Notes {
    pub fn reason(&mut self, reason: Reason) {
        self.reasons.insert(reason);
    }

    pub fn flag(&mut self, flag: VerdictFlags) {
        self.flags |= flag;
    }

    pub fn verdict(self, outcome: Outcome, reference: Option<Reference>) -> Verdict {
        Verdict {
            version: VERDICT_V,
            outcome,
            flags: self.flags,
            reasons: self.reasons,
            declared: reference
                .as_ref()
                .map(|r| r.claim.kind.edits().to_vec())
                .unwrap_or_default(),
            regions: Vec::new(),
            reference,
        }
    }

    pub fn no_record(self) -> Verdict {
        self.verdict(Outcome::NoRecord, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn reason_bits_are_dense_and_stable() {
        for (i, r) in Reason::ALL.iter().enumerate() {
            assert_eq!(*r as usize, i);
            assert_eq!(r.bit(), 1 << i);
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn reasons_set_operations() {
        let mut r = Reasons::EMPTY;
        assert!(r.is_empty());
        r.insert(Reason::Unconfirmed);
        r.insert(Reason::SealVerified);
        r.insert(Reason::Unconfirmed);
        assert_eq!(r.len(), 2);
        assert!(r.contains(Reason::SealVerified));
        assert!(!r.contains(Reason::SealBroken));
        assert_eq!(
            r.iter().collect::<Vec<_>>(),
            [Reason::SealVerified, Reason::Unconfirmed]
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn reasons_iterate_in_canonical_order_regardless_of_insertion() {
        let forward: Reasons = Reason::ALL.into_iter().collect();
        let backward: Reasons = Reason::ALL.into_iter().rev().collect();
        assert_eq!(forward, backward);
        assert_eq!(forward.iter().collect::<Vec<_>>(), Reason::ALL);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn reasons_reject_unknown_bits() {
        let all = (1u32 << Reason::ALL.len()) - 1;
        assert_eq!(Reasons::from_bits(all).map(Reasons::bits), Some(all));
        assert_eq!(Reasons::from_bits(all + 1), None);
        assert_eq!(Reasons::from_bits(u32::MAX), None);
        assert_eq!(Reasons::from_bits(0), Some(Reasons::EMPTY));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn region_normalizes_to_q16() {
        assert_eq!(
            Region::normalized(0, 0, 100, 50, (100, 50)),
            Some(Region {
                x: 0,
                y: 0,
                w: Q16_ONE,
                h: Q16_ONE
            })
        );
        assert_eq!(
            Region::normalized(50, 25, 25, 25, (100, 50)),
            Some(Region {
                x: 32768,
                y: 32768,
                w: 16384,
                h: 32768
            })
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn region_rounds_outward() {
        let r = Region::normalized(1, 1, 1, 1, (3, 3)).unwrap();
        assert_eq!((r.x, r.y), (21845, 21845));
        assert_eq!(r.x + r.w, 43691);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn region_clamps_and_rejects_degenerate() {
        assert_eq!(Region::normalized(0, 0, 0, 5, (10, 10)), None);
        assert_eq!(Region::normalized(0, 0, 5, 5, (0, 10)), None);
        assert_eq!(Region::normalized(10, 0, 5, 5, (10, 10)), None);
        let r = Region::normalized(8, 8, u32::MAX, u32::MAX, (10, 10)).unwrap();
        assert_eq!(r.x + r.w, Q16_ONE);
        assert_eq!(r.y + r.h, Q16_ONE);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn region_never_exceeds_unit_square_on_random_inputs() {
        let mut s = 0x1234_5678_u64;
        for _ in 0..2000 {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let v = |shift: u32, m: u64| ((s >> shift) % m) as u32;
            let frame = (v(0, 5000) + 1, v(13, 5000) + 1);
            if let Some(r) =
                Region::normalized(v(20, 6000), v(30, 6000), v(40, 6000), v(50, 6000), frame)
            {
                assert!(r.w > 0 && r.h > 0);
                assert!(r.x + r.w <= Q16_ONE && r.y + r.h <= Q16_ONE);
            }
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn notes_build_verdicts() {
        let mut n = Notes::default();
        n.reason(Reason::NoCandidates);
        n.flag(VerdictFlags::TRUNCATED_CANDIDATES);
        let v = n.no_record();
        assert_eq!(v.version, VERDICT_V);
        assert_eq!(v.outcome, Outcome::NoRecord);
        assert!(v.reasons.contains(Reason::NoCandidates));
        assert_eq!(v.flags, VerdictFlags::TRUNCATED_CANDIDATES);
        assert!(v.declared.is_empty() && v.regions.is_empty() && v.reference.is_none());
    }
}
