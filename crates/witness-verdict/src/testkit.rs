use crate::oracle::{MAX_EXACT, MAX_WATERMARK, Oracle, PdqHit, PdqProbe, Record};
use ed25519_dalek::Signer as _;
use p256::ecdsa::SigningKey as DeviceKey;
use std::collections::BTreeMap;
use witness_c2pa::{
    Action, CertParams, Generator, ManifestSpec, Signer, TrustAnchor, data_hash, seal_jpeg,
};
use witness_fingerprint::{MIH_RADIUS, PdqHash};
use witness_format::crypto::sha256;
use witness_format::{
    BootState, CaptureId, CertSerial, Checks, Confidence, Dims, FingerprintBundle, Freshness,
    GnssChecks, KeyHistory, KeyId, KeyInfo, KeyRef, Kind, LocationEvidence, LocationReasons,
    Provisioning, RecordId, SecurityLevel, TimeEvidence, VerdictFlags, WitnessClaim,
};
use witness_image::Rgb8;
use witness_watermark::{PAYLOAD_BITS, WatermarkError, Watermarker, encode as encode_payload};

pub const NOT_BEFORE_S: u64 = 1_735_689_600;
pub const NOT_AFTER_S: u64 = 2_051_222_400;
pub const NOW_S: u64 = 1_763_510_400;
pub const ENROLLED_NS: u64 = 1_763_000_000_000_000_000;
pub const GENERATOR: Generator<'static> = Generator {
    name: "Witness",
    version: "0.1.0",
};
pub const CAPTURED: &[Action<'static>] = &[Action::CAPTURED];
const SPKI_P256_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

fn params(subject: &str, serial: u64) -> CertParams<'_> {
    CertParams {
        subject,
        serial: CertSerial::from_u64(serial).expect("serial"),
        not_before: NOT_BEFORE_S,
        not_after: NOT_AFTER_S,
    }
}

pub struct TestCa {
    pub root_der: Vec<u8>,
    pub anchor: TrustAnchor,
    root: ed25519_dalek::SigningKey,
}

impl TestCa {
    pub fn new(seed: u8) -> Self {
        let root = ed25519_dalek::SigningKey::from_bytes(&[seed; 32]);
        let root_der = witness_c2pa::build_root(
            &params("CN=Witness Root CA,O=Witness", 1),
            &root.verifying_key().to_bytes(),
            |m| root.sign(m).to_bytes(),
        )
        .expect("root");
        let anchor = TrustAnchor::from_der(&root_der).expect("anchor");
        Self {
            root_der,
            anchor,
            root,
        }
    }

    pub fn device(&self, seed: u8, serial: u64) -> TestDevice {
        let key = DeviceKey::from_slice(&[seed.max(1); 32]).expect("device key");
        let mut spki = SPKI_P256_PREFIX.to_vec();
        spki.extend_from_slice(key.verifying_key().to_encoded_point(false).as_bytes());
        let leaf = witness_c2pa::build_leaf(
            &params("CN=Witness Device Key,O=Witness", serial),
            &spki,
            &self.anchor,
            |m| self.root.sign(m).to_bytes(),
        )
        .expect("leaf");
        TestDevice {
            key,
            spki,
            serial,
            chain: vec![leaf, self.root_der.clone()],
        }
    }
}

#[derive(Clone)]
pub struct TestDevice {
    pub key: DeviceKey,
    pub spki: Vec<u8>,
    pub serial: u64,
    pub chain: Vec<Vec<u8>>,
}

impl Signer for TestDevice {
    fn sign(&self, data: &[u8]) -> Vec<u8> {
        let sig: p256::ecdsa::Signature = self.key.sign(data);
        sig.to_bytes().to_vec()
    }

    fn cert_chain(&self) -> Vec<Vec<u8>> {
        self.chain.clone()
    }
}

impl TestDevice {
    pub fn key_id(&self) -> KeyId {
        KeyId::from_spki(&self.spki)
    }

    pub fn key_info(&self, history: KeyHistory) -> KeyInfo {
        KeyInfo {
            key_id: self.key_id(),
            pubkey: self.spki.clone(),
            security: SecurityLevel::Tee,
            provisioning: Provisioning::Rkp,
            freshness: Freshness::Fresh,
            patch: 202_511,
            enrolled_ns: ENROLLED_NS,
            active_until_ns: ENROLLED_NS + 60 * 86_400 * 1_000_000_000,
            history,
            chain_serials: vec!["1".into()],
        }
    }

