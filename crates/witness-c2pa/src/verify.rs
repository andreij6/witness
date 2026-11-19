use crate::app11;
use crate::assertions::{DATA_HASH, DataHash, WITNESS_CLAIM, cbor_assertion_hash};
use crate::claim::{self, ALG_SHA256, ClaimV2, LABEL_V2};
use crate::cose::Sign1;
use crate::jumbf::{SuperBox, uuid};
use crate::manifest::{self, Manifest};
use crate::x509::{self, Leaf, TrustAnchor};
use crate::{Error, Part};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};
use witness_format::{KeyId, WitnessClaim};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seal {
    pub claim: WitnessClaim,
    pub signer_spki: Vec<u8>,
    pub cert_serial: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessManifest {
    pub label: String,
    pub seal: Seal,
    pub witness_claim_cbor: Vec<u8>,
    pub c2pa_claim: Vec<u8>,
    pub cose_sign1: Vec<u8>,
    pub declared_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credentials {
    Absent,
    Other,
    Witness(Box<WitnessManifest>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub data_hash: [u8; 32],
    pub credentials: Credentials,
}

impl Report {
    pub fn witness(&self) -> Option<&WitnessManifest> {
        match &self.credentials {
            Credentials::Witness(w) => Some(w),
            _ => None,
        }
    }

    pub fn binding_intact(&self) -> bool {
        self.witness().is_some_and(|w| {
            w.declared_hash == self.data_hash && w.seal.claim.asset_hash == self.data_hash
        })
    }
}

fn check_alg(alg: Option<&str>) -> Result<(), Error> {
    match alg {
        None | Some(ALG_SHA256) => Ok(()),
        Some(_) => Err(Error::UnsupportedAlgorithm),
    }
}

fn signed_claim<'a>(
    c2pa_claim: &'a [u8],
    cose: &[u8],
    anchor: &TrustAnchor,
    at: Option<u64>,
) -> Result<(ClaimV2<'a>, Leaf), Error> {
    let sign1 = Sign1::parse(cose)?;
    let leaf = x509::verify_chain(&sign1.x5chain, anchor, at)?;
    sign1.verify(c2pa_claim, &leaf.key)?;
    let claim = ClaimV2::parse(c2pa_claim)?;
    check_alg(claim.alg)?;
    claim.all_assertions().try_for_each(|u| check_alg(u.alg))?;
    Ok((claim, leaf))
}

fn sole_created<'a>(claim: &ClaimV2<'a>, manifest: &str, label: &str) -> Result<&'a [u8], Error> {
    let mut found = claim
        .created
        .iter()
        .filter(|u| claim::assertion_label(u.url, manifest) == Some(label));
    match (found.next(), found.next()) {
        (Some(u), None) => Ok(u.hash),
        (None, _) => Err(Error::UnresolvedAssertion),
        _ => Err(Error::DuplicateAssertion),
    }
}

fn into_seal(witness_cbor: &[u8], leaf: Leaf) -> Result<Seal, Error> {
    let claim = WitnessClaim::from_cbor(witness_cbor).map_err(Error::WitnessClaim)?;
    if claim.key.key_id != KeyId::from_spki(&leaf.spki_der) {
        return Err(Error::KeyMismatch);
    }
    if claim.cert_serial.as_bytes() != leaf.serial.as_slice() {
        return Err(Error::SerialMismatch);
    }
    Ok(Seal {
        claim,
        signer_spki: leaf.spki_der,
        cert_serial: leaf.serial,
    })
}

pub fn verify_seal(
    witness_claim_cbor: &[u8],
    c2pa_claim_cbor: &[u8],
    cose_sign1: &[u8],
    anchor: &TrustAnchor,
    at: Option<u64>,
) -> Result<Seal, Error> {
    let (claim, leaf) = signed_claim(c2pa_claim_cbor, cose_sign1, anchor, at)?;
    let manifest = claim::signature_manifest(claim.signature).ok_or(Error::MalformedClaim)?;
    let declared = sole_created(&claim, manifest, WITNESS_CLAIM)?;
    if declared != cbor_assertion_hash(WITNESS_CLAIM, witness_claim_cbor)? {
        return Err(Error::AssertionHash);
    }
    into_seal(witness_claim_cbor, leaf)
}

fn mentions_witness(m: &SuperBox<'_>) -> bool {
    m.superboxes()
        .flatten()
        .filter(|c| c.desc.uuid == uuid::ASSERTIONS)
        .flat_map(|c| c.superboxes().flatten())
        .any(|a| a.label() == Some(WITNESS_CLAIM))
}

fn verify_manifest(
    m: &Manifest<'_>,
    anchor: &TrustAnchor,
    at: Option<u64>,
) -> Result<WitnessManifest, Error> {
    if m.claim_label != LABEL_V2 {
        return Err(Error::UnsupportedClaim);
    }
    let (claim, leaf) = signed_claim(m.claim, m.signature, anchor, at)?;
    if !claim::is_signature_uri(claim.signature, m.label) {
        return Err(Error::MalformedClaim);
    }
    for u in claim.all_assertions() {
        let label = claim::assertion_label(u.url, m.label).ok_or(Error::UnresolvedAssertion)?;
        let a = m.assertion(label).ok_or(Error::UnresolvedAssertion)?;
        if Sha256::digest(a.payload).as_slice() != u.hash {
            return Err(Error::AssertionHash);
        }
    }
    sole_created(&claim, m.label, WITNESS_CLAIM)?;
    sole_created(&claim, m.label, DATA_HASH)?;
    let witness_cbor = m
        .assertion(WITNESS_CLAIM)
        .ok_or(Error::MissingPart(Part::WitnessClaim))?
        .cbor()?;
    let dh = DataHash::parse(
        m.assertion(DATA_HASH)
            .ok_or(Error::MissingPart(Part::DataHash))?
            .cbor()?,
    )?;
    check_alg(dh.alg)?;
    let declared_hash: [u8; 32] = dh.hash.try_into().map_err(|_| Error::MalformedAssertion)?;
    Ok(WitnessManifest {
        label: m.label.into(),
        seal: into_seal(witness_cbor, leaf)?,
        witness_claim_cbor: witness_cbor.to_vec(),
        c2pa_claim: m.claim.to_vec(),
        cose_sign1: m.signature.to_vec(),
        declared_hash,
    })
}

pub fn verify_jpeg(jpeg: &[u8], anchor: &TrustAnchor, at: Option<u64>) -> Result<Report, Error> {
    let layout = app11::locate(jpeg)?;
    let data_hash = app11::hash_excluding(jpeg, &layout.excluded);
    let credentials = match &layout.store {
        None => Credentials::Absent,
        Some(store) => credentials(store, anchor, at)?,
    };
    Ok(Report {
        data_hash,
        credentials,
    })
}

fn credentials(store: &[u8], anchor: &TrustAnchor, at: Option<u64>) -> Result<Credentials, Error> {
    let active = manifest::active_manifest(store)?;
    if active.desc.uuid != uuid::MANIFEST {
        return Ok(Credentials::Other);
    }
    let m = match manifest::parse_manifest(&active) {
        Ok(m) => m,
        Err(e) if mentions_witness(&active) => return Err(e),
        Err(_) => return Ok(Credentials::Other),
    };
    if !m.has_assertion(WITNESS_CLAIM) {
        return Ok(Credentials::Other);
    }
    verify_manifest(&m, anchor, at).map(|w| Credentials::Witness(Box::new(w)))
}

#[cfg(test)]
mod tests;
