use candid::{Decode, Encode, Principal};
use ic_http_certification::{HttpRequest, HttpResponse};
use ic_response_verification::verify_request_response_pair;
use pocket_ic::{PocketIc, PocketIcBuilder};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use witness_canister_common::{Origins, PeerInit, Status};
use witness_format::{
    BlurClass, BootState, CaptureId, CertSerial, Checks, Confidence, Dims, Edit, Geohash5,
    GnssChecks, KeyId, KeyRef, Kind, LocationEvidence, LocationReasons, RecordId, Rect,
    SecurityLevel, TimeEvidence, WitnessClaim,
};
use witness_pages::{PageErr, PageResult, Removal, Shell};

const REGISTRY: &[u8] = &[0xab; 29];

fn registry() -> Principal {
    Principal::from_slice(REGISTRY)
}

fn wasm() -> Vec<u8> {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        if let Some(p) = std::env::var_os("WITNESS_PAGES_WASM") {
            return std::fs::read(p).expect("read WITNESS_PAGES_WASM");
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let target = root.join("target/pocket-pages");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let status = Command::new(cargo)
            .current_dir(&root)
            .args([
                "build",
                "-p",
                "witness-pages",
                "--target",
                "wasm32-unknown-unknown",
                "--release",
                "--features",
                "local-dev",
                "--target-dir",
            ])
            .arg(&target)
            .status()
            .expect("spawn cargo");
        assert!(status.success(), "wasm build failed");
        std::fs::read(target.join("wasm32-unknown-unknown/release/witness_pages.wasm"))
            .expect("read wasm")
    })
    .clone()
}

fn claim(counter: u64, kind: Kind, coarse: Option<&str>) -> Vec<u8> {
    let original = kind.is_original();
    WitnessClaim {
        capture_id: CaptureId::new(0x0002_0304_0506_0708, 1).unwrap(),
        kind,
        asset_hash: [0xaa; 32],
        pixel_hash: [0xbb; 32],
        fp_commit: [0xcc; 32],
        dims: Dims {
            width: 1080,
            height: 1350,
        },
        time: TimeEvidence {
            wall_ms: 1_700_000_000_000,
            gnss_ms: None,
            elapsed_ns: 5_000_000_000,
            boot_count: 3,
        },
        loc: LocationEvidence {
            commit: original.then_some([1; 32]),
            confidence: Confidence::Medium,
            reasons: LocationReasons::FUSED_FIX,
        },
        checks: Checks {
            key_security: SecurityLevel::StrongBox,
            boot: BootState::Verified,
            device_locked: true,
            mock_location: false,
            gnss: GnssChecks::RAW_AVAILABLE,
            recapture: 12,
        },
        key: KeyRef {
            key_id: KeyId::from_bytes([0x22; 16]),
            counter,
        },
        cert_serial: CertSerial::from_u64(0x1234).unwrap(),
        coarse: coarse.and_then(Geohash5::new),
    }
    .to_cbor()
    .unwrap()
}

fn blurred(counter: u64) -> Vec<u8> {
    let blur = Edit::Blur {
        rect: Rect {
            x: 1,
            y: 2,
            w: 3,
            h: 4,
        },
        class: BlurClass::Face,
        sigma_e2: 300,
    };
    claim(
        counter,
        Kind::Derivative {
            parent: RecordId::from_bytes([0x44; 10]),
            root: CaptureId::new(7, 0).unwrap(),
            edits: vec![blur, blur],
        },
        Some("9q8yy"),
    )
}

fn input(claim: Vec<u8>, dev: bool) -> Shell {
    Shell {
        record_id: RecordId::from_claim_cbor(&claim).to_string(),
        record_ix: 0,
        claim,
        media: Principal::from_slice(&[7; 10]),
        media_shard: 1,
        anchored_ns: 1_700_000_100_000_000_000,
        dev,
    }
}

struct Env {
    pic: PocketIc,
    id: Principal,
}

impl Env {
    fn new() -> Self {
        let pic = PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet()
            .build();
        let id = pic.create_canister();
        pic.add_cycles(id, 10_000_000_000_000);
        let init = PeerInit {
            registry: registry(),
            shard: 0,
            origins: None,
        };
        pic.install_canister(id, wasm(), Encode!(&init).unwrap(), None);
        Self { pic, id }
    }