    pub fn claim(&self, capture: u64, kind: Kind, counter: u64) -> WitnessClaim {
        WitnessClaim {
            capture_id: CaptureId::new(capture & CaptureId::RANDOM_MASK, 0).expect("capture id"),
            kind,
            asset_hash: [0; 32],
            pixel_hash: [0; 32],
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
                key_id: self.key_id(),
                counter,
            },
            cert_serial: CertSerial::from_u64(self.serial).expect("serial"),
            coarse: None,
        }
    }

    pub fn seal(
        &self,
        anchor: &TrustAnchor,
        jpeg: &[u8],
        mut claim: WitnessClaim,
    ) -> (Vec<u8>, WitnessClaim) {
        claim.asset_hash = data_hash(jpeg).expect("data hash");
        if let Ok(p) = witness_image::pixel_hash(jpeg) {
            claim.pixel_hash = p;
        }
        let cbor = claim.to_cbor().expect("claim cbor");
        let spec = ManifestSpec {
            id: [0x5a; 16],
            generator: GENERATOR,
            actions: CAPTURED,
            witness_claim: &cbor,
        };
        (
            seal_jpeg(jpeg, &spec, self, anchor).expect("seal").jpeg,
            claim,
        )
    }
}

pub struct FixedWatermarker {
    pub capture_id: Option<CaptureId>,
    pub flipped: usize,
}

impl FixedWatermarker {
    pub fn reading(capture_id: CaptureId) -> Self {
        Self {
            capture_id: Some(capture_id),
            flipped: 0,
        }
    }

    pub fn blank() -> Self {
        Self {
            capture_id: None,
            flipped: 0,
        }
    }
}

impl Watermarker for FixedWatermarker {
    fn encode(
        &self,
        cover: &[f32],
        _: &[f32; PAYLOAD_BITS],
        stego: &mut [f32],
    ) -> Result<(), WatermarkError> {
        if cover.len() != stego.len() {
            return Err(WatermarkError::Inference);
        }
        stego.copy_from_slice(cover);
        Ok(())
    }

    fn decode(&self, _: &[f32]) -> Result<[f32; PAYLOAD_BITS], WatermarkError> {
        let Some(id) = self.capture_id else {
            return Ok([0.0; PAYLOAD_BITS]);
        };
        let bits = encode_payload(id);
        Ok(core::array::from_fn(|i| {
            if bits[i] != (i < self.flipped) {
                4.0
            } else {
                -4.0
            }
        }))
    }
}

pub fn photo(width: u32, height: u32, seed: u64) -> Rgb8 {
    let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    let mut next = move || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let min = width.min(height).max(1);
    let base: [u32; 6] = core::array::from_fn(|_| (next() % 160) as u32 + 40);
    let shapes: Vec<(u32, u32, u32, u32, bool, [u8; 3])> = (0..140)
        .map(|_| {
            let v = next();
            let w = min / 40 + (v % u64::from(min / 9 + 1)) as u32;
            let h = min / 40 + ((v >> 16) % u64::from(min / 9 + 1)) as u32;
            let c = next();
            (
                (next() % u64::from(width)) as u32,
                (next() % u64::from(height)) as u32,
                w.max(1),
                h.max(1),
                v >> 60 < 5,
                [c as u8, (c >> 8) as u8, (c >> 16) as u8],
            )
        })
        .collect();
    let noise_seed = next();
    Rgb8::from_fn(width, height, |x, y| {
        let gx = x * 255 / width.max(1);
        let gy = y * 255 / height.max(1);
        let mut p = [
            ((base[0] * (255 - gx) + base[1] * gx) / 255) as u8,
            ((base[2] * (255 - gy) + base[3] * gy) / 255) as u8,
            ((base[4] * (510 - gx - gy) + base[5] * (gx + gy)) / 510) as u8,
        ];
        for &(sx, sy, sw, sh, disc, c) in &shapes {
            let inside = if disc {
                let (dx, dy) = (i64::from(x) - i64::from(sx), i64::from(y) - i64::from(sy));
                let (a, b) = (i64::from(sw), i64::from(sh));
                dx * dx * b * b + dy * dy * a * a <= (a * b).pow(2) / 4
            } else {
                x >= sx && y >= sy && x < sx + sw && y < sy + sh
            };
            if inside {
                p = c;
            }
        }
        let mut h = noise_seed ^ (u64::from(x) << 32 | u64::from(y));
        h = (h ^ (h >> 33)).wrapping_mul(0xFF51_AFD7_ED55_8CCD);
        h ^= h >> 29;
        let n = (h % 9) as i32 - 4;
        p.map(|v| (i32::from(v) + n).clamp(0, 255) as u8)
    })
    .expect("photo")
}

