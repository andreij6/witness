use crate::features::{Descriptor, hamming};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Match {
    pub query: u32,
    pub reference: u32,
    pub distance: u32,
}

pub(crate) fn match_descriptors(
    query: &[Descriptor],
    reference: &[Descriptor],
    max_distance: u32,
    ratio_percent: u32,
) -> Vec<Match> {
    let mut out = Vec::new();
    for (qi, q) in query.iter().enumerate() {
        let (mut best, mut second, mut best_index) = (u32::MAX, u32::MAX, 0usize);
        for (ri, r) in reference.iter().enumerate() {
            let d = hamming(q, r);
            if d < best {
                second = best;
                best = d;
                best_index = ri;
            } else if d < second {
                second = d;
            }
        }
        let distinctive =
            second == u32::MAX || (best as u64) * 100 < (second as u64) * ratio_percent as u64;
        if best <= max_distance && distinctive {
            out.push(Match {
                query: qi as u32,
                reference: best_index as u32,
                distance: best,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{Rng, SplitMix64};

    fn descriptor(seed: u64) -> Descriptor {
        let mut rng = SplitMix64::new(seed);
        core::array::from_fn(|_| rng.next_u64())
    }

    fn flip_bits(d: Descriptor, count: u32) -> Descriptor {
        let mut out = d;
        for i in 0..count as usize {
            out[i % 4] ^= 1 << (i * 7 % 64);
        }
        out
    }

    #[test]
    fn exact_duplicates_match() {
        let refs: Vec<Descriptor> = (0..20).map(descriptor).collect();
        let query = vec![refs[7], refs[3]];
        let m = match_descriptors(&query, &refs, 64, 80);
        assert_eq!(
            m,
            vec![
                Match {
                    query: 0,
                    reference: 7,
                    distance: 0
                },
                Match {
                    query: 1,
                    reference: 3,
                    distance: 0
                }
            ]
        );
    }

    #[test]
    fn noisy_descriptors_match_within_distance() {
        let refs: Vec<Descriptor> = (100..150).map(descriptor).collect();
        let query: Vec<Descriptor> = refs.iter().map(|&d| flip_bits(d, 20)).collect();
        let m = match_descriptors(&query, &refs, 64, 80);
        assert_eq!(m.len(), 50);
        assert!(m.iter().all(|x| x.query == x.reference && x.distance == 20));
    }

    #[test]
    fn far_descriptors_are_rejected() {
        let refs: Vec<Descriptor> = (0..10).map(descriptor).collect();
        let query = vec![descriptor(999)];
        assert!(match_descriptors(&query, &refs, 64, 80).is_empty());
    }

    #[test]
    fn ambiguous_matches_fail_ratio_test() {
        let base = descriptor(5);
        let refs = vec![flip_bits(base, 10), flip_bits(base, 11)];
        assert!(match_descriptors(&[base], &refs, 64, 80).is_empty());
        assert_eq!(match_descriptors(&[base], &refs, 64, 100).len(), 1);
    }

    #[test]
    fn single_reference_skips_ratio() {
        let base = descriptor(9);
        assert_eq!(
            match_descriptors(&[base], &[flip_bits(base, 5)], 64, 80).len(),
            1
        );
    }

    #[test]
    fn empty_inputs() {
        assert!(match_descriptors(&[], &[descriptor(1)], 64, 80).is_empty());
        assert!(match_descriptors(&[descriptor(1)], &[], 64, 80).is_empty());
    }
}