    fn put(&self, sender: Principal, inputs: &[Shell]) -> Result<Vec<PageResult>, String> {
        self.pic
            .update_call(
                self.id,
                sender,
                "put_shell",
                Encode!(&inputs.to_vec()).unwrap(),
            )
            .map(|b| Decode!(&b, Vec<PageResult>).unwrap())
            .map_err(|e| e.reject_message)
    }

    fn tombstone(&self, ids: &[String], why: Removal) -> Vec<PageResult> {
        let b = self
            .pic
            .update_call(
                self.id,
                registry(),
                "tombstone",
                Encode!(&ids.to_vec(), &why).unwrap(),
            )
            .unwrap();
        Decode!(&b, Vec<PageResult>).unwrap()
    }

    fn get(&self, url: &str) -> (HttpRequest<'static>, HttpResponse<'static>) {
        let req = HttpRequest::get(url.to_string())
            .with_certificate_version(2)
            .build();
        let b = self
            .pic
            .query_call(
                self.id,
                Principal::anonymous(),
                "http_request",
                Encode!(&req).unwrap(),
            )
            .unwrap();
        (req, Decode!(&b, HttpResponse<'static>).unwrap())
    }

    fn verify(&self, url: &str) -> HttpResponse<'static> {
        let (req, resp) = self.get(url);
        let info = verify_request_response_pair(
            req,
            resp.clone(),
            self.id.as_slice(),
            u128::from(self.pic.get_time().as_nanos_since_unix_epoch()),
            300_000_000_000,
            &self.pic.root_key().unwrap(),
            2,
        )
        .unwrap_or_else(|e| panic!("verification failed for {url}: {e:?}"));
        assert_eq!(info.verification_version, 2);
        let verified = info.response.expect("verified response");
        assert_eq!(verified.body, resp.body());
        assert_eq!(verified.status_code, Some(resp.status_code().as_u16()));
        resp
    }

    fn status(&self) -> Status {
        let b = self
            .pic
            .query_call(
                self.id,
                Principal::anonymous(),
                "status",
                Encode!().unwrap(),
            )
            .unwrap();
        Decode!(&b, Status).unwrap()
    }
}

fn ok_id(r: &PageResult) -> String {
    match r {
        PageResult::Ok(id) => id.clone(),
        e => panic!("expected ok, got {e:?}"),
    }
}

fn header<'a>(r: &'a HttpResponse, name: &str) -> Option<&'a str> {
    r.headers()
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

#[test]
fn certified_responses_verify_for_every_route() {
    let env = Env::new();
    let res = env
        .put(
            registry(),
            &[
                input(blurred(1), false),
                input(claim(2, Kind::Original, None), true),
                input(vec![1, 2, 3], false),
            ],
        )
        .unwrap();
    let shell = ok_id(&res[0]);
    let dev = ok_id(&res[1]);
    assert!(matches!(res[2], PageResult::Err(PageErr::Malformed(_))));

    let r = env.verify(&format!("/p/{shell}"));
    assert_eq!(r.status_code().as_u16(), 200);
    let body = String::from_utf8(r.body().to_vec()).unwrap();
    assert!(body.contains("Matches an original, with 2 declared blurs"));
    assert!(header(&r, "x-robots-tag").is_none());
    assert!(
        header(&r, "content-security-policy")
            .unwrap()
            .contains("script-src 'self' https://app.witness.app")
    );

    let r = env.verify(&format!("/p/{dev}?utm_source=x"));
    assert_eq!(header(&r, "x-robots-tag"), Some("noindex, nofollow"));
    assert!(String::from_utf8_lossy(r.body()).contains("class=\"dev\""));

    for url in [
        "/p/0000000000000000",
        "/",
        "/p",
        "/p/",
        "/nope/deeper",
        "/p/%3C*%3E",
        "/p/%3C%24%3E",
        "/p/0000000000000000/x",
    ] {
        assert_eq!(env.verify(url).status_code().as_u16(), 404, "{url}");
    }

    let a = env.verify("/v/ABCDEFGHJKMNPQRSTVWXYZ0123");
    let b = env.verify("/v/other/path");
    assert_eq!(a.body(), b.body());
    assert!(
        header(&a, "content-security-policy")
            .unwrap()
            .contains("'strict-dynamic'")
    );
    assert_eq!(header(&a, "referrer-policy"), Some("no-referrer"));

    let d = env.verify("/.well-known/ic-domains");
    assert_eq!(d.body(), b"witness.app\ncase.witness.app\n");

    let (req, mut tampered) = env.get(&format!("/p/{shell}"));
    tampered = HttpResponse::builder()
        .with_status_code(tampered.status_code())
        .with_headers(tampered.headers().to_vec())
        .with_body(b"<h1>forged</h1>".to_vec())
        .build();
    assert!(
        verify_request_response_pair(
            req,
            tampered,
            env.id.as_slice(),
            u128::from(env.pic.get_time().as_nanos_since_unix_epoch()),
            300_000_000_000,
            &env.pic.root_key().unwrap(),
            2,
        )
        .is_err()
    );
}

