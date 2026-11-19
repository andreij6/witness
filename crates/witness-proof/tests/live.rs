use serde_json::Value;
use witness_format::crypto::sha256;
use witness_format::{KeyInfo, cbor};
use witness_proof::proof::{ShardEntry, key_root, shard_map_hash, shards_root};
use witness_proof::{DEFAULT_MAX_AGE_NS, Error, IC_ROOT_KEY_DER, RecordParts, Trust};

const FIXTURE: &str = include_str!("data/local_registry.json");
const GOLDEN: &str = "42ad079acf88696e7dc5fcb919037417125008b3686e1f19689b79e79462713f";

struct Live {
    root_key: Vec<u8>,
    registry: Vec<u8>,
    v: Value,
}

fn bytes(v: &Value) -> Vec<u8> {
    hex::decode(v.as_str().expect("hex string")).expect("hex")
}

fn num(v: &Value) -> u64 {
    v.as_str().expect("decimal string").parse().expect("u64")
}

#[derive(Clone)]
struct Rec {
    record_ix: u64,
    record_id: String,
    claim: Vec<u8>,
    c2pa_claim: Vec<u8>,
    cose: Vec<u8>,
    anchored_ns: u64,
    leaf_count: u64,
    mmr_proof: Vec<Vec<u8>>,
    certificate: Vec<u8>,
}

impl Rec {
    fn parts(&self) -> RecordParts<'_, Vec<u8>> {
        RecordParts {
            record_ix: self.record_ix,
            record_id: &self.record_id,
            claim: &self.claim,
            c2pa_claim: &self.c2pa_claim,
            cose: &self.cose,
            anchored_ns: self.anchored_ns,
            leaf_count: self.leaf_count,
            mmr_proof: &self.mmr_proof,
            certificate: &self.certificate,
        }
    }
}

struct Key {
    info: KeyInfo,
    smt_proof: Vec<u8>,
    certificate: Vec<u8>,
}

struct ShardList {
    index: Vec<(u8, Vec<u8>, bool)>,
    media: Vec<(u8, Vec<u8>, bool)>,
    bundle: Vec<u8>,
}

fn entries(list: &[(u8, Vec<u8>, bool)]) -> impl ExactSizeIterator<Item = ShardEntry<'_>> {
    list.iter().map(|(shard, canister, open)| ShardEntry {
        shard: *shard,
        canister,
        open: *open,
    })
}

impl ShardList {
    fn hash(&self) -> [u8; 32] {
        shard_map_hash(entries(&self.index), entries(&self.media)).unwrap()
    }
}

impl Live {
    fn load() -> Self {
        let v: Value = serde_json::from_str(FIXTURE).expect("fixture json");
        Self {
            root_key: bytes(&v["rootKey"]),
            registry: bytes(&v["registry"]),
            v,
        }
    }

    fn time(&self, part: &str) -> u64 {
        num(&self.v[part]["timeNs"])
    }

    fn certified_data(&self, part: &str) -> Vec<u8> {
        bytes(&self.v[part]["certifiedData"])
    }

    fn trust(&self, part: &str) -> Trust<'_> {
        Trust {
            root_key: &self.root_key,
            canister: &self.registry,
            now_ns: self.time(part),
            max_age_ns: DEFAULT_MAX_AGE_NS,
        }
    }

    fn record(&self) -> Rec {
        let r = &self.v["record"];
        Rec {
            record_ix: num(&r["recordIx"]),
            record_id: r["recordId"].as_str().unwrap().to_owned(),
            claim: bytes(&r["claim"]),
            c2pa_claim: bytes(&r["c2paClaim"]),
            cose: bytes(&r["cose"]),
            anchored_ns: num(&r["anchoredNs"]),
            leaf_count: num(&r["leafCount"]),
            mmr_proof: r["mmrProof"]
                .as_array()
                .unwrap()
                .iter()
                .map(bytes)
                .collect(),
            certificate: bytes(&r["certificate"]),
        }
    }

    fn key(&self) -> Key {
        let k = &self.v["key"];
        Key {
            info: cbor::from_slice_canonical(&bytes(&k["info"])).expect("key cbor"),
            smt_proof: bytes(&k["smtProof"]),
            certificate: bytes(&k["certificate"]),
        }
    }

    fn shards(&self) -> ShardList {
        let s = &self.v["shards"];
        let list = |l: &Value| {
            l.as_array()
                .unwrap()
                .iter()
                .map(|e| {
                    (
                        e["shard"].as_u64().unwrap() as u8,
                        bytes(&e["canister"]),
                        e["open"].as_bool().unwrap(),
                    )
                })
                .collect()
        };
        ShardList {
            index: list(&s["index"]),
            media: list(&s["media"]),
            bundle: bytes(&s["bundle"]),
        }
    }
}

