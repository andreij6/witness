use candid::{CandidType, Decode, Encode, Principal};
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;
use serde_bytes::ByteBuf;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;
use witness_canister_common::limits::{MAX_CASE_CHUNKS, MAX_CHUNK_BYTES};
use witness_canister_common::{
    BeginUpload, CaseInfo, Chunk, EntitlementGrant, MediaErr, Origins, PeerInit, Status, Tier,
    UploadTarget,
};
use witness_format::crypto::sha256;
use witness_format::{CaseId, KeyId, RecordId};
use witness_media::http::{
    HttpResponse, STREAM_CHUNK, StreamingCallbackHttpResponse, StreamingStrategy, header,
};
use witness_media::policy::{CASE_TTL_NS, DAY_NS, PENDING_TTL_NS, STALE_UPLOAD_NS};
use witness_media::store::{Allow, Begun};

const SPKI_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

fn wasm() -> Vec<u8> {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let target = root.join("target/pocketic-media");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let status = Command::new(cargo)
            .current_dir(&root)
            .args([
                "build",
                "-p",
                "witness-media",
                "--target",
                "wasm32-unknown-unknown",
                "--release",
                "--features",
                "local-dev",
                "--target-dir",
            ])
            .arg(&target)
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .status()
            .unwrap_or_else(|e| panic!("cargo: {e}"));
        assert!(status.success(), "wasm build failed");
        std::fs::read(target.join("wasm32-unknown-unknown/release/witness_media.wasm"))
            .unwrap_or_else(|e| panic!("wasm: {e}"))
    })
    .clone()
}

fn registry() -> Principal {
    Principal::from_slice(&[0xAB; 10])
}

fn user() -> Principal {
    Principal::from_slice(&[7; 29])
}

struct Setup {
    pic: PocketIc,
    media: Principal,
}

fn new_pic() -> PocketIc {
    PocketIcBuilder::new()
        .with_nns_subnet()
        .with_application_subnet()
        .build()
}

fn install(pic: PocketIc) -> Setup {
    let media = pic.create_canister();
    pic.add_cycles(media, 100_000_000_000_000);
    let arg = Encode!(&PeerInit {
        registry: registry(),
        shard: 0,
        origins: None,
    })
    .unwrap_or_default();
    pic.install_canister(media, wasm(), arg, None);
    Setup { pic, media }
}

fn setup() -> Setup {
    install(new_pic())
}

fn advance_days(s: &Setup, days: u64) {
    for _ in 0..days {
        s.pic.advance_time(Duration::from_secs(86_400));
        s.pic.tick();
        s.pic.tick();
    }
}

impl Setup {
    fn update<R: CandidType + DeserializeOwned>(
        &self,
        sender: Principal,
        method: &str,
        arg: Vec<u8>,
    ) -> Result<R, String> {
        self.pic
            .update_call(self.media, sender, method, arg)
            .map_err(|e| e.reject_message)
            .map(|b| Decode!(&b, R).unwrap_or_else(|e| panic!("{method}: {e}")))
    }

    fn query<R: CandidType + DeserializeOwned>(&self, method: &str, arg: Vec<u8>) -> R {
        let b = self
            .pic
            .query_call(self.media, Principal::anonymous(), method, arg)
            .unwrap_or_else(|e| panic!("{method}: {}", e.reject_message));
        Decode!(&b, R).unwrap_or_else(|e| panic!("{method}: {e}"))
    }

    fn sync(&self, items: &[Allow]) -> Result<u32, String> {
        self.update(
            registry(),
            "sync",
            Encode!(&items.to_vec()).unwrap_or_default(),
        )
    }

    fn upload(&self, req: &BeginUpload, data: &[u8]) -> Result<String, MediaErr> {
        let begun: Result<Begun, MediaErr> = self
            .update(user(), "begin_upload", Encode!(req).unwrap_or_default())
            .unwrap_or_else(|e| panic!("{e}"));
        let begun = begun?;
        for ix in begun.missing {
            let start = ix as usize * MAX_CHUNK_BYTES;
            let end = data.len().min(start + MAX_CHUNK_BYTES);
            let c = Chunk {
                upload: begun.upload,
                ix,
                bytes: data[start..end].to_vec(),
            };
            let r: Result<(), MediaErr> = self
                .update(user(), "put_chunk", Encode!(&c).unwrap_or_default())
                .unwrap_or_else(|e| panic!("{e}"));
            r?;
        }
        self.update(user(), "commit", Encode!(&begun.upload).unwrap_or_default())
            .unwrap_or_else(|e| panic!("{e}"))
    }

