use crate::error::ContractError;
use crate::limits::{
    MAX_CASE_BYTES, MAX_CHUNK_BYTES, MAX_OG_BYTES, MAX_PLUS_CASE_CHUNKS, MAX_PREVIEW_BYTES,
};
use candid::{CandidType, Principal};
use serde::Deserialize;
use witness_format::crypto::{ed25519_verify, es256_verify};
use witness_format::{CaseId, KeyId, RecordId};

pub const UPLOAD_DOMAIN: &[u8] = b"witness/upload/v1";
pub const GRANT_DOMAIN: &[u8] = b"witness/grant/v1";

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    Plus,
}

impl Tier {
    pub const fn code(self) -> u8 {
        match self {
            Self::Plus => 0,
        }
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntitlementGrant {
    pub ii_principal: Principal,
    pub tier: Tier,
    pub quota_bytes: u64,
    pub expires_ns: u64,
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
}

impl EntitlementGrant {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let p = self.ii_principal.as_slice();
        let mut out = Vec::with_capacity(GRANT_DOMAIN.len() + 1 + p.len() + 1 + 16);
        out.extend_from_slice(GRANT_DOMAIN);
        out.push(p.len() as u8);
        out.extend_from_slice(p);
        out.push(self.tier.code());
        out.extend_from_slice(&self.quota_bytes.to_be_bytes());
        out.extend_from_slice(&self.expires_ns.to_be_bytes());
        out
    }

    pub fn verify(
        &self,
        signer: &[u8],
        caller: &Principal,
        now_ns: u64,
    ) -> Result<(), ContractError> {
        if self.ii_principal != *caller {
            return Err(ContractError::WrongCaller);
        }
        if now_ns >= self.expires_ns {
            return Err(ContractError::Expired);
        }
        ed25519_verify(signer, &self.signing_bytes(), &self.signature)
            .map_err(|_| ContractError::BadSignature)
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum UploadTarget {
    #[serde(rename = "preview")]
    Preview(String),
    #[serde(rename = "og")]
    Og(String),
    #[serde(rename = "full")]
    Full(String),
    #[serde(rename = "case_chunk")]
    CaseChunk {
        case_id: String,
        ix: u32,
        count: u32,
    },
}

impl UploadTarget {
    pub const fn code(&self) -> u8 {
        match self {
            Self::Preview(_) => 0,
            Self::Og(_) => 1,
            Self::Full(_) => 2,
            Self::CaseChunk { .. } => 3,
        }
    }

    pub fn record_id(&self) -> Result<Option<RecordId>, ContractError> {
        match self {
            Self::Preview(id) | Self::Og(id) | Self::Full(id) => RecordId::parse(id)
                .map(Some)
                .map_err(|_| ContractError::BadId),
            Self::CaseChunk { .. } => Ok(None),
        }
    }

    pub fn case_id(&self) -> Result<Option<CaseId>, ContractError> {
        match self {
            Self::CaseChunk { case_id, ix, count } => {
                if *count == 0 || ix >= count || *count > MAX_PLUS_CASE_CHUNKS {
                    return Err(ContractError::Invalid);
                }
                CaseId::parse(case_id)
                    .map(Some)
                    .map_err(|_| ContractError::BadId)
            }
            _ => Ok(None),
        }
    }

    pub const fn max_bytes(&self) -> Option<u64> {
        match self {
            Self::Preview(_) => Some(MAX_PREVIEW_BYTES),
            Self::Og(_) => Some(MAX_OG_BYTES),
            Self::Full(_) => None,
            Self::CaseChunk { .. } => Some(MAX_CASE_BYTES),
        }
    }

    fn write_signed(&self, out: &mut Vec<u8>) -> Result<(), ContractError> {
        out.push(self.code());
        if let Some(r) = self.record_id()? {
            out.extend_from_slice(r.as_bytes());
        }
        if let (Some(c), Self::CaseChunk { ix, count, .. }) = (self.case_id()?, self) {
            out.extend_from_slice(c.as_bytes());
            out.extend_from_slice(&ix.to_be_bytes());
            out.extend_from_slice(&count.to_be_bytes());
        }
        Ok(())
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BeginUpload {
    pub target: UploadTarget,
    pub size: u64,
    #[serde(with = "serde_bytes")]
    pub sha256: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub key_id: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    pub grant: Option<EntitlementGrant>,
}

impl BeginUpload {
    pub fn key_id(&self) -> Result<KeyId, ContractError> {
        KeyId::try_from(self.key_id.as_slice()).map_err(|_| ContractError::BadId)
    }

    pub fn chunk_count(&self) -> u64 {
        self.size.div_ceil(MAX_CHUNK_BYTES as u64)
    }

    pub fn check_shape(&self) -> Result<(), ContractError> {
        if self.sha256.len() != 32 || self.size == 0 {
            return Err(ContractError::BadLength);
        }
        if self.target.max_bytes().is_some_and(|m| self.size > m) {
            return Err(ContractError::TooLarge);
        }
        self.key_id()?;
        Ok(())
    }

    pub fn signing_bytes(&self) -> Result<Vec<u8>, ContractError> {
        self.check_shape()?;
        let mut out = Vec::with_capacity(UPLOAD_DOMAIN.len() + 16 + 1 + 16 + 8 + 8 + 32);
        out.extend_from_slice(UPLOAD_DOMAIN);
        out.extend_from_slice(&self.key_id);
        self.target.write_signed(&mut out)?;
        out.extend_from_slice(&self.size.to_be_bytes());
        out.extend_from_slice(&self.sha256);
        Ok(out)
    }

    pub fn verify(&self, pubkey: &[u8]) -> Result<(), ContractError> {
        if KeyId::from_spki(pubkey) != self.key_id()? {
            return Err(ContractError::BadKey);
        }
        let msg = self.signing_bytes()?;
        es256_verify(pubkey, &msg, &self.signature).map_err(|_| ContractError::BadSignature)
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaseInfo {
    pub expires_ns: u64,
    pub bytes: u64,
    pub chunks: u32,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chunk {
    pub upload: u64,
    pub ix: u32,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

impl Chunk {
    pub fn within_limits(&self) -> bool {
        !self.bytes.is_empty() && self.bytes.len() <= MAX_CHUNK_BYTES
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MediaErr {
    #[serde(rename = "malformed")]
    Malformed(String),
    #[serde(rename = "unauthorized")]
    Unauthorized,
    #[serde(rename = "not_found")]
    NotFound,
    #[serde(rename = "too_large")]
    TooLarge,
    #[serde(rename = "quota_exceeded")]
    QuotaExceeded,
    #[serde(rename = "grant_invalid")]
    GrantInvalid,
    #[serde(rename = "bad_chunk")]
    BadChunk,
    #[serde(rename = "hash_mismatch")]
    HashMismatch,
    #[serde(rename = "incomplete")]
    Incomplete,
    #[serde(rename = "retry_after_ns")]
    RetryAfterNs(u64),
}

impl From<ContractError> for MediaErr {
    fn from(e: ContractError) -> Self {
        match e {
            ContractError::TooLarge => Self::TooLarge,
            ContractError::BadSignature | ContractError::BadKey | ContractError::WrongCaller => {
                Self::Unauthorized
            }
            ContractError::Expired => Self::GrantInvalid,
            ContractError::BadId | ContractError::BadLength | ContractError::Invalid => {
                Self::Malformed(e.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests;
