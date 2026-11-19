use crate::limits::{MAX_PROBE_EXACT, MAX_PROBE_PDQ, MAX_PROBE_WATERMARKS};
use candid::CandidType;
use serde::Deserialize;
use serde_bytes::ByteBuf;

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Posting {
    pub record_ix: u64,
    pub wm_root: u64,
    #[serde(with = "serde_bytes")]
    pub asset_hash: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub pixel_hash: Vec<u8>,
    pub fingerprints: Option<ByteBuf>,
    pub root_ix: Option<u64>,
}

impl Posting {
    pub fn asset_key(&self) -> Option<[u8; 16]> {
        hash_key(&self.asset_hash)
    }

    pub fn pixel_key(&self) -> Option<[u8; 16]> {
        hash_key(&self.pixel_hash)
    }
}

pub fn hash_key(hash: &[u8]) -> Option<[u8; 16]> {
    if hash.len() != 32 {
        return None;
    }
    hash[..16].try_into().ok()
}

pub fn probe_within_limits(pdq: &[ByteBuf], watermarks: &[u64], exact: &[ByteBuf]) -> bool {
    pdq.len() <= MAX_PROBE_PDQ
        && watermarks.len() <= MAX_PROBE_WATERMARKS
        && exact.len() <= MAX_PROBE_EXACT
        && pdq.iter().all(|h| h.len() == 32)
        && exact.iter().all(|h| h.len() == 32)
}

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PdqHit {
    pub record_ix: u64,
    pub distance: u32,
}

#[derive(CandidType, Deserialize, Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ProbeResult {
    pub pdq: Vec<PdqHit>,
    pub watermark: Vec<u64>,
    pub exact: Vec<u64>,
    pub truncated_candidates: bool,
    pub indexed_upto: u64,
}

impl ProbeResult {
    pub fn is_empty(&self) -> bool {
        self.pdq.is_empty() && self.watermark.is_empty() && self.exact.is_empty()
    }

    pub fn covers(&self, record_ix: u64) -> bool {
        record_ix < self.indexed_upto
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprints {
    #[serde(with = "serde_bytes")]
    pub grid: Vec<u8>,
    pub shapes: Vec<(u8, ByteBuf)>,
    pub quality: u8,
    pub portrait: bool,
}

impl From<&witness_format::FingerprintBundle> for Fingerprints {
    fn from(b: &witness_format::FingerprintBundle) -> Self {
        Self {
            grid: b.grid_bytes().to_vec(),
            shapes: b
                .pdq()
                .iter()
                .map(|s| (s.tag, ByteBuf::from(s.hash.to_vec())))
                .collect(),
            quality: b.quality(),
            portrait: b.portrait(),
        }
    }
}

#[cfg(test)]
mod tests;
