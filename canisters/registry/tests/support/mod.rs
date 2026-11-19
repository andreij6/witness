#![allow(dead_code)]

pub mod attest;

use candid::utils::ArgumentEncoder;
use candid::{CandidType, Decode, Encode, Principal, encode_args};
use ic_certification::{Certificate, LookupResult};
use pocket_ic::{PocketIc, PocketIcBuilder, RejectResponse};
use serde::de::DeserializeOwned;
use serde_bytes::ByteBuf;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;
use witness_c2pa::{ManifestSpec, TrustAnchor, data_hash, seal_jpeg};
use witness_canister_common::{RegistryInit, Submission, SubmitResult};
use witness_core::image::{Orientation, encode_jpeg};
use witness_core::{Capture, ClaimTemplate, Manifest, SEAL_QUALITY};
use witness_format::{Kind, WitnessClaim, cbor};
use witness_registry::types::{EnrollResult, Enrollment};
use witness_verdict::testkit::{CAPTURED, GENERATOR, TestDevice, photo};

const STUB: &str = "registry_peer_stub";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cargo_build(args: &[&str]) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let target = root().join("target/registry-test");
    let status = Command::new(cargo)
        .current_dir(root())
        .args([
            "build",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--target-dir",
        ])
        .arg(&target)
        .args(args)
        .status()
        .expect("spawn cargo");
    assert!(status.success(), "cargo build {args:?} failed");
}

fn wasm_path(name: &str) -> PathBuf {
    root().join(format!(
        "target/registry-test/wasm32-unknown-unknown/release/{name}.wasm"
    ))
}

pub fn registry_wasm() -> &'static [u8] {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        if let Ok(p) = std::env::var("WITNESS_REGISTRY_WASM") {
            return std::fs::read(p).expect("read WITNESS_REGISTRY_WASM");
        }
        cargo_build(&["-p", "witness-registry", "--features", "dev-attestation"]);
        std::fs::read(wasm_path("witness_registry")).expect("registry wasm")
    })
}

pub fn stub_wasm() -> &'static [u8] {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        let manifest = root().join("canisters/registry/tests/stub/Cargo.toml");
        cargo_build(&["--manifest-path", manifest.to_str().expect("utf8 path")]);
        std::fs::read(wasm_path(STUB)).expect("stub wasm")
    })
}

pub fn user(n: u8) -> Principal {
    Principal::from_slice(&[n; 29])
}

pub struct Env {
    pub pic: PocketIc,
    pub registry: Principal,
    pub index: Principal,
    pub pages: Principal,
    pub media: Principal,
}

pub struct Device {
    pub dev: TestDevice,
    pub anchor: TrustAnchor,
    pub key_id: Vec<u8>,
}

impl Env {
    pub fn new() -> Self {
        let pic = PocketIcBuilder::new()
            .with_nns_subnet()
            .with_test_threshold_keys_subnet()
            .with_application_subnet()
            .build();
        let wall = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos() as u64;
        pic.set_time(pocket_ic::Time::from_nanos_since_unix_epoch(wall));
        pic.tick();
        let app = pic.topology().get_app_subnets()[0];
        let make = |wasm: &[u8], arg: Vec<u8>| {
            let id = pic.create_canister_on_subnet(None, None, app);
            pic.add_cycles(id, 100_000_000_000_000);
            pic.install_canister(id, wasm.to_vec(), arg, None);
            id
        };
        let index = make(stub_wasm(), Encode!().unwrap());
        let pages = make(stub_wasm(), Encode!().unwrap());
        let media = make(stub_wasm(), Encode!().unwrap());
        let registry = make(
            registry_wasm(),
            Encode!(&Self::init_arg(index, pages, media)).unwrap(),
        );
        let env = Self {
            pic,
            registry,
            index,
            pages,
            media,
        };
        env.wait_ready();
        env
    }

    pub fn init_arg(index: Principal, pages: Principal, media: Principal) -> RegistryInit {
        RegistryInit {
            index: vec![index],
            media: vec![media],
            pages,
        }
    }

    pub fn now(&self) -> u64 {
        self.pic.get_time().as_nanos_since_unix_epoch()
    }

    pub fn wait_ready(&self) {
        for _ in 0..100 {
            if self.query::<Option<ByteBuf>>("ca_root", ()).is_some() {
                return;
            }
            self.pic.tick();
        }
        panic!("registry never became ready");
    }

    pub fn query_raw(&self, method: &str, arg: Vec<u8>) -> Result<Vec<u8>, RejectResponse> {
        self.pic.query_call(self.registry, user(1), method, arg)
    }

    pub fn query<T: CandidType + DeserializeOwned>(
        &self,
        method: &str,
        arg: impl ArgumentEncoder,
    ) -> T {
        let bytes = self
            .query_raw(method, encode_args(arg).unwrap())
            .expect(method);
        Decode!(&bytes, T).expect("decode")
    }

    pub fn update_raw(&self, method: &str, arg: Vec<u8>) -> Result<Vec<u8>, RejectResponse> {
        self.pic.update_call(self.registry, user(1), method, arg)
    }

    pub fn update<T: CandidType + DeserializeOwned>(
        &self,
        method: &str,
        arg: impl ArgumentEncoder,
    ) -> T {
        let bytes = self
            .update_raw(method, encode_args(arg).unwrap())
            .expect(method);
        Decode!(&bytes, T).expect("decode")
    }

    pub fn enrollment(&self, seed: u64) -> Enrollment {
        let challenge: ByteBuf = self.query("challenge", ());
        let now_s = (self.now() / 1_000_000_000) as i64;
        let a = attest::Attestation::new(&challenge, now_s);
        Enrollment {
            chain: a
                .chain(&attest::device_key(seed))
                .into_iter()
                .map(ByteBuf::from)
                .collect(),
            key_id: None,
            late: None,
        }
    }

