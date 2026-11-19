use crate::dct::{Coefficients, PDQ_FREQS, PDQ_SIDE, pdq_dct};
use crate::dihedral::Dihedral;
use crate::hash::PdqHash;
use witness_image::{LumaView, jarosz_downsample_q8};

pub const MIN_HASHABLE_DIM: u32 = 5;
pub const MIN_INDEX_QUALITY: u8 = 50;
pub const MAX_QUALITY: u8 = 100;

const Q8_LUMA_SPAN: i32 = 255 * 256;
const QUALITY_DIVISOR: i32 = 90;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pdq {
    pub hash: PdqHash,
    pub quality: u8,
}

impl Pdq {
    pub const fn indexable(&self) -> bool {
        self.quality >= MIN_INDEX_QUALITY
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PdqDihedral {
    pub hashes: [PdqHash; 8],
    pub quality: u8,
}

impl PdqDihedral {
    pub const fn get(&self, transform: Dihedral) -> PdqHash {
        self.hashes[transform as usize]
    }

    pub const fn pdq(&self) -> Pdq {
        Pdq {
            hash: self.hashes[0],
            quality: self.quality,
        }
    }
}

pub(crate) struct Analysis {
    pub coefficients: Coefficients,
    pub quality: u8,
}

impl Analysis {
    pub fn new(luma: LumaView<'_>) -> Option<Self> {
        if luma.width() < MIN_HASHABLE_DIM || luma.height() < MIN_HASHABLE_DIM {
            return None;
        }
        let buffer = jarosz_downsample_q8(luma, PDQ_SIDE as u32, PDQ_SIDE as u32).ok()?;
        Some(Self {
            coefficients: pdq_dct(&buffer),
            quality: quality(&buffer),
        })
    }

    pub fn hash(&self, transform: Dihedral) -> PdqHash {
        match transform {
            Dihedral::Identity => hash_coefficients(&self.coefficients),
            other => hash_coefficients(&other.apply(&self.coefficients)),
        }
    }
}

pub fn pdq(luma: LumaView<'_>) -> Option<Pdq> {
    let analysis = Analysis::new(luma)?;
    Some(Pdq {
        hash: analysis.hash(Dihedral::Identity),
        quality: analysis.quality,
    })
}

pub fn pdq_dihedral(luma: LumaView<'_>) -> Option<PdqDihedral> {
    let analysis = Analysis::new(luma)?;
    Some(PdqDihedral {
        hashes: Dihedral::ALL.map(|d| analysis.hash(d)),
        quality: analysis.quality,
    })
}

pub(crate) fn hash_coefficients(coefficients: &Coefficients) -> PdqHash {
    let mut sorted = [0i64; PDQ_FREQS * PDQ_FREQS];
    sorted.copy_from_slice(coefficients.as_flattened());
    let (_, &mut median, _) = sorted.select_nth_unstable(PDQ_FREQS * PDQ_FREQS / 2 - 1);
    let mut hash = PdqHash::default();
    for (index, &value) in coefficients.as_flattened().iter().enumerate() {
        if value > median {
            hash.set_bit(index);
        }
    }
    hash
}

fn quality(buffer: &[u16]) -> u8 {
    let step = |u: u16, v: u16| ((u as i32 - v as i32) * 100 / Q8_LUMA_SPAN).unsigned_abs();
    let mut gradient_sum = 0u32;
    for (upper, lower) in buffer
        .chunks_exact(PDQ_SIDE)
        .zip(buffer.chunks_exact(PDQ_SIDE).skip(1))
    {
        gradient_sum += upper
            .iter()
            .zip(lower)
            .map(|(&u, &v)| step(u, v))
            .sum::<u32>();
    }
    for row in buffer.chunks_exact(PDQ_SIDE) {
        gradient_sum += row.windows(2).map(|w| step(w[0], w[1])).sum::<u32>();
    }
    (gradient_sum / QUALITY_DIVISOR as u32).min(MAX_QUALITY as u32) as u8
}

#[cfg(test)]
mod tests;
