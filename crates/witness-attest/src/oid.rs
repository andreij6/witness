pub const RSA_ENCRYPTION: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
pub const SHA256_WITH_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
pub const SHA384_WITH_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0c];
pub const SHA512_WITH_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0d];
pub const EC_PUBLIC_KEY: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
pub const ECDSA_SHA256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];
pub const ECDSA_SHA384: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x03];
pub const ECDSA_SHA512: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x04];
pub const PRIME256V1: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
pub const SECP384R1: &[u8] = &[0x2b, 0x81, 0x04, 0x00, 0x22];
pub const BASIC_CONSTRAINTS: &[u8] = &[0x55, 0x1d, 0x13];
pub const KEY_USAGE: &[u8] = &[0x55, 0x1d, 0x0f];
pub const KEY_DESCRIPTION: &[u8] = &[0x2b, 0x06, 0x01, 0x04, 0x01, 0xd6, 0x79, 0x02, 0x01, 0x11];
pub const PROVISIONING_INFO: &[u8] = &[0x2b, 0x06, 0x01, 0x04, 0x01, 0xd6, 0x79, 0x02, 0x01, 0x1e];

#[cfg(test)]
mod tests {
    use super::*;

    fn arcs(content: &[u8]) -> alloc::vec::Vec<u64> {
        let mut out = alloc::vec![u64::from(content[0] / 40), u64::from(content[0] % 40)];
        let mut acc = 0u64;
        for &b in &content[1..] {
            acc = (acc << 7) | u64::from(b & 0x7f);
            if b & 0x80 == 0 {
                out.push(acc);
                acc = 0;
            }
        }
        out
    }

    #[test]
    fn oids_decode_to_their_dotted_forms() {
        let cases: &[(&[u8], &[u64])] = &[
            (RSA_ENCRYPTION, &[1, 2, 840, 113549, 1, 1, 1]),
            (SHA256_WITH_RSA, &[1, 2, 840, 113549, 1, 1, 11]),
            (SHA384_WITH_RSA, &[1, 2, 840, 113549, 1, 1, 12]),
            (SHA512_WITH_RSA, &[1, 2, 840, 113549, 1, 1, 13]),
            (EC_PUBLIC_KEY, &[1, 2, 840, 10045, 2, 1]),
            (ECDSA_SHA256, &[1, 2, 840, 10045, 4, 3, 2]),
            (ECDSA_SHA384, &[1, 2, 840, 10045, 4, 3, 3]),
            (ECDSA_SHA512, &[1, 2, 840, 10045, 4, 3, 4]),
            (PRIME256V1, &[1, 2, 840, 10045, 3, 1, 7]),
            (SECP384R1, &[1, 3, 132, 0, 34]),
            (BASIC_CONSTRAINTS, &[2, 5, 29, 19]),
            (KEY_USAGE, &[2, 5, 29, 15]),
            (KEY_DESCRIPTION, &[1, 3, 6, 1, 4, 1, 11129, 2, 1, 17]),
            (PROVISIONING_INFO, &[1, 3, 6, 1, 4, 1, 11129, 2, 1, 30]),
        ];
        for (bytes, dotted) in cases {
            assert_eq!(arcs(bytes), *dotted);
        }
    }
}
