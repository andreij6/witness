pub mod certtree;
pub mod http;
pub mod pagestore;
pub mod paths;
pub mod policy;
pub mod records;
pub mod store;

use crate::http::{
    HttpResponse, Served, StreamingCallback, StreamingCallbackHttpResponse, StreamingToken,
    blob_response, chunk_len, chunk_token, header, not_found, plain,
};
use crate::policy::{EXPIRY_BATCH, EXPIRY_PERIOD, GRANT_SIGNER, LOCAL_ORIGINS};
use crate::store::{Allow, Begun, Ctx, Media};
use ic_cdk::api::{canister_self, msg_caller, time};
use ic_http_certification::{HttpRequest, Method};
use ic_stable_structures::StableCell;
use std::cell::RefCell;
use witness_canister_common::inspect::limit;
use witness_canister_common::limits::{CANDID_OVERHEAD_BYTES, MAX_CHUNK_BYTES};
use witness_canister_common::memory::{CONFIG, Memory, memory};
use witness_canister_common::{
    BeginUpload, CaseInfo, Chunk, MediaErr, MethodLimit, Origins, PeerInit, Status, Tick,
    TimerSpec, runtime,
};

pub const DID: &str = include_str!("../media.did");
const CALLBACK: &str = "http_request_streaming_callback";

const LIMITS: &[MethodLimit] = &[
    limit("begin_upload", 2 * 1024),
    limit("put_chunk", MAX_CHUNK_BYTES + CANDID_OVERHEAD_BYTES),
    limit("commit", 64),
    limit("count_checks", 4 * 1024),
    limit("sync", 128 * 1024),
    limit("erase", 64),
];

const REGISTRY_ONLY: [&str; 2] = ["sync", "erase"];

const TIMERS: &[TimerSpec] = &[TimerSpec {
    name: "expiry",
    period: EXPIRY_PERIOD,
    tick: expiry_tick,
}];

thread_local! {
    static CONFIG_CELL: RefCell<StableCell<PeerInit, Memory>> =
        RefCell::new(StableCell::init(memory(CONFIG), PeerInit::default()));
    static MEDIA: RefCell<Media<Memory>> = RefCell::new(Media::init(memory));
    static ORIGINS: RefCell<Origins> = RefCell::new(CONFIG_CELL.with_borrow(|c| c.get().origins()));
}

fn config() -> PeerInit {
    CONFIG_CELL.with_borrow(|c| c.get().clone())
}

fn certify(m: &Media<Memory>) {
    runtime::set_certified_root(&m.root_hash());
}

fn mutate<T>(f: impl FnOnce(&mut Media<Memory>) -> T) -> T {
    MEDIA.with_borrow_mut(|m| {
        let out = f(m);
        certify(m);
        out
    })
}

fn expiry_tick() -> Tick {
    Box::pin(async {
        mutate(|m| m.expire(time(), EXPIRY_BATCH));
    })
}

fn is_registry(config: &PeerInit, caller: candid::Principal) -> bool {
    config.is_configured() && caller == config.registry
}

fn require_registry() {
    if !is_registry(&config(), msg_caller()) {
        ic_cdk::trap("registry only");
    }
}

fn ctx() -> Ctx {
    Ctx {
        now_ns: time(),
        caller: msg_caller(),
        shard: config().shard,
        signer: GRANT_SIGNER,
    }
}

fn checked_origins(args: &PeerInit, local: bool) -> Result<Origins, String> {
    let origins = args.origins();
    origins.validate()?;
    if !local && !origins.is_secure() {
        return Err("origins must be https outside local-dev builds".into());
    }
    Ok(origins)
}

fn setup(args: PeerInit) {
    let origins = checked_origins(&args, LOCAL_ORIGINS).unwrap_or_else(|e| ic_cdk::trap(e));
    ORIGINS.set(origins);
    CONFIG_CELL.with_borrow_mut(|c| c.set(args));
    MEDIA.with_borrow(certify);
    runtime::start_timers(TIMERS);
}

#[ic_cdk::init]
fn init(args: PeerInit) {
    setup(args);
}

#[ic_cdk::post_upgrade]
fn post_upgrade(args: PeerInit) {
    setup(args);
}

#[ic_cdk::inspect_message]
fn inspect_message() {
    let method = ic_cdk::api::msg_method_name();
    if REGISTRY_ONLY.contains(&method.as_str()) && !is_registry(&config(), msg_caller()) {
        return;
    }
    runtime::inspect(LIMITS, None);
}

#[ic_cdk::update]
fn begin_upload(req: BeginUpload) -> Result<Begun, MediaErr> {
    let c = ctx();
    MEDIA.with_borrow_mut(|m| m.begin(&req, &c))
}

#[ic_cdk::update]
fn put_chunk(chunk: Chunk) -> Result<(), MediaErr> {
    MEDIA.with_borrow_mut(|m| m.put_chunk(&chunk, &msg_caller(), time()))
}

#[ic_cdk::update]
fn commit(upload: u64) -> Result<String, MediaErr> {
    mutate(|m| m.commit(upload, time()))
}

#[ic_cdk::update]
fn erase(record_id: String) -> Result<u32, MediaErr> {
    require_registry();
    mutate(|m| m.erase_record(&record_id))
}

#[ic_cdk::update]
fn sync(items: Vec<Allow>) -> u32 {
    require_registry();
    MEDIA.with_borrow_mut(|m| m.sync(&items))
}

#[ic_cdk::update]
fn count_checks(ids: Vec<String>) -> u32 {
    MEDIA.with_borrow_mut(|m| m.count_checks(&ids, time()))
}

