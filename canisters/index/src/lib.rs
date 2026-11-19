mod buckets;
mod guard;
mod multimap;
mod row;
mod store;
#[cfg(test)]
mod testkit;
mod trace;

pub use buckets::{BLOCK_BYTES, BLOCK_POSTINGS, Buckets};
pub use guard::{LIMITS, caller_allowed, is_registry, registry_method};
pub use multimap::Multimap;
pub use row::{ROW_BYTES, ROW_SHAPES, Row};
pub use store::{
    Index, MAX_DERIVATIVE_HITS, MAX_PDQ_DISTANCE, MAX_PDQ_HITS, Stats, TOP_CANDIDATES,
};
pub use trace::{TRACE_BYTES, Trace};

use ic_stable_structures::StableCell;
use serde_bytes::ByteBuf;
use std::cell::RefCell;
use witness_canister_common::memory::CONFIG;
use witness_canister_common::{
    Fingerprints, Memory, PeerInit, Posting, ProbeResult, Status, memory, runtime,
};

thread_local! {
    static INDEX: RefCell<Index<Memory>> = RefCell::new(Index::init(memory));
    static PEERS: RefCell<StableCell<PeerInit, Memory>> =
        RefCell::new(StableCell::init(memory(CONFIG), PeerInit::default()));
}

fn configure(args: PeerInit) {
    PEERS.with_borrow_mut(|c| c.set(args));
    INDEX.with_borrow(|_| ());
}

fn config() -> PeerInit {
    PEERS.with_borrow(|c| c.get().clone())
}

fn registry_only() {
    if !is_registry(&config(), ic_cdk::api::msg_caller()) {
        ic_cdk::trap("caller is not the registry");
    }
}

fn run_probe(pdq: &[ByteBuf], wm: &[u64], exact: &[ByteBuf]) -> ProbeResult {
    INDEX
        .with_borrow(|i| i.probe(pdq, wm, exact))
        .unwrap_or_else(|| ic_cdk::trap("malformed probe"))
}

fn run_put(postings: &[Posting]) {
    registry_only();
    INDEX.with_borrow_mut(|i| i.put_all(postings));
}

#[ic_cdk::init]
fn init(args: PeerInit) {
    configure(args);
}

#[ic_cdk::post_upgrade]
fn post_upgrade(args: PeerInit) {
    configure(args);
}

#[ic_cdk::inspect_message]
fn inspect_message() {
    let method = ic_cdk::api::msg_method_name();
    if caller_allowed(&config(), ic_cdk::api::msg_caller(), &method) {
        runtime::inspect(LIMITS, None);
    }
}

#[ic_cdk::update]
fn put(postings: Vec<Posting>) {
    run_put(&postings);
}

#[ic_cdk::update]
fn erase(record_ix: u64) {
    registry_only();
    INDEX.with_borrow_mut(|i| i.erase(record_ix));
}

#[ic_cdk::query]
fn probe(pdq: Vec<ByteBuf>, watermarks: Vec<u64>, exact: Vec<ByteBuf>) -> ProbeResult {
    run_probe(&pdq, &watermarks, &exact)
}

#[ic_cdk::query]
fn fingerprints(record_ix: u64) -> Option<Fingerprints> {
    INDEX.with_borrow(|i| i.fingerprints(record_ix))
}

#[ic_cdk::query]
fn derivatives(root_ix: u64) -> Vec<u64> {
    INDEX.with_borrow(|i| i.derivatives(root_ix))
}

#[ic_cdk::query]
fn status() -> Status {
    let stats = INDEX.with_borrow(Index::stats);
    let peers = config();
    let metrics = vec![
        ("records".to_string(), stats.records),
        ("rows".to_string(), stats.rows),
        ("posting_blocks".to_string(), stats.blocks),
        ("indexed_upto".to_string(), stats.indexed_upto),
        ("shard".to_string(), u64::from(peers.shard)),
    ];
    let flags = if peers.is_configured() {
        Vec::new()
    } else {
        vec!["unconfigured".to_string()]
    };
    runtime::status(env!("CARGO_PKG_VERSION"), metrics, flags)
}

#[cfg(feature = "bench")]
#[ic_cdk::query]
fn probe_instructions(pdq: Vec<ByteBuf>, watermarks: Vec<u64>, exact: Vec<ByteBuf>) -> u64 {
    let start = ic_cdk::api::performance_counter(0);
    std::hint::black_box(run_probe(&pdq, &watermarks, &exact));
    ic_cdk::api::performance_counter(0) - start
}

#[cfg(feature = "bench")]
#[ic_cdk::update]
fn put_instructions(postings: Vec<Posting>) -> u64 {
    let start = ic_cdk::api::performance_counter(0);
    run_put(&postings);
    ic_cdk::api::performance_counter(0) - start
}

ic_cdk::export_candid!();

#[cfg(test)]
mod tests {
    use candid_parser::utils::{CandidSource, service_compatible};

    #[test]
    fn exports_the_index_peer_methods_the_registry_calls() {
        let peer = witness_canister_common::peer::peer_did("IndexPeer");
        assert!(
            service_compatible(
                CandidSource::Text(&super::__export_service()),
                CandidSource::Text(&peer)
            )
            .is_ok()
        );
    }

    #[test]
    fn index_did_matches_exported_interface() {
        let exported: String = super::__export_service()
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .map(|l| format!("{l}\n"))
            .collect();
        if !cfg!(feature = "bench") {
            assert_eq!(exported, include_str!("../index.did"));
        }
    }
}
