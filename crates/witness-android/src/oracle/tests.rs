use super::*;
use crate::client::tests::{index, media, registry, reply, shard};
use crate::seal::SealedCapture;
use crate::seal::tests::sealed_original;
use crate::transport::mock::Mock;
use candid::{Principal, decode_args, decode_one, encode_args};
use serde_bytes::ByteBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use witness_canister_common::{CertifiedKey, Fingerprints, PdqHit as WirePdqHit, ProbeResult};
use witness_core::format::{KeyHistory, Kind, VerdictFlags, cbor};
use witness_core::merkle::key_of;
use witness_core::verdict::testkit::{ENROLLED_NS, TestCa, TestDevice};
use witness_proof::DEFAULT_MAX_AGE_NS;
use witness_proof::testkit::{TestLedger, TestSubnet};

pub(crate) const ANCHORED_NS: u64 = 1_763_510_401_000_000_000;
pub(crate) const CERT_NS: u64 = ANCHORED_NS + 2_000_000_000;
const SUBNET_SEED: u64 = 77;

pub(crate) fn root_key() -> Vec<u8> {
    TestSubnet::new(SUBNET_SEED).public_der()
}

pub(crate) fn cert_now() -> u64 {
    CERT_NS
}

pub(crate) struct World {
    pub ca: TestCa,
    pub dev: TestDevice,
    pub sealed: SealedCapture,
    pub ledger: TestLedger,
}

impl World {
    pub(crate) fn new(seed: u64) -> Self {
        let (ca, dev, sealed) = sealed_original(seed);
        let mut ledger =
            TestLedger::new(TestSubnet::new(SUBNET_SEED), registry().as_slice(), CERT_NS);
        let s = &sealed.submission;
        ledger.append(&s.claim, &s.c2pa_claim, &s.cose, ANCHORED_NS);
        let info = dev.key_info(KeyHistory::new(ENROLLED_NS));
        ledger.insert(key_of(&info.key_id), &info.leaf_hash());
        Self {
            ca,
            dev,
            sealed,
            ledger,
        }
    }

    pub(crate) fn record(&self) -> CertifiedRecord {
        let s = &self.sealed.submission;
        CertifiedRecord {
            record_ix: 0,
            record_id: self.sealed.record_id.to_string(),
            claim: s.claim.clone(),
            c2pa_claim: s.c2pa_claim.clone(),
            cose: s.cose.clone(),
            anchored_ns: ANCHORED_NS,
            mmr_proof: self.ledger.record_proof(0),
            leaf_count: self.ledger.leaf_count(),
            certificate: self.ledger.certificate(),
            media_shard: 0,
        }
    }

    pub(crate) fn key(&self) -> CertifiedKey {
        CertifiedKey {
            key: (&self.dev.key_info(KeyHistory::new(ENROLLED_NS))).into(),
            smt_proof: self.ledger.smt_bundle(key_of(&self.dev.key_id())),
            certificate: self.ledger.certificate(),
        }
    }

    pub(crate) fn bundle(&self) -> FingerprintBundle {
        let fp = self.sealed.submission.fingerprints.clone().unwrap();
        cbor::from_slice_canonical(&fp).unwrap()
    }

    pub(crate) fn shards() -> Shards {
        Shards {
            index: vec![shard(0, index(0))],
            media: vec![shard(0, media(0))],
        }
    }

    pub(crate) fn mock(&self, probe: ProbeResult) -> Mock {
        self.mock_serving(probe, self.record())
    }

    pub(crate) fn mock_serving(&self, probe: ProbeResult, record: CertifiedRecord) -> Mock {
        let by_ix = record.clone();
        let key = self.key();
        let key_id = self.dev.key_id();
        let fingerprints = Fingerprints::from(&self.bundle());
        let ca_root = self.ca.root_der.clone();
        Mock::default()
            .with_root_key(root_key())
            .on(registry(), "get", move |a| {
                let id: String = decode_one(a).unwrap();
                reply((id == record.record_id).then(|| record.clone()))
            })
            .on(registry(), "get_ix", move |a| {
                let ix: u64 = decode_one(a).unwrap();
                reply((ix == 0).then(|| by_ix.clone()))
            })
            .on(registry(), "get_key", move |a| {
                let id: ByteBuf = decode_one(a).unwrap();
                reply((id.as_slice() == key_id.as_bytes()).then(|| key.clone()))
            })
            .on(registry(), "shards", |_| {
                Ok(encode_args((World::shards(), ByteBuf::from(vec![0]))).unwrap())
            })
            .on(registry(), "ca_root", move |_| {
                reply(Some(ByteBuf::from(ca_root.clone())))
            })
            .on(index(0), "probe", move |_| reply(probe.clone()))
            .on(index(0), "fingerprints", move |a| {
                let ix: u64 = decode_one(a).unwrap();
                reply((ix == 0).then(|| fingerprints.clone()))
            })
            .on(index(0), "derivatives", |_| reply(vec![0u64]))
            .on(media(0), "http_request", |_| {
                reply(crate::client::HttpResponse {
                    status_code: 200,
                    body: b"preview".to_vec(),
                })
            })
    }
}

