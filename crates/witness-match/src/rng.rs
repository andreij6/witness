pub trait Rng {
    fn next_u64(&mut self) -> u64;

    fn below(&mut self, bound: u32) -> u32 {
        (((self.next_u64() >> 32) * bound as u64) >> 32) as u32
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }
}

impl Rng for SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl<R: Rng + ?Sized> Rng for &mut R {
    fn next_u64(&mut self) -> u64 {
        (**self).next_u64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix_matches_reference_vector() {
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(rng.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(SplitMix64::new(1).next_u64(), SplitMix64::new(2).next_u64());
    }

    #[test]
    fn below_stays_in_range() {
        let mut rng = SplitMix64::new(7);
        for bound in [1u32, 2, 3, 10, 1000, u32::MAX] {
            for _ in 0..500 {
                assert!(rng.below(bound) < bound);
            }
        }
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn below_is_roughly_uniform() {
        let mut rng = SplitMix64::new(99);
        let mut counts = [0u32; 8];
        for _ in 0..80_000 {
            counts[rng.below(8) as usize] += 1;
        }
        assert!(
            counts.iter().all(|&c| (9_000..11_000).contains(&c)),
            "{counts:?}"
        );
    }

    #[test]
    fn mutable_reference_forwards() {
        let mut base = SplitMix64::new(5);
        let expected = SplitMix64::new(5).next_u64();
        let mut by_ref = &mut base;
        assert_eq!(Rng::next_u64(&mut by_ref), expected);
    }
}
