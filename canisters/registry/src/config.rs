use core::time::Duration;
use witness_attest::Policy;
use witness_attest::policy::DAY_NS;

pub const SCHNORR_KEY: &str = match option_env!("WITNESS_SCHNORR_KEY") {
    Some(k) => k,
    None => "dfx_test_key",
};

pub const RELEASE_DIGEST: [u8; 32] = hex32_or(option_env!("WITNESS_RELEASE_DIGEST"), [0; 32]);
pub const DEBUG_DIGEST: [u8; 32] = hex32_or(option_env!("WITNESS_DEBUG_DIGEST"), [0xd1; 32]);

#[cfg(feature = "dev-attestation")]
pub const DIGESTS: &[[u8; 32]] = &[RELEASE_DIGEST, DEBUG_DIGEST];
#[cfg(not(feature = "dev-attestation"))]
pub const DIGESTS: &[[u8; 32]] = &[RELEASE_DIGEST];

pub const POLICY: Policy<'static> = Policy::witness(DIGESTS);

pub const ENROLL_FREE: u64 = 5;
pub const ENROLL_WINDOW_NS: u64 = 30 * DAY_NS;
pub const BACKOFF_BASE_NS: u64 = DAY_NS / 24;
pub const BACKOFF_MAX_SHIFT: u64 = 10;
pub const SUBMITS_PER_DAY: u64 = 2_000;
pub const ROOT_SUBJECT: &str = "CN=Witness Root CA,O=Witness";
pub const LEAF_SUBJECT: &str = "CN=Witness Device Key,O=Witness";
pub const ROOT_SERIAL: u64 = 1;
pub const ROOT_VALIDITY_S: u64 = 50 * 365 * 86_400;
pub const LEAF_VALIDITY_S: u64 = 10 * 365 * 86_400;
pub const DRAIN_PERIOD: Duration = Duration::from_secs(5);
pub const PUBLISH_PERIOD: Duration = Duration::from_secs(3_600);
pub const BOOTSTRAP_RETRY: Duration = Duration::from_secs(60);
pub const OUTBOX_SCAN: usize = 256;
pub const OUTBOX_TOTAL: usize = 1_024;
pub const PENDING_TTL_NS: u64 = 10 * 60 * 1_000_000_000;
pub const OUTBOX_BATCH: usize = 64;
pub const MAX_ENROLL_ARG: usize = 32 * 1024;
pub const MAX_ERASE_ARG: usize = 1024;

const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("digest must be hex"),
    }
}

pub const fn hex32(s: &str) -> [u8; 32] {
    let b = s.as_bytes();
    assert!(b.len() == 64, "digest must be 64 hex chars");
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (nibble(b[2 * i]) << 4) | nibble(b[2 * i + 1]);
        i += 1;
    }
    out
}

pub const fn hex32_or(s: Option<&str>, fallback: [u8; 32]) -> [u8; 32] {
    match s {
        Some(s) => hex32(s),
        None => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex32_parses_both_cases() {
        let s = "00ff10AbcdEF0123456789abcdef0123456789abcdef0123456789abcdef0123";
        let out = hex32(s);
        assert_eq!(&out[..4], &[0x00, 0xff, 0x10, 0xab]);
        assert_eq!(out[31], 0x23);
        assert_eq!(hex32(s), hex32(&s.to_ascii_lowercase()));
    }

    #[test]
    #[should_panic(expected = "64 hex")]
    fn hex32_rejects_wrong_length() {
        hex32("abcd");
    }

    #[test]
    #[should_panic(expected = "must be hex")]
    fn hex32_rejects_non_hex() {
        hex32(&"zz".repeat(32));
    }

    #[test]
    fn hex32_or_falls_back() {
        assert_eq!(hex32_or(None, [7; 32]), [7; 32]);
        assert_eq!(hex32_or(Some(&"01".repeat(32)), [7; 32]), [1; 32]);
    }

    #[test]
    fn policy_is_compiled_in() {
        assert_eq!(POLICY.release_digests, DIGESTS);
        assert!(DIGESTS.contains(&RELEASE_DIGEST));
        assert_eq!(POLICY.active_ns, 60 * DAY_NS);
        assert_eq!(cfg!(feature = "dev-attestation"), DIGESTS.len() == 2);
    }

    #[test]
    fn ca_validity_outlives_leaves() {
        const { assert!(ROOT_VALIDITY_S > 4 * LEAF_VALIDITY_S) };
        assert!(!SCHNORR_KEY.is_empty());
    }
}