pub(crate) fn probe(pdq: &[(u64, u32)], wm: &[u64], exact: &[u64]) -> ProbeResult {
    ProbeResult {
        pdq: pdq
            .iter()
            .map(|&(record_ix, distance)| WirePdqHit {
                record_ix,
                distance,
            })
            .collect(),
        watermark: wm.to_vec(),
        exact: exact.to_vec(),
        truncated_candidates: false,
        indexed_upto: 1,
    }
}

struct Fixture {
    rt: Runtime,
    client: IcpClient,
    shards: Shards,
    mock: Arc<Mock>,
    root_key: Vec<u8>,
    registry: Principal,
}

impl Fixture {
    fn new(mock: Mock) -> Self {
        let mock = Arc::new(mock);
        Self {
            rt: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
            client: IcpClient::new(mock.clone(), registry()),
            shards: World::shards(),
            mock,
            root_key: root_key(),
            registry: registry(),
        }
    }

    fn oracle(&self) -> NetOracle<'_> {
        self.oracle_with(&self.root_key, CERT_NS)
    }

    fn oracle_with<'a>(&'a self, root_key: &'a [u8], now_ns: u64) -> NetOracle<'a> {
        NetOracle::new(
            self.rt.handle(),
            &self.client,
            &self.shards,
            Trust {
                root_key,
                canister: self.registry.as_slice(),
                now_ns,
                max_age_ns: DEFAULT_MAX_AGE_NS,
            },
        )
    }
}

#[test]
fn to_record_accepts_consistent_records() {
    let w = World::new(1);
    let r = to_record(w.record()).unwrap();
    assert_eq!(r.record_id, w.sealed.record_id);
    assert_eq!(r.anchored_ns, ANCHORED_NS);
    assert_eq!(r.flags, VerdictFlags::EMPTY);
    assert_eq!(r.claim.key.key_id, w.dev.key_id());
    let mut late = w.record();
    late.anchored_ns = r.claim.time.wall_ms * 1_000_000 + 25 * 3_600_000_000_000;
    assert_eq!(to_record(late).unwrap().flags, VerdictFlags::LATE_ANCHOR);
}

#[test]
fn to_record_rejects_inconsistent_records() {
    let w = World::new(1);
    let mut bad_id = w.record();
    bad_id.record_id = "nope".into();
    assert!(to_record(bad_id).is_none());
    let mut other_id = w.record();
    other_id.record_id = RecordId::from_claim_cbor(b"other").to_string();
    assert!(to_record(other_id).is_none());
    let mut bad_claim = w.record();
    bad_claim.claim = vec![0xa0];
    bad_claim.record_id = RecordId::from_claim_cbor(&bad_claim.claim).to_string();
    assert!(to_record(bad_claim).is_none());
}

#[test]
fn to_bundle_round_trips_index_fingerprints() {
    let w = World::new(2);
    let b = w.bundle();
    let got = to_bundle(Fingerprints::from(&b), b.portrait()).unwrap();
    assert_eq!(got.pdq(), b.pdq());
    assert_eq!(got.grid(), b.grid());
    assert_eq!(got.portrait(), b.portrait());
    assert_eq!(got.quality(), 0);
}

#[test]
fn to_bundle_rejects_malformed_fingerprints() {
    let b = World::new(2).bundle();
    let mut short_grid = Fingerprints::from(&b);
    short_grid.grid.pop();
    assert!(to_bundle(short_grid, false).is_none());
    let mut bad_hash = Fingerprints::from(&b);
    bad_hash.shapes[0].1 = ByteBuf::from(vec![0; 31]);
    assert!(to_bundle(bad_hash, false).is_none());
    let mut dup = Fingerprints::from(&b);
    let first = dup.shapes[0].clone();
    dup.shapes.push(first);
    assert!(to_bundle(dup, false).is_none());
}

