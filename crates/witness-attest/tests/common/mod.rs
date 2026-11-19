#![allow(dead_code)]

use p256::ecdsa::signature::hazmat::PrehashSigner;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::vec::Vec;
use witness_attest::roots::{RootKind, TrustAnchor};
use witness_attest::time::unix_seconds;
use witness_attest::{Attested, Context, Enrollment, Error, LateOpening, Policy, RevocationList};

pub const AOSP_EC_ROOT: &[u8] = include_bytes!("../data/aosp_ec_root.der");
pub const AOSP_EC_INTERMEDIATE: &[u8] = include_bytes!("../data/aosp_ec_intermediate.der");
pub const AOSP_EC_INTERMEDIATE_KEY: &[u8] = include_bytes!("../data/aosp_ec_intermediate.key.der");
pub const AOSP_RSA_ROOT: &[u8] = include_bytes!("../data/aosp_rsa_root.der");
pub const AOSP_RSA_INTERMEDIATE: &[u8] = include_bytes!("../data/aosp_rsa_intermediate.der");
pub const AOSP_RSA_INTERMEDIATE_KEY: &[u8] =
    include_bytes!("../data/aosp_rsa_intermediate.key.der");
pub const GOOGLE_ROOTS: [&[u8]; 5] = [
    include_bytes!("../data/google_root_0.der"),
    include_bytes!("../data/google_root_1.der"),
    include_bytes!("../data/google_root_2.der"),
    include_bytes!("../data/google_root_3.der"),
    include_bytes!("../data/google_root_4.der"),
];

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    pub fn fill(&mut self, out: &mut [u8]) {
        out.iter_mut().for_each(|b| *b = self.next() as u8);
    }

    pub fn bytes<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        self.fill(&mut out);
        out
    }
}

pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

pub fn len_bytes(len: usize) -> Vec<u8> {
    if len < 0x80 {
        return std::vec![len as u8];
    }
    let be = (len as u64).to_be_bytes();
    let start = be.iter().position(|&b| b != 0).unwrap_or(7);
    let mut out = std::vec![0x80 | (8 - start) as u8];
    out.extend_from_slice(&be[start..]);
    out
}

pub fn tlv(tag: &[u8], content: &[u8]) -> Vec<u8> {
    let mut out = tag.to_vec();
    out.extend(len_bytes(content.len()));
    out.extend_from_slice(content);
    out
}

pub fn tag_bytes(class: u8, constructed: bool, number: u32) -> Vec<u8> {
    let first = (class << 6) | if constructed { 0x20 } else { 0 };
    if number < 31 {
        return std::vec![first | number as u8];
    }
    let mut groups = Vec::new();
    let mut n = number;
    loop {
        groups.push((n & 0x7f) as u8);
        n >>= 7;
        if n == 0 {
            break;
        }
    }
    let mut out = std::vec![first | 0x1f];
    for (i, g) in groups.iter().enumerate().rev() {
        out.push(if i == 0 { *g } else { g | 0x80 });
    }
    out
}

pub fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

pub fn seq(parts: &[&[u8]]) -> Vec<u8> {
    tlv(&[0x30], &cat(parts))
}

pub fn set(parts: &[&[u8]]) -> Vec<u8> {
    tlv(&[0x31], &cat(parts))
}

pub fn explicit(number: u32, inner: &[u8]) -> Vec<u8> {
    tlv(&tag_bytes(2, true, number), inner)
}

pub fn int_content(magnitude: &[u8]) -> Vec<u8> {
    let start = magnitude
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(magnitude.len());
    let m = &magnitude[start..];
    if m.is_empty() {
        return std::vec![0];
    }
    let mut out = Vec::new();
    if m[0] & 0x80 != 0 {
        out.push(0);
    }
    out.extend_from_slice(m);
    out
}

pub fn int(v: u64) -> Vec<u8> {
    tlv(&[0x02], &int_content(&v.to_be_bytes()))
}

pub fn int_raw(content: &[u8]) -> Vec<u8> {
    tlv(&[0x02], content)
}

pub fn enumerated(v: u64) -> Vec<u8> {
    tlv(&[0x0a], &int_content(&v.to_be_bytes()))
}

