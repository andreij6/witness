use crate::hash::Hash;
use witness_format::crypto::sha256_parts;

pub fn certified_data(mmr_root: &Hash, leaf_count: u64, smt_root: &Hash) -> Hash {
    sha256_parts(&[mmr_root, &leaf_count.to_be_bytes(), smt_root])
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_format::crypto::sha256;

    #[test]
    fn matches_flat_concatenation() {
        let m = [1u8; 32];
        let s = [2u8; 32];
        let mut flat = Vec::new();
        flat.extend_from_slice(&m);
        flat.extend_from_slice(&5u64.to_be_bytes());
        flat.extend_from_slice(&s);
        assert_eq!(certified_data(&m, 5, &s), sha256(&flat));
    }

    #[test]
    fn sensitive_to_every_input() {
        let base = certified_data(&[1; 32], 9, &[2; 32]);
        assert_ne!(base, certified_data(&[3; 32], 9, &[2; 32]));
        assert_ne!(base, certified_data(&[1; 32], 10, &[2; 32]));
        assert_ne!(base, certified_data(&[1; 32], 9, &[3; 32]));
        assert_ne!(base, certified_data(&[2; 32], 9, &[1; 32]));
    }

    #[test]
    fn deterministic_and_extreme_counts() {
        assert_eq!(
            certified_data(&[0; 32], 0, &[0; 32]),
            certified_data(&[0; 32], 0, &[0; 32])
        );
        assert_ne!(
            certified_data(&[0; 32], 0, &[0; 32]),
            certified_data(&[0; 32], u64::MAX, &[0; 32])
        );
    }
}