#[test]
fn tombstone_recertifies_in_same_update_and_blocks_resurrection() {
    let env = Env::new();
    let id = ok_id(&env.put(registry(), &[input(blurred(9), false)]).unwrap()[0]);
    assert_eq!(env.verify(&format!("/p/{id}")).status_code().as_u16(), 200);
    let res = env.tombstone(&[id.clone(), "garbage".into()], Removal::Takedown);
    assert_eq!(ok_id(&res[0]), id);
    assert!(matches!(res[1], PageResult::Err(PageErr::Malformed(_))));
    let r = env.verify(&format!("/p/{id}"));
    assert_eq!(r.status_code().as_u16(), 410);
    assert!(String::from_utf8_lossy(r.body()).contains("taken down"));
    assert_eq!(
        env.put(registry(), &[input(blurred(9), false)]).unwrap()[0],
        PageResult::Err(PageErr::Gone)
    );
    let unseen = RecordId::from_claim_cbor(&blurred(10)).to_string();
    env.tombstone(std::slice::from_ref(&unseen), Removal::Erased);
    assert_eq!(
        env.verify(&format!("/p/{unseen}")).status_code().as_u16(),
        410
    );
}

#[test]
fn only_registry_writes_and_batches_are_bounded() {
    let env = Env::new();
    let stranger = Principal::from_slice(&[0x11; 29]);
    let err = env.put(stranger, &[input(blurred(1), false)]).unwrap_err();
    assert!(err.contains("rejected"), "{err}");
    let err = env.put(Principal::anonymous(), &[]).unwrap_err();
    assert!(!err.is_empty());
    let big: Vec<Shell> = (0..65).map(|i| input(blurred(100 + i), false)).collect();
    assert!(
        env.put(registry(), &big)
            .unwrap_err()
            .contains("batch too large")
    );
    assert_eq!(
        env.status().metrics,
        vec![("pages".into(), 0), ("tree_nodes".into(), 0)]
    );
}

#[test]
fn upgrade_keeps_certified_pages_without_rebuild() {
    let env = Env::new();
    let inputs: Vec<Shell> = (0..64).map(|i| input(blurred(1000 + i), false)).collect();
    let ids: Vec<String> = env
        .put(registry(), &inputs)
        .unwrap()
        .iter()
        .map(ok_id)
        .collect();
    env.tombstone(&ids[..1], Removal::Revoked);
    let before = env.status();
    let init = PeerInit {
        registry: registry(),
        shard: 0,
        origins: None,
    };
    env.pic
        .upgrade_canister(env.id, wasm(), Encode!(&init).unwrap(), None)
        .unwrap();
    let after = env.status();
    assert_eq!(before.metrics, after.metrics);
    assert_eq!(after.metrics[0], ("pages".into(), 64));
    assert_eq!(after.metrics[1], ("tree_nodes".into(), 63));
    assert_eq!(
        env.verify(&format!("/p/{}", ids[0])).status_code().as_u16(),
        410
    );
    for id in &ids[1..] {
        assert_eq!(env.verify(&format!("/p/{id}")).status_code().as_u16(), 200);
    }
    assert_eq!(
        env.verify("/p/ZZZZZZZZZZZZZZZZ").status_code().as_u16(),
        404
    );
}

