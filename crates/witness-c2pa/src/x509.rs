use crate::ChainError;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};
use witness_format::crypto::{ED25519_KEY_LEN, Es256Key, ed25519_verify};
use x509_cert::Certificate;
use x509_cert::certificate::Version;
use x509_cert::der::asn1::ObjectIdentifier;
use x509_cert::der::{Decode, Encode, Reader, SliceReader};
use x509_cert::ext::Extension;
use x509_cert::ext::pkix::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectKeyIdentifier,
};
use x509_cert::name::Name;
use x509_cert::spki::AlgorithmIdentifierOwned;
use x509_cert::time::Validity;

pub mod oid {
    use super::ObjectIdentifier;

    pub const ED25519: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.101.112");
    pub const KEY_USAGE: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.15");
    pub const EXT_KEY_USAGE: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.37");
    pub const BASIC_CONSTRAINTS: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.19");
    pub const SUBJECT_KEY_ID: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.14");
    pub const AUTHORITY_KEY_ID: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.35");
    pub const KP_DOCUMENT_SIGNING: ObjectIdentifier =
        ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.36");
    pub const KP_C2PA_CLAIM_SIGNING: ObjectIdentifier =
        ObjectIdentifier::new_unwrap("1.3.6.1.4.1.62558.2.1");
    pub const ANY_EXT_KEY_USAGE: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.37.0");
}

pub const KEY_ID_LEN: usize = 20;
const MAX_EXTENSIONS: usize = 64;

pub fn key_identifier(subject_public_key: &[u8]) -> [u8; KEY_ID_LEN] {
    let h: [u8; 32] = Sha256::digest(subject_public_key).into();
    let mut id = [0u8; KEY_ID_LEN];
    id.copy_from_slice(&h[..KEY_ID_LEN]);
    id
}

pub fn ed25519_alg() -> AlgorithmIdentifierOwned {
    AlgorithmIdentifierOwned {
        oid: oid::ED25519,
        parameters: None,
    }
}

struct Parsed<'a> {
    cert: Certificate,
    tbs_raw: &'a [u8],
}

fn parse(der: &[u8]) -> Result<Parsed<'_>, ChainError> {
    let cert = Certificate::from_der(der).map_err(|_| ChainError::Malformed)?;
    let mut r = SliceReader::new(der).map_err(|_| ChainError::Malformed)?;
    let tbs_raw = r
        .sequence(|inner| {
            let tbs = inner.tlv_bytes()?;
            inner.tlv_bytes()?;
            inner.tlv_bytes()?;
            Ok(tbs)
        })
        .map_err(|_| ChainError::Malformed)?;
    Ok(Parsed { cert, tbs_raw })
}

fn check_ed25519_signed(
    p: &Parsed<'_>,
    issuer_key: &[u8; ED25519_KEY_LEN],
) -> Result<(), ChainError> {
    let alg = ed25519_alg();
    if p.cert.signature_algorithm != alg || p.cert.tbs_certificate.signature != alg {
        return Err(ChainError::Algorithm);
    }
    let sig = p.cert.signature.as_bytes().ok_or(ChainError::Signature)?;
    ed25519_verify(issuer_key, p.tbs_raw, sig).map_err(|_| ChainError::Signature)
}

fn check_validity(v: &Validity, at: Option<u64>) -> Result<(), ChainError> {
    let nb = v.not_before.to_unix_duration().as_secs();
    let na = v.not_after.to_unix_duration().as_secs();
    match at {
        Some(t) if t < nb || t > na => Err(ChainError::Validity),
        _ if nb > na => Err(ChainError::Validity),
        _ => Ok(()),
    }
}

#[derive(Default)]
struct Exts {
    key_usage: Option<(bool, KeyUsage)>,
    eku: Option<ExtendedKeyUsage>,
    basic: Option<(bool, BasicConstraints)>,
    ski: Option<Vec<u8>>,
    aki: Option<AuthorityKeyIdentifier>,
}

fn decode_ext<'a, T: Decode<'a>>(e: &'a Extension) -> Result<T, ChainError> {
    T::from_der(e.extn_value.as_bytes()).map_err(|_| ChainError::Malformed)
}

fn extensions(c: &Certificate) -> Result<Exts, ChainError> {
    let list: &[Extension] = c.tbs_certificate.extensions.as_deref().unwrap_or_default();
    if list.len() > MAX_EXTENSIONS {
        return Err(ChainError::Malformed);
    }
    let mut ids: Vec<&ObjectIdentifier> = list.iter().map(|e| &e.extn_id).collect();
    ids.sort_unstable();
    if ids.windows(2).any(|w| w[0] == w[1]) {
        return Err(ChainError::DuplicateExtension);
    }
    let mut x = Exts::default();
    for e in list {
        match e.extn_id {
            oid::KEY_USAGE => x.key_usage = Some((e.critical, decode_ext(e)?)),
            oid::EXT_KEY_USAGE => x.eku = Some(decode_ext(e)?),
            oid::BASIC_CONSTRAINTS => x.basic = Some((e.critical, decode_ext(e)?)),
            oid::SUBJECT_KEY_ID => {
                x.ski = Some(decode_ext::<SubjectKeyIdentifier>(e)?.0.as_bytes().to_vec())
            }
            oid::AUTHORITY_KEY_ID => x.aki = Some(decode_ext(e)?),
            _ if e.critical => return Err(ChainError::CriticalExtension),
            _ => {}
        }
    }
    Ok(x)
}