    fn get(&self, url: &str, origin: Option<&str>) -> HttpResponse {
        let mut headers = vec![("host".to_string(), "media.localhost".to_string())];
        if let Some(o) = origin {
            headers.push(("origin".into(), o.into()));
        }
        let req = ic_http_certification::HttpRequest::get(url)
            .with_headers(headers)
            .with_certificate_version(2)
            .build();
        self.query("http_request", Encode!(&req).unwrap_or_default())
    }

    fn verify(&self, url: &str, r: &HttpResponse) -> Result<(), String> {
        let req = ic_http_certification::HttpRequest::get(url)
            .with_certificate_version(2)
            .build();
        let resp = ic_http_certification::HttpResponse::builder()
            .with_status_code(
                ic_http_certification::StatusCode::from_u16(r.status_code).unwrap_or_default(),
            )
            .with_headers(r.headers.clone())
            .with_body(r.body.clone())
            .build();
        let root = self.pic.root_key().unwrap_or_default();
        ic_response_verification::verify_request_response_pair(
            req,
            resp,
            self.media.as_slice(),
            self.pic.get_time().as_nanos_since_unix_epoch() as u128,
            300_000_000_000,
            &root,
            2,
        )
        .map(|info| assert!(info.response.is_some()))
        .map_err(|e| format!("{e:?}"))
    }

    fn body(&self, url: &str) -> Option<Vec<u8>> {
        let r = self.get(url, None);
        if r.status_code != 200 {
            return None;
        }
        let mut body = r.body;
        let mut token = r
            .streaming_strategy
            .map(|StreamingStrategy::Callback { token, .. }| token);
        while let Some(t) = token {
            let next: StreamingCallbackHttpResponse = self.query(
                "http_request_streaming_callback",
                Encode!(&t).unwrap_or_default(),
            );
            body.extend_from_slice(&next.body);
            token = next.token;
        }
        Some(body)
    }

    fn status(&self) -> Status {
        self.query("status", Encode!().unwrap_or_default())
    }
}

struct Device {
    sk: SigningKey,
    spki: Vec<u8>,
    key_id: [u8; 16],
}

fn device(seed: u8) -> Device {
    let sk = SigningKey::from_slice(&[seed; 32]).unwrap_or_else(|e| panic!("{e}"));
    let mut spki = SPKI_PREFIX.to_vec();
    spki.extend_from_slice(sk.verifying_key().to_encoded_point(false).as_bytes());
    let key_id = KeyId::from_spki(&spki).to_bytes();
    Device { sk, spki, key_id }
}

fn allow(d: &Device, records: &[RecordId]) -> Vec<Allow> {
    let mut items = vec![Allow::Key {
        key_id: ByteBuf::from(d.key_id.to_vec()),
        pubkey: ByteBuf::from(d.spki.clone()),
    }];
    items.extend(records.iter().map(|r| Allow::Record {
        record_id: r.to_string(),
        key_id: ByteBuf::from(d.key_id.to_vec()),
    }));
    items
}

fn request(
    d: &Device,
    target: UploadTarget,
    data: &[u8],
    grant: Option<EntitlementGrant>,
) -> BeginUpload {
    let mut req = BeginUpload {
        target,
        size: data.len() as u64,
        sha256: sha256(data).to_vec(),
        key_id: d.key_id.to_vec(),
        signature: vec![],
        grant,
    };
    let sig: Signature = d.sk.sign(&req.signing_bytes().unwrap_or_default());
    req.signature = sig.to_bytes().to_vec();
    req
}

fn grant(expires_ns: u64) -> EntitlementGrant {
    let mut g = EntitlementGrant {
        ii_principal: user(),
        tier: Tier::Plus,
        quota_bytes: 1 << 30,
        expires_ns,
        signature: vec![],
    };
    let key = ed25519_dalek::SigningKey::from_bytes(&[0x57; 32]);
    g.signature = ed25519_dalek::Signer::sign(&key, &g.signing_bytes())
        .to_bytes()
        .to_vec();
    g
}

