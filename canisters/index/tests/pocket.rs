use candid::{CandidType, Decode, Encode, Principal};
use pocket_ic::PocketIc;
use serde_bytes::ByteBuf;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use witness_canister_common::{Fingerprints, PeerInit, Posting, ProbeResult, Status};
use witness_format::cbor::to_vec;
use witness_format::fingerprint::GRID_CELLS;
use witness_format::{FingerprintBundle, PdqShape};

const SHAPES: usize = 17;
const CYCLES: u128 = 100_000_000_000_000;

fn wasm() -> &'static [u8] {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let target = root.join("target/pocket-ic");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let status = Command::new(cargo)
            .current_dir(&root)
            .args([
                "build",
                "-p",
                "witness-index",
                "--target",
                "wasm32-unknown-unknown",
                "--release",
                "--features",
                "bench",
                "--target-dir",
            ])
            .arg(&target)
            .status()
            .expect("cargo runs");
        assert!(status.success(), "wasm build failed");
        std::fs::read(target.join("wasm32-unknown-unknown/release/witness_index.wasm"))
            .expect("wasm built")
    })
}

fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn hash(seed: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, chunk) in out.chunks_exact_mut(8).enumerate() {
        chunk.copy_from_slice(&mix(seed ^ (i as u64) << 56).to_be_bytes());
    }
    out
}

fn flip(mut h: [u8; 32], bits: u32, seed: u64) -> [u8; 32] {
    let mut picked = [false; 256];
    let mut state = seed;
    let mut flipped = 0;
    while flipped < bits {
        state = mix(state);
        let bit = (state % 256) as usize;
        if !picked[bit] {
            picked[bit] = true;
            h[bit / 8] ^= 1 << (bit % 8);
            flipped += 1;
        }
    }
    h
}

fn shape_hash(ix: u64, shape: usize) -> [u8; 32] {
    hash(ix.wrapping_mul(1_000).wrapping_add(shape as u64))
}

fn bundle(ix: u64) -> FingerprintBundle {
    let pdq = (0..SHAPES)
        .map(|s| PdqShape {
            tag: s as u8,
            hash: shape_hash(ix, s),
        })
        .collect();
    let grid: [u16; GRID_CELLS] = core::array::from_fn(|c| mix(ix ^ (c as u64) << 40) as u16);
    FingerprintBundle::new(pdq, 90, grid, false).unwrap()
}

fn posting(ix: u64) -> Posting {
    Posting {
        record_ix: ix,
        wm_root: 5_000_000 + ix % 4_000,
        asset_hash: hash(ix ^ 0xA55E7).to_vec(),
        pixel_hash: hash(ix ^ 0xB1C5E1).to_vec(),
        fingerprints: Some(ByteBuf::from(to_vec(&bundle(ix)))),
        root_ix: (!ix.is_multiple_of(10)).then_some(ix - ix % 10),
    }
}

fn buf(bytes: &[u8]) -> ByteBuf {
    ByteBuf::from(bytes.to_vec())
}

struct Env {
    pic: PocketIc,
    canister: Principal,
    registry: Principal,
}

impl Env {
    fn new() -> Self {
        let pic = PocketIc::new();
        let canister = pic.create_canister();
        pic.add_cycles(canister, CYCLES);
        let registry = Principal::from_slice(&[0xAB; 29]);
        let init = PeerInit {
            registry,
            shard: 3,
            origins: None,
        };
        pic.install_canister(canister, wasm().to_vec(), Encode!(&init).unwrap(), None);
        Self {
            pic,
            canister,
            registry,
        }
    }

    fn update(&self, sender: Principal, method: &str, arg: Vec<u8>) -> Result<Vec<u8>, String> {
        self.pic
            .update_call(self.canister, sender, method, arg)
            .map_err(|e| e.reject_message)
    }

    fn query<T: CandidType + for<'de> serde::Deserialize<'de>>(
        &self,
        method: &str,
        arg: Vec<u8>,
    ) -> T {
        let bytes = self
            .pic
            .query_call(self.canister, Principal::anonymous(), method, arg)
            .expect("query ok");
        Decode!(&bytes, T).unwrap()
    }

    fn put(&self, postings: &[Posting]) -> u64 {
        let bytes = self
            .update(
                self.registry,
                "put_instructions",
                Encode!(&postings).unwrap(),
            )
            .expect("put ok");
        Decode!(&bytes, u64).unwrap()
    }

    fn probe(&self, pdq: &[[u8; 32]], wm: &[u64], exact: &[[u8; 32]]) -> ProbeResult {
        let pdq: Vec<ByteBuf> = pdq.iter().map(|h| buf(h)).collect();
        let exact: Vec<ByteBuf> = exact.iter().map(|h| buf(h)).collect();
        self.query("probe", Encode!(&pdq, &wm, &exact).unwrap())
    }

    fn probe_cost(&self, pdq: &[[u8; 32]], wm: &[u64], exact: &[[u8; 32]]) -> u64 {
        let pdq: Vec<ByteBuf> = pdq.iter().map(|h| buf(h)).collect();
        let exact: Vec<ByteBuf> = exact.iter().map(|h| buf(h)).collect();
        self.query("probe_instructions", Encode!(&pdq, &wm, &exact).unwrap())
    }

    fn metric(&self, name: &str) -> u64 {
        let status: Status = self.query("status", Encode!().unwrap());
        status.metric(name).unwrap()
    }
}

