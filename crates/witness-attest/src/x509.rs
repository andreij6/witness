use crate::Error;
use crate::der::{self, Reader, Tag};
use crate::oid;
use crate::pubkey::PublicKey;
pub use crate::pubkey::SigAlg;
use crate::serial::Serial;
use crate::time::parse_time;

const KEY_CERT_SIGN: u8 = 0x04;
const MAX_SERIAL_OCTETS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicConstraints {
    pub ca: bool,
    pub path_len: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Certificate<'a> {
    pub der: &'a [u8],
    pub tbs: &'a [u8],
    pub sig_alg: SigAlg,
    pub signature: &'a [u8],
    pub serial: Serial,
    pub issuer: &'a [u8],
    pub subject: &'a [u8],
    pub not_before: i64,
    pub not_after: i64,
    pub spki: &'a [u8],
    pub basic_constraints: Option<BasicConstraints>,
    pub key_cert_sign: Option<bool>,
    pub key_description: Option<&'a [u8]>,
    pub provisioning_info: bool,
}

#[derive(Default)]
struct Extensions<'a> {
    basic_constraints: Option<BasicConstraints>,
    key_cert_sign: Option<bool>,
    key_description: Option<&'a [u8]>,
    provisioning_info: bool,
}

fn serial(value: &[u8]) -> Result<Serial, Error> {
    if value.is_empty() || value.len() > MAX_SERIAL_OCTETS + 1 {
        return Err(Error::Serial);
    }
    Serial::from_be_bytes(value).ok_or(Error::Serial)
}

fn basic_constraints(value: &[u8]) -> Result<BasicConstraints, Error> {
    let mut r = Reader::new(der::single(value, Tag::SEQUENCE)?);
    let ca = r
        .optional(Tag::BOOLEAN)?
        .map(der::boolean)
        .transpose()?
        .unwrap_or(false);
    let path_len = r
        .optional(Tag::INTEGER)?
        .map(|v| der::uint(v).and_then(|n| u32::try_from(n).map_err(|_| Error::Der)))
        .transpose()?;
    r.finish()?;
    Ok(BasicConstraints { ca, path_len })
}

fn key_cert_sign(value: &[u8]) -> Result<bool, Error> {
    let (_, bits) = der::bit_string(der::single(value, Tag::BIT_STRING)?)?;
    Ok(bits.first().is_some_and(|b| b & KEY_CERT_SIGN != 0))
}

fn set_once<T>(slot: &mut Option<T>, v: T) -> Result<(), Error> {
    if slot.replace(v).is_some() {
        Err(Error::DuplicateExtension)
    } else {
        Ok(())
    }
}

fn extensions(value: &[u8]) -> Result<Extensions<'_>, Error> {
    let mut list = Reader::new(der::single(value, Tag::SEQUENCE)?);
    if list.is_empty() {
        return Err(Error::Der);
    }
    let mut out = Extensions::default();
    while !list.is_empty() {
        let mut ext = Reader::new(list.expect(Tag::SEQUENCE)?);
        let id = ext.expect(Tag::OID)?;
        let critical = ext
            .optional(Tag::BOOLEAN)?
            .map(der::boolean)
            .transpose()?
            .unwrap_or(false);
        let body = ext.expect(Tag::OCTET_STRING)?;
        ext.finish()?;
        match id {
            oid::BASIC_CONSTRAINTS => {
                set_once(&mut out.basic_constraints, basic_constraints(body)?)?
            }
            oid::KEY_USAGE => set_once(&mut out.key_cert_sign, key_cert_sign(body)?)?,
            oid::KEY_DESCRIPTION => set_once(&mut out.key_description, body)?,
            oid::PROVISIONING_INFO if out.provisioning_info => {
                return Err(Error::DuplicateExtension);
            }
            oid::PROVISIONING_INFO => out.provisioning_info = true,
            _ if critical => return Err(Error::UnknownCritical),
            _ => {}
        }
    }
    Ok(out)
}

impl<'a> Certificate<'a> {
    pub fn parse(der_bytes: &'a [u8]) -> Result<Self, Error> {
        let mut cert = Reader::new(der::single(der_bytes, Tag::SEQUENCE)?);
        let tbs_tlv = cert.expect_tlv(Tag::SEQUENCE)?;
        let outer_alg = cert.expect(Tag::SEQUENCE)?;
        let signature = der::octet_aligned_bits(cert.expect(Tag::BIT_STRING)?)?;
        cert.finish()?;

        let mut tbs = Reader::new(tbs_tlv.value);
        let version = tbs.expect(Tag::context(0, true))?;
        if der::uint(der::single(version, Tag::INTEGER)?)? != 2 {
            return Err(Error::Version);
        }
        let serial = serial(tbs.expect(Tag::INTEGER)?)?;
        if tbs.expect(Tag::SEQUENCE)? != outer_alg {
            return Err(Error::AlgorithmMismatch);
        }
        let sig_alg = SigAlg::from_algorithm_identifier(outer_alg)?;
        let issuer = tbs.expect(Tag::SEQUENCE)?;
        let mut validity = Reader::new(tbs.expect(Tag::SEQUENCE)?);
        let not_before = parse_time(&validity.read()?)?;
        let not_after = parse_time(&validity.read()?)?;
        validity.finish()?;
        let subject = tbs.expect(Tag::SEQUENCE)?;
        let spki = tbs.expect_tlv(Tag::SEQUENCE)?.raw;
        tbs.optional(Tag::context(1, false))?;
        tbs.optional(Tag::context(2, false))?;
        let ext = tbs
            .optional(Tag::context(3, true))?
            .map(extensions)
            .transpose()?
            .unwrap_or_default();
        tbs.finish()?;

        Ok(Self {
            der: der_bytes,
            tbs: tbs_tlv.raw,
            sig_alg,
            signature,
            serial,
            issuer,
            subject,
            not_before,
            not_after,
            spki,
            basic_constraints: ext.basic_constraints,
            key_cert_sign: ext.key_cert_sign,
            key_description: ext.key_description,
            provisioning_info: ext.provisioning_info,
        })
    }

    pub fn public_key(&self) -> Result<PublicKey, Error> {
        PublicKey::from_spki(self.spki)
    }

    pub fn verify_signed_by(&self, issuer: &PublicKey) -> Result<(), Error> {
        issuer.verify(self.sig_alg, self.tbs, self.signature)
    }

    pub const fn valid_at(&self, unix_secs: i64) -> bool {
        self.not_before <= unix_secs && unix_secs <= self.not_after
    }

    pub const fn is_ca(&self) -> bool {
        matches!(
            self.basic_constraints,
            Some(BasicConstraints { ca: true, .. })
        )
    }
}

#[cfg(test)]
mod tests;