pub fn octets(b: &[u8]) -> Vec<u8> {
    tlv(&[0x04], b)
}

pub fn boolean(v: bool) -> Vec<u8> {
    tlv(&[0x01], &[if v { 0xff } else { 0 }])
}

pub fn null() -> Vec<u8> {
    std::vec![0x05, 0x00]
}

pub fn oid(content: &[u8]) -> Vec<u8> {
    tlv(&[0x06], content)
}

pub fn bits(b: &[u8]) -> Vec<u8> {
    let mut c = std::vec![0u8];
    c.extend_from_slice(b);
    tlv(&[0x03], &c)
}

pub fn time(unix: i64) -> Vec<u8> {
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let hms = std::format!(
        "{:02}{:02}{:02}{:02}{:02}Z",
        m,
        d,
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    );
    if (1950..2050).contains(&y) {
        tlv(&[0x17], std::format!("{:02}{hms}", y % 100).as_bytes())
    } else {
        tlv(&[0x18], std::format!("{y:04}{hms}").as_bytes())
    }
}

pub fn name(cn: &str) -> Vec<u8> {
    seq(&[&set(&[&seq(&[
        &oid(&[0x55, 0x04, 0x03]),
        &tlv(&[0x0c], cn.as_bytes()),
    ])])])
}

pub mod oids {
    pub const RSA_ENCRYPTION: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    pub const SHA256_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
    pub const SHA384_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0c];
    pub const SHA512_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0d];
    pub const SHA1_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x05];
    pub const EC_PUBLIC_KEY: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
    pub const ECDSA_SHA256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];
    pub const ECDSA_SHA384: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x03];
    pub const ECDSA_SHA512: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x04];
    pub const PRIME256V1: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
    pub const SECP384R1: &[u8] = &[0x2b, 0x81, 0x04, 0x00, 0x22];
    pub const SECP521R1: &[u8] = &[0x2b, 0x81, 0x04, 0x00, 0x23];
    pub const BASIC_CONSTRAINTS: &[u8] = &[0x55, 0x1d, 0x13];
    pub const KEY_USAGE: &[u8] = &[0x55, 0x1d, 0x0f];
    pub const SUBJECT_KEY_ID: &[u8] = &[0x55, 0x1d, 0x0e];
    pub const NAME_CONSTRAINTS: &[u8] = &[0x55, 0x1d, 0x1e];
    pub const KEY_DESCRIPTION: &[u8] =
        &[0x2b, 0x06, 0x01, 0x04, 0x01, 0xd6, 0x79, 0x02, 0x01, 0x11];
    pub const PROVISIONING_INFO: &[u8] =
        &[0x2b, 0x06, 0x01, 0x04, 0x01, 0xd6, 0x79, 0x02, 0x01, 0x1e];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alg {
    EcdsaSha256,
    EcdsaSha384,
    EcdsaSha512,
    RsaSha256,
    RsaSha384,
    RsaSha512,
}

impl Alg {
    pub fn identifier(self) -> Vec<u8> {
        match self {
            Self::EcdsaSha256 => seq(&[&oid(oids::ECDSA_SHA256)]),
            Self::EcdsaSha384 => seq(&[&oid(oids::ECDSA_SHA384)]),
            Self::EcdsaSha512 => seq(&[&oid(oids::ECDSA_SHA512)]),
            Self::RsaSha256 => seq(&[&oid(oids::SHA256_RSA), &null()]),
            Self::RsaSha384 => seq(&[&oid(oids::SHA384_RSA), &null()]),
            Self::RsaSha512 => seq(&[&oid(oids::SHA512_RSA), &null()]),
        }
    }

    fn digest(self, msg: &[u8]) -> Vec<u8> {
        match self {
            Self::EcdsaSha256 | Self::RsaSha256 => Sha256::digest(msg).to_vec(),
            Self::EcdsaSha384 | Self::RsaSha384 => Sha384::digest(msg).to_vec(),
            Self::EcdsaSha512 | Self::RsaSha512 => Sha512::digest(msg).to_vec(),
        }
    }
}