#[derive(Debug, Clone, Default)]
pub struct MemoryOracle {
    keys: BTreeMap<KeyId, KeyInfo>,
    records: Vec<Record>,
    by_id: BTreeMap<RecordId, usize>,
    fingerprints: BTreeMap<u64, FingerprintBundle>,
    references: BTreeMap<u64, Vec<u8>>,
    pub pdq_radius: u32,
    pub truncated: bool,
    pub pending: bool,
}

impl MemoryOracle {
    pub fn new() -> Self {
        Self {
            pdq_radius: MIH_RADIUS,
            ..Self::default()
        }
    }

    pub fn add_key(&mut self, key: KeyInfo) {
        self.keys.insert(key.key_id, key);
    }

    pub fn anchor(&mut self, claim: WitnessClaim, anchored_ns: u64) -> Record {
        let record = Record {
            record_ix: self.records.len() as u64,
            record_id: claim.record_id(),
            flags: crate::oracle::clock_flags(&claim, anchored_ns),
            claim,
            anchored_ns,
        };
        self.by_id.insert(record.record_id, self.records.len());
        self.records.push(record.clone());
        record
    }

    pub fn set_flags(&mut self, record_ix: u64, flags: VerdictFlags) {
        if let Some(r) = self.records.get_mut(record_ix as usize) {
            r.flags = flags;
        }
    }

    pub fn set_fingerprints(&mut self, record_ix: u64, bundle: FingerprintBundle) {
        self.fingerprints.insert(record_ix, bundle);
    }

    pub fn set_reference(&mut self, record_ix: u64, image: Vec<u8>) {
        self.references.insert(record_ix, image);
    }

    pub fn records(&self) -> &[Record] {
        &self.records
    }

    fn earliest(&self, pred: impl Fn(&Record) -> bool, cap: usize) -> Vec<Record> {
        let mut hits: Vec<&Record> = self.records.iter().filter(|r| pred(r)).collect();
        hits.sort_by_key(|r| (r.anchored_ns, r.record_ix));
        hits.into_iter().take(cap).cloned().collect()
    }
}

fn root_capture(claim: &WitnessClaim) -> CaptureId {
    match &claim.kind {
        Kind::Original => claim.capture_id,
        Kind::Derivative { root, .. } => *root,
    }
}

impl Oracle for MemoryOracle {
    fn get_key(&self, key_id: &KeyId) -> Option<KeyInfo> {
        self.keys.get(key_id).cloned()
    }

    fn get_record(&self, record_id: &RecordId) -> Option<Record> {
        self.by_id.get(record_id).map(|&i| self.records[i].clone())
    }

    fn exact(&self, asset_hash: &[u8; 32], pixel_hash: Option<&[u8; 32]>) -> Vec<Record> {
        let mut hits = self.earliest(|r| &r.claim.asset_hash == asset_hash, MAX_EXACT);
        if let Some(p) = pixel_hash {
            hits.extend(self.earliest(|r| &r.claim.pixel_hash == p, MAX_EXACT));
        }
        hits.sort_by_key(|r| r.record_ix);
        hits.dedup_by_key(|r| r.record_ix);
        hits
    }

    fn by_watermark(&self, capture_id: CaptureId) -> Vec<Record> {
        self.earliest(|r| root_capture(&r.claim) == capture_id, MAX_WATERMARK)
    }

    fn pdq_probe(&self, hashes: &[PdqHash]) -> PdqProbe {
        let hits = self
            .fingerprints
            .iter()
            .filter_map(|(&ix, bundle)| {
                let distance = hashes
                    .iter()
                    .flat_map(|q| {
                        bundle
                            .pdq()
                            .iter()
                            .map(move |s| q.distance(&PdqHash(s.hash)))
                    })
                    .min()?;
                (distance <= self.pdq_radius).then(|| PdqHit {
                    record: self.records[ix as usize].clone(),
                    distance,
                })
            })
            .collect();
        PdqProbe {
            hits,
            truncated: self.truncated,
        }
    }

    fn derivatives(&self, root: &Record) -> Vec<Record> {
        self.earliest(
            |r| matches!(&r.claim.kind, Kind::Derivative { parent, .. } if *parent == root.record_id),
            usize::MAX,
        )
    }

    fn reference_image(&self, record: &Record) -> Option<Vec<u8>> {
        self.references.get(&record.record_ix).cloned()
    }

    fn fingerprints(&self, record: &Record) -> Option<FingerprintBundle> {
        self.fingerprints.get(&record.record_ix).cloned()
    }

    fn index_pending(&self) -> bool {
        self.pending
    }
}
