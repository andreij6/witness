use crate::cbor::{self, Decode, Decoder, Encode, Encoder, Error};
use crate::edit::{Edit, presentation_only};
use crate::evidence::{Checks, LocationEvidence, TimeEvidence};
use crate::ids::{CaptureId, KeyId, RecordId};
use alloc::vec::Vec;
use core::fmt;

pub const MAX_EDITS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Original,
    Derivative {
        parent: RecordId,
        root: CaptureId,
        edits: Vec<Edit>,
    },
}

impl Kind {
    const ORIGINAL: u8 = 0;
    const DERIVATIVE: u8 = 1;

    pub const fn is_original(&self) -> bool {
        matches!(self, Self::Original)
    }

    pub fn edits(&self) -> &[Edit] {
        match self {
            Self::Original => &[],
            Self::Derivative { edits, .. } => edits,
        }
    }

    pub fn presentation_only(&self) -> bool {
        presentation_only(self.edits())
    }
}

impl Encode for Kind {
    fn encode(&self, e: &mut Encoder<'_>) {
        match self {
            Self::Original => {
                e.map(1);
                e.field(0, &Self::ORIGINAL);
            }
            Self::Derivative {
                parent,
                root,
                edits,
            } => {
                e.map(4);
                e.field(0, &Self::DERIVATIVE);
                e.field(1, parent);
                e.field(2, root);
                e.field(3, edits);
            }
        }
    }
}

impl<'a> Decode<'a> for Kind {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        let kind = match f.req::<u8>(d, 0)? {
            Self::ORIGINAL => Self::Original,
            Self::DERIVATIVE => {
                let parent = f.req(d, 1)?;
                let root = f.req(d, 2)?;
                let edits: Vec<Edit> = f.req(d, 3)?;
                if edits.len() > MAX_EDITS {
                    return Err(Error::Length);
                }
                Self::Derivative {
                    parent,
                    root,
                    edits,
                }
            }
            _ => return Err(Error::Range),
        };
        f.finish(d)?;
        Ok(kind)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dims {
    pub width: u32,
    pub height: u32,
}

impl Encode for Dims {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.put(&(self.width, self.height));
    }
}

impl<'a> Decode<'a> for Dims {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let (width, height) = <(u32, u32)>::decode(d)?;
        if width == 0 || height == 0 {
            return Err(Error::Invalid);
        }
        Ok(Self { width, height })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyRef {
    pub key_id: KeyId,
    pub counter: u64,
}

impl Encode for KeyRef {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.put(&(self.key_id, self.counter));
    }
}

impl<'a> Decode<'a> for KeyRef {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let (key_id, counter) = <(KeyId, u64)>::decode(d)?;
        Ok(Self { key_id, counter })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CertSerial {
    len: u8,
    bytes: [u8; Self::MAX_LEN],
}

impl CertSerial {
    pub const MAX_LEN: usize = 20;

    pub fn new(magnitude: &[u8]) -> Option<Self> {
        if magnitude.is_empty() || magnitude.len() > Self::MAX_LEN || magnitude[0] == 0 {
            return None;
        }
        let mut bytes = [0u8; Self::MAX_LEN];
        bytes[..magnitude.len()].copy_from_slice(magnitude);
        Some(Self {
            len: magnitude.len() as u8,
            bytes,
        })
    }

    pub fn from_u64(v: u64) -> Option<Self> {
        let b = v.to_be_bytes();
        Self::new(&b[(v.leading_zeros() / 8) as usize..])
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

impl fmt::Debug for CertSerial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CertSerial(")?;
        self.as_bytes()
            .iter()
            .try_for_each(|b| write!(f, "{b:02x}"))?;
        f.write_str(")")
    }
}

impl Encode for CertSerial {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.bytes(self.as_bytes());
    }
}

impl<'a> Decode<'a> for CertSerial {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        Self::new(d.bytes()?).ok_or(Error::Invalid)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Geohash5([u8; 5]);

impl Geohash5 {
    pub const ALPHABET: &'static [u8; 32] = b"0123456789bcdefghjkmnpqrstuvwxyz";

