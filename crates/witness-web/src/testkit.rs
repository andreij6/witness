use crate::case::{NONCE_BYTES, PLAIN_CHUNK, aad};
use crate::check::{Step, step};
use crate::oracle::{Answers, Need};
use crate::proof::{SHARDS_KEY, shard_map_hash};
use crate::wire::{
    B64, CertifiedKeyWire, CheckOptions, Dec, Fetched, FingerprintsWire, Hex, KeyWire, PdqHitWire,
    PdqWire, RecordWire, ShardInfoWire, ShardsWire,
};
use aes_gcm::aead::AeadInPlace;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use witness_core::fingerprint::PdqHash;
use witness_core::format::crypto::sha256;
use witness_core::format::{CaseId, KeyHistory, Kind, RecordId};
use witness_core::image::{Orientation, decode, encode_jpeg};
use witness_core::merkle::key_of;
use witness_core::verdict::{Oracle, Record};
use witness_core::{Capture, ClaimTemplate, Manifest, SEAL_QUALITY, SealedCapture, seal};
pub use witness_proof::testkit::{
    TestLedger, TestSubnet, delegated_certificate, delegation_tree, leb128, ranges_cbor, state_tree,
};
use witness_verdict::testkit::{
    CAPTURED, ENROLLED_NS, GENERATOR, MemoryOracle, TestCa, TestDevice, photo,
};

pub const MAX_ROUNDS: usize = 12;
pub const W: u32 = 480;
pub const H: u32 = 360;
const MANIFEST: Manifest<'static> = Manifest {
    id: [0x31; 16],
    generator: GENERATOR,
    actions: CAPTURED,
};

fn record_at(oracle: &MemoryOracle, ix: u64) -> Option<&Record> {
    usize::try_from(ix)
        .ok()
        .and_then(|i| oracle.records().get(i))
}

pub fn record_wire(record: &Record) -> RecordWire {
    RecordWire::from_parts(record, record.claim.to_cbor().expect("claim cbor"))
}

fn ixs(records: Vec<Record>) -> Vec<Dec> {
    records.iter().map(|r| Dec(r.record_ix)).collect()
}

pub fn answer(need: &Need, oracle: &MemoryOracle, f: &mut Fetched) {
    for k in &need.keys {
        let id = k.0.as_slice().try_into().expect("key id");
        f.keys.insert(
            k.clone(),
            oracle.get_key(&id).map(|i| KeyWire::from_info(&i)),
        );
    }
    for id in &need.records {
        let rid = RecordId::parse(id).expect("record id");
        f.records.insert(
            id.clone(),
            oracle.get_record(&rid).as_ref().map(record_wire),
        );
    }
    for ix in &need.record_ixs {
        match record_at(oracle, ix.0) {
            Some(r) => {
                f.records
                    .insert(r.record_id.to_string(), Some(record_wire(r)));
            }
            None => f.absent_ixs.push(*ix),
        }
    }
    if let Some(e) = &need.exact {
        let asset = e.asset.array("asset").expect("asset");
        let pixel = e.pixel.as_ref().map(|p| p.array("pixel").expect("pixel"));
        f.exact = Some(ixs(oracle.exact(&asset, pixel.as_ref())));
    }
    for cid in &need.watermark {
        let capture = witness_core::format::CaptureId::from_u64(cid.0).expect("capture id");
        f.watermark.insert(*cid, ixs(oracle.by_watermark(capture)));
    }
    if let Some(hashes) = &need.pdq {
        let hashes: Vec<PdqHash> = hashes
            .iter()
            .map(|h| PdqHash(h.array("pdq").expect("pdq")))
            .collect();
        let probe = oracle.pdq_probe(&hashes);
        f.pdq = Some(PdqWire {
            hits: probe
                .hits
                .iter()
                .map(|h| PdqHitWire {
                    ix: Dec(h.record.record_ix),
                    distance: h.distance,
                })
                .collect(),
            truncated: probe.truncated,
        });
    }
    for root in &need.derivatives {
        let found = record_at(oracle, root.0)
            .map(|r| ixs(oracle.derivatives(r)))
            .unwrap_or_default();
        f.derivatives.insert(*root, found);
    }
    for ix in &need.fingerprints {
        let bundle = record_at(oracle, ix.0).and_then(|r| oracle.fingerprints(r));
        f.fingerprints
            .insert(*ix, bundle.as_ref().map(FingerprintsWire::from_bundle));
    }
    for id in &need.references {
        let rid = RecordId::parse(id).expect("record id");
        let image = oracle
            .get_record(&rid)
            .and_then(|r| oracle.reference_image(&r));
        f.references.insert(id.clone(), image.map(B64));
    }
    f.index_pending = oracle.index_pending();
}

pub fn replay(
    input: &[u8],
    options: &CheckOptions,
    oracle: &MemoryOracle,
) -> (Step, usize, Fetched) {
    let mut fetched = Fetched::default();
    for round in 1..=MAX_ROUNDS {
        let answers = Answers::from_fetched(fetched.clone()).expect("answers");
        let s = step(input, options, &answers).expect("step");
        if s.done() {
            return (s, round, fetched);
        }
        answer(&s.need, oracle, &mut fetched);
    }
    panic!("replay did not converge in {MAX_ROUNDS} rounds");
}

pub fn jpeg_of(seed: u64, q: u8) -> Vec<u8> {
    encode_jpeg(&photo(W, H, seed), q).expect("jpeg")
}

pub fn recompress(bytes: &[u8], q: u8) -> Vec<u8> {
    encode_jpeg(&decode(bytes).expect("decode"), q).expect("jpeg")
}

