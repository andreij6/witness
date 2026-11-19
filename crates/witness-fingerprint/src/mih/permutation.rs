use super::{MIH_CHUNK_BITS, MIH_CHUNKS};
use crate::hash::{HASH_BITS, PdqHash};

pub const PERMUTATION_SEED: u64 = 0x5749_544E_4553_5331;
pub const PERMUTATION: [u8; HASH_BITS] = generate(PERMUTATION_SEED);
const SOURCE_OF: [u8; HASH_BITS] = invert(&PERMUTATION);

const fn splitmix(state: u64) -> (u64, u64) {
    let next = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = next;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (next, z ^ (z >> 31))
}

pub(crate) const fn generate(seed: u64) -> [u8; HASH_BITS] {
    let mut table = [0u8; HASH_BITS];
    let mut i = 0;
    while i < HASH_BITS {
        table[i] = i as u8;
        i += 1;
    }
    let mut state = seed;
    let mut i = HASH_BITS - 1;
    while i > 0 {
        let (next, random) = splitmix(state);
        state = next;
        let j = ((random as u128 * (i as u128 + 1)) >> 64) as usize;
        let swap = table[i];
        table[i] = table[j];
        table[j] = swap;
        i -= 1;
    }
    table
}

const fn invert(table: &[u8; HASH_BITS]) -> [u8; HASH_BITS] {
    let mut inverse = [0u8; HASH_BITS];
    let mut i = 0;
    while i < HASH_BITS {
        inverse[table[i] as usize] = i as u8;
        i += 1;
    }
    inverse
}

pub fn permute(hash: &PdqHash) -> [u64; 4] {
    let mut out = [0u64; 4];
    for (target, &source) in SOURCE_OF.iter().enumerate() {
        out[target / 64] |= (hash.bit(source as usize) as u64) << (target % 64);
    }
    out
}

pub fn chunks(hash: &PdqHash) -> [u16; MIH_CHUNKS] {
    let words = permute(hash);
    core::array::from_fn(|chunk| {
        let bit = chunk * MIH_CHUNK_BITS;
        (words[bit / 64] >> (bit % 64)) as u16
    })
}
