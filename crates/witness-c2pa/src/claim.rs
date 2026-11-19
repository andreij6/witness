use crate::Error;
use crate::cbor::{array_len, map_len, set_once, whole};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use minicbor::Decoder;
use witness_format::cbor::Encoder;

pub const LABEL_V2: &str = "c2pa.claim.v2";
pub const LABEL_V1: &str = "c2pa.claim";
pub const SIGNATURE_LABEL: &str = "c2pa.signature";
pub const ASSERTIONS_LABEL: &str = "c2pa.assertions";
pub const STORE_LABEL: &str = "c2pa";
pub const ALG_SHA256: &str = "sha256";
const JUMBF_PREFIX: &str = "self#jumbf=";
const MAX_URIS: u64 = 1024;

pub fn manifest_label(id: &[u8; 16]) -> String {
    format!("urn:c2pa:{}", uuid_v4_string(id))
}

pub fn instance_id(id: &[u8; 16]) -> String {
    format!("xmp:iid:{}", uuid_v4_string(id))
}

pub fn uuid_v4_string(id: &[u8; 16]) -> String {
    let mut b = *id;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let mut s = String::with_capacity(36);
    for (i, byte) in b.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            s.push('-');
        }
        s.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        s.push(char::from_digit(u32::from(byte & 0xf), 16).unwrap_or('0'));
    }
    s
}

pub fn assertion_uri(manifest: &str, assertion: &str) -> String {
    format!("{JUMBF_PREFIX}/{STORE_LABEL}/{manifest}/{ASSERTIONS_LABEL}/{assertion}")
}

pub fn signature_uri(manifest: &str) -> String {
    format!("{JUMBF_PREFIX}/{STORE_LABEL}/{manifest}/{SIGNATURE_LABEL}")
}

fn manifest_relative<'a>(uri: &'a str, manifest: &str) -> Option<&'a str> {
    let path = uri.strip_prefix(JUMBF_PREFIX)?;
    match path.strip_prefix('/') {
        Some(abs) => abs
            .strip_prefix(STORE_LABEL)?
            .strip_prefix('/')?
            .strip_prefix(manifest)?
            .strip_prefix('/'),
        None => Some(path),
    }
}

pub fn assertion_label<'a>(uri: &'a str, manifest: &str) -> Option<&'a str> {
    manifest_relative(uri, manifest)?
        .strip_prefix(ASSERTIONS_LABEL)?
        .strip_prefix('/')
        .filter(|l| !l.is_empty() && !l.contains('/'))
}

pub fn is_signature_uri(uri: &str, manifest: &str) -> bool {
    manifest_relative(uri, manifest) == Some(SIGNATURE_LABEL)
}

pub fn signature_manifest(uri: &str) -> Option<&str> {
    let path = uri.strip_prefix(JUMBF_PREFIX)?;
    match path.strip_prefix('/') {
        Some(abs) => abs
            .strip_prefix(STORE_LABEL)?
            .strip_prefix('/')?
            .strip_suffix(SIGNATURE_LABEL)?
            .strip_suffix('/')
            .filter(|m| !m.is_empty() && !m.contains('/')),
        None => (path == SIGNATURE_LABEL).then_some(""),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashedUri<'a> {
    pub url: &'a str,
    pub alg: Option<&'a str>,
    pub hash: &'a [u8],
}

impl<'a> HashedUri<'a> {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let (mut url, mut alg, mut hash) = (None, None, None);
        for _ in 0..map_len(d)? {
            match d.str()? {
                "url" => set_once(&mut url, d.str()?, Error::MalformedClaim)?,
                "alg" => set_once(&mut alg, d.str()?, Error::MalformedClaim)?,
                "hash" => set_once(&mut hash, d.bytes()?, Error::MalformedClaim)?,
                _ => return Err(Error::MalformedClaim),
            }
        }
        Ok(Self {
            url: url.ok_or(Error::MalformedClaim)?,
            alg,
            hash: hash.ok_or(Error::MalformedClaim)?,
        })
    }

    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(2 + usize::from(self.alg.is_some()));
        e.text("url");
        e.text(self.url);
        if let Some(a) = self.alg {
            e.text("alg");
            e.text(a);
        }
        e.text("hash");
        e.bytes(self.hash);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generator<'a> {
    pub name: &'a str,
    pub version: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimV2<'a> {
    pub instance_id: &'a str,
    pub generator_name: &'a str,
    pub signature: &'a str,
    pub created: Vec<HashedUri<'a>>,
    pub gathered: Vec<HashedUri<'a>>,
    pub alg: Option<&'a str>,
}