#[derive(Clone)]
pub enum TestKey {
    P256(p256::ecdsa::SigningKey),
    P384(p384::ecdsa::SigningKey),
    Rsa(rsa::RsaPrivateKey),
}

impl TestKey {
    pub fn p256(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        loop {
            if let Ok(k) = p256::ecdsa::SigningKey::from_slice(&rng.bytes::<32>()) {
                return Self::P256(k);
            }
        }
    }

    pub fn p384(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        loop {
            if let Ok(k) = p384::ecdsa::SigningKey::from_slice(&rng.bytes::<48>()) {
                return Self::P384(k);
            }
        }
    }

    pub fn aosp_rsa() -> Self {
        Self::Rsa(rsa::RsaPrivateKey::from_pkcs1_der(AOSP_RSA_INTERMEDIATE_KEY).unwrap())
    }

    pub fn aosp_ec() -> Self {
        Self::P256(p256::ecdsa::SigningKey::from_slice(&AOSP_EC_INTERMEDIATE_KEY[7..39]).unwrap())
    }

    pub fn default_alg(&self) -> Alg {
        match self {
            Self::P256(_) => Alg::EcdsaSha256,
            Self::P384(_) => Alg::EcdsaSha384,
            Self::Rsa(_) => Alg::RsaSha256,
        }
    }

    pub fn spki(&self) -> Vec<u8> {
        match self {
            Self::P256(k) => seq(&[
                &seq(&[&oid(oids::EC_PUBLIC_KEY), &oid(oids::PRIME256V1)]),
                &bits(k.verifying_key().to_encoded_point(false).as_bytes()),
            ]),
            Self::P384(k) => seq(&[
                &seq(&[&oid(oids::EC_PUBLIC_KEY), &oid(oids::SECP384R1)]),
                &bits(k.verifying_key().to_encoded_point(false).as_bytes()),
            ]),
            Self::Rsa(k) => seq(&[
                &seq(&[&oid(oids::RSA_ENCRYPTION), &null()]),
                &bits(&seq(&[
                    &int_raw(&int_content(&k.n().to_bytes_be())),
                    &int_raw(&int_content(&k.e().to_bytes_be())),
                ])),
            ]),
        }
    }

    pub fn sign(&self, alg: Alg, msg: &[u8]) -> Vec<u8> {
        let digest = alg.digest(msg);
        match self {
            Self::P256(k) => {
                let s: p256::ecdsa::Signature = k.sign_prehash(&digest).unwrap();
                s.to_der().as_bytes().to_vec()
            }
            Self::P384(k) => {
                let s: p384::ecdsa::Signature = k.sign_prehash(&digest).unwrap();
                s.to_der().as_bytes().to_vec()
            }
            Self::Rsa(k) => {
                let padding = match alg {
                    Alg::RsaSha384 => rsa::Pkcs1v15Sign::new::<Sha384>(),
                    Alg::RsaSha512 => rsa::Pkcs1v15Sign::new::<Sha512>(),
                    _ => rsa::Pkcs1v15Sign::new::<Sha256>(),
                };
                k.sign(padding, &digest).unwrap()
            }
        }
    }
}

pub fn extension(id: &[u8], critical: bool, value: &[u8]) -> Vec<u8> {
    if critical {
        seq(&[&oid(id), &boolean(true), &octets(value)])
    } else {
        seq(&[&oid(id), &octets(value)])
    }
}

pub fn basic_constraints(ca: bool, path_len: Option<u64>) -> Vec<u8> {
    let mut inner = Vec::new();
    if ca {
        inner.extend(boolean(true));
    }
    if let Some(p) = path_len {
        inner.extend(int(p));
    }
    extension(oids::BASIC_CONSTRAINTS, true, &seq(&[&inner]))
}

pub fn key_usage(cert_sign: bool) -> Vec<u8> {
    let value = if cert_sign {
        tlv(&[0x03], &[0x01, 0x86])
    } else {
        tlv(&[0x03], &[0x07, 0x80])
    };
    extension(oids::KEY_USAGE, true, &value)
}

pub fn key_description_ext(kd: &[u8]) -> Vec<u8> {
    extension(oids::KEY_DESCRIPTION, false, kd)
}

