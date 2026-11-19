#![allow(dead_code)]

extern crate std;

use ed25519_dalek::Signer as _;
use p256::ecdsa::SigningKey as P256Key;
use std::vec;
use std::vec::Vec;
use witness_c2pa::{Action, CertParams, Generator, ManifestSpec, Signer, TrustAnchor};
use witness_format::crypto::sha256;
use witness_format::*;

pub const NOT_BEFORE: u64 = 1_735_689_600;
pub const NOT_AFTER: u64 = 2_051_222_400;
pub const NOW: u64 = 1_763_510_400;
pub const LEAF_SERIAL: u64 = 0x01_0203_0405;
pub const GENERATOR: Generator<'static> = Generator {
    name: "Witness",
    version: "0.1.0",
};
pub const ACTIONS: &[Action<'static>] = &[Action::CAPTURED];

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

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
}

pub fn root_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[7; 32])
}

pub fn other_root_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[8; 32])
}

pub fn device_key() -> P256Key {
    P256Key::from_slice(&[0x42; 32]).unwrap()
}

pub const SPKI_P256_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

pub fn spki(key: &P256Key) -> Vec<u8> {
    let mut v = SPKI_P256_PREFIX.to_vec();
    v.extend_from_slice(key.verifying_key().to_encoded_point(false).as_bytes());
    v
}

pub fn root_params() -> CertParams<'static> {
    CertParams {
        subject: "CN=Witness Root CA,O=Witness",
        serial: CertSerial::from_u64(1).unwrap(),
        not_before: NOT_BEFORE,
        not_after: NOT_AFTER,
    }
}

pub fn leaf_params() -> CertParams<'static> {
    CertParams {
        subject: "CN=Witness Device Key,O=Witness",
        serial: CertSerial::from_u64(LEAF_SERIAL).unwrap(),
        not_before: NOT_BEFORE,
        not_after: NOT_AFTER,
    }
}

pub fn root_der_with(key: &ed25519_dalek::SigningKey) -> Vec<u8> {
    witness_c2pa::build_root(&root_params(), &key.verifying_key().to_bytes(), |m| {
        key.sign(m).to_bytes()
    })
    .unwrap()
}

pub fn root_der() -> Vec<u8> {
    root_der_with(&root_key())
}

pub fn anchor() -> TrustAnchor {
    TrustAnchor::from_der(&root_der()).unwrap()
}

pub fn leaf_der_for(key: &P256Key, params: &CertParams<'_>) -> Vec<u8> {
    let root = root_key();
    witness_c2pa::build_leaf(params, &spki(key), &anchor(), |m| root.sign(m).to_bytes()).unwrap()
}

pub fn leaf_der() -> Vec<u8> {
    leaf_der_for(&device_key(), &leaf_params())
}

#[derive(Clone)]
pub struct TestSigner {
    pub key: P256Key,
    pub chain: Vec<Vec<u8>>,
    pub der: bool,
}

impl TestSigner {
    pub fn new(der: bool) -> Self {
        Self {
            key: device_key(),
            chain: vec![leaf_der(), root_der()],
            der,
        }
    }
}

impl Signer for TestSigner {
    fn sign(&self, data: &[u8]) -> Vec<u8> {
        let sig: p256::ecdsa::Signature = self.key.sign(data);
        if self.der {
            sig.to_der().as_bytes().to_vec()
        } else {
            sig.to_bytes().to_vec()
        }
    }

    fn cert_chain(&self) -> Vec<Vec<u8>> {
        self.chain.clone()
    }
}

pub fn claim_for(asset_hash: [u8; 32], spki_der: &[u8], serial: u64) -> WitnessClaim {
    WitnessClaim {
        capture_id: CaptureId::from_random_bytes([9, 8, 7, 6, 5, 4, 3], 0).unwrap(),
        kind: Kind::Original,
        asset_hash,
        pixel_hash: sha256(b"decoded rgb8"),
        fp_commit: sha256(b"fingerprints"),
        dims: Dims {
            width: 64,
            height: 48,
        },
        time: TimeEvidence {
            wall_ms: 1_763_510_400_000,
            gnss_ms: None,
            elapsed_ns: 77,
            boot_count: 12,
        },
        loc: LocationEvidence {
            commit: None,
            confidence: Confidence::Low,
            reasons: LocationReasons::EMPTY,
        },
        checks: Checks {
            key_security: SecurityLevel::Tee,
            boot: BootState::Verified,
            device_locked: true,
            mock_location: false,
            gnss: GnssChecks::EMPTY,
            recapture: 0,
        },
        key: KeyRef {
            key_id: KeyId::from_spki(spki_der),
            counter: 1,
        },
        cert_serial: CertSerial::from_u64(serial).unwrap(),
        coarse: None,
    }
}

pub fn claim_cbor_for(jpeg: &[u8]) -> Vec<u8> {
    let h = witness_c2pa::data_hash(jpeg).unwrap();
    claim_for(h, &spki(&device_key()), LEAF_SERIAL)
        .to_cbor()
        .unwrap()
}

pub fn spec<'a>(claim: &'a [u8]) -> ManifestSpec<'a> {
    ManifestSpec {
        id: [0x5a; 16],
        generator: GENERATOR,
        actions: ACTIONS,
        witness_claim: claim,
    }
}

fn segment(out: &mut Vec<u8>, marker: u8, contents: &[u8]) {
    out.extend_from_slice(&[0xff, marker]);
    out.extend_from_slice(&((contents.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(contents);
}

pub fn jpeg_with(rng: &mut Rng, extra_app: &[(u8, Vec<u8>)], entropy_len: usize) -> Vec<u8> {
    let mut out = vec![0xff, 0xd8];
    segment(&mut out, 0xe0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0");
    for (m, c) in extra_app {
        segment(&mut out, *m, c);
    }
    segment(&mut out, 0xdb, &rng.bytes(65));
    segment(&mut out, 0xc0, &[8, 0, 48, 0, 64, 1, 1, 0x11, 0]);
    segment(&mut out, 0xc4, &rng.bytes(30));
    segment(&mut out, 0xda, &[1, 1, 0, 0, 63, 0]);
    for i in 0..entropy_len {
        let b = rng.next() as u8;
        out.push(b);
        if b == 0xff {
            out.push(0x00);
        }
        if i % 97 == 96 {
            out.extend_from_slice(&[0xff, 0xd0 + (i / 97 % 8) as u8]);
        }
    }
    out.extend_from_slice(&[0xff, 0xd9]);
    out
}

pub fn jpeg(seed: u64) -> Vec<u8> {
    jpeg_with(&mut Rng::new(seed), &[], 512)
}

pub fn sealed(jpeg: &[u8]) -> witness_c2pa::Sealed {
    let claim = claim_cbor_for(jpeg);
    witness_c2pa::seal_jpeg(jpeg, &spec(&claim), &TestSigner::new(true), &anchor()).unwrap()
}