#[test]
fn registry_only_writes_and_public_reads() {
    let env = Env::new();
    let batch: Vec<Posting> = (0..20).map(posting).collect();
    let stranger = Principal::from_slice(&[0xCD; 29]);
    for sender in [Principal::anonymous(), stranger] {
        assert!(env.update(sender, "put", Encode!(&batch).unwrap()).is_err());
        assert!(
            env.update(sender, "erase", Encode!(&1u64).unwrap())
                .is_err()
        );
    }
    assert_eq!(env.metric("records"), 0);
    env.update(env.registry, "put", Encode!(&batch).unwrap())
        .unwrap();
    env.update(env.registry, "put", Encode!(&batch).unwrap())
        .unwrap();
    assert_eq!(env.metric("records"), 20);
    assert_eq!(env.metric("indexed_upto"), 20);
    assert_eq!(env.metric("shard"), 3);

    let r = env.probe(
        &[flip(shape_hash(7, 4), 10, 1)],
        &[5_000_007],
        &[hash(7 ^ 0xA55E7)],
    );
    assert_eq!(r.pdq.first().map(|h| h.record_ix), Some(7));
    assert!(r.pdq[0].distance <= 10);
    assert_eq!(r.watermark, [7]);
    assert_eq!(r.exact, [7]);
    assert_eq!(r.indexed_upto, 20);

    let fp: Option<Fingerprints> = env.query("fingerprints", Encode!(&7u64).unwrap());
    assert_eq!(fp, Some(Fingerprints::from(&bundle(7))));
    let d: Vec<u64> = env.query("derivatives", Encode!(&10u64).unwrap());
    assert_eq!(d, (11..20).collect::<Vec<_>>());

    env.update(env.registry, "erase", Encode!(&7u64).unwrap())
        .unwrap();
    let r = env.probe(&[shape_hash(7, 4)], &[5_000_007], &[hash(7 ^ 0xA55E7)]);
    assert!(r.is_empty());
    let fp: Option<Fingerprints> = env.query("fingerprints", Encode!(&7u64).unwrap());
    assert_eq!(fp, None);
}

#[test]
fn malformed_probe_and_oversized_ingress_rejected() {
    let env = Env::new();
    let too_many: Vec<ByteBuf> = vec![buf(&[0; 32]); 17];
    let bad = env.pic.query_call(
        env.canister,
        Principal::anonymous(),
        "probe",
        Encode!(&too_many, &Vec::<u64>::new(), &Vec::<ByteBuf>::new()).unwrap(),
    );
    assert!(bad.is_err());
    let short: Vec<ByteBuf> = vec![buf(&[0; 31])];
    let bad = env.pic.query_call(
        env.canister,
        Principal::anonymous(),
        "probe",
        Encode!(&short, &Vec::<u64>::new(), &Vec::<ByteBuf>::new()).unwrap(),
    );
    assert!(bad.is_err());
    let huge = vec![0u8; 200];
    assert!(env.update(Principal::anonymous(), "status", huge).is_err());
}

#[test]
fn upgrade_keeps_index_without_rebuild() {
    let env = Env::new();
    let batch: Vec<Posting> = (0..64).map(posting).collect();
    env.put(&batch);
    let before = env.probe(&[shape_hash(33, 0)], &[5_000_033], &[]);
    let init = PeerInit {
        registry: env.registry,
        shard: 3,
        origins: None,
    };
    env.pic
        .upgrade_canister(env.canister, wasm().to_vec(), Encode!(&init).unwrap(), None)
        .unwrap();
    assert_eq!(env.probe(&[shape_hash(33, 0)], &[5_000_033], &[]), before);
    assert_eq!(env.metric("records"), 64);
    env.put(&[posting(64)]);
    assert_eq!(env.metric("indexed_upto"), 65);
}

#[test]
fn probe_instructions_over_ten_thousand_captures() {
    let env = Env::new();
    let total = 10_000u64;
    let mut put_max = 0u64;
    let mut put_sum = 0u64;
    let ixs: Vec<u64> = (0..total).collect();
    for chunk in ixs.chunks(64) {
        let batch: Vec<Posting> = chunk.iter().map(|&ix| posting(ix)).collect();
        let cost = env.put(&batch);
        put_max = put_max.max(cost);
        put_sum += cost;
    }
    assert_eq!(env.metric("records"), total);
    let queries: Vec<[u8; 32]> = (0..16u64)
        .map(|q| flip(shape_hash(q * 611 + 5, (q % SHAPES as u64) as usize), 12, q))
        .collect();
    let one = env.probe_cost(&queries[..1], &[], &[]);
    let sixteen = env.probe_cost(&queries, &[], &[]);
    let full = env.probe_cost(
        &queries,
        &[
            5_000_001, 5_000_002, 5_000_003, 5_000_004, 5_000_005, 5_000_006, 5_000_007, 5_000_008,
        ],
        &[
            hash(1 ^ 0xA55E7),
            hash(2 ^ 0xB1C5E1),
            hash(3 ^ 0xA55E7),
            hash(4 ^ 0xA55E7),
        ],
    );
    let miss = env.probe_cost(
        &(0..16).map(|q| hash(0xDEAD_0000 + q)).collect::<Vec<_>>(),
        &[],
        &[],
    );
    let r = env.probe(&queries, &[], &[]);
    for q in 0..16u64 {
        assert!(r.pdq.iter().any(|h| h.record_ix == q * 611 + 5));
    }
    let stable = env.metric("posting_blocks");
    println!(
        "index bench: captures={total} shapes={SHAPES} posting_blocks={stable} \
         put_avg_per_batch64={} put_max_per_batch64={put_max} put_avg_per_capture={} \
         probe_1_hash={one} probe_16_hashes={sixteen} probe_full={full} probe_16_miss={miss}",
        put_sum / total.div_ceil(64),
        put_sum / total,
    );
    assert!(full < 2_000_000_000);
}