    pub fn device_from(&self, seed: u64, r: EnrollResult) -> Device {
        let EnrollResult::Ok(e) = r else {
            panic!("enroll failed: {r:?}")
        };
        let anchor = TrustAnchor::from_der(&e.ca_cert).expect("ca");
        let leaf = witness_c2pa::x509::verify_chain(&[&e.c2pa_cert, &e.ca_cert], &anchor, None)
            .expect("leaf");
        let key = attest::device_key(seed);
        assert_eq!(leaf.spki_der, attest::spki(&key));
        let serial = leaf
            .serial
            .iter()
            .fold(0u64, |a, b| (a << 8) | u64::from(*b));
        Device {
            dev: TestDevice {
                key,
                spki: leaf.spki_der,
                serial,
                chain: vec![e.c2pa_cert, e.ca_cert],
            },
            anchor,
            key_id: e.key_id,
        }
    }

    pub fn enroll(&self, seed: u64) -> Device {
        let r: EnrollResult = self.update("enroll", (self.enrollment(seed),));
        self.device_from(seed, r)
    }

    pub fn submit(&self, batch: Vec<Submission>) -> Vec<SubmitResult> {
        self.update("submit", (batch,))
    }

    pub fn drain(&self) {
        for _ in 0..3 {
            self.pic.advance_time(Duration::from_secs(6));
            for _ in 0..5 {
                self.pic.tick();
            }
        }
    }

    pub fn calls(&self, stub: Principal) -> Vec<(String, ByteBuf)> {
        let bytes = self
            .pic
            .query_call(stub, user(1), "calls", Encode!().unwrap())
            .expect("calls");
        Decode!(&bytes, Vec<(String, ByteBuf)>).expect("decode calls")
    }

    pub fn set_down(&self, stub: Principal, down: bool) {
        self.pic
            .update_call(stub, user(1), "set_down", Encode!(&down).unwrap())
            .expect("set_down");
    }

    pub fn upgrade(&self) {
        let arg = Encode!(&Self::init_arg(self.index, self.pages, self.media)).unwrap();
        self.pic
            .upgrade_canister(self.registry, registry_wasm().to_vec(), arg, None)
            .expect("upgrade");
    }

    pub fn verify_certificate(&self, certificate: &[u8], expected: &[u8; 32]) {
        let root_key = self.pic.root_key().expect("root key");
        let cert: Certificate = serde_cbor::from_slice(certificate).expect("certificate cbor");
        let key = match &cert.delegation {
            None => root_key,
            Some(d) => {
                let inner: Certificate =
                    serde_cbor::from_slice(&d.certificate).expect("delegation cbor");
                check_signature(&inner, &root_key);
                match inner
                    .tree
                    .lookup_path([b"subnet".as_slice(), &d.subnet_id, b"public_key"])
                {
                    LookupResult::Found(k) => k.to_vec(),
                    other => panic!("subnet key missing: {other:?}"),
                }
            }
        };
        check_signature(&cert, &key);
        match cert.tree.lookup_path([
            b"canister".as_slice(),
            self.registry.as_slice(),
            b"certified_data",
        ]) {
            LookupResult::Found(d) => assert_eq!(d, expected.as_slice(), "certified_data mismatch"),
            other => panic!("certified_data missing: {other:?}"),
        }
    }
}

fn check_signature(cert: &Certificate, key_der: &[u8]) {
    let raw = &key_der[key_der.len() - 96..];
    let mut msg = b"\x0dic-state-root".to_vec();
    msg.extend_from_slice(&cert.tree.digest());
    ic_verify_bls_signature::verify_bls_signature(&cert.signature, &msg, raw)
        .expect("bls signature");
}

pub fn template(d: &Device, capture: u64, kind: Kind, counter: u64) -> ClaimTemplate {
    let c = d.dev.claim(capture, kind, counter);
    ClaimTemplate {
        capture_id: c.capture_id,
        kind: c.kind,
        time: c.time,
        loc: c.loc,
        checks: c.checks,
        key: c.key,
        cert_serial: c.cert_serial,
        coarse: c.coarse,
    }
}

pub fn capture(d: &Device, capture: u64, kind: Kind, counter: u64) -> Submission {
    let jpeg = encode_jpeg(&photo(160, 120, capture), 95).unwrap();
    let manifest = Manifest {
        id: [0x11; 16],
        generator: GENERATOR,
        actions: CAPTURED,
    };
    let c = Capture {
        image: &jpeg,
        orientation: Orientation::Normal,
        quality: SEAL_QUALITY,
        mark: None,
    };
    let sealed = witness_core::seal(
        &c,
        template(d, capture, kind, counter),
        &manifest,
        &d.dev,
        &d.anchor,
    )
    .expect("seal");
    Submission::from_parts(sealed.submission, Some(cbor::to_vec(&sealed.fingerprints)))
}

pub fn quick(d: &Device, claim: WitnessClaim, seed: u64) -> Submission {
    let jpeg = encode_jpeg(&photo(32, 24, seed), 90).unwrap();
    let mut claim = claim;
    claim.asset_hash = data_hash(&jpeg).unwrap();
    let cbor = claim.to_cbor().unwrap();
    let spec = ManifestSpec {
        id: [0x5a; 16],
        generator: GENERATOR,
        actions: CAPTURED,
        witness_claim: &cbor,
    };
    let s = seal_jpeg(&jpeg, &spec, &d.dev, &d.anchor).unwrap();
    Submission {
        claim: cbor,
        c2pa_claim: s.c2pa_claim,
        cose: s.cose_sign1,
        fingerprints: None,
    }
}
