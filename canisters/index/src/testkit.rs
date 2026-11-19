use ic_stable_structures::VectorMemory;
use witness_fingerprint::PdqHash;
use witness_format::fingerprint::GRID_CELLS;
use witness_format::{FingerprintBundle, PdqShape};

pub type Mem = VectorMemory;

pub fn mem() -> Mem {
    Mem::default()
}

pub fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub fn hash(seed: u64) -> PdqHash {
    let s = mix(seed);
    PdqHash::from_words([mix(s), mix(s ^ 1), mix(s ^ 2), mix(s ^ 3)])
}

pub fn flip(base: &PdqHash, bits: usize, seed: u64) -> PdqHash {
    let mut out = *base;
    let mut picked = [false; 256];
    let mut state = seed;
    let mut n = 0;
    while n < bits {
        state = mix(state);
        let bit = (state % 256) as usize;
        if !picked[bit] {
            picked[bit] = true;
            out.0[31 - bit / 8] ^= 1 << (bit % 8);
            n += 1;
        }
    }
    out
}

pub fn bundle(seed: u64, shapes: usize, quality: u8, portrait: bool) -> FingerprintBundle {
    let pdq = (0..shapes)
        .map(|i| PdqShape {
            tag: i as u8,
            hash: hash(seed.wrapping_mul(1_000).wrapping_add(i as u64)).0,
        })
        .collect();
    let grid: [u16; GRID_CELLS] = core::array::from_fn(|i| mix(seed ^ (i as u64) << 32) as u16);
    FingerprintBundle::new(pdq, quality, grid, portrait).expect("valid bundle")
}
