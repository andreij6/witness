use core::fmt;

pub const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

const INVALID: u8 = 0xff;
const SKIP: u8 = 0xfe;

const DECODE: [u8; 256] = {
    let mut t = [INVALID; 256];
    let mut i = 0;
    while i < 32 {
        let c = ALPHABET[i];
        t[c as usize] = i as u8;
        t[c.to_ascii_lowercase() as usize] = i as u8;
        i += 1;
    }
    t[b'I' as usize] = 1;
    t[b'i' as usize] = 1;
    t[b'L' as usize] = 1;
    t[b'l' as usize] = 1;
    t[b'O' as usize] = 0;
    t[b'o' as usize] = 0;
    t[b'-' as usize] = SKIP;
    t
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base32Error {
    Length,
    Symbol(u8),
    Padding,
}

impl fmt::Display for Base32Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length => f.write_str("wrong number of base32 symbols"),
            Self::Symbol(c) => write!(f, "invalid base32 symbol 0x{c:02x}"),
            Self::Padding => f.write_str("non-zero base32 padding bits"),
        }
    }
}

impl core::error::Error for Base32Error {}

pub const fn encoded_len(bytes: usize) -> usize {
    (bytes * 8).div_ceil(5)
}

pub fn encode(bytes: &[u8], out: &mut [u8]) {
    assert_eq!(out.len(), encoded_len(bytes.len()));
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut o = 0;
    for &b in bytes {
        acc = (acc << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out[o] = ALPHABET[((acc >> bits) & 31) as usize];
            o += 1;
        }
    }
    if bits > 0 {
        out[o] = ALPHABET[((acc << (5 - bits)) & 31) as usize];
    }
}

