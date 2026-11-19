use crate::Error;
use crate::der::{self, CLASS_CONTEXT, Reader, Tag};
use alloc::vec::Vec;

pub mod tag {
    pub const PURPOSE: u32 = 1;
    pub const ALGORITHM: u32 = 2;
    pub const KEY_SIZE: u32 = 3;
    pub const EC_CURVE: u32 = 10;
    pub const CREATION_DATETIME: u32 = 701;
    pub const ORIGIN: u32 = 702;
    pub const ROOT_OF_TRUST: u32 = 704;
    pub const OS_PATCH_LEVEL: u32 = 706;
    pub const ATTESTATION_APPLICATION_ID: u32 = 709;
    pub const VENDOR_PATCH_LEVEL: u32 = 718;
    pub const BOOT_PATCH_LEVEL: u32 = 719;
}

pub const PURPOSE_SIGN: u64 = 2;
pub const ALGORITHM_EC: u64 = 3;
pub const CURVE_P256: u64 = 1;
pub const ORIGIN_GENERATED: u64 = 0;

const KNOWN: [u32; 11] = [
    tag::PURPOSE,
    tag::ALGORITHM,
    tag::KEY_SIZE,
    tag::EC_CURVE,
    tag::CREATION_DATETIME,
    tag::ORIGIN,
    tag::ROOT_OF_TRUST,
    tag::OS_PATCH_LEVEL,
    tag::ATTESTATION_APPLICATION_ID,
    tag::VENDOR_PATCH_LEVEL,
    tag::BOOT_PATCH_LEVEL,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KmSecurity {
    Software,
    Tee,
    StrongBox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifiedBoot {
    Verified,
    SelfSigned,
    Unverified,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootOfTrust<'a> {
    pub verified_boot_key: &'a [u8],
    pub device_locked: bool,
    pub state: VerifiedBoot,
    pub verified_boot_hash: Option<&'a [u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuthList<'a> {
    pub purposes: u64,
    pub algorithm: Option<u64>,
    pub key_size: Option<u64>,
    pub ec_curve: Option<u64>,
    pub creation_ms: Option<u64>,
    pub origin: Option<u64>,
    pub root_of_trust: Option<RootOfTrust<'a>>,
    pub os_patch_level: Option<u32>,
    pub attestation_application_id: Option<&'a [u8]>,
    pub vendor_patch_level: Option<u32>,
    pub boot_patch_level: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyDescription<'a> {
    pub attestation_version: u32,
    pub attestation_security: KmSecurity,
    pub keymint_version: u32,
    pub keymint_security: KmSecurity,
    pub challenge: &'a [u8],
    pub unique_id: &'a [u8],
    pub software: AuthList<'a>,
    pub hardware: AuthList<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppId<'a> {
    pub packages: Vec<(&'a [u8], u64)>,
    pub digests: Vec<&'a [u8]>,
}

fn small(v: &[u8]) -> Result<u32, Error> {
    u32::try_from(der::uint(v)?).map_err(|_| Error::Der)
}

fn enumerated(v: &[u8]) -> Result<u64, Error> {
    der::uint(v)
}

impl KmSecurity {
    fn parse(v: &[u8]) -> Result<Self, Error> {
        match enumerated(v)? {
            0 => Ok(Self::Software),
            1 => Ok(Self::Tee),
            2 => Ok(Self::StrongBox),
            _ => Err(Error::KeyDescription),
        }
    }
}

impl VerifiedBoot {
    fn parse(v: &[u8]) -> Result<Self, Error> {
        match enumerated(v)? {
            0 => Ok(Self::Verified),
            1 => Ok(Self::SelfSigned),
            2 => Ok(Self::Unverified),
            3 => Ok(Self::Failed),
            _ => Err(Error::KeyDescription),
        }
    }
}

impl<'a> RootOfTrust<'a> {
    fn parse(v: &'a [u8]) -> Result<Self, Error> {
        let mut r = Reader::new(der::single(v, Tag::SEQUENCE)?);
        let verified_boot_key = r.expect(Tag::OCTET_STRING)?;
        let device_locked = der::boolean(r.expect(Tag::BOOLEAN)?)?;
        let state = VerifiedBoot::parse(r.expect(Tag::ENUMERATED)?)?;
        let verified_boot_hash = r.optional(Tag::OCTET_STRING)?;
        r.finish()?;
        Ok(Self {
            verified_boot_key,
            device_locked,
            state,
            verified_boot_hash,
        })
    }
}

fn purposes(v: &[u8]) -> Result<u64, Error> {
    let mut set = Reader::new(der::single(v, Tag::SET)?);
    let mut mask = 0u64;
    while !set.is_empty() {
        let p = der::uint(set.expect(Tag::INTEGER)?)?;
        if p < 64 {
            mask |= 1 << p;
        }
    }
    Ok(mask)
}

fn int_field(v: &[u8]) -> Result<u64, Error> {
    der::uint(der::single(v, Tag::INTEGER)?)
}

fn small_field(v: &[u8]) -> Result<u32, Error> {
    small(der::single(v, Tag::INTEGER)?)
}

impl<'a> AuthList<'a> {
    pub fn parse(v: &'a [u8]) -> Result<Self, Error> {
        let mut r = Reader::new(v);
        let mut out = Self::default();
        let mut seen = 0u16;
        while !r.is_empty() {
            let tlv = r.read()?;
            if tlv.tag.class != CLASS_CONTEXT || !tlv.tag.constructed {
                return Err(Error::KeyDescription);
            }
            let Some(bit) = KNOWN.iter().position(|&k| k == tlv.tag.number) else {
                continue;
            };
            if seen & (1 << bit) != 0 {
                return Err(Error::KeyDescription);
            }
            seen |= 1 << bit;
            let x = tlv.value;
            match tlv.tag.number {
                tag::PURPOSE => out.purposes = purposes(x)?,
                tag::ALGORITHM => out.algorithm = Some(int_field(x)?),
                tag::KEY_SIZE => out.key_size = Some(int_field(x)?),
                tag::EC_CURVE => out.ec_curve = Some(int_field(x)?),
                tag::CREATION_DATETIME => out.creation_ms = Some(int_field(x)?),
                tag::ORIGIN => out.origin = Some(int_field(x)?),
                tag::ROOT_OF_TRUST => out.root_of_trust = Some(RootOfTrust::parse(x)?),
                tag::OS_PATCH_LEVEL => out.os_patch_level = Some(small_field(x)?),
                tag::ATTESTATION_APPLICATION_ID => {
                    out.attestation_application_id = Some(der::single(x, Tag::OCTET_STRING)?)
                }
                tag::VENDOR_PATCH_LEVEL => out.vendor_patch_level = Some(small_field(x)?),
                _ => out.boot_patch_level = Some(small_field(x)?),
            }
        }
        Ok(out)
    }

    pub const fn has_purpose(&self, purpose: u64) -> bool {
        purpose < 64 && self.purposes & (1 << purpose) != 0
    }
}

impl<'a> KeyDescription<'a> {
    pub fn parse(v: &'a [u8]) -> Result<Self, Error> {
        Self::parse_inner(v).map_err(|_| Error::KeyDescription)
    }

    fn parse_inner(v: &'a [u8]) -> Result<Self, Error> {
        let mut r = Reader::new(der::single(v, Tag::SEQUENCE)?);
        let attestation_version = small(r.expect(Tag::INTEGER)?)?;
        let attestation_security = KmSecurity::parse(r.expect(Tag::ENUMERATED)?)?;
        let keymint_version = small(r.expect(Tag::INTEGER)?)?;
        let keymint_security = KmSecurity::parse(r.expect(Tag::ENUMERATED)?)?;
        let challenge = r.expect(Tag::OCTET_STRING)?;
        let unique_id = r.expect(Tag::OCTET_STRING)?;
        let software = AuthList::parse(r.expect(Tag::SEQUENCE)?)?;
        let hardware = AuthList::parse(r.expect(Tag::SEQUENCE)?)?;
        Ok(Self {
            attestation_version,
            attestation_security,
            keymint_version,
            keymint_security,
            challenge,
            unique_id,
            software,
            hardware,
        })
    }
}

impl<'a> AppId<'a> {
    pub fn parse(v: &'a [u8]) -> Result<Self, Error> {
        Self::parse_inner(v).map_err(|_| Error::KeyDescription)
    }

    fn parse_inner(v: &'a [u8]) -> Result<Self, Error> {
        let mut r = Reader::new(der::single(v, Tag::SEQUENCE)?);
        let mut infos = Reader::new(r.expect(Tag::SET)?);
        let mut digests = Reader::new(r.expect(Tag::SET)?);
        r.finish()?;
        let mut out = Self::default();
        while !infos.is_empty() {
            let mut info = Reader::new(infos.expect(Tag::SEQUENCE)?);
            let name = info.expect(Tag::OCTET_STRING)?;
            let version = der::uint(info.expect(Tag::INTEGER)?)?;
            info.finish()?;
            out.packages.push((name, version));
        }
        while !digests.is_empty() {
            out.digests.push(digests.expect(Tag::OCTET_STRING)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
