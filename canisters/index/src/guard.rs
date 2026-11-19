use candid::Principal;
use witness_canister_common::PeerInit;
use witness_canister_common::inspect::{MethodLimit, limit};
use witness_canister_common::limits::{
    CANDID_OVERHEAD_BYTES, MAX_FINGERPRINTS_BYTES, MAX_PROBE_EXACT, MAX_PROBE_PDQ,
    MAX_PROBE_WATERMARKS, MAX_SUBMIT_BATCH,
};

const POSTING_OVERHEAD_BYTES: usize = 256;
const SMALL_ARG_BYTES: usize = 64;
const PUT_BYTES: usize =
    MAX_SUBMIT_BATCH * (MAX_FINGERPRINTS_BYTES + POSTING_OVERHEAD_BYTES) + CANDID_OVERHEAD_BYTES;
const PROBE_BYTES: usize =
    (MAX_PROBE_PDQ + MAX_PROBE_EXACT) * 40 + MAX_PROBE_WATERMARKS * 8 + CANDID_OVERHEAD_BYTES;

pub const LIMITS: &[MethodLimit] = &[
    limit("put", PUT_BYTES),
    limit("put_instructions", PUT_BYTES),
    limit("erase", SMALL_ARG_BYTES),
    limit("probe", PROBE_BYTES),
    limit("probe_instructions", PROBE_BYTES),
    limit("fingerprints", SMALL_ARG_BYTES),
    limit("derivatives", SMALL_ARG_BYTES),
    limit("status", SMALL_ARG_BYTES),
];

pub fn registry_method(method: &str) -> bool {
    matches!(method, "put" | "put_instructions" | "erase")
}

pub fn is_registry(config: &PeerInit, caller: Principal) -> bool {
    config.is_configured() && config.registry == caller
}

pub fn caller_allowed(config: &PeerInit, caller: Principal, method: &str) -> bool {
    !registry_method(method) || is_registry(config, caller)
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_canister_common::arg_allowed;

    fn config() -> PeerInit {
        PeerInit {
            registry: Principal::from_slice(&[7; 10]),
            shard: 0,
            origins: None,
        }
    }

    #[test]
    fn registry_methods_need_the_registry() {
        let c = config();
        for m in ["put", "erase", "put_instructions"] {
            assert!(registry_method(m));
            assert!(caller_allowed(&c, c.registry, m));
            assert!(!caller_allowed(&c, Principal::anonymous(), m));
            assert!(!caller_allowed(&c, Principal::from_slice(&[8; 10]), m));
        }
    }

    #[test]
    fn public_methods_open_to_all() {
        let c = config();
        for m in ["probe", "fingerprints", "derivatives", "status"] {
            assert!(!registry_method(m));
            assert!(caller_allowed(&c, Principal::anonymous(), m));
        }
    }

    #[test]
    fn unconfigured_canister_trusts_nobody() {
        let c = PeerInit::default();
        assert!(!is_registry(&c, Principal::anonymous()));
        assert!(!caller_allowed(&c, Principal::anonymous(), "put"));
    }

    #[test]
    fn method_names_match_exactly() {
        assert!(!registry_method("Put"));
        assert!(!registry_method("put "));
        assert!(!registry_method(""));
    }

    #[test]
    fn limits_cover_every_method_and_reject_unknown() {
        for l in LIMITS {
            assert!(arg_allowed(LIMITS, None, l.method, l.max_arg_bytes));
            assert!(!arg_allowed(LIMITS, None, l.method, l.max_arg_bytes + 1));
        }
        assert!(!arg_allowed(LIMITS, None, "unknown", 0));
        const { assert!(PUT_BYTES < witness_canister_common::limits::MAX_INGRESS_BYTES) };
        const { assert!(PROBE_BYTES > (MAX_PROBE_PDQ + MAX_PROBE_EXACT) * 32) };
    }
}