fn bytes(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(29) ^ seed)
        .collect()
}

fn rid(n: u8) -> RecordId {
    RecordId::from_bytes([n; 10])
}

fn now_ns(s: &Setup) -> u64 {
    s.pic.get_time().as_nanos_since_unix_epoch()
}

#[test]
fn preview_round_trip_with_certified_headers() {
    let s = setup();
    let d = device(1);
    assert_eq!(s.sync(&allow(&d, &[rid(1)])), Ok(2));
    let data = bytes(40_000, 1);
    let url = s
        .upload(
            &request(&d, UploadTarget::Preview(rid(1).to_string()), &data, None),
            &data,
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    let r = s.get(&url, Some("https://witness.app"));
    assert_eq!(r.status_code, 200);
    assert_eq!(r.body, data);
    assert!(r.streaming_strategy.is_none());
    assert_eq!(header(&r.headers, "content-type"), Some("image/jpeg"));
    assert!(header(&r.headers, "cache-control").is_some_and(|v| v.contains("immutable")));
    let cert = header(&r.headers, "ic-certificate").unwrap_or_default();
    assert!(
        cert.contains("version=2") && !cert.contains("certificate=::"),
        "{cert}"
    );
    assert_eq!(s.verify(&url, &r), Ok(()));
    assert_eq!(
        header(&r.headers, "access-control-allow-origin"),
        Some("https://witness.app")
    );
    let local = s.get(&url, Some("http://localhost:5173"));
    assert_eq!(
        header(&local.headers, "access-control-allow-origin"),
        Some("http://localhost:5173")
    );
    for missing in [
        "/p/nothing".to_string(),
        format!("/o/{}", rid(1)),
        format!("/p/{}", rid(2)),
        "/".to_string(),
        "/x/y".to_string(),
    ] {
        let r = s.get(&missing, None);
        assert_eq!(r.status_code, 404, "{missing}");
        assert_eq!(s.verify(&missing, &r), Ok(()), "{missing}");
    }
}

#[test]
fn registry_only_methods_reject_others() {
    let s = setup();
    let d = device(1);
    let r: Result<u32, String> =
        s.update(user(), "sync", Encode!(&allow(&d, &[])).unwrap_or_default());
    assert!(r.is_err());
    let r: Result<Result<u32, MediaErr>, String> = s.update(
        user(),
        "erase",
        Encode!(&rid(1).to_string()).unwrap_or_default(),
    );
    assert!(r.is_err());
}

#[test]
fn erase_removes_media() {
    let s = setup();
    let d = device(1);
    s.sync(&allow(&d, &[rid(1)])).unwrap_or_default();
    let data = bytes(1000, 2);
    let url = s
        .upload(
            &request(&d, UploadTarget::Og(rid(1).to_string()), &data, None),
            &data,
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(s.body(&url), Some(data));
    let r: Result<Result<u32, MediaErr>, String> = s.update(
        registry(),
        "erase",
        Encode!(&rid(1).to_string()).unwrap_or_default(),
    );
    assert_eq!(r, Ok(Ok(1)));
    let gone = s.get(&url, None);
    assert_eq!(gone.status_code, 404);
    assert_eq!(gone.body, b"not found");
    assert_eq!(s.verify(&url, &gone), Ok(()));
}

#[test]
fn configured_local_origins_get_cors() {
    let s = setup();
    let d = device(1);
    s.sync(&allow(&d, &[rid(1)])).unwrap_or_default();
    let data = bytes(2_000, 3);
    let url = s
        .upload(
            &request(&d, UploadTarget::Preview(rid(1).to_string()), &data, None),
            &data,
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    let web = "http://web-id.localhost:8090";
    let pages = "http://pages-id.localhost:8090";
    let cors = |origin: &str| {
        header(
            &s.get(&url, Some(origin)).headers,
            "access-control-allow-origin",
        )
        .map(str::to_owned)
    };
    assert_eq!(cors(web), None);
    let origins = Origins {
        site: pages.into(),
        case: pages.into(),
        app: web.into(),
        api: "http://localhost:8090".into(),
        canister: "http://*.localhost:8090".into(),
    };
    let arg = |o: Origins| {
        Encode!(&PeerInit {
            registry: registry(),
            shard: 0,
            origins: Some(o),
        })
        .unwrap_or_default()
    };
    s.pic
        .upgrade_canister(s.media, wasm(), arg(origins.clone()), None)
        .unwrap_or_else(|e| panic!("{}", e.reject_message));
    assert_eq!(cors(web).as_deref(), Some(web));
    assert_eq!(cors(pages).as_deref(), Some(pages));
    assert_eq!(cors("https://witness.app"), None);
    assert_eq!(cors("http://other-id.localhost:8090"), None);
    let r = s.get(&url, Some(web));
    assert_eq!(s.verify(&url, &r), Ok(()));
    let bad = Origins {
        app: "http://web-id.localhost:8090/boot.mjs".into(),
        ..origins
    };
    assert!(
        s.pic
            .upgrade_canister(s.media, wasm(), arg(bad), None)
            .is_err()
    );
    assert_eq!(cors(web).as_deref(), Some(web));
}

#[test]
fn large_full_resolution_streams() {
    let s = setup();
    let d = device(3);
    s.sync(&allow(&d, &[rid(3)])).unwrap_or_default();
    let data = bytes(2 * STREAM_CHUNK as usize + 12_345, 3);
    let req = request(
        &d,
        UploadTarget::Full(rid(3).to_string()),
        &data,
        Some(grant(now_ns(&s) + 86_400_000_000_000)),
    );
    let url = s.upload(&req, &data).unwrap_or_else(|e| panic!("{e:?}"));
    let first = s.get(&url, None);
    assert_eq!(first.body.len() as u64, STREAM_CHUNK);
    assert!(first.streaming_strategy.is_some());
    assert_eq!(s.body(&url), Some(data));
}

#[test]
fn unsigned_or_unknown_uploads_fail() {
    let s = setup();
    let d = device(1);
    let data = bytes(10, 1);
    let req = request(&d, UploadTarget::Preview(rid(1).to_string()), &data, None);
    assert!(matches!(
        s.upload(&req, &data),
        Err(MediaErr::RetryAfterNs(_))
    ));
    s.sync(&allow(&d, &[rid(1)])).unwrap_or_default();
    let mut bad = req.clone();
    bad.signature[3] ^= 1;
    assert_eq!(s.upload(&bad, &data), Err(MediaErr::Unauthorized));
    let full = request(&d, UploadTarget::Full(rid(1).to_string()), &data, None);
    assert_eq!(s.upload(&full, &data), Err(MediaErr::GrantInvalid));
}

#[test]
fn expiry_timer_deletes_cases_and_abandoned_uploads() {
    let s = setup();
    let d = device(1);
    s.sync(&allow(&d, &[rid(1)])).unwrap_or_default();
    let case = CaseId::new(0, [5; 15]);
    let data = bytes(5000, 4);
    let target = UploadTarget::CaseChunk {
        case_id: case.to_string(),
        ix: 0,
        count: 1,
    };
    let url = s
        .upload(&request(&d, target, &data, None), &data)
        .unwrap_or_else(|e| panic!("{e:?}"));
    let pending = request(&d, UploadTarget::Preview(rid(1).to_string()), &data, None);
    let begun: Result<Begun, MediaErr> = s
        .update(
            user(),
            "begin_upload",
            Encode!(&pending).unwrap_or_default(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(begun.is_ok());
    assert_eq!(s.status().metric("blobs"), Some(2));
    advance_days(&s, PENDING_TTL_NS.div_ceil(DAY_NS) + 1);
    assert_eq!(s.status().metric("blobs"), Some(1));
    assert_eq!(s.body(&url), Some(data));
    advance_days(&s, CASE_TTL_NS.div_ceil(DAY_NS));
    assert_eq!(s.get(&url, None).status_code, 404);
    let st = s.status();
    assert_eq!(st.metric("blobs"), Some(0));
    assert_eq!(st.metric("pages_free"), st.metric("pages_total"));
    let timer = st.timers.iter().find(|t| t.name == "expiry");
    assert!(timer.is_some_and(|t| t.runs >= 2 && t.period_ns == 600_000_000_000));
}

#[test]
fn upgrade_keeps_blobs_and_certification() {
    let s = setup();
    let d = device(1);
    s.sync(&allow(&d, &[rid(1)])).unwrap_or_default();
    let data = bytes(70_000, 6);
    let url = s
        .upload(
            &request(&d, UploadTarget::Preview(rid(1).to_string()), &data, None),
            &data,
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    let before = s.get(&url, None);
    let arg = Encode!(&PeerInit {
        registry: registry(),
        shard: 0,
        origins: None,
    })
    .unwrap_or_default();
    s.pic
        .upgrade_canister(s.media, wasm(), arg, None)
        .unwrap_or_else(|e| panic!("{}", e.reject_message));
    let after = s.get(&url, None);
    assert_eq!(after.body, data);
    let tree = |r: &HttpResponse| {
        header(&r.headers, "ic-certificate")
            .and_then(|v| v.split("tree=:").nth(1).map(str::to_owned))
            .unwrap_or_default()
    };
    assert_eq!(tree(&before), tree(&after));
    let st = s.status();
    assert!(st.has_flag("local-dev"));
    assert!(!st.has_flag("plus-disabled"));
    assert!(st.timers.iter().any(|t| t.name == "expiry"));
}

#[test]
fn check_counter_counts_known_records() {
    let s = setup();
    let d = device(1);
    s.sync(&allow(&d, &[rid(1)])).unwrap_or_default();
    let ids = vec![rid(1).to_string(), rid(2).to_string()];
    let n: Result<u32, String> =
        s.update(user(), "count_checks", Encode!(&ids).unwrap_or_default());
    assert_eq!(n, Ok(1));
    let counts: Vec<u64> = s.query("checks", Encode!(&ids).unwrap_or_default());
    assert_eq!(counts, vec![1, 0]);
}

#[test]
fn oversized_ingress_is_rejected_by_inspect() {
    let s = setup();
    let c = Chunk {
        upload: 0,
        ix: 0,
        bytes: vec![0; MAX_CHUNK_BYTES + 16 * 1024],
    };
    let r: Result<Result<(), MediaErr>, String> =
        s.update(user(), "put_chunk", Encode!(&c).unwrap_or_default());
    assert!(r.is_err());
}

#[test]
fn gateway_verifies_certified_responses() {
    let mut pic = new_pic();
    let endpoint = pic.make_live(None);
    let mut s = install(pic);
    let d = device(1);
    s.sync(&allow(&d, &[rid(1), rid(2)])).unwrap_or_default();
    let small = bytes(30_000, 8);
    let small_url = s
        .upload(
            &request(&d, UploadTarget::Preview(rid(1).to_string()), &small, None),
            &small,
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    let big = bytes(STREAM_CHUNK as usize + 4_321, 9);
    let big_req = request(
        &d,
        UploadTarget::Full(rid(2).to_string()),
        &big,
        Some(grant(now_ns(&s) + DAY_NS)),
    );
    let big_url = s.upload(&big_req, &big).unwrap_or_else(|e| panic!("{e:?}"));
    let port = endpoint.port().unwrap_or(80);
    let client = reqwest::blocking::Client::new();
    let fetch = |path: &str| {
        let r = client
            .get(format!("http://127.0.0.1:{port}{path}"))
            .header("host", format!("{}.localhost:{port}", s.media))
            .send()
            .unwrap_or_else(|e| panic!("{e}"));
        (
            r.status().as_u16(),
            r.bytes().map(|b| b.to_vec()).unwrap_or_default(),
        )
    };
    for (url, want) in [(small_url, small), (big_url, big)] {
        let (status, body) = fetch(&url);
        assert_eq!(status, 200, "{}", String::from_utf8_lossy(&body));
        assert_eq!(body, want);
    }
    for missing in [format!("/p/{}", rid(9)), "/nothing/here".to_string()] {
        let (status, body) = fetch(&missing);
        assert_eq!(status, 404, "{missing}: {}", String::from_utf8_lossy(&body));
        assert_eq!(body, b"not found");
    }
    s.pic.stop_live();
}

fn begin_as(s: &Setup, caller: Principal, req: &BeginUpload) -> Result<Begun, MediaErr> {
    s.update(caller, "begin_upload", Encode!(req).unwrap_or_default())
        .unwrap_or_else(|e| panic!("{e}"))
}

fn put_as(s: &Setup, caller: Principal, upload: u64, data: &[u8]) -> Result<(), MediaErr> {
    let c = Chunk {
        upload,
        ix: 0,
        bytes: data.to_vec(),
    };
    s.update(caller, "put_chunk", Encode!(&c).unwrap_or_default())
        .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn replayed_begin_from_another_caller_cannot_take_over() {
    let s = setup();
    let d = device(1);
    s.sync(&allow(&d, &[rid(1)])).unwrap_or_default();
    let data = bytes(1000, 9);
    let req = request(&d, UploadTarget::Preview(rid(1).to_string()), &data, None);
    let first = begin_as(&s, user(), &req).unwrap_or_else(|e| panic!("{e:?}"));
    let attacker = Principal::from_slice(&[0xAA; 29]);
    for caller in [attacker, Principal::anonymous()] {
        assert!(matches!(
            begin_as(&s, caller, &req),
            Err(MediaErr::RetryAfterNs(_))
        ));
        assert_eq!(
            put_as(&s, caller, first.upload, &data),
            Err(MediaErr::Unauthorized)
        );
    }
    assert_eq!(put_as(&s, user(), first.upload, &data), Ok(()));
    let url: Result<String, MediaErr> = s
        .update(user(), "commit", Encode!(&first.upload).unwrap_or_default())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(s.body(&url.unwrap_or_default()), Some(data));
}

#[test]
fn idle_upload_can_be_resumed_by_a_new_caller_after_the_stale_timeout() {
    let s = setup();
    let d = device(1);
    s.sync(&allow(&d, &[rid(1)])).unwrap_or_default();
    let data = bytes(1000, 10);
    let req = request(&d, UploadTarget::Preview(rid(1).to_string()), &data, None);
    let first = begin_as(&s, user(), &req).unwrap_or_else(|e| panic!("{e:?}"));
    let other = Principal::from_slice(&[0xBB; 29]);
    s.pic.advance_time(Duration::from_nanos(STALE_UPLOAD_NS));
    let again = begin_as(&s, other, &req).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(again.upload, first.upload);
    assert_eq!(
        put_as(&s, user(), first.upload, &data),
        Err(MediaErr::Unauthorized)
    );
    assert_eq!(put_as(&s, other, first.upload, &data), Ok(()));
}

#[test]
fn case_info_exposes_expiry_and_plus_cases_grow_past_free_limits() {
    let s = setup();
    let d = device(1);
    s.sync(&allow(&d, &[])).unwrap_or_default();
    let case = CaseId::new(0, [6; 15]);
    let info = |c: &CaseId| -> Option<CaseInfo> {
        s.query("case_info", Encode!(&c.to_string()).unwrap_or_default())
    };
    assert_eq!(info(&case), None);
    let data = bytes(3000, 11);
    let target = |ix: u32, count: u32| UploadTarget::CaseChunk {
        case_id: case.to_string(),
        ix,
        count,
    };
    let start = now_ns(&s);
    s.upload(&request(&d, target(0, 2), &data, None), &data)
        .unwrap_or_else(|e| panic!("{e:?}"));
    let got = info(&case).unwrap_or_else(|| panic!("no info"));
    assert_eq!((got.bytes, got.chunks), (3000, 2));
    assert!(got.expires_ns >= start + CASE_TTL_NS && got.expires_ns <= now_ns(&s) + CASE_TTL_NS);
    let plus = CaseId::new(0, [7; 15]);
    let big = |g: Option<EntitlementGrant>| {
        request(
            &d,
            UploadTarget::CaseChunk {
                case_id: plus.to_string(),
                ix: 0,
                count: MAX_CASE_CHUNKS + 1,
            },
            &data,
            g,
        )
    };
    assert_eq!(begin_as(&s, user(), &big(None)), Err(MediaErr::TooLarge));
    let g = grant(now_ns(&s) + DAY_NS);
    s.upload(&big(Some(g)), &data)
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        info(&plus).map(|i| (i.bytes, i.chunks)),
        Some((3000, MAX_CASE_CHUNKS + 1))
    );
}
