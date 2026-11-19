use crate::error::ContractError;
use crate::limits::{MAX_FINGERPRINTS_BYTES, MAX_NONCE_BYTES};
use candid::{CandidType, Principal};
use serde::Deserialize;
use serde_bytes::ByteBuf;
use witness_format as wf;
use witness_format::crypto::{domain, es256_verify};
use witness_format::{KeyHistory, KeyId, RecordId};

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Security {
    StrongBox,
    Tee,
    Dev,
}

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prov {
    Rkp,
    Factory,
    Dev,
}

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fresh {
    Fresh,
    Late,
}

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyStatus {
    Active,
    Retired,
    Suspended,
    Revoked,
}

macro_rules! mirror {
    ($ours:ident, $theirs:path, $($v:ident),+) => {
        impl From<$theirs> for $ours {
            fn from(v: $theirs) -> Self {
                use $theirs as T;
                match v { $(T::$v => Self::$v),+ }
            }
        }
        impl From<$ours> for $theirs {
            fn from(v: $ours) -> Self {
                use $theirs as T;
                match v { $($ours::$v => T::$v),+ }
            }
        }
    };
}

mirror!(Security, wf::SecurityLevel, StrongBox, Tee, Dev);
mirror!(Prov, wf::Provisioning, Rkp, Factory, Dev);
mirror!(Fresh, wf::Freshness, Fresh, Late);
mirror!(
    KeyStatus,
    wf::KeyStatus,
    Active,
    Retired,
    Suspended,
    Revoked
);

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusChange {
    pub status: KeyStatus,
    pub since_ns: u64,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyInfo {
    #[serde(with = "serde_bytes")]
    pub key_id: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub pubkey: Vec<u8>,
    pub security: Security,
    pub provisioning: Prov,
    pub freshness: Fresh,
    pub patch: u32,
    pub enrolled_ns: u64,
    pub active_until_ns: u64,
    pub history: Vec<StatusChange>,
    pub chain_serials: Vec<String>,
}

impl From<&wf::KeyInfo> for KeyInfo {
    fn from(k: &wf::KeyInfo) -> Self {
        Self {
            key_id: k.key_id.as_bytes().to_vec(),
            pubkey: k.pubkey.clone(),
            security: k.security.into(),
            provisioning: k.provisioning.into(),
            freshness: k.freshness.into(),
            patch: k.patch,
            enrolled_ns: k.enrolled_ns,
            active_until_ns: k.active_until_ns,
            history: k
                .history
                .changes()
                .iter()
                .map(|c| StatusChange {
                    status: c.status.into(),
                    since_ns: c.since_ns,
                })
                .collect(),
            chain_serials: k.chain_serials.clone(),
        }
    }
}

impl TryFrom<KeyInfo> for wf::KeyInfo {
    type Error = ContractError;

    fn try_from(k: KeyInfo) -> Result<Self, ContractError> {
        let key_id = KeyId::try_from(k.key_id.as_slice()).map_err(|_| ContractError::BadId)?;
        let history = KeyHistory::from_changes(
            k.history
                .into_iter()
                .map(|c| wf::StatusChange {
                    status: c.status.into(),
                    since_ns: c.since_ns,
                })
                .collect(),
        )
        .map_err(|_| ContractError::Invalid)?;
        let info = wf::KeyInfo {
            key_id,
            pubkey: k.pubkey,
            security: k.security.into(),
            provisioning: k.provisioning.into(),
            freshness: k.freshness.into(),
            patch: k.patch,
            enrolled_ns: k.enrolled_ns,
            active_until_ns: k.active_until_ns,
            history,
            chain_serials: k.chain_serials,
        };
        info.validate().map_err(|_| ContractError::Invalid)?;
        Ok(info)
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Submission {
    #[serde(with = "serde_bytes")]
    pub claim: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub c2pa_claim: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub cose: Vec<u8>,
    pub fingerprints: Option<ByteBuf>,
}

impl Submission {
    pub fn within_limits(&self) -> bool {
        let part_ok = |p: &[u8]| !p.is_empty() && p.len() <= wf::Submission::MAX_PART;
        part_ok(&self.claim)
            && part_ok(&self.c2pa_claim)
            && part_ok(&self.cose)
            && self
                .fingerprints
                .as_ref()
                .is_none_or(|f| !f.is_empty() && f.len() <= MAX_FINGERPRINTS_BYTES)
    }

    pub fn record_id(&self) -> RecordId {
        RecordId::from_claim_cbor(&self.claim)
    }

    pub fn into_parts(self) -> (wf::Submission, Option<Vec<u8>>) {
        (
            wf::Submission {
                claim_cbor: self.claim,
                c2pa_claim_cbor: self.c2pa_claim,
                cose: self.cose,
            },
            self.fingerprints.map(ByteBuf::into_vec),
        )
    }

    pub fn from_parts(s: wf::Submission, fingerprints: Option<Vec<u8>>) -> Self {
        Self {
            claim: s.claim_cbor,
            c2pa_claim: s.c2pa_claim_cbor,
            cose: s.cose,
            fingerprints: fingerprints.map(ByteBuf::from),
        }
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Anchored {
    pub record_ix: u64,
    pub record_id: String,
    pub anchored_ns: u64,
    pub media_shard: u8,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubmitErr {
    #[serde(rename = "malformed")]
    Malformed(String),
    #[serde(rename = "too_large")]
    TooLarge,
    #[serde(rename = "bad_signature")]
    BadSignature,
    #[serde(rename = "unknown_key")]
    UnknownKey,
    #[serde(rename = "key_distrusted")]
    KeyDistrusted,
    #[serde(rename = "key_expired")]
    KeyExpired,
    #[serde(rename = "counter_reused")]
    CounterReused,
    #[serde(rename = "parent_unknown")]
    ParentUnknown,
    #[serde(rename = "fingerprint_mismatch")]
    FingerprintMismatch,
    #[serde(rename = "retry_after_ns")]
    RetryAfterNs(u64),
}

impl SubmitErr {
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::RetryAfterNs(_))
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubmitResult {
    #[serde(rename = "ok")]
    Ok(Anchored),
    #[serde(rename = "dup")]
    Dup(Anchored),
    #[serde(rename = "err")]
    Err(SubmitErr),
}

impl SubmitResult {
    pub const fn anchored(&self) -> Option<&Anchored> {
        match self {
            Self::Ok(a) | Self::Dup(a) => Some(a),
            Self::Err(_) => None,
        }
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CertifiedRecord {
    pub record_ix: u64,
    pub record_id: String,
    #[serde(with = "serde_bytes")]
    pub claim: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub c2pa_claim: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub cose: Vec<u8>,
    pub anchored_ns: u64,
    pub mmr_proof: Vec<ByteBuf>,
    pub leaf_count: u64,
    #[serde(with = "serde_bytes")]
    pub certificate: Vec<u8>,
    pub media_shard: u8,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CertifiedKey {
    pub key: KeyInfo,
    #[serde(with = "serde_bytes")]
    pub smt_proof: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub certificate: Vec<u8>,
}

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShardInfo {
    pub shard: u8,
    pub canister: Principal,
    pub open: bool,
}

#[derive(CandidType, Deserialize, Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Shards {
    pub index: Vec<ShardInfo>,
    pub media: Vec<ShardInfo>,
}

impl Shards {
    pub fn open_index(&self) -> Option<&ShardInfo> {
        self.index.iter().find(|s| s.open)
    }

    pub fn media_shard(&self, shard: u8) -> Option<&ShardInfo> {
        self.media.iter().find(|s| s.shard == shard)
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EraseAuth {
    #[serde(rename = "device")]
    Device(#[serde(with = "serde_bytes")] Vec<u8>),
    #[serde(rename = "ii")]
    Ii,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EraseRequest {
    pub record_id: String,
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    pub auth: EraseAuth,
}

impl EraseRequest {
    pub fn record_id(&self) -> Result<RecordId, ContractError> {
        RecordId::parse(&self.record_id).map_err(|_| ContractError::BadId)
    }

    pub fn signing_bytes(&self) -> Result<Vec<u8>, ContractError> {
        erase_signing_bytes(&self.record_id()?, &self.nonce)
    }

    pub fn verify_device(&self, pubkey: &[u8]) -> Result<(), ContractError> {
        let EraseAuth::Device(sig) = &self.auth else {
            return Err(ContractError::BadSignature);
        };
        let msg = self.signing_bytes()?;
        es256_verify(pubkey, &msg, sig).map_err(|_| ContractError::BadSignature)
    }
}

pub fn erase_signing_bytes(record_id: &RecordId, nonce: &[u8]) -> Result<Vec<u8>, ContractError> {
    if nonce.is_empty() || nonce.len() > MAX_NONCE_BYTES {
        return Err(ContractError::BadLength);
    }
    let id = record_id.as_bytes();
    let mut out = Vec::with_capacity(domain::ERASE.len() + id.len() + nonce.len());
    out.extend_from_slice(domain::ERASE);
    out.extend_from_slice(id);
    out.extend_from_slice(nonce);
    Ok(out)
}

#[cfg(test)]
mod tests;