pub fn provisioning_info_ext() -> Vec<u8> {
    extension(oids::PROVISIONING_INFO, false, &[0xa1, 0x01, 0x08])
}

#[derive(Clone)]
pub struct CertSpec {
    pub version: u64,
    pub serial: Vec<u8>,
    pub subject: Vec<u8>,
    pub not_before: i64,
    pub not_after: i64,
    pub extensions: Vec<Vec<u8>>,
    pub alg: Option<Alg>,
    pub tbs_alg_override: Option<Vec<u8>>,
}

pub fn build_cert(
    spec: &CertSpec,
    subject_key: &TestKey,
    issuer_name: &[u8],
    signer: &TestKey,
) -> Vec<u8> {
    let alg = spec.alg.unwrap_or_else(|| signer.default_alg());
    let alg_id = alg.identifier();
    let tbs_alg = spec
        .tbs_alg_override
        .clone()
        .unwrap_or_else(|| alg_id.clone());
    let validity = seq(&[&time(spec.not_before), &time(spec.not_after)]);
    let ext = if spec.extensions.is_empty() {
        Vec::new()
    } else {
        explicit(
            3,
            &seq(&spec
                .extensions
                .iter()
                .map(Vec::as_slice)
                .collect::<Vec<_>>()),
        )
    };
    let tbs = seq(&[
        &explicit(0, &int(spec.version)),
        &int_raw(&spec.serial),
        &tbs_alg,
        issuer_name,
        &validity,
        &spec.subject,
        &subject_key.spki(),
        &ext,
    ]);
    let sig = signer.sign(alg, &tbs);
    seq(&[&tbs, &alg_id, &bits(&sig)])
}

pub mod kd {
    use super::*;

    pub const PURPOSE: u32 = 1;
    pub const ALGORITHM: u32 = 2;
    pub const KEY_SIZE: u32 = 3;
    pub const EC_CURVE: u32 = 10;
    pub const NO_AUTH_REQUIRED: u32 = 503;
    pub const CREATION_DATETIME: u32 = 701;
    pub const ORIGIN: u32 = 702;
    pub const ROOT_OF_TRUST: u32 = 704;
    pub const OS_VERSION: u32 = 705;
    pub const OS_PATCH_LEVEL: u32 = 706;
    pub const APP_ID: u32 = 709;
    pub const VENDOR_PATCH_LEVEL: u32 = 718;
    pub const BOOT_PATCH_LEVEL: u32 = 719;

    pub fn purpose(values: &[u64]) -> Vec<u8> {
        let ints: Vec<Vec<u8>> = values.iter().map(|&v| int(v)).collect();
        explicit(
            PURPOSE,
            &set(&ints.iter().map(Vec::as_slice).collect::<Vec<_>>()),
        )
    }

    pub fn int_field(tag: u32, v: u64) -> Vec<u8> {
        explicit(tag, &int(v))
    }

    pub fn null_field(tag: u32) -> Vec<u8> {
        explicit(tag, &null())
    }

    pub fn root_of_trust(locked: bool, state: u64) -> Vec<u8> {
        explicit(
            ROOT_OF_TRUST,
            &seq(&[
                &octets(&[0x11; 32]),
                &boolean(locked),
                &enumerated(state),
                &octets(&[0x22; 32]),
            ]),
        )
    }

    pub fn app_id_der(packages: &[(&[u8], u64)], digests: &[&[u8]]) -> Vec<u8> {
        let infos: Vec<Vec<u8>> = packages
            .iter()
            .map(|(n, v)| seq(&[&octets(n), &int(*v)]))
            .collect();
        let ds: Vec<Vec<u8>> = digests.iter().map(|d| octets(d)).collect();
        seq(&[
            &set(&infos.iter().map(Vec::as_slice).collect::<Vec<_>>()),
            &set(&ds.iter().map(Vec::as_slice).collect::<Vec<_>>()),
        ])
    }

    pub fn app_id(packages: &[(&[u8], u64)], digests: &[&[u8]]) -> Vec<u8> {
        explicit(APP_ID, &octets(&app_id_der(packages, digests)))
    }