fn digest(l: &Live) -> String {
    let r = l.record();
    let k = l.key();
    let s = l.shards();
    let rc = l.trust("record").record(&r.parts()).unwrap();
    let kc = l
        .trust("key")
        .key(&k.info, &k.smt_proof, &k.certificate)
        .unwrap();
    let sc = l.trust("shards").shards(&s.bundle, &s.hash()).unwrap();
    let mut input = Vec::new();
    for c in [&rc, &kc, &sc] {
        input.extend_from_slice(&c.data);
        input.extend_from_slice(&c.time_ns.to_be_bytes());
    }
    input.extend_from_slice(&r.parts().root().unwrap());
    input.extend_from_slice(&key_root(&k.info, &k.smt_proof).unwrap());
    input.extend_from_slice(&shards_root(&s.bundle, &s.hash()).unwrap().1);
    input.extend_from_slice(&s.hash());
    hex::encode(sha256(&input))
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn live_replica_record_key_and_shards_verify() {
    let l = Live::load();
    let r = l.record();
    let c = l.trust("record").record(&r.parts()).unwrap();
    assert_eq!(c.data, l.certified_data("record"));
    assert_eq!(c.time_ns, l.time("record"));
    let k = l.key();
    let c = l
        .trust("key")
        .key(&k.info, &k.smt_proof, &k.certificate)
        .unwrap();
    assert_eq!(c.data, l.certified_data("key"));
    let s = l.shards();
    let c = l.trust("shards").shards(&s.bundle, &s.hash()).unwrap();
    assert_eq!(c.data, l.certified_data("shards"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn live_verification_is_deterministic_and_matches_golden() {
    let l = Live::load();
    let d = digest(&l);
    assert_eq!(d, digest(&l));
    assert_eq!(d, GOLDEN);
}

fn flip(b: &mut [u8], i: usize) {
    b[i] ^= 0x01;
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn tampered_live_certificate_is_rejected() {
    let l = Live::load();
    let t = l.trust("record");
    let r = l.record();
    let n = r.certificate.len();
    for i in [3, n / 3, n / 2, n - 20, n - 1] {
        let mut bad = r.clone();
        flip(&mut bad.certificate, i);
        assert!(t.record(&bad.parts()).is_err(), "byte {i}");
    }
    let mut cut = r.clone();
    cut.certificate.truncate(n - 1);
    assert!(t.record(&cut.parts()).is_err());
    let k = l.key();
    let mut cert = k.certificate.clone();
    let mid = cert.len() / 2;
    flip(&mut cert, mid);
    assert!(l.trust("key").key(&k.info, &k.smt_proof, &cert).is_err());
    let s = l.shards();
    let mut bundle = s.bundle.clone();
    flip(&mut bundle, 40);
    assert!(l.trust("shards").shards(&bundle, &s.hash()).is_err());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn tampered_live_proofs_are_rejected() {
    let l = Live::load();
    let t = l.trust("record");
    let r = l.record();
    for i in 0..r.mmr_proof.len() {
        let mut bad = r.clone();
        flip(&mut bad.mmr_proof[i], 7);
        assert_eq!(
            t.record(&bad.parts()),
            Err(Error::Proof("certified data mismatch")),
            "path {i}"
        );
    }
    let mut short = r.clone();
    short.mmr_proof.remove(0);
    assert!(t.record(&short.parts()).is_err());
    let k = l.key();
    let kt = l.trust("key");
    for i in [0, k.smt_proof.len() / 2, k.smt_proof.len() - 1] {
        let mut smt = k.smt_proof.clone();
        flip(&mut smt, i);
        assert!(kt.key(&k.info, &smt, &k.certificate).is_err(), "smt {i}");
    }
    let s = l.shards();
    let mut moved = l.shards();
    moved.media[0].1[0] ^= 1;
    assert_eq!(
        l.trust("shards").shards(&s.bundle, &moved.hash()),
        Err(Error::Proof("certified data mismatch"))
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn tampered_live_leaf_is_rejected() {
    let l = Live::load();
    let t = l.trust("record");
    let r = l.record();
    let mismatch = Err(Error::Proof("certified data mismatch"));
    let cases = [
        Rec {
            anchored_ns: r.anchored_ns + 1,
            ..r.clone()
        },
        Rec {
            record_ix: r.record_ix + 1,
            ..r.clone()
        },
        Rec {
            leaf_count: r.leaf_count + 1,
            ..r.clone()
        },
        Rec {
            c2pa_claim: [r.c2pa_claim.as_slice(), &[0]].concat(),
            ..r.clone()
        },
        Rec {
            cose: r.cose[1..].to_vec(),
            ..r.clone()
        },
    ];
    for c in &cases {
        assert!(t.record(&c.parts()).is_err());
    }
    assert_eq!(t.record(&cases[0].parts()), mismatch);
    let k = l.key();
    let kt = l.trust("key");
    let mut patch = k.info.clone();
    patch.patch += 1;
    assert_eq!(kt.key(&patch, &k.smt_proof, &k.certificate), mismatch);
    let mut until = k.info.clone();
    until.active_until_ns -= 1;
    assert_eq!(kt.key(&until, &k.smt_proof, &k.certificate), mismatch);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn tampered_live_claim_is_rejected() {
    let l = Live::load();
    let t = l.trust("record");
    let r = l.record();
    let mut claim = r.clone();
    let n = claim.claim.len();
    flip(&mut claim.claim, n / 2);
    assert_eq!(t.record(&claim.parts()), Err(Error::Malformed("record id")));
    claim.record_id = witness_format::RecordId::from_claim_cbor(&claim.claim).to_string();
    assert_eq!(
        t.record(&claim.parts()),
        Err(Error::Proof("certified data mismatch"))
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn live_trust_inputs_are_enforced() {
    let l = Live::load();
    let r = l.record();
    let base = l.trust("record");
    let late = Trust {
        now_ns: base.now_ns + DEFAULT_MAX_AGE_NS + 1,
        ..base
    };
    assert_eq!(late.record(&r.parts()), Err(Error::Certificate("stale")));
    let early = Trust {
        now_ns: base.now_ns - DEFAULT_MAX_AGE_NS - 1,
        ..base
    };
    assert_eq!(early.record(&r.parts()), Err(Error::Certificate("stale")));
    let edge = Trust {
        now_ns: base.now_ns + DEFAULT_MAX_AGE_NS,
        ..base
    };
    assert!(edge.record(&r.parts()).is_ok());
    let mainnet = Trust {
        root_key: &IC_ROOT_KEY_DER,
        ..base
    };
    assert_eq!(
        mainnet.record(&r.parts()),
        Err(Error::Certificate("signature"))
    );
    let mut other = l.registry.clone();
    let n = other.len();
    other[n - 3] ^= 1;
    let elsewhere = Trust {
        canister: &other,
        ..base
    };
    assert!(elsewhere.record(&r.parts()).is_err());
}