fn check_common(c: &Certificate) -> Result<(), ChainError> {
    let t = &c.tbs_certificate;
    if t.version != Version::V3 {
        return Err(ChainError::Version);
    }
    if t.issuer_unique_id.is_some() || t.subject_unique_id.is_some() {
        return Err(ChainError::Malformed);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustAnchor {
    der: Vec<u8>,
    subject: Name,
    key: [u8; ED25519_KEY_LEN],
    key_id: Vec<u8>,
    validity: Validity,
}

impl TrustAnchor {
    pub fn from_der(der: &[u8]) -> Result<Self, ChainError> {
        let p = parse(der)?;
        check_common(&p.cert)?;
        let t = &p.cert.tbs_certificate;
        if t.issuer != t.subject {
            return Err(ChainError::Issuer);
        }
        let spki = &t.subject_public_key_info;
        if spki.algorithm != ed25519_alg() {
            return Err(ChainError::PublicKey);
        }
        let key: [u8; ED25519_KEY_LEN] = spki
            .subject_public_key
            .as_bytes()
            .and_then(|k| k.try_into().ok())
            .ok_or(ChainError::PublicKey)?;
        check_ed25519_signed(&p, &key)?;
        check_validity(&t.validity, None)?;
        let x = extensions(&p.cert)?;
        match x.basic {
            Some((_, b)) if b.ca => {}
            _ => return Err(ChainError::BasicConstraints),
        }
        if x.key_usage.is_some_and(|(_, ku)| !ku.key_cert_sign()) {
            return Err(ChainError::KeyUsage);
        }
        if x.eku.is_some() {
            return Err(ChainError::ExtendedKeyUsage);
        }
        let key_id = x.ski.unwrap_or_else(|| key_identifier(&key).to_vec());
        Ok(Self {
            der: der.to_vec(),
            subject: t.subject.clone(),
            key,
            key_id,
            validity: t.validity,
        })
    }

    pub fn der(&self) -> &[u8] {
        &self.der
    }

    pub fn public_key(&self) -> &[u8; ED25519_KEY_LEN] {
        &self.key
    }

    pub fn key_id(&self) -> &[u8] {
        &self.key_id
    }

    pub fn subject(&self) -> &Name {
        &self.subject
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    pub spki_der: Vec<u8>,
    pub serial: Vec<u8>,
    pub key: Es256Key,
}

pub fn verify_leaf(der: &[u8], anchor: &TrustAnchor, at: Option<u64>) -> Result<Leaf, ChainError> {
    let p = parse(der)?;
    check_common(&p.cert)?;
    let t = &p.cert.tbs_certificate;
    if t.issuer != anchor.subject {
        return Err(ChainError::Issuer);
    }
    check_ed25519_signed(&p, &anchor.key)?;
    check_validity(&t.validity, at)?;
    check_validity(&anchor.validity, at)?;
    let x = extensions(&p.cert)?;
    match x.key_usage {
        Some((true, ku)) if ku.digital_signature() && !ku.key_cert_sign() && !ku.crl_sign() => {}
        _ => return Err(ChainError::KeyUsage),
    }
    match x.eku {
        Some(eku)
            if !eku.0.contains(&oid::ANY_EXT_KEY_USAGE)
                && (eku.0.contains(&oid::KP_DOCUMENT_SIGNING)
                    || eku.0.contains(&oid::KP_C2PA_CLAIM_SIGNING)) => {}
        _ => return Err(ChainError::ExtendedKeyUsage),
    }
    if x.basic.is_some_and(|(_, b)| b.ca) {
        return Err(ChainError::BasicConstraints);
    }
    match x.aki.and_then(|a| a.key_identifier) {
        Some(id) if id.as_bytes() == anchor.key_id.as_slice() => {}
        _ => return Err(ChainError::KeyIdentifier),
    }
    let spki_der = t
        .subject_public_key_info
        .to_der()
        .map_err(|_| ChainError::Malformed)?;
    let key = Es256Key::from_spki(&spki_der).map_err(|_| ChainError::PublicKey)?;
    let serial = positive_serial(t.serial_number.as_bytes())?;
    Ok(Leaf {
        spki_der,
        serial,
        key,
    })
}

fn positive_serial(raw: &[u8]) -> Result<Vec<u8>, ChainError> {
    match raw {
        [] => Err(ChainError::Serial),
        [b, ..] if b & 0x80 != 0 => Err(ChainError::Serial),
        _ => {
            let start = raw.iter().position(|&b| b != 0).ok_or(ChainError::Serial)?;
            Ok(raw[start..].to_vec())
        }
    }
}

pub fn verify_chain(
    chain: &[&[u8]],
    anchor: &TrustAnchor,
    at: Option<u64>,
) -> Result<Leaf, ChainError> {
    match chain {
        [leaf] => verify_leaf(leaf, anchor, at),
        [leaf, root] if *root == anchor.der.as_slice() => verify_leaf(leaf, anchor, at),
        [_, _] => Err(ChainError::Anchor),
        _ => Err(ChainError::Length),
    }
}

#[cfg(test)]
mod tests;
