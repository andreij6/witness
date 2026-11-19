pub mod ca;
pub mod config;
pub mod enroll;
pub mod erase;
mod ic;
pub mod outbox;
pub mod proof;
pub mod quota;
pub mod read;
pub mod record;
pub mod state;
pub mod status;
pub mod store;
pub mod submit;
pub mod types;

use core::time::Duration;
use serde_bytes::ByteBuf;
use types::{EnrollResult, Enrolled, Enrollment, EraseResult};
use witness_canister_common::inspect::limit;
use witness_canister_common::limits::MAX_INGRESS_BYTES;
use witness_canister_common::{
    CertifiedKey, CertifiedRecord, EraseRequest, MethodLimit, RegistryInit, Shards, Status,
    Submission, SubmitResult, runtime,
};

const INSPECT: &[MethodLimit] = &[
    limit("submit", MAX_INGRESS_BYTES),
    limit("enroll", config::MAX_ENROLL_ARG),
    limit("erase_mine", config::MAX_ERASE_ARG),
];

fn now() -> u64 {
    ic_cdk::api::time()
}

fn setup(args: RegistryInit) {
    let roots = store::with(|s| {
        s.configure(args);
        s.roots()
    });
    ic::certify(&roots);
    runtime::start_timers(&ic::TIMERS);
    let missing = store::with(|s| {
        let st = s.state.get();
        st.secret().is_none() || st.ca_root().is_none()
    });
    if missing {
        ic::schedule_bootstrap(Duration::ZERO);
    }
}

#[ic_cdk::init]
fn init(args: RegistryInit) {
    setup(args);
}

#[ic_cdk::post_upgrade]
fn post_upgrade(args: RegistryInit) {
    setup(args);
}

#[ic_cdk::inspect_message]
fn inspect_message() {
    runtime::inspect(INSPECT, None);
}

#[ic_cdk::query]
fn challenge() -> ByteBuf {
    let secret =
        store::with(|s| s.state.get().secret()).unwrap_or_else(|| ic_cdk::trap("not ready"));
    ByteBuf::from(witness_attest::issue_fresh(&secret, now()).to_vec())
}

#[ic_cdk::query]
fn ca_root() -> Option<ByteBuf> {
    store::with(|s| s.state.get().ca_root().map(|r| ByteBuf::from(r.to_vec())))
}

#[ic_cdk::update]
async fn enroll(req: Enrollment) -> EnrollResult {
    let ticket = match store::with(|s| enroll::begin(s, &req, &config::POLICY, now())) {
        Ok(t) => t,
        Err(e) => return EnrollResult::Err(e),
    };
    let issued = ic::sign(ticket.tbs.clone())
        .await
        .and_then(|sig| ca::finish(&ticket.tbs, &sig));
    match issued {
        Ok(c2pa_cert) => {
            let (roots, ca_cert) = store::with(|s| {
                let roots = enroll::commit(s, &ticket);
                (roots, s.state.get().ca_root().unwrap_or_default().to_vec())
            });
            ic::certify(&roots);
            EnrollResult::Ok(Enrolled {
                key_id: ticket.info.key_id.as_bytes().to_vec(),
                c2pa_cert,
                ca_cert,
            })
        }
        Err(e) => {
            store::with(|s| enroll::rollback(s, &ticket));
            EnrollResult::Err(e)
        }
    }
}

#[ic_cdk::update]
fn submit(batch: Vec<Submission>) -> Vec<SubmitResult> {
    match store::with(|s| submit::submit(s, batch, now())) {
        Ok((results, roots)) => {
            ic::certify(&roots);
            results
        }
        Err(e) => ic_cdk::trap(e),
    }
}

#[ic_cdk::query]
fn get(record_id: String) -> Option<CertifiedRecord> {
    store::with(|s| read::record_by_id(s, &record_id, runtime::certificate()))
}

#[ic_cdk::query]
fn get_ix(record_ix: u64) -> Option<CertifiedRecord> {
    store::with(|s| read::record(s, record_ix, runtime::certificate()))
}

#[ic_cdk::query]
fn get_key(key_id: ByteBuf) -> Option<CertifiedKey> {
    store::with(|s| read::key(s, &key_id, runtime::certificate()))
}

#[ic_cdk::query]
fn by_capture(capture_id: u64) -> Vec<String> {
    store::with(|s| read::by_capture(s, capture_id))
}

#[ic_cdk::query]
fn shards() -> (Shards, ByteBuf) {
    store::with(|s| read::shards(s, &runtime::certificate()))
}

#[ic_cdk::query]
fn consistency(old_size: u64) -> Vec<ByteBuf> {
    store::with(|s| read::consistency(s, old_size)).unwrap_or_else(|e| ic_cdk::trap(e))
}

#[ic_cdk::update]
fn erase_mine(req: EraseRequest) -> EraseResult {
    store::with(|s| erase::erase_mine(s, &req, now())).into()
}

#[ic_cdk::query]
fn status() -> Status {
    let (metrics, flags) = store::with(|s| (status::metrics(s), status::flags(s)));
    runtime::status(env!("CARGO_PKG_VERSION"), metrics, flags)
}

ic_cdk::export_candid!();

#[cfg(test)]
mod did {
    #[test]
    fn registry_did_is_current() {
        assert_eq!(
            super::__export_service().trim(),
            include_str!("../registry.did").trim()
        );
    }

    #[test]
    fn inspect_limits_cover_every_update() {
        let did = include_str!("../registry.did");
        for line in did
            .lines()
            .filter(|l| l.contains("->") && !l.contains("query") && l.starts_with("  "))
        {
            let name = line.trim().split(' ').next().unwrap_or_default();
            assert!(
                super::INSPECT.iter().any(|l| l.method == name),
                "{name} not inspected"
            );
        }
    }
}
