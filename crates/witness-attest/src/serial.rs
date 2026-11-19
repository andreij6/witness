use alloc::string::String;
use witness_format::key::{MAX_SERIAL_HEX, normalize_serial_hex};

pub const SERIAL_BYTES: usize = MAX_SERIAL_HEX / 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Serial([u8; SERIAL_BYTES]);

const fn hex_value(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl Serial {
    pub const ZERO: Self = Self([0; SERIAL_BYTES]);

    pub fn from_be_bytes(bytes: &[u8]) -> Option<Self> {
        let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
        let digits = &bytes[start..];
        if digits.len() > SERIAL_BYTES {
            return None;
        }
        let mut out = [0u8; SERIAL_BYTES];
        out[SERIAL_BYTES - digits.len()..].copy_from_slice(digits);
        Some(Self(out))
    }

    pub fn from_hex(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        if b.is_empty() {
            return None;
        }
        let start = b.iter().position(|&c| c != b'0').unwrap_or(b.len());
        let digits = &b[start..];
        if digits.len() > MAX_SERIAL_HEX {
            return None;
        }
        let mut out = [0u8; SERIAL_BYTES];
        for (i, &c) in digits.iter().rev().enumerate() {
            let v = hex_value(c)?;
            out[SERIAL_BYTES - 1 - i / 2] |= if i % 2 == 0 { v } else { v << 4 };
        }
        Some(Self(out))
    }

    pub fn from_decimal(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        if b.is_empty() {
            return None;
        }
        let mut out = [0u8; SERIAL_BYTES];
        for &c in b {
            if !c.is_ascii_digit() {
                return None;
            }
            let mut carry = u32::from(c - b'0');
            for limb in out.iter_mut().rev() {
                let v = u32::from(*limb) * 10 + carry;
                *limb = v as u8;
                carry = v >> 8;
            }
            if carry != 0 {
                return None;
            }
        }
        Some(Self(out))
    }

    pub fn magnitude(&self) -> &[u8] {
        let start = self.0.iter().position(|&b| b != 0).unwrap_or(SERIAL_BYTES);
        &self.0[start..]
    }

    pub const fn as_padded(&self) -> &[u8; SERIAL_BYTES] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        normalize_serial_hex(self.magnitude())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Rng;
    use alloc::vec::Vec;
    use witness_format::key::is_normalized_serial_hex;

    #[test]
    fn normalization_vectors() {
        let cases: &[(&[u8], &str)] = &[
            (&[], "0"),
            (&[0], "0"),
            (&[0, 0, 0], "0"),
            (&[1], "1"),
            (&[0x0f], "f"),
            (&[0x10], "10"),
            (&[0x00, 0x80], "80"),
            (&[0x00, 0x0a, 0xbc], "abc"),
            (
                &[0xf9, 0x20, 0x09, 0xe8, 0x53, 0xb6, 0xb0, 0x45],
                "f92009e853b6b045",
            ),
            (
                &[0x00, 0xe8, 0xfa, 0x19, 0x63, 0x14, 0xd2, 0xfa, 0x18],
                "e8fa196314d2fa18",
            ),
            (
                &[0xff; 32],
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ),
        ];
        for (bytes, hex) in cases {
            let s = Serial::from_be_bytes(bytes).unwrap();
            assert_eq!(s.to_hex(), *hex);
            assert_eq!(Serial::from_hex(hex), Some(s));
            assert!(is_normalized_serial_hex(&s.to_hex()));
        }
    }

    #[test]
    fn hex_text_is_case_and_padding_insensitive() {
        let want = Serial::from_hex("c8966fcb2fbb0d7a").unwrap();
        for text in [
            "C8966FCB2FBB0D7A",
            "00c8966fcb2fbb0d7a",
            "0000C8966fCb2fbb0d7a",
        ] {
            assert_eq!(Serial::from_hex(text), Some(want));
        }
        assert_eq!(Serial::from_hex("000"), Some(Serial::ZERO));
        assert_eq!(Serial::from_hex("0abc").unwrap().to_hex(), "abc");
    }

    #[test]
    fn hex_rejects_bad_text() {
        let too_long: String = core::iter::repeat_n('1', 65).collect();
        let padded_ok: String = core::iter::repeat_n('0', 10)
            .chain(core::iter::repeat_n('f', 64))
            .collect();
        assert_eq!(Serial::from_hex(""), None);
        assert_eq!(Serial::from_hex("12g4"), None);
        assert_eq!(Serial::from_hex("-1"), None);
        assert_eq!(Serial::from_hex(" 12"), None);
        assert_eq!(Serial::from_hex("0x12"), None);
        assert_eq!(Serial::from_hex(&too_long), None);
        assert!(Serial::from_hex(&padded_ok).is_some());
    }

    #[test]
    fn be_bytes_rejects_oversized() {
        assert_eq!(Serial::from_be_bytes(&[1; 33]), None);
        let mut padded = [0u8; 40];
        padded[39] = 7;
        assert_eq!(Serial::from_be_bytes(&padded).unwrap().to_hex(), "7");
    }

    #[test]
    fn decimal_vectors() {
        let cases = [
            ("0", "0"),
            ("10", "a"),
            ("255", "ff"),
            ("256", "100"),
            ("6681152659205225093", "5cb838f1fe157a85"),
            ("18446744073709551615", "ffffffffffffffff"),
            ("18446744073709551616", "10000000000000000"),
            (
                "340282366920938463463374607431768211455",
                "ffffffffffffffffffffffffffffffff",
            ),
            ("000123", "7b"),
        ];
        for (dec, hex) in cases {
            assert_eq!(Serial::from_decimal(dec).unwrap().to_hex(), hex, "{dec}");
        }
        assert_eq!(Serial::from_decimal(""), None);
        assert_eq!(Serial::from_decimal("12a"), None);
        assert_eq!(Serial::from_decimal("-5"), None);
        let max: String = core::iter::repeat_n('9', 77).collect();
        assert!(Serial::from_decimal(&max).is_some());
        let over: String = core::iter::repeat_n('9', 78).collect();
        assert_eq!(Serial::from_decimal(&over), None);
    }

    #[test]
    fn decimal_matches_u128_on_random_values() {
        let mut rng = Rng::new(11);
        for _ in 0..2000 {
            let v = (u128::from(rng.next()) << 64) | u128::from(rng.next());
            let v = v >> rng.below(128);
            let s = Serial::from_decimal(&alloc::format!("{v}")).unwrap();
            assert_eq!(s.to_hex(), alloc::format!("{v:x}"));
        }
    }

    #[test]
    fn ordering_is_numeric() {
        let mut rng = Rng::new(3);
        let mut values: Vec<u128> = (0..500)
            .map(|_| ((u128::from(rng.next()) << 64) | u128::from(rng.next())) >> rng.below(128))
            .collect();
        let mut serials: Vec<Serial> = values
            .iter()
            .map(|v| Serial::from_be_bytes(&v.to_be_bytes()).unwrap())
            .collect();
        values.sort_unstable();
        serials.sort_unstable();
        for (v, s) in values.iter().zip(&serials) {
            assert_eq!(s.to_hex(), alloc::format!("{v:x}"));
        }
    }

    #[test]
    fn hex_round_trips_random_bytes() {
        let mut rng = Rng::new(5);
        for _ in 0..2000 {
            let mut bytes = [0u8; 32];
            let len = rng.below(33) as usize;
            rng.fill(&mut bytes[..len]);
            let s = Serial::from_be_bytes(&bytes[..len]).unwrap();
            let hex = s.to_hex();
            assert!(is_normalized_serial_hex(&hex));
            assert_eq!(Serial::from_hex(&hex), Some(s));
            assert_eq!(Serial::from_hex(&hex.to_ascii_uppercase()), Some(s));
            assert_eq!(Serial::from_be_bytes(s.magnitude()), Some(s));
        }
    }
}
