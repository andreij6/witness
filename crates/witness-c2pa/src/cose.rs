use crate::cbor::{array_len, int_key, is_null, map_len, set_once, whole};
use crate::{Error, Part};
use alloc::vec::Vec;
use minicbor::Decoder;
use minicbor::data::{Tag, Type};
use witness_format::cbor::Encoder;
use witness_format::crypto::{ES256_RAW_LEN, Es256Key};

pub const TAG_SIGN1: u64 = 18;
pub const ALG_ES256: i64 = -7;
pub const HDR_ALG: i64 = 1;
pub const HDR_CRIT: i64 = 2;
pub const HDR_X5CHAIN: i64 = 33;
const CONTEXT: &str = "Signature1";
const MAX_CHAIN: u64 = 8;
const TAG_MAJOR: u8 = 6;

pub fn protected_header(chain: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + chain.iter().map(|c| c.len() + 5).sum::<usize>());
    let mut e = Encoder::new(&mut out);
    e.map(2);
    e.i64(HDR_ALG);
    e.i64(ALG_ES256);
    e.i64(HDR_X5CHAIN);
    match chain {
        [one] => e.bytes(one),
        many => {
            e.array(many.len());
            many.iter().for_each(|c| e.bytes(c));
        }
    }
    out
}

pub fn sig_structure(protected: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(protected.len() + payload.len() + 32);
    let mut e = Encoder::new(&mut out);
    e.array(4);
    e.text(CONTEXT);
    e.bytes(protected);
    e.bytes(&[]);
    e.bytes(payload);
    out
}

pub fn encode_sign1(protected: &[u8], signature: &[u8; ES256_RAW_LEN]) -> Vec<u8> {
    let mut out = Vec::with_capacity(protected.len() + ES256_RAW_LEN + 16);
    let mut e = Encoder::new(&mut out);
    e.head(TAG_MAJOR, TAG_SIGN1);
    e.array(4);
    e.bytes(protected);
    e.map(0);
    e.null();
    e.bytes(signature);
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sign1<'a> {
    pub protected: &'a [u8],
    pub alg: i64,
    pub x5chain: Vec<&'a [u8]>,
    pub signature: &'a [u8],
}

fn x5chain<'a>(d: &mut Decoder<'a>) -> Result<Vec<&'a [u8]>, Error> {
    match d.datatype()? {
        Type::Bytes => Ok(alloc::vec![d.bytes()?]),
        Type::Array => {
            let n = array_len(d)?;
            if n == 0 || n > MAX_CHAIN {
                return Err(Error::MalformedCose);
            }
            (0..n).map(|_| Ok(d.bytes()?)).collect()
        }
        _ => Err(Error::MalformedCose),
    }
}

type Protected<'a> = (i64, Option<Vec<&'a [u8]>>);

fn parse_protected(bytes: &[u8]) -> Result<Protected<'_>, Error> {
    whole(bytes, |d| {
        let (mut alg, mut chain) = (None, None);
        let n = map_len(d)?;
        let mut seen = Vec::with_capacity(n.min(16) as usize);
        for _ in 0..n {
            let key = int_key(d)?;
            seen.push(key);
            match key {
                HDR_ALG => set_once(&mut alg, int_key(d)?, Error::MalformedCose)?,
                HDR_X5CHAIN => set_once(&mut chain, x5chain(d)?, Error::MalformedCose)?,
                HDR_CRIT => return Err(Error::MalformedCose),
                _ => d.skip()?,
            }
        }
        seen.sort_unstable();
        if seen.windows(2).any(|w| w[0] == w[1]) {
            return Err(Error::MalformedCose);
        }
        Ok((alg.ok_or(Error::MalformedCose)?, chain))
    })
}

impl<'a> Sign1<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        whole(bytes, |d| {
            if d.tag()? != Tag::new(TAG_SIGN1) {
                return Err(Error::MalformedCose);
            }
            if array_len(d)? != 4 {
                return Err(Error::MalformedCose);
            }
            let protected = d.bytes()?;
            for _ in 0..map_len(d)? {
                header_key(d)?;
                d.skip()?;
            }
            if !is_null(d)? {
                return Err(Error::MalformedCose);
            }
            d.null()?;
            let signature = d.bytes()?;
            let (alg, chain) = parse_protected(protected)?;
            let x5chain = chain.ok_or(Error::MissingPart(Part::X5Chain))?;
            Ok(Self {
                protected,
                alg,
                x5chain,
                signature,
            })
        })
    }

    pub fn verify(&self, payload: &[u8], key: &Es256Key) -> Result<(), Error> {
        if self.alg != ALG_ES256 {
            return Err(Error::UnsupportedAlgorithm);
        }
        if self.signature.len() != ES256_RAW_LEN {
            return Err(Error::Signature);
        }
        key.verify(&sig_structure(self.protected, payload), self.signature)
            .map_err(|_| Error::Signature)
    }
}

fn header_key(d: &mut Decoder<'_>) -> Result<(), Error> {
    match d.datatype()? {
        Type::String => d.str().map(drop).map_err(Error::from),
        _ => int_key(d).map(drop),
    }
}

#[cfg(test)]
mod tests;
