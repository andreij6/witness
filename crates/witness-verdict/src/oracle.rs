use witness_fingerprint::PdqHash;
use witness_format::{
    CaptureId, FingerprintBundle, KeyId, KeyInfo, RecordId, VerdictFlags, WitnessClaim,
};

pub const MAX_EXACT: usize = 8;
pub const MAX_WATERMARK: usize = 32;
pub const MAX_DERIVATIVES: usize = 32;
pub const LATE_ANCHOR_MS: u64 = 24 * 3_600_000;
pub const CLOCK_TOLERANCE_MS: u64 = 60_000;

pub fn clock_flags(claim: &WitnessClaim, anchored_ns: u64) -> VerdictFlags {
    let mut f = VerdictFlags::EMPTY;
    if (anchored_ns / 1_000_000).saturating_sub(claim.time.wall_ms) > LATE_ANCHOR_MS {
        f |= VerdictFlags::LATE_ANCHOR;
    }
    if claim
        .time
        .gnss_ms
        .is_some_and(|g| g.abs_diff(claim.time.wall_ms) > CLOCK_TOLERANCE_MS)
    {
        f |= VerdictFlags::CLOCK_MISMATCH;
    }
    f
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub record_ix: u64,
    pub record_id: RecordId,
    pub claim: WitnessClaim,
    pub anchored_ns: u64,
    pub flags: VerdictFlags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdqHit {
    pub record: Record,
    pub distance: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PdqProbe {
    pub hits: Vec<PdqHit>,
    pub truncated: bool,
}

pub trait Oracle {
    fn get_key(&self, key_id: &KeyId) -> Option<KeyInfo>;
    fn get_record(&self, record_id: &RecordId) -> Option<Record>;
    fn exact(&self, asset_hash: &[u8; 32], pixel_hash: Option<&[u8; 32]>) -> Vec<Record>;
    fn by_watermark(&self, capture_id: CaptureId) -> Vec<Record>;
    fn pdq_probe(&self, hashes: &[PdqHash]) -> PdqProbe;
    fn derivatives(&self, root: &Record) -> Vec<Record>;
    fn reference_image(&self, record: &Record) -> Option<Vec<u8>>;
    fn fingerprints(&self, record: &Record) -> Option<FingerprintBundle>;
    fn index_pending(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::TestCa;
    use witness_format::Kind;

    const HOUR_NS: u64 = 3_600_000_000_000;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn clock_flags_follow_the_24_hour_and_gnss_rules() {
        let ca = TestCa::new(1);
        let mut claim = ca.device(2, 2).claim(1, Kind::Original, 1);
        claim.time.wall_ms = 1_000;
        claim.time.gnss_ms = None;
        let at = |ms: u64| ms * 1_000_000;
        assert_eq!(clock_flags(&claim, at(1_000)), VerdictFlags::EMPTY);
        assert_eq!(
            clock_flags(&claim, at(1_000) + 24 * HOUR_NS),
            VerdictFlags::EMPTY
        );
        assert_eq!(
            clock_flags(&claim, at(1_001) + 24 * HOUR_NS),
            VerdictFlags::LATE_ANCHOR
        );
        claim.time.wall_ms = 1_000 + LATE_ANCHOR_MS + 1;
        assert_eq!(clock_flags(&claim, at(1_000)), VerdictFlags::EMPTY);
        claim.time.wall_ms = 1_000;
        claim.time.gnss_ms = Some(1_000 + CLOCK_TOLERANCE_MS);
        assert_eq!(clock_flags(&claim, at(1_000)), VerdictFlags::EMPTY);
        claim.time.gnss_ms = Some(1_000 + CLOCK_TOLERANCE_MS + 1);
        assert_eq!(clock_flags(&claim, at(1_000)), VerdictFlags::CLOCK_MISMATCH);
        claim.time.gnss_ms = Some(0);
        claim.time.wall_ms = CLOCK_TOLERANCE_MS + 1;
        assert_eq!(
            clock_flags(&claim, u64::MAX),
            VerdictFlags::CLOCK_MISMATCH | VerdictFlags::LATE_ANCHOR
        );
    }
}
