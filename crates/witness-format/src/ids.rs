use crate::base32::{self, Base32Error};
use crate::cbor::{Decode, Decoder, Encode, Encoder, Error};
use crate::crypto::sha256;
use alloc::string::String;
use core::fmt;
use core::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaptureId(u64);

impl CaptureId {
    pub const BITS: u32 = 61;
    pub const RANDOM_BITS: u32 = 56;
    pub const SCHEMA_BITS: u32 = 5;
    pub const RANDOM_MASK: u64 = (1 << Self::RANDOM_BITS) - 1;
    pub const MAX: u64 = (1 << Self::BITS) - 1;

    pub const fn new(random: u64, schema: u8) -> Option<Self> {
        if random > Self::RANDOM_MASK || schema >= 1 << Self::SCHEMA_BITS {
            return None;
        }
        Some(Self(((schema as u64) << Self::RANDOM_BITS) | random))
    }

    pub const fn from_random_bytes(random: [u8; 7], schema: u8) -> Option<Self> {
        let mut v = 0u64;
        let mut i = 0;
        while i < 7 {
            v = (v << 8) | random[i] as u64;
            i += 1;
        }
        Self::new(v, schema)
    }

    pub const fn from_u64(v: u64) -> Option<Self> {
        if v > Self::MAX { None } else { Some(Self(v)) }
    }

    pub const fn to_u64(self) -> u64 {
        self.0
    }

    pub const fn schema(self) -> u8 {
        (self.0 >> Self::RANDOM_BITS) as u8
    }

    pub const fn random(self) -> u64 {
        self.0 & Self::RANDOM_MASK
    }
}

impl Encode for CaptureId {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.u64(self.0);
    }
}

impl<'a> Decode<'a> for CaptureId {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        Self::from_u64(d.u64()?).ok_or(Error::Range)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdParseError {
    Base32(Base32Error),
}

impl From<Base32Error> for IdParseError {
    fn from(e: Base32Error) -> Self {
        Self::Base32(e)
    }
}

impl fmt::Display for IdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Base32(e) => e.fmt(f),
        }
    }
}

impl core::error::Error for IdParseError {}

macro_rules! byte_id {
    ($name:ident, $len:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name([u8; $len]);

        impl $name {
            pub const LEN: usize = $len;

            pub const fn from_bytes(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }

            pub const fn as_bytes(&self) -> &[u8; $len] {
                &self.0
            }

            pub const fn to_bytes(self) -> [u8; $len] {
                self.0
            }
        }

        impl TryFrom<&[u8]> for $name {
            type Error = Error;
            fn try_from(b: &[u8]) -> Result<Self, Error> {
                b.try_into().map(Self).map_err(|_| Error::Length)
            }
        }

        impl Encode for $name {
            fn encode(&self, e: &mut Encoder<'_>) {
                e.bytes(&self.0);
            }
        }

        impl<'a> Decode<'a> for $name {
            fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
                d.byte_array().map(Self)
            }
        }
    };
}

macro_rules! base32_id {
    ($name:ident, $len:literal, $chars:literal) => {
        impl $name {
            pub const TEXT_LEN: usize = $chars;

            pub fn to_base32(&self) -> [u8; $chars] {
                let mut out = [0u8; $chars];
                base32::encode(&self.0, &mut out);
                out
            }

            pub fn parse(text: &str) -> Result<Self, IdParseError> {
                let mut out = [0u8; $len];
                base32::decode(text.as_bytes(), &mut out)?;
                Ok(Self(out))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let text = self.to_base32();
                f.write_str(core::str::from_utf8(&text).map_err(|_| fmt::Error)?)
            }
        }

        impl FromStr for $name {
            type Err = IdParseError;
            fn from_str(s: &str) -> Result<Self, IdParseError> {
                Self::parse(s)
            }
        }
    };
}

byte_id!(RecordId, 10);
byte_id!(KeyId, 16);
byte_id!(CaseId, 16);
base32_id!(RecordId, 10, 16);
base32_id!(CaseId, 16, 26);

impl RecordId {
    pub const GROUP: usize = 4;
    pub const GROUPED_LEN: usize = 19;

    pub fn from_claim_cbor(claim_cbor: &[u8]) -> Self {
        let h = sha256(claim_cbor);
        let mut id = [0u8; 10];
        id.copy_from_slice(&h[..10]);
        Self(id)
    }

    pub fn to_grouped(&self) -> [u8; 19] {
        let plain = self.to_base32();
        let mut out = [b'-'; 19];
        for (i, chunk) in plain.chunks_exact(Self::GROUP).enumerate() {
            let at = i * (Self::GROUP + 1);
            out[at..at + Self::GROUP].copy_from_slice(chunk);
        }
        out
    }

    pub fn grouped(&self) -> String {
        self.to_grouped().iter().map(|&b| char::from(b)).collect()
    }
}

impl KeyId {
    pub fn from_spki(spki_der: &[u8]) -> Self {
        let h = sha256(spki_der);
        let mut id = [0u8; 16];
        id.copy_from_slice(&h[..16]);
        Self(id)
    }
}

impl CaseId {
    pub const fn new(shard: u8, random: [u8; 15]) -> Self {
        let mut id = [0u8; 16];
        id[0] = shard;
        let mut i = 0;
        while i < 15 {
            id[i + 1] = random[i];
            i += 1;
        }
        Self(id)
    }

    pub const fn shard(&self) -> u8 {
        self.0[0]
    }

    pub fn random(&self) -> &[u8] {
        &self.0[1..]
    }
}

#[cfg(test)]
mod tests;
