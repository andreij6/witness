use crate::paths::Kind;
use core::time::Duration;

pub const SECOND_NS: u64 = 1_000_000_000;
pub const DAY_NS: u64 = 86_400 * SECOND_NS;
pub const MAX_FULL_BYTES: u64 = 64 * 1024 * 1024;
pub const PENDING_TTL_NS: u64 = DAY_NS;
pub const STALE_UPLOAD_NS: u64 = 15 * 60 * SECOND_NS;
pub const CASE_TTL_NS: u64 = 30 * DAY_NS;
pub const PLUS_GRACE_NS: u64 = 30 * DAY_NS;
pub const EXPIRY_PERIOD: Duration = Duration::from_secs(600);
pub const EXPIRY_PERIOD_NS: u64 = 600 * SECOND_NS;
pub const EXPIRY_BATCH: usize = 256;
pub const SYNC_RETRY_NS: u64 = 5 * SECOND_NS;
pub const MAX_SYNC_BATCH: usize = 512;
pub const MAX_SPKI_BYTES: usize = 256;
pub const MAX_CHECK_BATCH: usize = 32;
pub const CHECK_BUDGET: u32 = 100_000;
pub const PREVIEWS_PER_DAY: u32 = 50;
pub const OGS_PER_DAY: u32 = 50;
pub const CASES_PER_DAY: u32 = 20;

pub const LOCAL_DEV_SIGNER: [u8; 32] = [
    0xc8, 0x40, 0x69, 0xfa, 0x22, 0x16, 0xc6, 0xed, 0xc1, 0xd5, 0x2c, 0xa8, 0x3a, 0xa1, 0x1c, 0x26,
    0x7d, 0xd5, 0x44, 0x0a, 0xfa, 0x52, 0x60, 0x90, 0xca, 0x43, 0xeb, 0xd0, 0xca, 0x86, 0xa9, 0xcf,
];

pub const GRANT_SIGNER: Option<[u8; 32]> = if cfg!(feature = "local-dev") {
    Some(LOCAL_DEV_SIGNER)
} else {
    None
};

pub const LOCAL_ORIGINS: bool = cfg!(feature = "local-dev");

pub const fn daily_limit(kind: Kind) -> Option<u32> {
    match kind {
        Kind::Preview => Some(PREVIEWS_PER_DAY),
        Kind::Og => Some(OGS_PER_DAY),
        Kind::Case => Some(CASES_PER_DAY),
        Kind::Full => None,
    }
}

pub const fn day(now_ns: u64) -> u32 {
    (now_ns / DAY_NS) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_dev_signer_is_the_test_seed_key() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[0x57; 32]);
        assert_eq!(key.verifying_key().to_bytes(), LOCAL_DEV_SIGNER);
    }

    #[test]
    fn grant_signer_only_in_local_dev() {
        assert_eq!(GRANT_SIGNER.is_some(), cfg!(feature = "local-dev"));
        assert_eq!(LOCAL_ORIGINS, cfg!(feature = "local-dev"));
    }

    #[test]
    fn limits() {
        assert_eq!(daily_limit(Kind::Preview), Some(50));
        assert_eq!(daily_limit(Kind::Og), Some(50));
        assert_eq!(daily_limit(Kind::Case), Some(20));
        assert_eq!(daily_limit(Kind::Full), None);
        assert_eq!(EXPIRY_PERIOD.as_nanos() as u64, EXPIRY_PERIOD_NS);
        assert_eq!(STALE_UPLOAD_NS, 900 * SECOND_NS);
        const { assert!(STALE_UPLOAD_NS < PENDING_TTL_NS) };
        assert_eq!(
            MAX_FULL_BYTES % witness_canister_common::limits::MAX_CHUNK_BYTES as u64,
            0
        );
    }

    #[test]
    fn day_buckets() {
        assert_eq!(day(0), 0);
        assert_eq!(day(DAY_NS - 1), 0);
        assert_eq!(day(DAY_NS), 1);
        assert_eq!(day(u64::MAX), (u64::MAX / DAY_NS) as u32);
    }
}
