use witness_format::crypto::hash_ds;

pub type Hash = [u8; 32];

pub const ZERO: Hash = [0; 32];

const MMR_LEAF: &[u8] = &[0x00];
const MMR_NODE: &[u8] = &[0x01];
const MMR_BAG: &[u8] = &[0x02];
const MMR_ROOT: &[u8] = &[0x03];
const SMT_LEAF: &[u8] = &[0x10];
const SMT_NODE: &[u8] = &[0x11];

pub fn mmr_leaf(data: &[u8]) -> Hash {
    hash_ds(MMR_LEAF, &[data])
}

pub fn mmr_node(left: &Hash, right: &Hash) -> Hash {
    hash_ds(MMR_NODE, &[left, right])
}

pub fn mmr_bag(left_peak: &Hash, right_bag: &Hash) -> Hash {
    hash_ds(MMR_BAG, &[left_peak, right_bag])
}

pub fn mmr_root(leaf_count: u64, bag: &Hash) -> Hash {
    hash_ds(MMR_ROOT, &[&leaf_count.to_be_bytes(), bag])
}

pub fn smt_leaf(key: u128, value: &Hash) -> Hash {
    hash_ds(SMT_LEAF, &[&key.to_be_bytes(), value])
}

pub fn smt_node(left: &Hash, right: &Hash) -> Hash {
    hash_ds(SMT_NODE, &[left, right])
}

pub(crate) fn bag_step(acc: Option<Hash>, peak: &Hash) -> Hash {
    acc.map_or(*peak, |right| mmr_bag(peak, &right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_format::crypto::sha256;

    fn tagged(tag: u8, parts: &[&[u8]]) -> Hash {
        let mut buf = vec![tag];
        parts.iter().for_each(|p| buf.extend_from_slice(p));
        sha256(&buf)
    }

    #[test]
    fn each_function_matches_its_tagged_preimage() {
        let a = [7u8; 32];
        let b = [9u8; 32];
        assert_eq!(mmr_leaf(b"abc"), tagged(0x00, &[b"abc"]));
        assert_eq!(mmr_node(&a, &b), tagged(0x01, &[&a, &b]));
        assert_eq!(mmr_bag(&a, &b), tagged(0x02, &[&a, &b]));
        assert_eq!(mmr_root(3, &a), tagged(0x03, &[&3u64.to_be_bytes(), &a]));
        assert_eq!(smt_leaf(5, &a), tagged(0x10, &[&5u128.to_be_bytes(), &a]));
        assert_eq!(smt_node(&a, &b), tagged(0x11, &[&a, &b]));
    }

    #[test]
    fn domains_never_collide_on_same_payload() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let mut ab = a.to_vec();
        ab.extend_from_slice(&b);
        let outs = [
            mmr_leaf(&ab),
            mmr_node(&a, &b),
            mmr_bag(&a, &b),
            smt_node(&a, &b),
        ];
        for (i, x) in outs.iter().enumerate() {
            for y in &outs[i + 1..] {
                assert_ne!(x, y);
            }
        }
    }

    #[test]
    fn node_hash_is_order_sensitive() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        assert_ne!(mmr_node(&a, &b), mmr_node(&b, &a));
        assert_ne!(smt_node(&a, &b), smt_node(&b, &a));
        assert_ne!(mmr_bag(&a, &b), mmr_bag(&b, &a));
    }

    #[test]
    fn leaf_of_empty_input_is_defined() {
        assert_eq!(mmr_leaf(&[]), sha256(&[0x00]));
    }

    #[test]
    fn smt_leaf_binds_key() {
        let v = [3u8; 32];
        assert_ne!(smt_leaf(0, &v), smt_leaf(1, &v));
        assert_ne!(smt_leaf(u128::MAX, &v), smt_leaf(u128::MAX - 1, &v));
    }

    #[test]
    fn root_binds_leaf_count() {
        let bag = [4u8; 32];
        assert_ne!(mmr_root(1, &bag), mmr_root(2, &bag));
        assert_ne!(mmr_root(0, &bag), mmr_root(u64::MAX, &bag));
    }

    #[test]
    fn bag_step_starts_with_peak_then_bags() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        assert_eq!(bag_step(None, &a), a);
        assert_eq!(bag_step(Some(b), &a), mmr_bag(&a, &b));
    }
}