    #[derive(Clone)]
    pub struct Kd {
        pub attestation_version: u64,
        pub attestation_security: u64,
        pub keymint_version: u64,
        pub keymint_security: u64,
        pub challenge: Vec<u8>,
        pub unique_id: Vec<u8>,
        pub software: Vec<Vec<u8>>,
        pub hardware: Vec<Vec<u8>>,
        pub trailing: Vec<u8>,
    }

    impl Kd {
        pub fn der(&self) -> Vec<u8> {
            let list = |l: &Vec<Vec<u8>>| seq(&l.iter().map(Vec::as_slice).collect::<Vec<_>>());
            seq(&[
                &int(self.attestation_version),
                &enumerated(self.attestation_security),
                &int(self.keymint_version),
                &enumerated(self.keymint_security),
                &octets(&self.challenge),
                &octets(&self.unique_id),
                &list(&self.software),
                &list(&self.hardware),
                &self.trailing,
            ])
        }

        pub fn remove(list: &mut Vec<Vec<u8>>, tag: u32) {
            let prefix = tag_bytes(2, true, tag);
            list.retain(|f| !f.starts_with(&prefix));
        }

        pub fn replace(list: &mut Vec<Vec<u8>>, tag: u32, field: Vec<u8>) {
            Self::remove(list, tag);
            list.push(field);
        }
    }
}

pub const RELEASE_DIGEST: [u8; 32] = [0xd1; 32];
pub const OTHER_DIGEST: [u8; 32] = [0xd2; 32];
pub const RELEASE_DIGESTS: &[[u8; 32]] = &[OTHER_DIGEST, RELEASE_DIGEST];
pub const SECRET: [u8; 32] = [0x5e; 32];
pub const PATCH: u64 = 202_611;
pub const APP_VERSION: u64 = 42;
pub const HOUR: i64 = 3600;
pub const DAY: i64 = 86_400;

