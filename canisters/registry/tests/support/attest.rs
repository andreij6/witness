#![allow(dead_code)]

use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::Signer as _;

pub const AOSP_EC_ROOT: &[u8] =
    include_bytes!("../../../../crates/witness-attest/tests/data/aosp_ec_root.der");
pub const AOSP_EC_INTERMEDIATE: &[u8] =
    include_bytes!("../../../../crates/witness-attest/tests/data/aosp_ec_intermediate.der");
const AOSP_EC_INTERMEDIATE_KEY: &[u8] =
    include_bytes!("../../../../crates/witness-attest/tests/data/aosp_ec_intermediate.key.der");

pub const DEBUG_DIGEST: [u8; 32] = [0xd1; 32];
pub const PATCH: u64 = 202_611;
pub const APP_VERSION: u64 = 42;

const SPKI_P256_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];
const ECDSA_SHA256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];
const KEY_USAGE: &[u8] = &[0x55, 0x1d, 0x0f];
const KEY_DESCRIPTION: &[u8] = &[0x2b, 0x06, 0x01, 0x04, 0x01, 0xd6, 0x79, 0x02, 0x01, 0x11];
const COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03];

pub fn device_key(seed: u64) -> SigningKey {
    let mut s = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    loop {
        let mut bytes = [0u8; 32];
        for b in &mut bytes {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            *b = s as u8;
        }
        if let Ok(k) = SigningKey::from_slice(&bytes) {
            return k;
        }
    }
}

pub fn spki(key: &SigningKey) -> Vec<u8> {
    let mut out = SPKI_P256_PREFIX.to_vec();
    out.extend_from_slice(key.verifying_key().to_encoded_point(false).as_bytes());
    out
}

fn len_bytes(len: usize) -> Vec<u8> {
    if len < 0x80 {
        return vec![len as u8];
    }
    let be = (len as u64).to_be_bytes();
    let start = be.iter().position(|&b| b != 0).unwrap_or(7);
    let mut out = vec![0x80 | (8 - start) as u8];
    out.extend_from_slice(&be[start..]);
    out
}

fn tlv(tag: &[u8], content: &[u8]) -> Vec<u8> {
    let mut out = tag.to_vec();
    out.extend(len_bytes(content.len()));
    out.extend_from_slice(content);
    out
}

fn seq(parts: &[&[u8]]) -> Vec<u8> {
    tlv(&[0x30], &parts.concat())
}

fn set(parts: &[&[u8]]) -> Vec<u8> {
    tlv(&[0x31], &parts.concat())
}

fn context(number: u32, inner: &[u8]) -> Vec<u8> {
    let tag = if number < 31 {
        vec![0xa0 | number as u8]
    } else {
        let mut groups = Vec::new();
        let mut n = number;
        loop {
            groups.push((n & 0x7f) as u8);
            n >>= 7;
            if n == 0 {
                break;
            }
        }
        let mut t = vec![0xbf];
        for (i, g) in groups.iter().enumerate().rev() {
            t.push(if i == 0 { *g } else { g | 0x80 });
        }
        t
    };
    tlv(&tag, inner)
}

fn int(v: u64) -> Vec<u8> {
    let be = v.to_be_bytes();
    let start = be.iter().position(|&b| b != 0).unwrap_or(7);
    let mut c = Vec::new();
    if be[start] & 0x80 != 0 {
        c.push(0);
    }
    c.extend_from_slice(&be[start..]);
    tlv(&[0x02], &c)
}

fn enumerated(v: u8) -> Vec<u8> {
    tlv(&[0x0a], &[v])
}

fn octets(b: &[u8]) -> Vec<u8> {
    tlv(&[0x04], b)
}

fn oid(b: &[u8]) -> Vec<u8> {
    tlv(&[0x06], b)
}

fn bits(b: &[u8]) -> Vec<u8> {
    let mut c = vec![0u8];
    c.extend_from_slice(b);
    tlv(&[0x03], &c)
}

fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn time(unix: i64) -> Vec<u8> {
    let (y, m, d) = civil(unix.div_euclid(86_400));
    let s = unix.rem_euclid(86_400);
    let hms = format!(
        "{m:02}{d:02}{:02}{:02}{:02}Z",
        s / 3600,
        s / 60 % 60,
        s % 60
    );
    if (1950..2050).contains(&y) {
        tlv(&[0x17], format!("{:02}{hms}", y % 100).as_bytes())
    } else {
        tlv(&[0x18], format!("{y:04}{hms}").as_bytes())
    }
}

fn name(cn: &str) -> Vec<u8> {
    seq(&[&set(&[&seq(&[
        &oid(COMMON_NAME),
        &tlv(&[0x0c], cn.as_bytes()),
    ])])])
}

pub struct Attestation {
    pub challenge: Vec<u8>,
    pub created_ms: u64,
    pub package: Vec<u8>,
    pub version: u64,
    pub digest: [u8; 32],
    pub patch: u64,
    pub not_before: i64,
    pub not_after: i64,
    pub serial: u64,
}

impl Attestation {
    pub fn new(challenge: &[u8], now_s: i64) -> Self {
        Self {
            challenge: challenge.to_vec(),
            created_ms: (now_s as u64 - 30) * 1000,
            package: b"app.witness".to_vec(),
            version: APP_VERSION,
            digest: DEBUG_DIGEST,
            patch: PATCH,
            not_before: now_s - 60,
            not_after: now_s + 3650 * 86_400,
            serial: 0x51_0001,
        }
    }

    fn key_description(&self) -> Vec<u8> {
        let app_id = seq(&[
            &set(&[&seq(&[&octets(&self.package), &int(self.version)])]),
            &set(&[&octets(&self.digest)]),
        ]);
        let software = seq(&[
            &context(1, &set(&[&int(2)])),
            &context(2, &int(3)),
            &context(3, &int(256)),
            &context(10, &int(1)),
            &context(701, &int(self.created_ms)),
            &context(702, &int(0)),
            &context(706, &int(self.patch)),
            &context(709, &octets(&app_id)),
        ]);
        seq(&[
            &int(300),
            &enumerated(0),
            &int(300),
            &enumerated(0),
            &octets(&self.challenge),
            &octets(&[]),
            &software,
            &seq(&[]),
        ])
    }

    pub fn leaf(&self, device: &SigningKey) -> Vec<u8> {
        let alg = seq(&[&oid(ECDSA_SHA256)]);
        let exts = context(
            3,
            &seq(&[
                &seq(&[
                    &oid(KEY_USAGE),
                    &tlv(&[0x01], &[0xff]),
                    &octets(&tlv(&[0x03], &[0x07, 0x80])),
                ]),
                &seq(&[&oid(KEY_DESCRIPTION), &octets(&self.key_description())]),
            ]),
        );
        let tbs = seq(&[
            &context(0, &int(2)),
            &int(self.serial),
            &alg,
            &name("Droid CA2"),
            &seq(&[&time(self.not_before), &time(self.not_after)]),
            &name("Android Keystore Key"),
            &spki(device),
            &exts,
        ]);
        let issuer = SigningKey::from_slice(&AOSP_EC_INTERMEDIATE_KEY[7..39]).expect("aosp key");
        let sig: p256::ecdsa::Signature = issuer.sign(&tbs);
        seq(&[&tbs, &alg, &bits(sig.to_der().as_bytes())])
    }

    pub fn chain(&self, device: &SigningKey) -> Vec<Vec<u8>> {
        vec![
            self.leaf(device),
            AOSP_EC_INTERMEDIATE.to_vec(),
            AOSP_EC_ROOT.to_vec(),
        ]
    }
}