pub fn decode(text: &[u8], out: &mut [u8]) -> Result<(), Base32Error> {
    let want = encoded_len(out.len());
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut symbols = 0usize;
    let mut o = 0;
    for &c in text {
        let v = DECODE[c as usize];
        if v == SKIP {
            continue;
        }
        if v == INVALID {
            return Err(Base32Error::Symbol(c));
        }
        symbols += 1;
        if symbols > want {
            return Err(Base32Error::Length);
        }
        acc = (acc << 5) | u32::from(v);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out[o] = (acc >> bits) as u8;
            o += 1;
        }
    }
    if symbols != want {
        return Err(Base32Error::Length);
    }
    if acc & ((1 << bits) - 1) != 0 {
        return Err(Base32Error::Padding);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc<const N: usize, const M: usize>(b: [u8; N]) -> [u8; M] {
        let mut out = [0u8; M];
        encode(&b, &mut out);
        out
    }

    #[test]
    fn encoded_len_matches_bit_count() {
        assert_eq!(encoded_len(0), 0);
        assert_eq!(encoded_len(1), 2);
        assert_eq!(encoded_len(5), 8);
        assert_eq!(encoded_len(10), 16);
        assert_eq!(encoded_len(16), 26);
    }

    #[test]
    fn golden_vectors() {
        assert_eq!(&enc::<5, 8>([0; 5]), b"00000000");
        assert_eq!(&enc::<5, 8>([0xff; 5]), b"ZZZZZZZZ");
        assert_eq!(&enc::<1, 2>([0xf8]), b"Z0");
        assert_eq!(&enc::<5, 8>([0x08, 0x86, 0x42, 0x98, 0xe8]), b"12345678");
        assert_eq!(&enc::<5, 8>([0x00, 0x44, 0x32, 0x14, 0xc7]), b"01234567");
    }

    #[test]
    fn decode_is_case_insensitive_and_maps_confusables() {
        let mut a = [0u8; 5];
        let mut b = [0u8; 5];
        decode(b"abcdefgh", &mut a).unwrap();
        decode(b"ABCDEFGH", &mut b).unwrap();
        assert_eq!(a, b);
        decode(b"1I1L1i1l", &mut a).unwrap();
        decode(b"11111111", &mut b).unwrap();
        assert_eq!(a, b);
        decode(b"0OoO0000", &mut a).unwrap();
        assert_eq!(a, [0; 5]);
    }

    #[test]
    fn decode_tolerates_hyphens_anywhere() {
        let mut a = [0u8; 5];
        decode(b"-1234-5678-", &mut a).unwrap();
        assert_eq!(a, [0x08, 0x86, 0x42, 0x98, 0xe8]);
        decode(b"1-2-3-4-5-6-7-8", &mut a).unwrap();
        assert_eq!(a, [0x08, 0x86, 0x42, 0x98, 0xe8]);
    }

    #[test]
    fn decode_rejects_bad_symbols() {
        let mut a = [0u8; 5];
        for bad in [b'U', b'u', b' ', b'=', b'*', 0x80, 0xff, 0] {
            let mut s = *b"12345678";
            s[3] = bad;
            assert_eq!(decode(&s, &mut a), Err(Base32Error::Symbol(bad)));
        }
    }

    #[test]
    fn decode_rejects_wrong_length() {
        let mut a = [0u8; 5];
        assert_eq!(decode(b"1234567", &mut a), Err(Base32Error::Length));
        assert_eq!(decode(b"123456789", &mut a), Err(Base32Error::Length));
        assert_eq!(decode(b"", &mut a), Err(Base32Error::Length));
        assert_eq!(decode(b"--------", &mut a), Err(Base32Error::Length));
    }

    #[test]
    fn decode_rejects_nonzero_padding() {
        let mut a = [0u8; 1];
        assert_eq!(decode(b"Z0", &mut a), Ok(()));
        assert_eq!(a, [0xf8]);
        assert_eq!(decode(b"Z1", &mut a), Err(Base32Error::Padding));
        assert_eq!(decode(b"Z3", &mut a), Err(Base32Error::Padding));
        assert_eq!(decode(b"Z4", &mut a), Ok(()));
        assert_eq!(a, [0xf9]);
    }

    #[test]
    fn empty_roundtrip() {
        let mut out = [0u8; 0];
        encode(&[], &mut out);
        assert_eq!(decode(b"", &mut []), Ok(()));
        assert_eq!(decode(b"---", &mut []), Ok(()));
    }

    #[test]
    fn roundtrip_all_lengths_seeded() {
        let mut s = 0x9e37_79b9_7f4a_7c15u64;
        for len in 0..40usize {
            for _ in 0..50 {
                let mut bytes = [0u8; 40];
                for b in bytes[..len].iter_mut() {
                    s ^= s << 13;
                    s ^= s >> 7;
                    s ^= s << 17;
                    *b = s as u8;
                }
                let mut text = [0u8; 64];
                let n = encoded_len(len);
                encode(&bytes[..len], &mut text[..n]);
                assert!(text[..n].iter().all(|c| ALPHABET.contains(c)));
                let mut back = [0u8; 40];
                decode(&text[..n], &mut back[..len]).unwrap();
                assert_eq!(bytes[..len], back[..len]);
                let lower: [u8; 64] = text.map(|c| c.to_ascii_lowercase());
                decode(&lower[..n], &mut back[..len]).unwrap();
                assert_eq!(bytes[..len], back[..len]);
            }
        }
    }

    #[test]
    fn encoding_preserves_byte_order() {
        let mut prev = [0u8; 16];
        encode(&[0u8; 10], &mut prev);
        let mut v = 0u128;
        for step in 1..200u128 {
            v += step * 0x1234_5678_9abc;
            let b = v.to_be_bytes();
            let mut cur = [0u8; 16];
            encode(&b[6..], &mut cur);
            assert!(cur > prev);
            prev = cur;
        }
    }

    #[test]
    fn error_display() {
        assert!(!alloc::format!("{}", Base32Error::Symbol(b'U')).is_empty());
        assert!(!alloc::format!("{}", Base32Error::Length).is_empty());
        assert!(!alloc::format!("{}", Base32Error::Padding).is_empty());
    }
}