    pub fn new(text: &str) -> Option<Self> {
        let bytes: [u8; 5] = text.as_bytes().try_into().ok()?;
        bytes
            .iter()
            .all(|c| Self::ALPHABET.contains(c))
            .then_some(Self(bytes))
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl fmt::Debug for Geohash5 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Geohash5({})", self.as_str())
    }
}

impl fmt::Display for Geohash5 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Encode for Geohash5 {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.text(self.as_str());
    }
}

impl<'a> Decode<'a> for Geohash5 {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        Self::new(d.text()?).ok_or(Error::Invalid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessClaim {
    pub capture_id: CaptureId,
    pub kind: Kind,
    pub asset_hash: [u8; 32],
    pub pixel_hash: [u8; 32],
    pub fp_commit: [u8; 32],
    pub dims: Dims,
    pub time: TimeEvidence,
    pub loc: LocationEvidence,
    pub checks: Checks,
    pub key: KeyRef,
    pub cert_serial: CertSerial,
    pub coarse: Option<Geohash5>,
}

impl WitnessClaim {
    pub const VERSION: u64 = 2;

    pub fn validate(&self) -> Result<(), Error> {
        match &self.kind {
            Kind::Original if self.coarse.is_some() => Err(Error::Invalid),
            Kind::Derivative { .. } if self.loc.commit.is_some() => Err(Error::Invalid),
            Kind::Derivative { edits, .. } if edits.len() > MAX_EDITS => Err(Error::Length),
            _ if self.dims.width == 0 || self.dims.height == 0 => Err(Error::Invalid),
            _ => Ok(()),
        }
    }

    pub fn to_cbor(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        Ok(cbor::to_vec(self))
    }

    pub fn from_cbor(bytes: &[u8]) -> Result<Self, Error> {
        cbor::from_slice_canonical(bytes)
    }

    pub fn record_id(&self) -> RecordId {
        RecordId::from_claim_cbor(&cbor::to_vec(self))
    }
}

impl Encode for WitnessClaim {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(12 + usize::from(self.coarse.is_some()));
        e.field(0, &Self::VERSION);
        e.field(1, &self.capture_id);
        e.field(2, &self.kind);
        e.field(3, &self.asset_hash);
        e.field(4, &self.pixel_hash);
        e.field(5, &self.fp_commit);
        e.field(6, &self.dims);
        e.field(7, &self.time);
        e.field(8, &self.loc);
        e.field(9, &self.checks);
        e.field(10, &self.key);
        e.field(11, &self.cert_serial);
        e.opt_field(12, self.coarse.as_ref());
    }
}

impl<'a> Decode<'a> for WitnessClaim {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        f.version(d, Self::VERSION)?;
        let claim = Self {
            capture_id: f.req(d, 1)?,
            kind: f.req(d, 2)?,
            asset_hash: f.req(d, 3)?,
            pixel_hash: f.req(d, 4)?,
            fp_commit: f.req(d, 5)?,
            dims: f.req(d, 6)?,
            time: f.req(d, 7)?,
            loc: f.req(d, 8)?,
            checks: f.req(d, 9)?,
            key: f.req(d, 10)?,
            cert_serial: f.req(d, 11)?,
            coarse: f.opt(d, 12)?,
        };
        f.finish(d)?;
        claim.validate()?;
        Ok(claim)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Submission {
    pub claim_cbor: Vec<u8>,
    pub c2pa_claim_cbor: Vec<u8>,
    pub cose: Vec<u8>,
}

impl Submission {
    pub const VERSION: u64 = 1;
    pub const MAX_PART: usize = 64 * 1024;

    pub fn record_id(&self) -> RecordId {
        RecordId::from_claim_cbor(&self.claim_cbor)
    }

    pub fn claim(&self) -> Result<WitnessClaim, Error> {
        WitnessClaim::from_cbor(&self.claim_cbor)
    }

    pub fn within_limits(&self) -> bool {
        [&self.claim_cbor, &self.c2pa_claim_cbor, &self.cose]
            .iter()
            .all(|p| !p.is_empty() && p.len() <= Self::MAX_PART)
    }
}

impl Encode for Submission {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(4);
        e.field(0, &Self::VERSION);
        e.field(1, self.claim_cbor.as_slice());
        e.field(2, self.c2pa_claim_cbor.as_slice());
        e.field(3, self.cose.as_slice());
    }
}

impl<'a> Decode<'a> for Submission {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        f.version(d, Self::VERSION)?;
        let s = Self {
            claim_cbor: f.req::<&[u8]>(d, 1)?.to_vec(),
            c2pa_claim_cbor: f.req::<&[u8]>(d, 2)?.to_vec(),
            cose: f.req::<&[u8]>(d, 3)?.to_vec(),
        };
        f.finish(d)?;
        if s.within_limits() {
            Ok(s)
        } else {
            Err(Error::Length)
        }
    }
}

#[cfg(test)]
mod tests;