fn local_origins(env: &Env) -> Origins {
    let me = format!("http://{}.localhost:8090", env.id);
    Origins {
        site: me.clone(),
        case: me,
        app: "http://web-id.localhost:8090".into(),
        api: "http://localhost:8090".into(),
        canister: "http://*.localhost:8090".into(),
    }
}

#[test]
fn upgrade_to_new_origins_recertifies_every_page() {
    let env = Env::new();
    let inputs: Vec<Shell> = (0..8).map(|i| input(blurred(2000 + i), false)).collect();
    let ids: Vec<String> = env
        .put(registry(), &inputs)
        .unwrap()
        .iter()
        .map(ok_id)
        .collect();
    env.tombstone(&ids[..1], Removal::Erased);
    let origins = local_origins(&env);
    let init = PeerInit {
        registry: registry(),
        shard: 0,
        origins: Some(origins.clone()),
    };
    env.pic
        .upgrade_canister(env.id, wasm(), Encode!(&init).unwrap(), None)
        .unwrap();
    assert_eq!(
        env.verify(&format!("/p/{}", ids[0])).status_code().as_u16(),
        410
    );
    for id in &ids[1..] {
        let r = env.verify(&format!("/p/{id}"));
        assert_eq!(r.status_code().as_u16(), 200);
        assert_eq!(
            header(&r, "content-security-policy")
                .map(|c| c
                    .contains("script-src 'self' http://web-id.localhost:8090 'wasm-unsafe-eval'")),
            Some(true)
        );
    }
    let fresh = input(blurred(2100), false);
    let id = ok_id(&env.put(registry(), &[fresh]).unwrap()[0]);
    let body = String::from_utf8(env.verify(&format!("/p/{id}")).body().to_vec()).unwrap();
    assert!(body.contains(
        "<script type=\"module\" src=\"http://web-id.localhost:8090/boot.mjs\"></script>"
    ));
    assert!(body.contains(&format!("href=\"{}/p/{id}\"", origins.site)));
    let case = String::from_utf8(env.verify("/v/abc").body().to_vec()).unwrap();
    assert!(case.contains("src=\"http://web-id.localhost:8090/boot.mjs\""));
    assert_eq!(
        env.verify("/p/ZZZZZZZZZZZZZZZZ").status_code().as_u16(),
        404
    );
    let bad = PeerInit {
        origins: Some(Origins {
            app: "javascript:alert(1)".into(),
            ..origins
        }),
        ..init
    };
    assert!(
        env.pic
            .upgrade_canister(env.id, wasm(), Encode!(&bad).unwrap(), None)
            .is_err()
    );
    assert_eq!(env.verify(&format!("/p/{id}")).status_code().as_u16(), 200);
}

#[test]
fn http_gateway_serves_verified_pages() {
    let mut env = Env::new();
    let id = ok_id(&env.put(registry(), &[input(blurred(77), false)]).unwrap()[0]);
    let gone = ok_id(&env.put(registry(), &[input(blurred(78), false)]).unwrap()[0]);
    env.tombstone(std::slice::from_ref(&gone), Removal::Erased);
    let url = env.pic.make_live(None);
    let host = format!("{}.localhost", env.id);
    let client = reqwest::blocking::Client::new();
    let fetch = |path: &str| {
        let r = client
            .get(format!("http://127.0.0.1:{}{path}", url.port().unwrap()))
            .header(reqwest::header::HOST, &host)
            .send()
            .unwrap();
        let status = r.status().as_u16();
        let robots = r
            .headers()
            .get("x-robots-tag")
            .map(|v| v.to_str().unwrap().to_string());
        (status, robots, r.text().unwrap())
    };
    let (status, robots, body) = fetch(&format!("/p/{id}"));
    assert_eq!(status, 200, "{body}");
    assert!(robots.is_none());
    assert!(body.contains("Matches an original, with 2 declared blurs"));
    let (status, robots, _) = fetch(&format!("/p/{gone}"));
    assert_eq!(
        (status, robots.as_deref()),
        (410, Some("noindex, nofollow"))
    );
    let (status, _, _) = fetch("/p/0000000000000000");
    assert_eq!(status, 404);
    let (status, robots, body) = fetch("/v/SOMECASE");
    assert_eq!(
        (status, robots.as_deref()),
        (200, Some("noindex, nofollow"))
    );
    assert!(body.contains("Shared case"));
    let (status, _, body) = fetch("/.well-known/ic-domains");
    assert_eq!(
        (status, body.as_str()),
        (200, "witness.app\ncase.witness.app\n")
    );
    env.pic.stop_live();
}

