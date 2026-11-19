use super::DEPTH;
use crate::hash::{Hash, ZERO, smt_node};
use std::sync::LazyLock;

static EMPTY: LazyLock<[Hash; DEPTH as usize + 1]> = LazyLock::new(|| {
    let mut table = [ZERO; DEPTH as usize + 1];
    for h in 1..table.len() {
        table[h] = smt_node(&table[h - 1], &table[h - 1]);
    }
    table
});

pub fn empty(height: u8) -> &'static Hash {
    &EMPTY[usize::from(height)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaf_level_is_zero() {
        assert_eq!(*empty(0), ZERO);
    }

    #[test]
    fn each_level_hashes_previous() {
        for h in 1..=DEPTH {
            assert_eq!(*empty(h), smt_node(empty(h - 1), empty(h - 1)));
        }
    }

    #[test]
    fn all_levels_distinct() {
        let mut all: Vec<Hash> = (0..=DEPTH).map(|h| *empty(h)).collect();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), usize::from(DEPTH) + 1);
    }

    #[test]
    #[should_panic]
    fn height_above_depth_panics() {
        let _ = empty(DEPTH + 1);
    }
}
