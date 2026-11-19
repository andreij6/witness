use crate::Error;
use crate::cbor::{array_len, map_len, set_once, whole};
use crate::jumbf::{self, CBOR, uuid};
use alloc::vec::Vec;
use minicbor::Decoder;
use sha2::{Digest, Sha256};
use witness_format::cbor::Encoder;

pub const ACTIONS: &str = "c2pa.actions.v2";
pub const DATA_HASH: &str = "c2pa.hash.data";
pub const WITNESS_CLAIM: &str = "app.witness.claim";
pub const CREATED: &str = "c2pa.created";
pub const OPENED: &str = "c2pa.opened";
pub const DIGITAL_CAPTURE: &str = "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture";
const DATA_HASH_NAME: &str = "jumbf manifest";
const MAX_EXCLUSIONS: u64 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action<'a> {
    pub action: &'a str,
    pub digital_source_type: Option<&'a str>,
}

impl Action<'static> {
    pub const CAPTURED: Self = Self {
        action: CREATED,
        digital_source_type: Some(DIGITAL_CAPTURE),
    };
}

pub fn actions_cbor(actions: &[Action<'_>]) -> Result<Vec<u8>, Error> {
    match actions.first() {
        Some(a) if a.action == CREATED || a.action == OPENED => {}
        _ => return Err(Error::MalformedAssertion),
    }
    if actions[1..]
        .iter()
        .any(|a| a.action == CREATED || a.action == OPENED)
    {
        return Err(Error::MalformedAssertion);
    }
    let mut out = Vec::new();
    let mut e = Encoder::new(&mut out);
    e.map(1);
    e.text("actions");
    e.array(actions.len());
    for a in actions {
        e.map(1 + usize::from(a.digital_source_type.is_some()));
        e.text("action");
        e.text(a.action);
        if let Some(t) = a.digital_source_type {
            e.text("digitalSourceType");
            e.text(t);
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exclusion {
    pub start: u64,
    pub length: u64,
}

pub fn data_hash_cbor(exclusion: Exclusion, hash: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    let mut e = Encoder::new(&mut out);
    e.map(5);
    e.text("exclusions");
    e.array(1);
    e.map(2);
    e.text("start");
    e.u64(exclusion.start);
    e.text("length");
    e.u64(exclusion.length);
    e.text("name");
    e.text(DATA_HASH_NAME);
    e.text("alg");
    e.text(crate::claim::ALG_SHA256);
    e.text("hash");
    e.bytes(hash);
    e.text("pad");
    e.bytes(&[]);
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataHash<'a> {
    pub exclusions: Vec<Exclusion>,
    pub alg: Option<&'a str>,
    pub hash: &'a [u8],
}

fn exclusion(d: &mut Decoder<'_>) -> Result<Exclusion, Error> {
    let (mut start, mut length) = (None, None);
    for _ in 0..map_len(d)? {
        match d.str()? {
            "start" => set_once(&mut start, d.u64()?, Error::MalformedAssertion)?,
            "length" => set_once(&mut length, d.u64()?, Error::MalformedAssertion)?,
            _ => return Err(Error::MalformedAssertion),
        }
    }
    Ok(Exclusion {
        start: start.ok_or(Error::MalformedAssertion)?,
        length: length.ok_or(Error::MalformedAssertion)?,
    })
}

impl<'a> DataHash<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        whole(bytes, |d| {
            let (mut excl, mut alg, mut hash, mut name, mut pad, mut pad2) =
                (None, None, None, None, None, None);
            for _ in 0..map_len(d)? {
                match d.str()? {
                    "exclusions" => {
                        let n = array_len(d)?;
                        if n > MAX_EXCLUSIONS {
                            return Err(Error::MalformedAssertion);
                        }
                        let v = (0..n)
                            .map(|_| exclusion(d))
                            .collect::<Result<Vec<_>, _>>()?;
                        set_once(&mut excl, v, Error::MalformedAssertion)?
                    }
                    "alg" => set_once(&mut alg, d.str()?, Error::MalformedAssertion)?,
                    "hash" => set_once(&mut hash, d.bytes()?, Error::MalformedAssertion)?,
                    "name" => set_once(&mut name, d.str()?, Error::MalformedAssertion)?,
                    "pad" => set_once(&mut pad, d.bytes()?, Error::MalformedAssertion)?,
                    "pad2" => set_once(&mut pad2, d.bytes()?, Error::MalformedAssertion)?,
                    _ => return Err(Error::MalformedAssertion),
                }
            }
            Ok(Self {
                exclusions: excl.unwrap_or_default(),
                alg,
                hash: hash.ok_or(Error::MalformedAssertion)?,
            })
        })
    }
}

pub fn cbor_assertion_box(label: &str, cbor: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(cbor.len() + label.len() + 48);
    jumbf::write_superbox(&mut out, &uuid::CBOR, label, |o| {
        jumbf::write_box(o, CBOR, cbor)
    })?;
    Ok(out)
}

pub fn box_hash(superbox: &[u8]) -> [u8; 32] {
    Sha256::digest(&superbox[8..]).into()
}

pub fn cbor_assertion_hash(label: &str, cbor: &[u8]) -> Result<[u8; 32], Error> {
    cbor_assertion_box(label, cbor).map(|b| box_hash(&b))
}

#[cfg(test)]
mod tests;
