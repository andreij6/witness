use hmac::{Hmac, Mac};
use sha2::Sha256;
use witness_format::crypto::{domain, hash_ds};

pub const FRESH_LEN: usize = 40;
pub const LATE_LEN: usize = 32;
pub const SECRET_LEN: usize = 32;

type HmacSha256 = Hmac<Sha256>;

fn mac(secret: &[u8; SECRET_LEN], issued: &[u8; 8]) -> HmacSha256 {
    let mut m = HmacSha256::new_from_slice(secret).expect("hmac accepts any key length");
    m.update(issued);
    m
}

pub fn issue_fresh(secret: &[u8; SECRET_LEN], now_ns: u64) -> [u8; FRESH_LEN] {
    let issued = now_ns.to_be_bytes();
    let mut out = [0u8; FRESH_LEN];
    out[..8].copy_from_slice(&issued);
    out[8..].copy_from_slice(&mac(secret, &issued).finalize().into_bytes());
    out
}

pub fn fresh_issued_ns(secret: &[u8; SECRET_LEN], challenge: &[u8]) -> Option<u64> {
    let challenge: &[u8; FRESH_LEN] = challenge.try_into().ok()?;
    let (issued, tag) = challenge.split_at(8);
    let issued: [u8; 8] = issued.try_into().ok()?;
    mac(secret, &issued).verify_slice(tag).ok()?;
    Some(u64::from_be_bytes(issued))
}

pub fn late_challenge(install_nonce: &[u8], wall_ms: u64) -> [u8; LATE_LEN] {
    hash_ds(
        domain::OFFLINE_CHALLENGE,
        &[install_nonce, &wall_ms.to_be_bytes()],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Rng;

    #[test]
    fn fresh_round_trips() {
        let mut rng = Rng::new(1);
        for _ in 0..500 {
            let secret = rng.bytes::<32>();
            let now = rng.next();
            let c = issue_fresh(&secret, now);
            assert_eq!(&c[..8], &now.to_be_bytes());
            assert_eq!(fresh_issued_ns(&secret, &c), Some(now));
        }
    }

    #[test]
    fn fresh_is_deterministic_and_secret_bound() {
        let a = issue_fresh(&[1; 32], 5);
        assert_eq!(a, issue_fresh(&[1; 32], 5));
        assert_ne!(a, issue_fresh(&[2; 32], 5));
        assert_ne!(a, issue_fresh(&[1; 32], 6));
        assert_eq!(fresh_issued_ns(&[2; 32], &a), None);
    }

    #[test]
    fn fresh_rejects_every_single_bit_flip() {
        let secret = [7u8; 32];
        let c = issue_fresh(&secret, 1_700_000_000_000_000_000);
        for i in 0..FRESH_LEN * 8 {
            let mut t = c;
            t[i / 8] ^= 1 << (i % 8);
            assert_eq!(fresh_issued_ns(&secret, &t), None, "bit {i}");
        }
    }

    #[test]
    fn fresh_rejects_wrong_lengths() {
        let secret = [7u8; 32];
        let c = issue_fresh(&secret, 99);
        assert_eq!(fresh_issued_ns(&secret, &c[..39]), None);
        let mut longer = [0u8; 41];
        longer[..40].copy_from_slice(&c);
        assert_eq!(fresh_issued_ns(&secret, &longer), None);
        assert_eq!(fresh_issued_ns(&secret, &[]), None);
    }

    #[test]
    fn late_matches_format_domain_hash() {
        let nonce = [9u8; 16];
        let expected = witness_format::crypto::sha256_parts(&[
            b"witness-offline",
            &nonce,
            &1_700_000_000_123u64.to_be_bytes(),
        ]);
        assert_eq!(late_challenge(&nonce, 1_700_000_000_123), expected);
        assert_ne!(late_challenge(&nonce, 1_700_000_000_124), expected);
        assert_ne!(late_challenge(&[9u8; 15], 1_700_000_000_123), expected);
    }

    #[test]
    fn late_matches_the_kotlin_preimage_vector() {
        let mut pre = b"witness-offline".to_vec();
        pre.extend_from_slice(&[1, 2, 3, 0, 0, 0x01, 0x8b, 0xcf, 0xe5, 0x68, 0x00]);
        let c = late_challenge(&[1, 2, 3], 1_700_000_000_000);
        assert_eq!(c, witness_format::crypto::sha256(&pre));
        assert_eq!(c[..8], [0x5b, 0x68, 0xe9, 0x71, 0xb8, 0x88, 0x79, 0x67]);
    }
}