#[test]
fn get_key_and_record_come_from_registry() {
    let w = World::new(3);
    let f = Fixture::new(w.mock(probe(&[], &[], &[])));
    let o = f.oracle();
    let key = o.get_key(&w.dev.key_id()).unwrap();
    assert_eq!(key.pubkey, w.dev.spki);
    assert!(o.get_key(&KeyId::from_bytes([9; 16])).is_none());
    let r = o.get_record(&w.sealed.record_id).unwrap();
    assert_eq!(r.record_ix, 0);
    assert!(o.get_record(&RecordId::from_claim_cbor(b"x")).is_none());
    assert!(o.into_failure().is_none());
}

#[test]
fn get_key_rejects_a_different_key() {
    let w = World::new(4);
    let other = w.ca.device(0x55, 9);
    let key = CertifiedKey {
        key: (&other.key_info(KeyHistory::new(ENROLLED_NS))).into(),
        smt_proof: Vec::new(),
        certificate: Vec::new(),
    };
    let f =
        Fixture::new(Mock::default().on(registry(), "get_key", move |_| reply(Some(key.clone()))));
    let o = f.oracle();
    assert!(o.get_key(&w.dev.key_id()).is_none());
    assert!(o.rejected());
    assert!(o.into_failure().is_none());
}

#[test]
fn honest_reads_are_verified_without_raising_the_flag() {
    let w = World::new(12);
    let f = Fixture::new(w.mock(probe(&[(0, 1)], &[0], &[0])));
    let o = f.oracle();
    assert!(o.get_key(&w.dev.key_id()).is_some());
    assert!(o.get_record(&w.sealed.record_id).is_some());
    assert_eq!(o.pdq_probe(&[PdqHash([3; 32])]).hits.len(), 1);
    assert!(!o.rejected());
    assert!(o.into_failure().is_none());
}

fn rejected_record(w: &World, tamper: impl FnOnce(&mut CertifiedRecord)) -> bool {
    let mut r = w.record();
    tamper(&mut r);
    let f = Fixture::new(w.mock_serving(probe(&[], &[], &[]), r));
    let o = f.oracle();
    let gone = o.get_record(&w.sealed.record_id).is_none() && o.exact(&[0; 32], None).is_empty();
    gone && o.rejected() && o.into_failure().is_none()
}

#[test]
fn tampered_records_read_as_absent_and_raise_the_flag() {
    let w = World::new(13);
    let f = Fixture::new(w.mock_serving(probe(&[], &[], &[0]), w.record()));
    assert_eq!(f.oracle().exact(&[0; 32], None).len(), 1);
    assert!(rejected_record(&w, |r| r.anchored_ns += 1));
    assert!(rejected_record(&w, |r| r.cose.push(0)));
    assert!(rejected_record(&w, |r| r.claim[3] ^= 1));
    assert!(rejected_record(&w, |r| r.mmr_proof[0][0] ^= 1));
    assert!(rejected_record(&w, |r| r.leaf_count += 1));
    assert!(rejected_record(&w, |r| {
        let n = r.certificate.len();
        r.certificate[n - 1] ^= 1;
    }));
    assert!(rejected_record(&w, |r| r.certificate.clear()));
}

#[test]
fn records_must_sit_at_the_requested_index() {
    let w = World::new(14);
    let r = w.record();
    let f = Fixture::new(
        w.mock(probe(&[], &[], &[]))
            .on(registry(), "get_ix", move |_| reply(Some(r.clone()))),
    );
    let o = f.oracle();
    assert!(o.pdq_probe(&[PdqHash([0; 32])]).hits.is_empty());
    assert!(!o.rejected());
    let p = probe(&[(5, 1)], &[], &[]);
    let r = w.record();
    let f = Fixture::new(
        w.mock(p)
            .on(registry(), "get_ix", move |_| reply(Some(r.clone()))),
    );
    let o = f.oracle();
    assert!(o.pdq_probe(&[PdqHash([0; 32])]).hits.is_empty());
    assert!(o.rejected());
}

#[test]
fn a_record_served_for_another_id_is_rejected() {
    let w = World::new(15);
    let r = w.record();
    let f = Fixture::new(
        w.mock(probe(&[], &[], &[]))
            .on(registry(), "get", move |_| reply(Some(r.clone()))),
    );
    let o = f.oracle();
    assert!(o.get_record(&RecordId::from_claim_cbor(b"x")).is_none());
    assert!(o.rejected());
}

