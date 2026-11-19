use candid::types::internal::TypeContainer;
use candid::types::subtype::{Gamma, equal};
use candid::{CandidType, TypeEnv};
use candid_parser::utils::CandidSource;
use witness_canister_common::*;

fn did_env() -> TypeEnv {
    CandidSource::Text(DID)
        .load()
        .unwrap_or_else(|e| panic!("common.did does not parse: {e}"))
        .0
}

fn assert_matches<T: CandidType>(name: &str) {
    let did = did_env();
    let expected = did
        .find_type(name)
        .unwrap_or_else(|e| panic!("{name} missing from common.did: {e}"))
        .clone();
    let mut c = TypeContainer::new();
    let actual = c.add::<T>();
    let mut env = did.clone();
    let actual = env.merge_type(c.env, actual);
    equal(&mut Gamma::new(), &env, &actual, &expected)
        .unwrap_or_else(|e| panic!("{name}: Rust type differs from common.did: {e}"));
}

macro_rules! contract {
    ($($t:ident),+ $(,)?) => {
        #[test]
        fn rust_types_match_common_did() {
            $(assert_matches::<$t>(stringify!($t));)+
        }

        #[test]
        fn every_did_type_has_a_rust_twin() {
            let names: &[&str] = &[$(stringify!($t)),+];
            let did = did_env();
            let mut missing: Vec<&str> = did.0.keys().map(String::as_str).filter(|k| !names.contains(k)).collect();
            missing.sort_unstable();
            assert!(missing.is_empty(), "no Rust type for {missing:?}");
        }
    };
}

contract!(
    Security,
    Prov,
    Fresh,
    KeyStatus,
    StatusChange,
    KeyInfo,
    Submission,
    Anchored,
    SubmitErr,
    SubmitResult,
    CertifiedRecord,
    CertifiedKey,
    ShardInfo,
    Shards,
    EraseAuth,
    EraseRequest,
    Posting,
    PdqHit,
    ProbeResult,
    Fingerprints,
    Tier,
    EntitlementGrant,
    UploadTarget,
    BeginUpload,
    CaseInfo,
    Chunk,
    MediaErr,
    TimerStatus,
    Status,
    RegistryInit,
    PeerInit,
    Origins,
    Shell,
    Removal,
    PageErr,
    PageResult,
    Allow,
    IndexPeer,
    PagesPeer,
    MediaPeer,
);

#[test]
fn mismatched_type_is_detected() {
    let did = did_env();
    let expected = did
        .find_type("Anchored")
        .cloned()
        .unwrap_or_else(|e| panic!("{e}"));
    let mut c = TypeContainer::new();
    let actual = c.add::<ShardInfo>();
    let mut env = did.clone();
    let actual = env.merge_type(c.env, actual);
    assert!(equal(&mut Gamma::new(), &env, &actual, &expected).is_err());
}

#[test]
fn mismatched_service_is_detected() {
    let did = did_env();
    let expected = did
        .find_type("PagesPeer")
        .cloned()
        .unwrap_or_else(|e| panic!("{e}"));
    let mut c = TypeContainer::new();
    let actual = c.add::<MediaPeer>();
    let mut env = did.clone();
    let actual = env.merge_type(c.env, actual);
    assert!(equal(&mut Gamma::new(), &env, &actual, &expected).is_err());
}
