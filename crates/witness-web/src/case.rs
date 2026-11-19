use crate::error::{Error, Result};
use aes_gcm::aead::AeadInPlace;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce, Tag};
use base64::Engine;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use witness_core::format::CaseId;
use witness_core::format::crypto::{ct_eq, domain, sha256};

pub const KEY_BYTES: usize = 32;
pub const NONCE_BYTES: usize = 12;
pub const TAG_BYTES: usize = 16;
pub const OVERHEAD: usize = NONCE_BYTES + TAG_BYTES;
pub const MAX_WIRE_CHUNK: usize = 1 << 20;
pub const PLAIN_CHUNK: usize = MAX_WIRE_CHUNK - OVERHEAD;
pub const MAX_CASE_CHUNKS: u32 = 16;
pub const MAX_PLUS_CASE_CHUNKS: u32 = 256;
pub const MAX_CHUNKS: u32 = MAX_PLUS_CASE_CHUNKS;
pub const AAD_LEN: usize = 15 + CaseId::LEN + 4 + 4 + 32;
pub const KEY_PARAM: &str = "k=";

const URL_SAFE_ANY_PADDING: GeneralPurpose = GeneralPurpose::new(
    &base64::alphabet::URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

pub fn key_from_fragment(fragment: &str) -> Result<[u8; KEY_BYTES]> {
    let param = fragment
        .trim_start_matches('#')
        .split('&')
        .find_map(|p| p.strip_prefix(KEY_PARAM))
        .ok_or(Error::Case("key missing"))?;
    let key = URL_SAFE_ANY_PADDING
        .decode(param)
        .map_err(|_| Error::Case("key encoding"))?;
    key.try_into().map_err(|_| Error::Case("key length"))
}

pub fn aad(case_id: &CaseId, ix: u32, count: u32, bundle_hash: &[u8; 32]) -> [u8; AAD_LEN] {
    let mut out = [0u8; AAD_LEN];
    let (d, rest) = out.split_at_mut(domain::CASE.len());
    d.copy_from_slice(domain::CASE);
    let (id, rest) = rest.split_at_mut(CaseId::LEN);
    id.copy_from_slice(case_id.as_bytes());
    rest[..4].copy_from_slice(&ix.to_be_bytes());
    rest[4..8].copy_from_slice(&count.to_be_bytes());
    rest[8..].copy_from_slice(bundle_hash);
    out
}

fn decrypt_into(cipher: &Aes256Gcm, aad: &[u8], chunk: &[u8], slot: &mut [u8]) -> Result<()> {
    let (nonce, rest) = chunk.split_at(NONCE_BYTES);
    let (body, tag) = rest.split_at(slot.len());
    slot.copy_from_slice(body);
    cipher
        .decrypt_in_place_detached(Nonce::from_slice(nonce), aad, slot, Tag::from_slice(tag))
        .map_err(|_| Error::Case("authentication"))
}

pub struct CaseOpener {
    cipher: Aes256Gcm,
    case_id: CaseId,
    count: u32,
    bundle_hash: [u8; 32],
    bundle: Vec<u8>,
    last_len: usize,
    received: Vec<u64>,
}

impl CaseOpener {
    pub fn new(fragment: &str, case_id: &str, count: u32, bundle_hash: &[u8]) -> Result<Self> {
        let key = key_from_fragment(fragment)?;
        let case_id = CaseId::parse(case_id).map_err(|_| Error::Case("case id"))?;
        if count == 0 || count > MAX_CHUNKS {
            return Err(Error::Case("chunk count"));
        }
        let bundle_hash = bundle_hash
            .try_into()
            .map_err(|_| Error::Case("bundle hash"))?;
        Ok(Self {
            cipher: Aes256Gcm::new(&key.into()),
            case_id,
            count,
            bundle_hash,
            bundle: Vec::new(),
            last_len: 0,
            received: vec![0; count.div_ceil(64) as usize],
        })
    }

    pub fn add(&mut self, ix: u32, chunk: &[u8]) -> Result<()> {
        if ix >= self.count {
            return Err(Error::Case("chunk index"));
        }
        if chunk.len() < OVERHEAD || chunk.len() > MAX_WIRE_CHUNK {
            return Err(Error::Case("chunk size"));
        }
        let plain_len = chunk.len() - OVERHEAD;
        let last = ix + 1 == self.count;
        if !last && plain_len != PLAIN_CHUNK {
            return Err(Error::Case("chunk size"));
        }
        let start = ix as usize * PLAIN_CHUNK;
        let end = start + plain_len;
        if self.bundle.len() < end {
            self.bundle.resize(end, 0);
        }
        self.set(ix, false);
        let aad = aad(&self.case_id, ix, self.count, &self.bundle_hash);
        decrypt_into(&self.cipher, &aad, chunk, &mut self.bundle[start..end])?;
        if last {
            self.last_len = plain_len;
        }
        self.set(ix, true);
        Ok(())
    }

    fn set(&mut self, ix: u32, on: bool) {
        let (word, bit) = ((ix / 64) as usize, 1u64 << (ix % 64));
        if on {
            self.received[word] |= bit;
        } else {
            self.received[word] &= !bit;
        }
    }

    fn has(&self, ix: u32) -> bool {
        self.received[(ix / 64) as usize] & (1u64 << (ix % 64)) != 0
    }

    pub fn missing(&self) -> Vec<u32> {
        (0..self.count).filter(|ix| !self.has(*ix)).collect()
    }

    pub fn finish(mut self) -> Result<Vec<u8>> {
        if !self.missing().is_empty() {
            return Err(Error::Case("missing chunk"));
        }
        let total = (self.count as usize - 1) * PLAIN_CHUNK + self.last_len;
        self.bundle.truncate(total);
        if self.bundle.len() == total && ct_eq(&sha256(&self.bundle), &self.bundle_hash) {
            Ok(self.bundle)
        } else {
            Err(Error::Case("bundle hash"))
        }
    }
}

#[cfg(test)]
mod tests;
