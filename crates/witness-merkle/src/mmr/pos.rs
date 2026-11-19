pub(crate) const fn mmr_size(leaves: u64) -> u64 {
    2 * leaves - leaves.count_ones() as u64
}

pub(crate) const fn node_pos(start: u64, height: u32) -> u64 {
    mmr_size(start) + (2u64 << height) - 2
}

pub(crate) const fn high_bits(n: u64, height: u32) -> u64 {
    (n >> height >> 1) << height << 1
}

pub(crate) const fn is_peak_of(n: u64, start: u64, height: u32) -> bool {
    (n >> height) & 1 == 1 && start == high_bits(n, height)
}

pub(crate) fn leaves_for_size(size: u64) -> Option<u64> {
    let mut rest = size;
    let mut leaves = 0u64;
    for h in (0..63u32).rev() {
        let mountain = (2u64 << h) - 1;
        if rest >= mountain {
            rest -= mountain;
            leaves |= 1 << h;
        }
    }
    (rest == 0).then_some(leaves)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mountain {
    pub start: u64,
    pub height: u32,
}

impl Mountain {
    pub const fn pos(self) -> u64 {
        node_pos(self.start, self.height)
    }

    pub const fn contains(self, leaf: u64) -> bool {
        leaf >= self.start && leaf - self.start < (1u64 << self.height)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Mountains {
    n: u64,
    bits: u64,
}

impl Mountains {
    pub const fn new(n: u64) -> Self {
        Self { n, bits: n }
    }

    fn take(&mut self, height: u32) -> Mountain {
        self.bits ^= 1 << height;
        Mountain {
            start: high_bits(self.n, height),
            height,
        }
    }
}

impl Iterator for Mountains {
    type Item = Mountain;

    fn next(&mut self) -> Option<Mountain> {
        (self.bits != 0).then(|| self.take(63 - self.bits.leading_zeros()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.bits.count_ones() as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Mountains {
    fn next_back(&mut self) -> Option<Mountain> {
        (self.bits != 0).then(|| self.take(self.bits.trailing_zeros()))
    }
}

impl ExactSizeIterator for Mountains {}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_sizes(max: u64) -> Vec<u64> {
        let mut sizes = vec![0u64];
        let mut size = 0u64;
        for n in 0..max {
            size += 1 + u64::from(n.trailing_ones());
            sizes.push(size);
        }
        sizes
    }

    #[test]
    fn mmr_size_matches_append_simulation() {
        for (n, size) in naive_sizes(2000).into_iter().enumerate() {
            assert_eq!(mmr_size(n as u64), size);
        }
    }

    #[test]
    fn leaves_for_size_inverts_mmr_size() {
        let valid = naive_sizes(2000);
        for (n, &size) in valid.iter().enumerate() {
            assert_eq!(leaves_for_size(size), Some(n as u64));
        }
        let last = *valid.last().unwrap_or(&0);
        let invalid = (0..=last)
            .filter(|s| valid.binary_search(s).is_err())
            .count();
        assert!(invalid > 0);
        for s in 0..=last {
            assert_eq!(
                leaves_for_size(s).is_some(),
                valid.binary_search(&s).is_ok()
            );
        }
    }

    #[test]
    fn leaves_for_size_extremes() {
        assert_eq!(leaves_for_size(0), Some(0));
        assert_eq!(leaves_for_size(2), None);
        assert_eq!(leaves_for_size(mmr_size(1 << 62)), Some(1 << 62));
        assert_eq!(
            leaves_for_size(mmr_size((1 << 62) - 1)),
            Some((1 << 62) - 1)
        );
        assert_eq!(leaves_for_size(u64::MAX), None);
    }

    #[test]
    fn node_pos_of_leaf_and_known_positions() {
        let expected_leaf_pos = [0u64, 1, 3, 4, 7, 8, 10, 11, 15];
        for (i, &p) in expected_leaf_pos.iter().enumerate() {
            assert_eq!(node_pos(i as u64, 0), p);
        }
        assert_eq!(node_pos(0, 1), 2);
        assert_eq!(node_pos(2, 1), 5);
        assert_eq!(node_pos(0, 2), 6);
        assert_eq!(node_pos(0, 3), 14);
        assert_eq!(node_pos(8, 2), 21);
    }

    #[test]
    fn high_bits_handles_top_height() {
        assert_eq!(high_bits(u64::MAX, 63), 0);
        assert_eq!(high_bits(u64::MAX, 0), u64::MAX - 1);
        assert_eq!(high_bits(0b1011, 1), 0b1000);
    }

    #[test]
    fn mountains_partition_leaves_in_order() {
        for n in 0..600u64 {
            let ms: Vec<Mountain> = Mountains::new(n).collect();
            assert_eq!(ms.len(), n.count_ones() as usize);
            let mut next = 0;
            let mut prev_h = u32::MAX;
            for m in &ms {
                assert_eq!(m.start, next);
                assert!(m.height < prev_h);
                assert!(is_peak_of(n, m.start, m.height));
                next += 1 << m.height;
                prev_h = m.height;
            }
            assert_eq!(next, n);
            let back: Vec<Mountain> = Mountains::new(n).rev().collect();
            assert_eq!(back.into_iter().rev().collect::<Vec<_>>(), ms);
            for leaf in 0..n {
                assert_eq!(ms.iter().filter(|m| m.contains(leaf)).count(), 1);
            }
        }
    }

    #[test]
    fn mountains_mixed_ends_and_size_hint() {
        let mut it = Mountains::new(0b1101);
        assert_eq!(it.len(), 3);
        assert_eq!(
            it.next(),
            Some(Mountain {
                start: 0,
                height: 3
            })
        );
        assert_eq!(
            it.next_back(),
            Some(Mountain {
                start: 12,
                height: 0
            })
        );
        assert_eq!(it.len(), 1);
        assert_eq!(
            it.next(),
            Some(Mountain {
                start: 8,
                height: 2
            })
        );
        assert_eq!(it.next(), None);
        assert_eq!(it.next_back(), None);
    }

    #[test]
    fn mountains_of_max_value() {
        let ms: Vec<Mountain> = Mountains::new(u64::MAX).collect();
        assert_eq!(ms.len(), 64);
        assert_eq!(
            ms[0],
            Mountain {
                start: 0,
                height: 63
            }
        );
        assert_eq!(
            ms[63],
            Mountain {
                start: u64::MAX - 1,
                height: 0
            }
        );
        assert!(ms[63].contains(u64::MAX - 1));
        assert!(!ms[63].contains(u64::MAX));
    }

    #[test]
    fn is_peak_rejects_non_peaks() {
        assert!(is_peak_of(6, 0, 2));
        assert!(is_peak_of(6, 4, 1));
        assert!(!is_peak_of(6, 0, 1));
        assert!(!is_peak_of(6, 4, 0));
        assert!(!is_peak_of(6, 2, 1));
        assert!(!is_peak_of(0, 0, 0));
    }

    #[test]
    fn contains_bounds() {
        let m = Mountain {
            start: 8,
            height: 2,
        };
        assert!(!m.contains(7));
        assert!(m.contains(8));
        assert!(m.contains(11));
        assert!(!m.contains(12));
    }
}