#[ic_cdk::query]
fn case_info(case_id: String) -> Option<CaseInfo> {
    MEDIA.with_borrow(|m| m.case_info(&case_id))
}

#[ic_cdk::query]
fn checks(ids: Vec<String>) -> Vec<u64> {
    MEDIA.with_borrow(|m| m.checks(&ids))
}

#[ic_cdk::query]
fn http_request(req: HttpRequest<'static>) -> HttpResponse {
    if req.method() != Method::GET {
        return plain(405, "method not allowed");
    }
    MEDIA.with_borrow(|m| {
        let found = m.lookup(req.url()).and_then(|info| {
            Some((
                info,
                m.witness(&info.path)?,
                m.read(info.blob, 0, chunk_len(info.size, 0))?,
            ))
        });
        let Some((info, witness, body)) = found else {
            let path = req.get_path().unwrap_or_else(|_| "/".into());
            return not_found(&m.absence_witness(&path), &runtime::certificate());
        };
        ORIGINS.with_borrow(|origins| {
            blob_response(Served {
                path: &info.path,
                blob: info.blob,
                size: info.size,
                body,
                witness: &witness,
                certificate: &runtime::certificate(),
                origin: header(req.headers(), "origin"),
                origins,
                local: LOCAL_ORIGINS,
                callback: Some(StreamingCallback::new(canister_self(), CALLBACK.into())),
            })
        })
    })
}

#[ic_cdk::query]
fn http_request_streaming_callback(token: StreamingToken) -> StreamingCallbackHttpResponse {
    MEDIA.with_borrow(|m| {
        let size = m.blob_size(token.blob).unwrap_or(0);
        let len = chunk_len(size, token.offset);
        StreamingCallbackHttpResponse {
            body: m.read(token.blob, token.offset, len).unwrap_or_default(),
            token: chunk_token(token.blob, size, token.offset, len),
        }
    })
}

#[ic_cdk::query]
fn status() -> Status {
    let mut flags = Vec::new();
    if LOCAL_ORIGINS {
        flags.push("local-dev".to_owned());
    }
    if GRANT_SIGNER.is_none() {
        flags.push("plus-disabled".to_owned());
    }
    if !config().is_configured() {
        flags.push("unconfigured".to_owned());
    }
    runtime::status(
        env!("CARGO_PKG_VERSION"),
        MEDIA.with_borrow(|m| m.metrics()),
        flags,
    )
}

ic_cdk::export_candid!();

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(did: &str) -> String {
        did.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned()
    }

    #[test]
    fn candid_file_matches_exported_interface() {
        assert_eq!(strip(&__export_service()), strip(DID));
    }

    #[test]
    fn exported_interface_is_compatible_with_did_file() {
        let new = candid_parser::utils::CandidSource::Text(&__export_service());
        let old = candid_parser::utils::CandidSource::Text(DID);
        assert!(candid_parser::utils::service_compatible(new, old).is_ok());
    }

    #[test]
    fn limits_cover_every_update() {
        let names: Vec<&str> = LIMITS.iter().map(|l| l.method).collect();
        assert_eq!(
            names,
            [
                "begin_upload",
                "put_chunk",
                "commit",
                "count_checks",
                "sync",
                "erase"
            ]
        );
        assert!(REGISTRY_ONLY.iter().all(|m| names.contains(m)));
        assert!(!witness_canister_common::arg_allowed(
            LIMITS, None, "status", 1
        ));
        assert!(!witness_canister_common::arg_allowed(
            LIMITS, None, "erase", 65
        ));
        assert!(witness_canister_common::arg_allowed(
            LIMITS,
            None,
            "put_chunk",
            MAX_CHUNK_BYTES + 100
        ));
    }

    #[test]
    fn unconfigured_canister_trusts_no_registry() {
        let anon = candid::Principal::anonymous();
        let reg = candid::Principal::from_slice(&[7; 10]);
        assert!(!is_registry(&PeerInit::default(), anon));
        let c = PeerInit {
            registry: reg,
            shard: 0,
            origins: None,
        };
        assert!(is_registry(&c, reg));
        assert!(!is_registry(&c, anon));
        assert!(!is_registry(&c, candid::Principal::from_slice(&[8; 10])));
    }

    #[test]
    fn exports_the_media_peer_methods_the_registry_calls() {
        let peer = witness_canister_common::peer::peer_did("MediaPeer");
        assert!(
            candid_parser::utils::service_compatible(
                candid_parser::utils::CandidSource::Text(&__export_service()),
                candid_parser::utils::CandidSource::Text(&peer)
            )
            .is_ok()
        );
    }

    #[test]
    fn origins_are_checked_at_install() {
        let local = Origins {
            site: "http://pages.localhost:8090".into(),
            case: "http://pages.localhost:8090".into(),
            app: "http://web.localhost:8090".into(),
            api: "http://localhost:8090".into(),
            canister: "http://*.localhost:8090".into(),
        };
        let with = |o: Option<Origins>| PeerInit {
            registry: candid::Principal::from_slice(&[7; 10]),
            shard: 0,
            origins: o,
        };
        assert_eq!(checked_origins(&with(None), false), Ok(Origins::default()));
        assert_eq!(
            checked_origins(&with(Some(local.clone())), true),
            Ok(local.clone())
        );
        assert!(checked_origins(&with(Some(local.clone())), false).is_err());
        let bad = Origins {
            site: "https://witness.app/x".into(),
            ..Origins::default()
        };
        assert!(checked_origins(&with(Some(bad)), true).is_err());
    }

    #[test]
    fn timers_are_valid() {
        assert_eq!(witness_canister_common::validate_specs(TIMERS), Ok(()));
        assert_eq!(TIMERS[0].period.as_secs(), 600);
    }
}
