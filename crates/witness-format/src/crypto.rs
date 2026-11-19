use core::fmt;
use ecdsa::signature::hazmat::PrehashVerifier;
use p256::NistP256;
use p256::pkcs8::DecodePublicKey;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub mod domain {
    pub const LOCATION: &[u8] = b"witness/loc/v1";
    pub const ERASE: &[u8] = b"witness/erase/v1";
    pub const CASE: &[u8] = b"witness/case/v1";
    pub const OFFLINE_CHALLENGE: &[u8] = b"witness-offline";
}

pub const ES256_RAW_LEN: usize = 64;
pub const ES256_DER_MAX_LEN: usize = 72;
pub const ED25519_SIG_LEN: usize = 64;
pub const ED25519_KEY_LEN: usize = 32;

type P256Signature = ecdsa::Signature<NistP256>;
type P256VerifyingKey = ecdsa::VerifyingKey<NistP256>;

pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

pub fn sha256_parts(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    parts.iter().for_each(|p| h.update(p));
    h.finalize().into()
}

pub fn hash_ds(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(domain);
    parts.iter().for_each(|p| h.update(p));
    h.finalize().into()
}

pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigError {
    Key,
    Encoding,
    HighS,
    Invalid,
}

impl fmt::Display for SigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Key => "malformed public key",
            Self::Encoding => "malformed signature",
            Self::HighS => "signature is not low-S",
            Self::Invalid => "signature verification failed",
        })
    }
}

impl core::error::Error for SigError {}

fn parse_es256_sig(sig: &[u8]) -> Result<P256Signature, SigError> {
    if sig.len() == ES256_RAW_LEN {
        P256Signature::from_slice(sig)
    } else {
        P256Signature::from_der(sig)
    }
    .map_err(|_| SigError::Encoding)
}

fn to_raw(sig: &P256Signature) -> [u8; ES256_RAW_LEN] {
    sig.to_bytes().into()
}

pub fn es256_sig_to_raw(sig: &[u8]) -> Result<[u8; ES256_RAW_LEN], SigError> {
    parse_es256_sig(sig).map(|s| to_raw(&s))
}

pub fn es256_der_to_raw(der: &[u8]) -> Result<[u8; ES256_RAW_LEN], SigError> {
    P256Signature::from_der(der)
        .map(|s| to_raw(&s))
        .map_err(|_| SigError::Encoding)
}

pub fn es256_raw_to_der(raw: &[u8; ES256_RAW_LEN]) -> Result<DerSignature, SigError> {
    let sig = P256Signature::from_slice(raw).map_err(|_| SigError::Encoding)?;
    let der = sig.to_der();
    let bytes = der.as_bytes();
    let mut buf = [0u8; ES256_DER_MAX_LEN];
    buf[..bytes.len()].copy_from_slice(bytes);
    Ok(DerSignature {
        buf,
        len: bytes.len() as u8,
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DerSignature {
    buf: [u8; ES256_DER_MAX_LEN],
    len: u8,
}

impl DerSignature {
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..usize::from(self.len)]
    }
}

impl fmt::Debug for DerSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DerSignature")
            .field(&self.as_bytes())
            .finish()
    }
}

pub fn es256_is_low_s(sig: &[u8]) -> Result<bool, SigError> {
    parse_es256_sig(sig).map(|s| s.normalize_s().is_none())
}

pub fn es256_normalize_low_s(sig: &[u8]) -> Result<[u8; ES256_RAW_LEN], SigError> {
    let s = parse_es256_sig(sig)?;
    Ok(to_raw(&s.normalize_s().unwrap_or(s)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Es256Key(P256VerifyingKey);

impl Es256Key {
    pub fn from_sec1(bytes: &[u8]) -> Result<Self, SigError> {
        P256VerifyingKey::from_sec1_bytes(bytes)
            .map(Self)
            .map_err(|_| SigError::Key)
    }

    pub fn from_spki(der: &[u8]) -> Result<Self, SigError> {
        P256VerifyingKey::from_public_key_der(der)
            .map(Self)
            .map_err(|_| SigError::Key)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, SigError> {
        match bytes.first() {
            Some(0x30) => Self::from_spki(bytes),
            _ => Self::from_sec1(bytes),
        }
    }

    pub fn sec1_uncompressed(&self) -> [u8; 65] {
        let point = self.0.to_encoded_point(false);
        let mut out = [0u8; 65];
        out.copy_from_slice(point.as_bytes());
        out
    }

    pub fn verify(&self, msg: &[u8], sig: &[u8]) -> Result<(), SigError> {
        self.verify_prehash(&sha256(msg), sig)
    }

    pub fn verify_prehash(&self, digest: &[u8; 32], sig: &[u8]) -> Result<(), SigError> {
        let s = parse_es256_sig(sig)?;
        self.0
            .verify_prehash(digest, &s)
            .map_err(|_| SigError::Invalid)
    }

    pub fn verify_low_s(&self, msg: &[u8], sig: &[u8]) -> Result<(), SigError> {
        if !es256_is_low_s(sig)? {
            return Err(SigError::HighS);
        }
        self.verify(msg, sig)
    }
}

pub fn es256_verify(pubkey: &[u8], msg: &[u8], sig: &[u8]) -> Result<(), SigError> {
    Es256Key::parse(pubkey)?.verify(msg, sig)
}

pub fn es256_verify_low_s(pubkey: &[u8], msg: &[u8], sig: &[u8]) -> Result<(), SigError> {
    Es256Key::parse(pubkey)?.verify_low_s(msg, sig)
}

pub fn ed25519_verify(pubkey: &[u8], msg: &[u8], sig: &[u8]) -> Result<(), SigError> {
    let pk: &[u8; ED25519_KEY_LEN] = pubkey.try_into().map_err(|_| SigError::Key)?;
    let sig: &[u8; ED25519_SIG_LEN] = sig.try_into().map_err(|_| SigError::Encoding)?;
    let key = ed25519_dalek::VerifyingKey::from_bytes(pk).map_err(|_| SigError::Key)?;
    key.verify_strict(msg, &ed25519_dalek::Signature::from_bytes(sig))
        .map_err(|_| SigError::Invalid)
}

#[cfg(test)]
mod tests;