pub fn now_secs() -> i64 {
    unix_seconds(2026, 11, 19, 12, 0, 0).unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootAlg {
    P384,
    Rsa,
    P256,
}

#[derive(Clone)]
pub struct Fixture {
    pub keys: Vec<TestKey>,
    pub certs: Vec<CertSpec>,
    pub kd: kd::Kd,
    pub leaf_kd: bool,
    pub root_kind: RootKind,
    pub now_ns: u64,
    pub secret: [u8; 32],
    pub revocations: RevocationList,
    pub late: Option<(Vec<u8>, u64)>,
    pub key_id: Option<witness_format::KeyId>,
    pub policy: Policy<'static>,
}

pub fn good_hardware() -> Vec<Vec<u8>> {
    std::vec![
        kd::purpose(&[2, 3]),
        kd::int_field(kd::ALGORITHM, 3),
        kd::int_field(kd::KEY_SIZE, 256),
        kd::int_field(kd::EC_CURVE, 1),
        kd::null_field(kd::NO_AUTH_REQUIRED),
        kd::int_field(kd::ORIGIN, 0),
        kd::root_of_trust(true, 0),
        kd::int_field(kd::OS_VERSION, 160_000),
        kd::int_field(kd::OS_PATCH_LEVEL, PATCH),
        kd::int_field(kd::VENDOR_PATCH_LEVEL, 20_261_105),
        kd::int_field(kd::BOOT_PATCH_LEVEL, 20_261_105),
    ]
}

pub fn good_software(creation_ms: u64) -> Vec<Vec<u8>> {
    std::vec![
        kd::int_field(kd::CREATION_DATETIME, creation_ms),
        kd::app_id(&[(b"app.witness", APP_VERSION)], &[&RELEASE_DIGEST]),
    ]
}

impl Fixture {
    pub fn new(root: RootAlg) -> Self {
        let now = now_secs();
        let now_ns = now as u64 * 1_000_000_000;
        let root_key = match root {
            RootAlg::P384 => TestKey::p384(400),
            RootAlg::Rsa => TestKey::aosp_rsa(),
            RootAlg::P256 => TestKey::p256(401),
        };
        let keys = std::vec![
            TestKey::p256(100),
            TestKey::p256(200),
            TestKey::p384(300),
            root_key
        ];
        let ca_exts = || std::vec![basic_constraints(true, None), key_usage(true)];
        let spec = |i: u8, cn: &str, nb: i64, na: i64, extensions: Vec<Vec<u8>>| CertSpec {
            version: 2,
            serial: std::vec![0x10 + i, 0xab, i],
            subject: name(cn),
            not_before: nb,
            not_after: na,
            extensions,
            alg: None,
            tbs_alg_override: None,
        };
        let certs = std::vec![
            spec(
                0,
                "leaf",
                now - 60,
                now + 3650 * DAY,
                std::vec![key_usage(false)]
            ),
            spec(
                1,
                "attestation key",
                now - 30 * DAY,
                now + 30 * DAY,
                ca_exts()
            ),
            spec(
                2,
                "intermediate",
                now - 365 * DAY,
                now + 365 * DAY,
                ca_exts()
            ),
            spec(3, "root", now - 3650 * DAY, now + 3650 * DAY, ca_exts()),
        ];
        let kd = kd::Kd {
            attestation_version: 300,
            attestation_security: 1,
            keymint_version: 300,
            keymint_security: 1,
            challenge: witness_attest::issue_fresh(&SECRET, now_ns - 600_000_000_000).to_vec(),
            unique_id: Vec::new(),
            software: good_software((now as u64 - 60) * 1000),
            hardware: good_hardware(),
            trailing: Vec::new(),
        };
        let mut policy = Policy::witness(RELEASE_DIGESTS);
        policy.anchors = &[];
        policy.min_patch = 202_601;
        policy.min_version = 40;
        Self {
            keys,
            certs,
            kd,
            leaf_kd: true,
            root_kind: if root == RootAlg::P384 {
                RootKind::GoogleEc
            } else {
                RootKind::GoogleRsa
            },
            now_ns,
            secret: SECRET,
            revocations: RevocationList::empty(),
            late: None,
            key_id: None,
            policy,
        }
    }

    pub fn now(&self) -> i64 {
        (self.now_ns / 1_000_000_000) as i64
    }

    pub fn len(&self) -> usize {
        self.certs.len()
    }

    pub fn root_spki(&self) -> Vec<u8> {
        self.keys[self.keys.len() - 1].spki()
    }

    pub fn leaf_spki(&self) -> Vec<u8> {
        self.keys[0].spki()
    }

    pub fn remove_intermediate(&mut self, index: usize) {
        self.keys.remove(index);
        self.certs.remove(index);
    }

    pub fn build(&self) -> Vec<Vec<u8>> {
        let n = self.certs.len();
        (0..n)
            .map(|i| {
                let mut spec = self.certs[i].clone();
                if i == 0 && self.leaf_kd {
                    spec.extensions.push(key_description_ext(&self.kd.der()));
                }
                let signer = (i + 1).min(n - 1);
                build_cert(
                    &spec,
                    &self.keys[i],
                    &self.certs[signer].subject,
                    &self.keys[signer],
                )
            })
            .collect()
    }

    pub fn run_chain(&self, chain: &[Vec<u8>]) -> Result<Attested, Error> {
        let spki = self.root_spki();
        let anchors = [TrustAnchor {
            spki: &spki,
            kind: self.root_kind,
        }];
        let policy = Policy {
            anchors: &anchors,
            ..self.policy
        };
        let ctx = Context {
            now_ns: self.now_ns,
            challenge_secret: &self.secret,
            revocations: &self.revocations,
        };
        let late = self.late.as_ref().map(|(n, w)| LateOpening {
            install_nonce: n,
            wall_ms: *w,
        });
        witness_attest::verify(
            &Enrollment {
                chain,
                key_id: self.key_id,
                late,
            },
            &policy,
            &ctx,
        )
    }

    pub fn run(&self) -> Result<Attested, Error> {
        self.run_chain(&self.build())
    }

    pub fn make_late(&mut self, nonce: &[u8], wall_ms: u64) {
        self.kd.challenge = witness_attest::late_challenge(nonce, wall_ms).to_vec();
        self.late = Some((nonce.to_vec(), wall_ms));
    }
}