fn pin(path: &str, fill: char) -> witness_pages::pins::Pin {
    witness_pages::pins::Pin {
        path: path.into(),
        sri: format!("sha384-{}", fill.to_string().repeat(64)),
    }
}

#[test]
fn upgrade_pins_the_case_shell_and_recertifies_it() {
    let env = Env::new();
    assert!(env.status().flags.contains(&"case-unpinned".to_string()));
    let id = ok_id(&env.put(registry(), &[input(blurred(3000), false)]).unwrap()[0]);
    let gone = ok_id(&env.put(registry(), &[input(blurred(3001), false)]).unwrap()[0]);
    env.tombstone(std::slice::from_ref(&gone), Removal::Takedown);
    let unpinned = env.verify("/v/abc").body().to_vec();
    let not_found = env.verify("/nope").body().to_vec();
    let tomb = env.verify(&format!("/p/{gone}")).body().to_vec();
    let init = PeerInit {
        registry: registry(),
        shard: 0,
        origins: None,
    };
    let upgrade = |pins: Option<Vec<witness_pages::pins::Pin>>| {
        env.pic
            .upgrade_canister(env.id, wasm(), Encode!(&init, &pins).unwrap(), None)
    };
    upgrade(Some(vec![pin("boot.mjs", 'B'), pin("skiko.wasm", 'W')])).unwrap();
    assert!(!env.status().flags.contains(&"case-unpinned".to_string()));
    let case = String::from_utf8(env.verify("/v/abc").body().to_vec()).unwrap();
    assert_ne!(case.as_bytes(), unpinned.as_slice());
    let b = "B".repeat(64);
    let w = "W".repeat(64);
    assert!(case.contains(&format!(
        "<script type=\"importmap\" nonce=\"{n}\">{{\"integrity\":{{\"https://app.witness.app/boot.mjs\":\"sha384-{b}\",\"https://app.witness.app/skiko.wasm\":\"sha384-{w}\"}}}}</script>",
        n = witness_pages::html::CASE_NONCE.as_str()
    )));
    assert!(case.contains(&format!(
        "integrity=\"sha384-{b}\" crossorigin=\"anonymous\" src=\"https://app.witness.app/boot.mjs\""
    )));
    let csp = header(&env.verify("/v/x/y"), "content-security-policy")
        .unwrap()
        .to_string();
    assert!(csp.contains("'nonce-") && csp.contains("'strict-dynamic'"));
    assert_eq!(env.verify("/nope").status_code().as_u16(), 404);
    assert_eq!(
        env.verify(&format!("/p/{gone}")).status_code().as_u16(),
        410
    );
    assert_eq!(env.verify(&format!("/p/{gone}")).body(), tomb.as_slice());
    assert_eq!(env.verify("/nope").body(), not_found.as_slice());
    assert_eq!(env.verify(&format!("/p/{id}")).status_code().as_u16(), 200);

    upgrade(Some(vec![pin("boot.mjs", 'C')])).unwrap();
    let case = String::from_utf8(env.verify("/v/abc").body().to_vec()).unwrap();
    assert!(case.contains(&format!("integrity=\"sha384-{}\"", "C".repeat(64))));
    assert!(!case.contains("skiko.wasm"));

    for bad in [
        Some(Vec::new()),
        Some(vec![pin("skiko.wasm", 'W')]),
        Some(vec![pin("boot.mjs", 'B'), pin("../x.js", 'X')]),
        Some(vec![witness_pages::pins::Pin {
            path: "boot.mjs".into(),
            sri: "sha256-abc".into(),
        }]),
    ] {
        assert!(upgrade(bad).is_err());
    }
    let case_after = env.verify("/v/abc").body().to_vec();
    assert_eq!(case_after, case.as_bytes());

    upgrade(None).unwrap();
    assert!(env.status().flags.contains(&"case-unpinned".to_string()));
    let case = String::from_utf8(env.verify("/v/abc").body().to_vec()).unwrap();
    assert!(!case.contains("importmap") && !case.contains("integrity"));
}
