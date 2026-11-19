pub mod api;
pub mod cert;
pub mod critbit;
pub mod html;
pub mod http;
pub mod pins;
pub mod site;
pub mod store;
pub mod view;

pub use api::MAX_BATCH;
pub use witness_canister_common::{PageErr, PageResult, Removal, Shell};

use ic_http_certification::{HttpRequest, HttpResponse};
use ic_stable_structures::StableCell;
use pins::{Pin, Pins};
use site::Site;
use std::cell::RefCell;
use store::Store;
use witness_canister_common::inspect::limit;
use witness_canister_common::memory::{self, Memory};
use witness_canister_common::{MethodLimit, Origins, PeerInit, Status, limits, runtime};

thread_local! {
    static CONFIG: RefCell<StableCell<PeerInit, Memory>> =
        RefCell::new(StableCell::init(memory::memory(memory::CONFIG), PeerInit::default()));
    static STORE: RefCell<Store<Memory>> = RefCell::new(Store::init(
        memory::memory(memory::pages::SHELLS),
        memory::memory(memory::pages::CERT_TREE),
        memory::memory(memory::pages::STATE),
    ));
    static SITE: RefCell<Site> = RefCell::new(Site::new(CONFIG.with_borrow(|c| c.get().origins()), &Pins::default()));
    static PINNED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

const LOCAL_ORIGINS: bool = cfg!(feature = "local-dev");

fn checked_origins(args: &PeerInit, local: bool) -> Result<Origins, String> {
    let origins = args.origins();
    origins.validate()?;
    if !local && !origins.is_secure() {
        return Err("origins must be https outside local-dev builds".into());
    }
    Ok(origins)
}

fn configure(args: PeerInit, pins: Option<Vec<Pin>>) {
    let origins = checked_origins(&args, LOCAL_ORIGINS).unwrap_or_else(|e| ic_cdk::trap(e));
    let pins = Pins::from_arg(pins).unwrap_or_else(|e| ic_cdk::trap(e));
    let changed = CONFIG.with_borrow(|c| c.get().origins()) != origins;
    CONFIG.with_borrow_mut(|c| c.set(args));
    PINNED.set(!pins.is_empty());
    SITE.set(Site::new(origins, &pins));
    if changed {
        SITE.with_borrow(|site| STORE.with_borrow_mut(|s| api::rehash(s, site)));
    }
    certify();
}

fn certify() {
    let root = SITE.with_borrow(|site| STORE.with_borrow(|s| site.certified_root(s.root_hash())));
    runtime::set_certified_root(&root);
}

const INGRESS: &[MethodLimit] = &[
    limit("put_shell", limits::MAX_INGRESS_BYTES),
    limit("tombstone", 64 * 1024),
];

fn caller_is_registry() -> bool {
    let caller = ic_cdk::api::msg_caller();
    CONFIG.with_borrow(|c| c.get().is_configured() && c.get().registry == caller)
}

fn require_registry(batch: usize) {
    if !caller_is_registry() {
        ic_cdk::trap("caller is not the registry");
    }
    if batch > MAX_BATCH {
        ic_cdk::trap("batch too large");
    }
}

#[ic_cdk::init]
fn init(args: PeerInit, pins: Option<Vec<Pin>>) {
    configure(args, pins);
}

#[ic_cdk::post_upgrade]
fn post_upgrade(args: PeerInit, pins: Option<Vec<Pin>>) {
    configure(args, pins);
}

#[ic_cdk::inspect_message]
fn inspect_message() {
    if caller_is_registry() {
        runtime::inspect(INGRESS, None);
    }
}

#[ic_cdk::update]
fn put_shell(inputs: Vec<Shell>) -> Vec<PageResult> {
    require_registry(inputs.len());
    let out = SITE.with_borrow(|site| {
        STORE.with_borrow_mut(|s| inputs.iter().map(|i| api::put_shell(s, site, i)).collect())
    });
    certify();
    out
}

#[ic_cdk::update]
fn tombstone(record_ids: Vec<String>, why: Removal) -> Vec<PageResult> {
    require_registry(record_ids.len());
    let out = SITE.with_borrow(|site| {
        STORE.with_borrow_mut(|s| {
            record_ids
                .iter()
                .map(|id| api::tombstone(s, site, id, why))
                .collect()
        })
    });
    certify();
    out
}

#[ic_cdk::query]
fn http_request(req: HttpRequest<'static>) -> HttpResponse<'static> {
    let path = req.get_path().unwrap_or_else(|_| "/".to_string());
    let certificate = runtime::certificate();
    SITE.with_borrow(|site| STORE.with_borrow(|s| site::serve(site, s, &path, &certificate)))
}

#[ic_cdk::query]
fn status() -> Status {
    let metrics = STORE.with_borrow(|s| {
        vec![
            ("pages".to_string(), s.len()),
            ("tree_nodes".to_string(), s.node_count()),
        ]
    });
    let mut flags = Vec::new();
    if !CONFIG.with_borrow(|c| c.get().is_configured()) {
        flags.push("unconfigured".to_string());
    }
    if !PINNED.get() {
        flags.push("case-unpinned".to_string());
    }
    runtime::status(env!("CARGO_PKG_VERSION"), metrics, flags)
}

ic_cdk::export_candid!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_origins_need_a_local_dev_build() {
        let local = Origins {
            site: "http://pages.localhost:8090".into(),
            case: "http://pages.localhost:8090".into(),
            app: "http://web.localhost:8090".into(),
            api: "http://localhost:8090".into(),
            canister: "http://*.localhost:8090".into(),
        };
        let secure = Origins {
            app: "https://app.witness.app".into(),
            ..Origins::default()
        };
        let with = |o: Option<Origins>| PeerInit {
            registry: candid::Principal::from_slice(&[7; 10]),
            shard: 0,
            origins: o,
        };
        assert_eq!(checked_origins(&with(None), false), Ok(Origins::default()));
        assert_eq!(
            checked_origins(&with(Some(secure.clone())), false),
            Ok(secure)
        );
        assert_eq!(
            checked_origins(&with(Some(local.clone())), true),
            Ok(local.clone())
        );
        assert!(checked_origins(&with(Some(local.clone())), false).is_err());
        let mixed = Origins {
            app: "http://web.localhost:8090".into(),
            ..Origins::default()
        };
        assert!(checked_origins(&with(Some(mixed)), false).is_err());
        let bad = Origins {
            site: "https://witness.app/x".into(),
            ..Origins::default()
        };
        assert!(checked_origins(&with(Some(bad)), true).is_err());
        assert_eq!(LOCAL_ORIGINS, cfg!(feature = "local-dev"));
    }

    #[test]
    fn init_args_decode_with_or_without_pins() {
        use crate::pins::tests::{SRI_A, pin};
        let init = PeerInit {
            registry: candid::Principal::from_slice(&[7; 10]),
            shard: 0,
            origins: None,
        };
        let bytes = candid::encode_one(&init).unwrap();
        let (back, pins): (PeerInit, Option<Vec<Pin>>) = candid::decode_args(&bytes).unwrap();
        assert_eq!((back.shard, pins), (0, None));
        let with = Some(vec![pin(pins::BOOT, SRI_A)]);
        let bytes = candid::encode_args((&init, &with)).unwrap();
        let (_, pins): (PeerInit, Option<Vec<Pin>>) = candid::decode_args(&bytes).unwrap();
        assert_eq!(Pins::from_arg(pins).unwrap().boot(), Some(SRI_A));
    }

    #[test]
    fn pages_did_matches_exported_interface() {
        let exported: String = __export_service()
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_eq!(exported, include_str!("../pages.did"));
    }

    #[test]
    fn exports_the_pages_peer_methods_the_registry_calls() {
        use candid_parser::utils::{CandidSource, service_compatible};
        let peer = witness_canister_common::peer::peer_did("PagesPeer");
        assert!(
            service_compatible(
                CandidSource::Text(&__export_service()),
                CandidSource::Text(&peer)
            )
            .is_ok()
        );
    }

    #[test]
    fn ingress_limits_cover_update_methods_only() {
        let names: Vec<&str> = INGRESS.iter().map(|l| l.method).collect();
        assert_eq!(names, ["put_shell", "tombstone"]);
        assert!(witness_canister_common::arg_allowed(
            INGRESS,
            None,
            "put_shell",
            1024
        ));
        assert!(!witness_canister_common::arg_allowed(
            INGRESS,
            None,
            "http_request",
            1
        ));
    }
}
