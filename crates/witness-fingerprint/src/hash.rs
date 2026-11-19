use core::fmt;

pub const HASH_BYTES: usize = 32;
pub const HASH_BITS: usize = HASH_BYTES * 8;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct PdqHash(pub [u8; HASH_BYTES]);

impl PdqHash {
    pub const fn bytes(&self) -> &[u8; HASH_BYTES] {
        &self.0
    }

    pub fn words(&self) -> [u64; 4] {
        let mut words = [0u64; 4];
        for (word, chunk) in words.iter_mut().zip(self.0.chunks_exact(8)) {
            *word = u64::from_be_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
        }
        words
    }

    pub fn from_words(words: [u64; 4]) -> Self {
        let mut bytes = [0u8; HASH_BYTES];
        for (chunk, word) in bytes.chunks_exact_mut(8).zip(words) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        Self(bytes)
    }

    pub const fn bit(&self, index: usize) -> bool {
        (self.0[HASH_BYTES - 1 - index / 8] >> (index % 8)) & 1 == 1
    }

    pub const fn set_bit(&mut self, index: usize) {
        self.0[HASH_BYTES - 1 - index / 8] |= 1 << (index % 8);
    }

    pub fn count_ones(&self) -> u32 {
        self.words().iter().map(|w| w.count_ones()).sum()
    }

    pub fn distance(&self, other: &Self) -> u32 {
        let (a, b) = (self.words(), other.words());
        (a[0] ^ b[0]).count_ones()
            + (a[1] ^ b[1]).count_ones()
            + (a[2] ^ b[2]).count_ones()
            + (a[3] ^ b[3]).count_ones()
    }

    pub fn to_hex(&self) -> [u8; HASH_BYTES * 2] {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut out = [0u8; HASH_BYTES * 2];
        for (pair, byte) in out.chunks_exact_mut(2).zip(self.0) {
            pair[0] = DIGITS[(byte >> 4) as usize];
            pair[1] = DIGITS[(byte & 15) as usize];
        }
        out
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let digits = hex.as_bytes();
        if digits.len() != HASH_BYTES * 2 {
            return None;
        }
        let nibble = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
        let mut bytes = [0u8; HASH_BYTES];
        for (byte, pair) in bytes.iter_mut().zip(digits.chunks_exact(2)) {
            *byte = (nibble(pair[0])? << 4) | nibble(pair[1])?;
        }
        Some(Self(bytes))
    }
}

impl fmt::Debug for PdqHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PdqHash({self})")
    }
}

impl fmt::Display for PdqHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_hex()
            .iter()
            .try_for_each(|&c| fmt::Write::write_char(f, c as char))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Rng;

    fn random(rng: &mut Rng) -> PdqHash {
        PdqHash::from_words([rng.next(), rng.next(), rng.next(), rng.next()])
    }

    #[test]
    fn words_round_trip() {
        let mut rng = Rng::new(1);
        for _ in 0..500 {
            let hash = random(&mut rng);
            assert_eq!(PdqHash::from_words(hash.words()), hash);
        }
    }

    #[test]
    fn distance_matches_bitwise_definition() {
        let mut rng = Rng::new(2);
        for _ in 0..500 {
            let (a, b) = (random(&mut rng), random(&mut rng));
            let naive = (0..HASH_BITS).filter(|&i| a.bit(i) != b.bit(i)).count() as u32;
            assert_eq!(a.distance(&b), naive);
            assert_eq!(a.distance(&b), b.distance(&a));
            assert_eq!(a.distance(&a), 0);
        }
    }

    #[test]
    fn distance_extremes() {
        let zero = PdqHash::default();
        let ones = PdqHash([0xFF; HASH_BYTES]);
        assert_eq!(zero.distance(&ones), 256);
        assert_eq!(ones.count_ones(), 256);
        assert_eq!(zero.count_ones(), 0);
    }

    #[test]
    fn bit_layout_matches_reference_hex_order() {
        let mut hash = PdqHash::default();
        hash.set_bit(0);
        assert_eq!(hash.0[31], 1);
        hash.set_bit(255);
        assert_eq!(hash.0[0], 0x80);
        hash.set_bit(9);
        assert_eq!(hash.0[30], 2);
        assert!(hash.bit(0) && hash.bit(9) && hash.bit(255) && !hash.bit(1));
        assert_eq!(hash.count_ones(), 3);
    }

    #[test]
    fn hex_round_trip_and_display() {
        let mut rng = Rng::new(3);
        for _ in 0..200 {
            let hash = random(&mut rng);
            let text = hash.to_string();
            assert_eq!(text.len(), 64);
            assert_eq!(PdqHash::from_hex(&text), Some(hash));
            assert_eq!(PdqHash::from_hex(&text.to_uppercase()), Some(hash));
        }
        let known = "f8f8f0cee0f4a84f06370a22038f63f0b36e2ed596621e1d33e6b39c4e9c9b22";
        let hash = PdqHash::from_hex(known).unwrap();
        assert_eq!(hash.0[0], 0xf8);
        assert_eq!(hash.to_string(), known);
        assert!(format!("{hash:?}").contains(known));
    }

    #[test]
    fn hex_rejects_malformed() {
        assert_eq!(PdqHash::from_hex(""), None);
        assert_eq!(PdqHash::from_hex(&"0".repeat(63)), None);
        assert_eq!(PdqHash::from_hex(&"0".repeat(65)), None);
        assert_eq!(PdqHash::from_hex(&"g".repeat(64)), None);
        assert_eq!(PdqHash::from_hex(&"é".repeat(32)), None);
        assert_eq!(PdqHash::from_hex(&"0".repeat(64)), Some(PdqHash::default()));
    }
}