pub struct World {
    pub ca: TestCa,
    pub dev: TestDevice,
    pub oracle: MemoryOracle,
    counter: u64,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self::new_with_seed(41)
    }

    pub fn new_with_seed(seed: u8) -> Self {
        let ca = TestCa::new(seed);
        let dev = ca.device(0x44, 0x0B0C);
        let mut oracle = MemoryOracle::new();
        oracle.add_key(dev.key_info(KeyHistory::new(ENROLLED_NS)));
        Self {
            ca,
            dev,
            oracle,
            counter: 0,
        }
    }

    pub fn options(&self) -> CheckOptions {
        CheckOptions {
            anchor: Hex(self.ca.root_der.clone()),
            ..CheckOptions::default()
        }
    }

    pub fn seal(&mut self, image: &[u8], kind: Kind) -> SealedCapture {
        self.counter += 1;
        let c = self.dev.claim(0x5EED + self.counter, kind, self.counter);
        let template = ClaimTemplate {
            capture_id: c.capture_id,
            kind: c.kind,
            time: c.time,
            loc: c.loc,
            checks: c.checks,
            key: c.key,
            cert_serial: c.cert_serial,
            coarse: c.coarse,
        };
        let capture = Capture {
            image,
            orientation: Orientation::Normal,
            quality: SEAL_QUALITY,
            mark: None,
        };
        seal(&capture, template, &MANIFEST, &self.dev, &self.ca.anchor).expect("seal")
    }

    pub fn anchor(&mut self, s: &SealedCapture, reference: bool) -> Record {
        let record = self
            .oracle
            .anchor(s.claim.clone(), ENROLLED_NS + 1_000 * self.counter);
        self.oracle
            .set_fingerprints(record.record_ix, s.fingerprints.clone());
        if reference {
            self.oracle
                .set_reference(record.record_ix, recompress(&s.jpeg, 85));
        }
        record
    }

    pub fn original(&mut self, seed: u64, reference: bool) -> SealedCapture {
        let s = self.seal(&jpeg_of(seed, 95), Kind::Original);
        self.anchor(&s, reference);
        s
    }
}

pub fn seal_case(key: &[u8; 32], case_id: &CaseId, bundle: &[u8], nonce_seed: u8) -> Vec<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.into());
    let hash = sha256(bundle);
    let parts: Vec<&[u8]> = if bundle.is_empty() {
        vec![&[]]
    } else {
        bundle.chunks(PLAIN_CHUNK).collect()
    };
    let count = parts.len() as u32;
    parts
        .iter()
        .enumerate()
        .map(|(ix, plain)| {
            let ix = ix as u32;
            let nonce = [nonce_seed.wrapping_add(ix as u8); NONCE_BYTES];
            let mut body = plain.to_vec();
            let tag = cipher
                .encrypt_in_place_detached(
                    Nonce::from_slice(&nonce),
                    &aad(case_id, ix, count, &hash),
                    &mut body,
                )
                .expect("seal chunk");
            let mut out = nonce.to_vec();
            out.extend_from_slice(&body);
            out.extend_from_slice(&tag);
            out
        })
        .collect()
}

pub const LEDGER_CANISTER: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 7, 1, 1];
pub const LEDGER_NOW: u64 = 1_763_510_400_000_000_000;

pub struct Ledger {
    pub records: Vec<RecordWire>,
    pub key: CertifiedKeyWire,
    pub shards: ShardsWire,
    pub subnet: TestSubnet,
}

pub fn shard_list() -> (Vec<ShardInfoWire>, Vec<ShardInfoWire>) {
    let info = |shard, byte, open| ShardInfoWire {
        shard,
        canister: Hex(vec![byte; 10]),
        open,
    };
    (
        vec![info(0, 1, true), info(1, 2, false)],
        vec![info(0, 3, true)],
    )
}

pub fn ledger(n: usize) -> Ledger {
    let mut w = World::new();
    let mut t = TestLedger::new(TestSubnet::new(11), &LEDGER_CANISTER, LEDGER_NOW);
    let mut records: Vec<RecordWire> = (0..n)
        .map(|i| {
            let s = w.seal(&jpeg_of(i as u64 + 1, 90), Kind::Original);
            let r = RecordWire {
                record_ix: Dec(i as u64),
                record_id: s.record_id.to_string(),
                claim: Hex(s.submission.claim_cbor.clone()),
                c2pa_claim: Hex(s.submission.c2pa_claim_cbor.clone()),
                cose: Hex(s.submission.cose.clone()),
                anchored_ns: Dec(ENROLLED_NS + i as u64),
                ..RecordWire::default()
            };
            t.append(&r.claim.0, &r.c2pa_claim.0, &r.cose.0, r.anchored_ns.0);
            r
        })
        .collect();
    let key = KeyWire::from_info(&w.dev.key_info(KeyHistory::new(ENROLLED_NS)));
    let info = key.to_info().expect("key");
    let (index, media) = shard_list();
    let mut shards = ShardsWire {
        index,
        media,
        bundle: Hex::default(),
    };
    t.insert(key_of(&info.key_id), &info.leaf_hash());
    t.insert(SHARDS_KEY, &shard_map_hash(&shards).expect("shard hash"));
    let cert = t.certificate();
    for r in &mut records {
        r.mmr_proof = t
            .record_proof(r.record_ix.0)
            .into_iter()
            .map(|h| Hex(h.into_vec()))
            .collect();
        r.leaf_count = Dec(t.leaf_count());
        r.certificate = Hex(cert.clone());
    }
    shards.bundle = Hex(t.shards_bundle(SHARDS_KEY));
    Ledger {
        records,
        key: CertifiedKeyWire {
            key,
            smt_proof: Hex(t.smt_bundle(key_of(&info.key_id))),
            certificate: Hex(cert),
        },
        shards,
        subnet: t.subnet,
    }
}