#[test]
fn stale_or_foreign_certificates_are_rejected() {
    let w = World::new(16);
    let f = Fixture::new(w.mock(probe(&[], &[], &[])));
    let late = f.oracle_with(&f.root_key, CERT_NS + DEFAULT_MAX_AGE_NS + 1);
    assert!(late.get_record(&w.sealed.record_id).is_none());
    assert!(late.get_key(&w.dev.key_id()).is_none());
    assert!(late.rejected());
    let early = f.oracle_with(&f.root_key, CERT_NS - DEFAULT_MAX_AGE_NS);
    assert!(early.get_record(&w.sealed.record_id).is_some());
    assert!(!early.rejected());
    let other = TestSubnet::new(SUBNET_SEED + 1).public_der();
    let foreign = f.oracle_with(&other, CERT_NS);
    assert!(foreign.get_record(&w.sealed.record_id).is_none());
    assert!(foreign.rejected());
    let ic = f.oracle_with(&witness_proof::IC_ROOT_KEY_DER, CERT_NS);
    assert!(ic.get_key(&w.dev.key_id()).is_none());
    assert!(ic.rejected());
}

#[test]
fn tampered_keys_read_as_absent_and_raise_the_flag() {
    let w = World::new(17);
    for tamper in [
        |k: &mut CertifiedKey| k.key.patch += 1,
        |k: &mut CertifiedKey| k.smt_proof[0] ^= 1,
        |k: &mut CertifiedKey| k.certificate.truncate(8),
        |k: &mut CertifiedKey| k.key.chain_serials.push("ab".into()),
    ] {
        let mut key = w.key();
        tamper(&mut key);
        let f = Fixture::new(
            Mock::default().on(registry(), "get_key", move |_| reply(Some(key.clone()))),
        );
        let o = f.oracle();
        assert!(o.get_key(&w.dev.key_id()).is_none());
        assert!(o.rejected());
        assert!(o.into_failure().is_none());
    }
}

#[test]
fn exact_sends_both_hashes_and_returns_records() {
    let w = World::new(5);
    let f = Fixture::new(w.mock(probe(&[], &[], &[0, 3])));
    let claim = &w.sealed.claim;
    let asset: [u8; 32] = claim.asset_hash.clone().try_into().unwrap();
    let pixel: [u8; 32] = claim.pixel_hash.clone().try_into().unwrap();
    let got = f.oracle().exact(&asset, Some(&pixel));
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].record_id, w.sealed.record_id);
    let sent = f
        .mock
        .calls()
        .into_iter()
        .find(|c| c.method == "probe")
        .unwrap();
    let (pdq, wm, exact): (Vec<ByteBuf>, Vec<u64>, Vec<ByteBuf>) = decode_args(&sent.arg).unwrap();
    assert!(pdq.is_empty() && wm.is_empty());
    assert_eq!(
        exact,
        vec![ByteBuf::from(asset.to_vec()), ByteBuf::from(pixel.to_vec())]
    );
}

#[test]
fn by_watermark_probes_capture_id() {
    let w = World::new(6);
    let f = Fixture::new(w.mock(probe(&[], &[0], &[])));
    let got = f.oracle().by_watermark(w.sealed.claim.capture_id);
    assert_eq!(got.len(), 1);
    let sent = f
        .mock
        .calls()
        .into_iter()
        .find(|c| c.method == "probe")
        .unwrap();
    let (_, wm, _): (Vec<ByteBuf>, Vec<u64>, Vec<ByteBuf>) = decode_args(&sent.arg).unwrap();
    assert_eq!(wm, vec![w.sealed.claim.capture_id.to_u64()]);
}

#[test]
fn pdq_probe_pairs_hits_with_records_and_drops_missing() {
    let w = World::new(7);
    let mut p = probe(&[(0, 12), (5, 3)], &[], &[]);
    p.truncated_candidates = true;
    let f = Fixture::new(w.mock(p));
    let got = f.oracle().pdq_probe(&[PdqHash([1; 32])]);
    assert_eq!(got.hits.len(), 1);
    assert_eq!(got.hits[0].distance, 12);
    assert_eq!(got.hits[0].record.record_ix, 0);
    assert!(got.truncated);
}

#[test]
fn derivatives_and_fingerprints_use_the_index() {
    let w = World::new(8);
    let f = Fixture::new(w.mock(probe(&[], &[], &[])));
    let o = f.oracle();
    let root = to_record(w.record()).unwrap();
    assert_eq!(o.derivatives(&root).len(), 1);
    let b = o.fingerprints(&root).unwrap();
    assert_eq!(b.pdq(), w.bundle().pdq());
    assert_eq!(b.grid(), w.bundle().grid());
}

