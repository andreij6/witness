use crate::Error;
use crate::x509::{TrustAnchor, ed25519_alg, key_identifier, oid};
use alloc::vec;
use alloc::vec::Vec;
use core::str::FromStr;
use core::time::Duration;
use witness_format::CertSerial;
use witness_format::crypto::{ED25519_KEY_LEN, ED25519_SIG_LEN, Es256Key};
use x509_cert::certificate::{TbsCertificate, Version};
use x509_cert::der::asn1::{BitString, GeneralizedTime, ObjectIdentifier, OctetString, UtcTime};
use x509_cert::der::{Decode, Encode, Header, Length, Tag};
use x509_cert::ext::Extension;
use x509_cert::ext::pkix::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, KeyUsages,
    SubjectKeyIdentifier,
};
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::spki::SubjectPublicKeyInfoOwned;
use x509_cert::time::{Time, Validity};

const UTC_TIME_END: u64 = 2_524_608_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertParams<'a> {
    pub subject: &'a str,
    pub serial: CertSerial,
    pub not_before: u64,
    pub not_after: u64,
}

fn der_err<T>(_: T) -> Error {
    Error::MalformedCertificate
}

fn time(secs: u64) -> Result<Time, Error> {
    let d = Duration::from_secs(secs);
    if secs < UTC_TIME_END {
        UtcTime::from_unix_duration(d).map(Time::UtcTime)
    } else {
        GeneralizedTime::from_unix_duration(d).map(Time::GeneralTime)
    }
    .map_err(der_err)
}

fn extension(
    id: ObjectIdentifier,
    critical: bool,
    value: &impl Encode,
) -> Result<Extension, Error> {
    Ok(Extension {
        extn_id: id,
        critical,
        extn_value: OctetString::new(value.to_der().map_err(der_err)?).map_err(der_err)?,
    })
}

fn tbs(
    p: &CertParams<'_>,
    issuer: Name,
    spki: SubjectPublicKeyInfoOwned,
    extensions: Vec<Extension>,
) -> Result<Vec<u8>, Error> {
    if p.not_before >= p.not_after {
        return Err(Error::MalformedCertificate);
    }
    TbsCertificate {
        version: Version::V3,
        serial_number: SerialNumber::new(p.serial.as_bytes()).map_err(der_err)?,
        signature: ed25519_alg(),
        issuer,
        validity: Validity {
            not_before: time(p.not_before)?,
            not_after: time(p.not_after)?,
        },
        subject: Name::from_str(p.subject).map_err(der_err)?,
        subject_public_key_info: spki,
        issuer_unique_id: None,
        subject_unique_id: None,
        extensions: Some(extensions),
    }
    .to_der()
    .map_err(der_err)
}

fn ski(spki: &SubjectPublicKeyInfoOwned) -> Result<Extension, Error> {
    let raw = spki.subject_public_key.raw_bytes();
    let id = OctetString::new(key_identifier(raw).to_vec()).map_err(der_err)?;
    extension(oid::SUBJECT_KEY_ID, false, &SubjectKeyIdentifier(id))
}

pub fn root_tbs(p: &CertParams<'_>, public_key: &[u8; ED25519_KEY_LEN]) -> Result<Vec<u8>, Error> {
    let spki = SubjectPublicKeyInfoOwned {
        algorithm: ed25519_alg(),
        subject_public_key: BitString::from_bytes(public_key).map_err(der_err)?,
    };
    let exts = vec![
        extension(
            oid::BASIC_CONSTRAINTS,
            true,
            &BasicConstraints {
                ca: true,
                path_len_constraint: Some(0),
            },
        )?,
        extension(
            oid::KEY_USAGE,
            true,
            &KeyUsage(KeyUsages::KeyCertSign | KeyUsages::CRLSign),
        )?,
        ski(&spki)?,
    ];
    let subject = Name::from_str(p.subject).map_err(der_err)?;
    tbs(p, subject, spki, exts)
}

pub fn leaf_tbs(
    p: &CertParams<'_>,
    spki_der: &[u8],
    issuer: &TrustAnchor,
) -> Result<Vec<u8>, Error> {
    Es256Key::from_spki(spki_der).map_err(|_| Error::MalformedCertificate)?;
    let spki = SubjectPublicKeyInfoOwned::from_der(spki_der).map_err(der_err)?;
    let aki = AuthorityKeyIdentifier {
        key_identifier: Some(OctetString::new(issuer.key_id().to_vec()).map_err(der_err)?),
        authority_cert_issuer: None,
        authority_cert_serial_number: None,
    };
    let exts = vec![
        extension(
            oid::BASIC_CONSTRAINTS,
            true,
            &BasicConstraints {
                ca: false,
                path_len_constraint: None,
            },
        )?,
        extension(
            oid::KEY_USAGE,
            true,
            &KeyUsage(KeyUsages::DigitalSignature.into()),
        )?,
        extension(
            oid::EXT_KEY_USAGE,
            false,
            &ExtendedKeyUsage(vec![oid::KP_DOCUMENT_SIGNING]),
        )?,
        ski(&spki)?,
        extension(oid::AUTHORITY_KEY_ID, false, &aki)?,
    ];
    tbs(p, issuer.subject().clone(), spki, exts)
}

pub fn assemble(tbs_der: &[u8], signature: &[u8; ED25519_SIG_LEN]) -> Result<Vec<u8>, Error> {
    TbsCertificate::from_der(tbs_der).map_err(der_err)?;
    let alg = ed25519_alg().to_der().map_err(der_err)?;
    let sig = BitString::from_bytes(signature)
        .and_then(|b| b.to_der())
        .map_err(der_err)?;
    let body = tbs_der.len() + alg.len() + sig.len();
    let header = Header::new(Tag::Sequence, Length::try_from(body).map_err(der_err)?)
        .and_then(|h| h.to_der())
        .map_err(der_err)?;
    let mut out = Vec::with_capacity(header.len() + body);
    [header.as_slice(), tbs_der, &alg, &sig]
        .iter()
        .for_each(|part| out.extend_from_slice(part));
    Ok(out)
}

pub fn build_root(
    p: &CertParams<'_>,
    public_key: &[u8; ED25519_KEY_LEN],
    sign: impl FnOnce(&[u8]) -> [u8; ED25519_SIG_LEN],
) -> Result<Vec<u8>, Error> {
    let t = root_tbs(p, public_key)?;
    assemble(&t, &sign(&t))
}

pub fn build_leaf(
    p: &CertParams<'_>,
    spki_der: &[u8],
    issuer: &TrustAnchor,
    sign: impl FnOnce(&[u8]) -> [u8; ED25519_SIG_LEN],
) -> Result<Vec<u8>, Error> {
    let t = leaf_tbs(p, spki_der, issuer)?;
    assemble(&t, &sign(&t))
}

#[cfg(test)]
mod tests;