fn uri_list<'a>(d: &mut Decoder<'a>) -> Result<Vec<HashedUri<'a>>, Error> {
    let n = array_len(d)?;
    if n > MAX_URIS {
        return Err(Error::MalformedClaim);
    }
    (0..n).map(|_| HashedUri::decode(d)).collect()
}

fn generator_name<'a>(d: &mut Decoder<'a>) -> Result<&'a str, Error> {
    let mut name = None;
    for _ in 0..map_len(d)? {
        match d.str()? {
            "name" => set_once(&mut name, d.str()?, Error::MalformedClaim)?,
            _ => d.skip()?,
        }
    }
    name.ok_or(Error::MalformedClaim)
}

impl<'a> ClaimV2<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        whole(bytes, |d| {
            let (mut iid, mut generator, mut sig, mut created, mut gathered, mut alg) =
                (None, None, None, None, None, None);
            let mut redacted = None;
            for _ in 0..map_len(d)? {
                match d.str()? {
                    "instanceID" => set_once(&mut iid, d.str()?, Error::MalformedClaim)?,
                    "claim_generator_info" => {
                        set_once(&mut generator, generator_name(d)?, Error::MalformedClaim)?
                    }
                    "signature" => set_once(&mut sig, d.str()?, Error::MalformedClaim)?,
                    "created_assertions" => {
                        set_once(&mut created, uri_list(d)?, Error::MalformedClaim)?
                    }
                    "gathered_assertions" => {
                        set_once(&mut gathered, uri_list(d)?, Error::MalformedClaim)?
                    }
                    "redacted_assertions" => {
                        d.skip()?;
                        set_once(&mut redacted, (), Error::MalformedClaim)?
                    }
                    "alg" => set_once(&mut alg, d.str()?, Error::MalformedClaim)?,
                    "alg_soft" | "dc:title" | "metadata" => d.skip()?,
                    _ => return Err(Error::MalformedClaim),
                }
            }
            if redacted.is_some() {
                return Err(Error::UnsupportedClaim);
            }
            let created = created.ok_or(Error::MalformedClaim)?;
            if created.is_empty() {
                return Err(Error::MalformedClaim);
            }
            Ok(Self {
                instance_id: iid.ok_or(Error::MalformedClaim)?,
                generator_name: generator.ok_or(Error::MalformedClaim)?,
                signature: sig.ok_or(Error::MalformedClaim)?,
                created,
                gathered: gathered.unwrap_or_default(),
                alg,
            })
        })
    }

    pub fn all_assertions(&self) -> impl Iterator<Item = &HashedUri<'a>> {
        self.created.iter().chain(self.gathered.iter())
    }
}

pub fn write_claim(
    out: &mut Vec<u8>,
    instance_id: &str,
    generator: &Generator<'_>,
    signature_uri: &str,
    created: &[HashedUri<'_>],
) {
    let mut e = Encoder::new(out);
    e.map(5);
    e.text("instanceID");
    e.text(instance_id);
    e.text("claim_generator_info");
    e.map(2);
    e.text("name");
    e.text(generator.name);
    e.text("version");
    e.text(generator.version);
    e.text("signature");
    e.text(signature_uri);
    e.text("created_assertions");
    e.array(created.len());
    created.iter().for_each(|u| u.encode(&mut e));
    e.text("alg");
    e.text(ALG_SHA256);
}

#[cfg(test)]
mod tests;