#[test]
fn reference_image_only_for_derivatives() {
    let w = World::new(9);
    let f = Fixture::new(w.mock(probe(&[], &[], &[])));
    let o = f.oracle();
    let mut r = to_record(w.record()).unwrap();
    assert!(o.reference_image(&r).is_none());
    assert!(f.mock.calls().iter().all(|c| c.method != "http_request"));
    r.claim.kind = Kind::Derivative {
        parent: r.record_id,
        root: r.claim.capture_id,
        edits: Vec::new(),
    };
    assert_eq!(o.reference_image(&r), Some(b"preview".to_vec()));
    let req: crate::client::HttpRequest = decode_one(
        &f.mock
            .calls()
            .into_iter()
            .find(|c| c.method == "http_request")
            .unwrap()
            .arg,
    )
    .unwrap();
    assert_eq!(req.url, format!("/p/{}", r.record_id));
}

#[test]
fn network_failures_are_recorded_not_hidden() {
    let w = World::new(10);
    let f = Fixture::new(Mock::default());
    let o = f.oracle();
    assert!(o.get_key(&w.dev.key_id()).is_none());
    assert!(o.exact(&[0; 32], None).is_empty());
    assert!(o.pdq_probe(&[PdqHash([0; 32])]).hits.is_empty());
    let err = o.into_failure().unwrap();
    assert!(matches!(err, WitnessError::Network { ref reason } if reason.contains("get_key")));
}

#[test]
fn oracle_answers_are_deterministic() {
    let w = World::new(11);
    let f = Fixture::new(w.mock(probe(&[(0, 4)], &[0], &[0])));
    let a = f.oracle().pdq_probe(&[PdqHash([2; 32])]);
    let b = f.oracle().pdq_probe(&[PdqHash([2; 32])]);
    assert_eq!(a, b);
}

#[test]
fn live_replica_answers_verify_and_tampering_raises_the_flag() {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../witness-proof/tests/data/local_registry.json"
    ))
    .unwrap();
    let hex = |v: &serde_json::Value| hex::decode(v.as_str().unwrap()).unwrap();
    let num = |v: &serde_json::Value| v.as_str().unwrap().parse::<u64>().unwrap();
    let (r, k) = (&v["record"], &v["key"]);
    let record = CertifiedRecord {
        record_ix: num(&r["recordIx"]),
        record_id: r["recordId"].as_str().unwrap().into(),
        claim: hex(&r["claim"]),
        c2pa_claim: hex(&r["c2paClaim"]),
        cose: hex(&r["cose"]),
        anchored_ns: num(&r["anchoredNs"]),
        mmr_proof: r["mmrProof"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| ByteBuf::from(hex(h)))
            .collect(),
        leaf_count: num(&r["leafCount"]),
        certificate: hex(&r["certificate"]),
        media_shard: 0,
    };
    let info: KeyInfo = cbor::from_slice_canonical(&hex(&k["info"])).unwrap();
    let key = CertifiedKey {
        key: (&info).into(),
        smt_proof: hex(&k["smtProof"]),
        certificate: hex(&k["certificate"]),
    };
    let reg = Principal::from_slice(&hex(&v["registry"]));
    let root = hex(&v["rootKey"]);
    let now = num(&r["timeNs"]);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let shards = World::shards();
    let run = |record: CertifiedRecord| {
        let key = key.clone();
        let mock = Arc::new(
            Mock::default()
                .on(reg, "get", move |_| reply(Some(record.clone())))
                .on(reg, "get_key", move |_| reply(Some(key.clone()))),
        );
        let client = IcpClient::new(mock, reg);
        let trust = Trust {
            root_key: &root,
            canister: reg.as_slice(),
            now_ns: now,
            max_age_ns: DEFAULT_MAX_AGE_NS,
        };
        let o = NetOracle::new(rt.handle(), &client, &shards, trust);
        let id = RecordId::parse(r["recordId"].as_str().unwrap()).unwrap();
        let found = o.get_record(&id).is_some() && o.get_key(&info.key_id).is_some();
        (found, o.rejected())
    };
    assert_eq!(run(record.clone()), (true, false));
    let mut forged = record;
    forged.anchored_ns += 1;
    assert_eq!(run(forged), (false, true));
}
