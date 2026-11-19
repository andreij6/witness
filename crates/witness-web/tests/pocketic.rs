#![cfg(not(target_arch = "wasm32"))]

use candid::{Decode, Encode, Principal};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde_bytes::ByteBuf;
use std::path::PathBuf;
use std::process::Command;
use witness_canister_common::{RegistryInit, ShardInfo, Shards};
use witness_web::certificate::DEFAULT_MAX_AGE_NS;
use witness_web::proof::{Trust, verify_shards};
use witness_web::wire::{Hex, ShardInfoWire, ShardsWire};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn registry_wasm() -> Vec<u8> {
    if let Ok(p) = std::env::var("WITNESS_REGISTRY_WASM") {
        return std::fs::read(p).expect("read WITNESS_REGISTRY_WASM");
    }
    let target = root().join("target/web-pocket");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(root())
        .args([
            "build",
            "-p",
            "witness-registry",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--target-dir",
        ])
        .arg(&target)
        .status()
        .expect("cargo build");
    assert!(status.success(), "registry wasm build failed");
    std::fs::read(target.join("wasm32-unknown-unknown/release/witness_registry.wasm"))
        .expect("registry wasm")
}

fn wire(s: &Shards, bundle: &[u8]) -> ShardsWire {
    let info = |i: &ShardInfo| ShardInfoWire {
        shard: i.shard,
        canister: Hex(i.canister.as_slice().to_vec()),
        open: i.open,
    };
    ShardsWire {
        index: s.index.iter().map(info).collect(),
        media: s.media.iter().map(info).collect(),
        bundle: Hex(bundle.to_vec()),
    }
}

fn setup() -> (PocketIc, Principal) {
    let pic = PocketIcBuilder::new()
        .with_nns_subnet()
        .with_application_subnet()
        .build();
    let app = pic.topology().get_app_subnets()[0];
    let registry = pic.create_canister_on_subnet(None, None, app);
    pic.add_cycles(registry, 10_000_000_000_000);
    let peer = |b: u8| Principal::from_slice(&[b; 10]);
    let init = RegistryInit {
        index: vec![peer(1)],
        media: vec![peer(2), peer(3)],
        pages: peer(4),
    };
    pic.install_canister(registry, registry_wasm(), Encode!(&init).unwrap(), None);
    pic.tick();
    (pic, registry)
}

#[test]
fn live_registry_shard_map_verifies_against_the_replica_root_key() {
    let (pic, registry) = setup();
    let reply = pic
        .query_call(
            registry,
            Principal::anonymous(),
            "shards",
            Encode!().unwrap(),
        )
        .expect("shards query");
    let (shards, bundle) = Decode!(&reply, Shards, ByteBuf).expect("decode shards");
    assert_eq!(shards.media.len(), 2);
    let len = u32::from_be_bytes(bundle[..4].try_into().unwrap()) as usize;
    let cert: ic_certification::Certificate = serde_cbor::from_slice(&bundle[4..4 + len]).unwrap();
    assert!(
        cert.delegation.is_some(),
        "application subnet certificates carry a delegation"
    );
    let root_key = pic.root_key().expect("root key");
    let now = pic.get_time().as_nanos_since_unix_epoch();
    let trust = Trust {
        root_key: &root_key,
        canister: registry.as_slice(),
        now_ns: now,
        max_age_ns: DEFAULT_MAX_AGE_NS,
    };
    let w = wire(&shards, &bundle);
    let v = verify_shards(&w, &trust).expect("verify live certificate");
    assert_eq!(v["media"], 2);
    let mut moved = w.clone();
    moved.media[1].canister = Hex(vec![9; 10]);
    assert!(verify_shards(&moved, &trust).is_err());
    let other = Trust {
        canister: &[0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
        ..trust
    };
    assert!(verify_shards(&w, &other).is_err());
    let late = Trust {
        now_ns: now + 2 * DEFAULT_MAX_AGE_NS,
        ..trust
    };
    assert!(verify_shards(&w, &late).is_err());
    let mut wrong_root = root_key.clone();
    let n = wrong_root.len();
    wrong_root[n - 1] ^= 1;
    let forged = Trust {
        root_key: &wrong_root,
        ..trust
    };
    assert!(verify_shards(&w, &forged).is_err());
}
