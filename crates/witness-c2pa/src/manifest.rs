use crate::claim::{ASSERTIONS_LABEL, LABEL_V2, SIGNATURE_LABEL, STORE_LABEL};
use crate::jumbf::{self, CBOR, SuperBox, uuid};
use crate::{Error, Part};
use alloc::vec::Vec;

const MAX_ASSERTIONS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assertion<'a> {
    pub label: &'a str,
    pub uuid: jumbf::Uuid,
    pub payload: &'a [u8],
    pub content: &'a [u8],
}

impl<'a> Assertion<'a> {
    fn from_superbox(sb: SuperBox<'a>) -> Result<Self, Error> {
        Ok(Self {
            label: sb.label().ok_or(Error::MalformedJumbf)?,
            uuid: sb.desc.uuid,
            payload: sb.payload,
            content: sb.content,
        })
    }

    pub fn cbor(&self) -> Result<&'a [u8], Error> {
        if self.uuid != uuid::CBOR {
            return Err(Error::MalformedAssertion);
        }
        match jumbf::single_box(self.content)? {
            b if b.kind == CBOR => Ok(b.payload),
            _ => Err(Error::MalformedAssertion),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest<'a> {
    pub label: &'a str,
    pub claim_label: &'a str,
    pub claim: &'a [u8],
    pub signature: &'a [u8],
    pub assertions: Vec<Assertion<'a>>,
}

impl<'a> Manifest<'a> {
    pub fn assertion(&self, label: &str) -> Option<&Assertion<'a>> {
        self.assertions
            .binary_search_by(|a| a.label.cmp(label))
            .ok()
            .map(|i| &self.assertions[i])
    }

    pub fn has_assertion(&self, label: &str) -> bool {
        self.assertion(label).is_some()
    }
}

pub fn store_manifests(store: &[u8]) -> Result<Vec<SuperBox<'_>>, Error> {
    let root = SuperBox::from_bytes(store)?;
    if root.desc.uuid != uuid::STORE || root.label() != Some(STORE_LABEL) {
        return Err(Error::MalformedJumbf);
    }
    let manifests = root.superboxes().collect::<Result<Vec<_>, _>>()?;
    if manifests.is_empty() {
        return Err(Error::MissingPart(Part::Manifest));
    }
    Ok(manifests)
}

pub fn active_manifest(store: &[u8]) -> Result<SuperBox<'_>, Error> {
    store_manifests(store)?
        .pop()
        .ok_or(Error::MissingPart(Part::Manifest))
}

fn put_once<'a>(slot: &mut Option<SuperBox<'a>>, sb: SuperBox<'a>) -> Result<(), Error> {
    match slot.replace(sb) {
        None => Ok(()),
        Some(_) => Err(Error::MalformedJumbf),
    }
}

pub fn parse_manifest<'a>(m: &SuperBox<'a>) -> Result<Manifest<'a>, Error> {
    if m.desc.uuid != uuid::MANIFEST {
        return Err(Error::MalformedJumbf);
    }
    let label = m.label().ok_or(Error::MalformedJumbf)?;
    let (mut store, mut claim, mut sig) = (None, None, None);
    for child in m.superboxes() {
        let child = child?;
        match child.desc.uuid {
            uuid::ASSERTIONS => put_once(&mut store, child)?,
            uuid::CLAIM => put_once(&mut claim, child)?,
            uuid::SIGNATURE => put_once(&mut sig, child)?,
            _ => {}
        }
    }
    let store = store.ok_or(Error::MissingPart(Part::AssertionStore))?;
    let claim = claim.ok_or(Error::MissingPart(Part::Claim))?;
    let sig = sig.ok_or(Error::MissingPart(Part::Signature))?;
    if sig.label() != Some(SIGNATURE_LABEL) || store.label() != Some(ASSERTIONS_LABEL) {
        return Err(Error::MalformedJumbf);
    }
    let mut assertions = Vec::new();
    for a in store.superboxes() {
        if assertions.len() == MAX_ASSERTIONS {
            return Err(Error::TooLarge);
        }
        assertions.push(Assertion::from_superbox(a?)?);
    }
    assertions.sort_unstable_by(|a, b| a.label.cmp(b.label));
    if assertions.windows(2).any(|w| w[0].label == w[1].label) {
        return Err(Error::DuplicateAssertion);
    }
    Ok(Manifest {
        label,
        claim_label: claim.label().ok_or(Error::MalformedJumbf)?,
        claim: claim.only_child(CBOR)?,
        signature: sig.only_child(CBOR)?,
        assertions,
    })
}

pub fn write_store(
    out: &mut Vec<u8>,
    manifest_label: &str,
    assertion_boxes: &[&[u8]],
    claim: &[u8],
    cose: &[u8],
) -> Result<(), Error> {
    jumbf::write_superbox(out, &uuid::STORE, STORE_LABEL, |o| {
        jumbf::write_superbox(o, &uuid::MANIFEST, manifest_label, |o| {
            jumbf::write_superbox(o, &uuid::ASSERTIONS, ASSERTIONS_LABEL, |o| {
                assertion_boxes.iter().for_each(|b| o.extend_from_slice(b));
                Ok(())
            })?;
            jumbf::write_superbox(o, &uuid::CLAIM, LABEL_V2, |o| {
                jumbf::write_box(o, CBOR, claim)
            })?;
            jumbf::write_superbox(o, &uuid::SIGNATURE, SIGNATURE_LABEL, |o| {
                jumbf::write_box(o, CBOR, cose)
            })?;
            Ok(())
        })?;
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests;
