use crate::cbor::{self, Decode, Decoder, Encode, Encoder, Error};
use crate::crypto::sha256;
use alloc::vec::Vec;

pub const GRID_CELLS: usize = 192;
pub const GRID_BYTES: usize = GRID_CELLS * 2;
pub const GRID_LONG: usize = 16;
pub const GRID_SHORT: usize = 12;
pub const MAX_PDQ_SHAPES: usize = 32;
pub const MAX_PDQ_QUALITY: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PdqShape {
    pub tag: u8,
    pub hash: [u8; 32],
}

impl Encode for PdqShape {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.array(2);
        e.put(&self.tag);
        e.put(&self.hash);
    }
}

impl<'a> Decode<'a> for PdqShape {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        d.array_of(2)?;
        Ok(Self {
            tag: u8::decode(d)?,
            hash: d.byte_array()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FingerprintBundle {
    pdq: Vec<PdqShape>,
    quality: u8,
    grid: [u16; GRID_CELLS],
    portrait: bool,
}

impl FingerprintBundle {
    pub const VERSION: u64 = 1;

    pub fn new(
        mut pdq: Vec<PdqShape>,
        quality: u8,
        grid: [u16; GRID_CELLS],
        portrait: bool,
    ) -> Result<Self, Error> {
        pdq.sort_unstable_by_key(|s| s.tag);
        Self::checked(pdq, quality, grid, portrait)
    }

    fn checked(
        pdq: Vec<PdqShape>,
        quality: u8,
        grid: [u16; GRID_CELLS],
        portrait: bool,
    ) -> Result<Self, Error> {
        if pdq.len() > MAX_PDQ_SHAPES || quality > MAX_PDQ_QUALITY {
            return Err(Error::Range);
        }
        if pdq.windows(2).any(|w| w[0].tag >= w[1].tag) {
            return Err(Error::KeyOrder);
        }
        Ok(Self {
            pdq,
            quality,
            grid,
            portrait,
        })
    }

    pub fn pdq(&self) -> &[PdqShape] {
        &self.pdq
    }

    pub fn shape(&self, tag: u8) -> Option<&PdqShape> {
        self.pdq
            .binary_search_by_key(&tag, |s| s.tag)
            .ok()
            .map(|i| &self.pdq[i])
    }

    pub const fn quality(&self) -> u8 {
        self.quality
    }

    pub const fn grid(&self) -> &[u16; GRID_CELLS] {
        &self.grid
    }

    pub const fn portrait(&self) -> bool {
        self.portrait
    }

    pub const fn grid_dims(&self) -> (usize, usize) {
        if self.portrait {
            (GRID_SHORT, GRID_LONG)
        } else {
            (GRID_LONG, GRID_SHORT)
        }
    }

    pub fn cell(&self, col: usize, row: usize) -> Option<u16> {
        let (cols, rows) = self.grid_dims();
        (col < cols && row < rows).then(|| self.grid[row * cols + col])
    }

    pub fn grid_bytes(&self) -> [u8; GRID_BYTES] {
        let mut out = [0u8; GRID_BYTES];
        for (dst, v) in out.chunks_exact_mut(2).zip(self.grid.iter()) {
            dst.copy_from_slice(&v.to_be_bytes());
        }
        out
    }

    pub fn fp_commit(&self) -> [u8; 32] {
        sha256(&cbor::to_vec(self))
    }
}

impl Encode for FingerprintBundle {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.map(5);
        e.field(0, &Self::VERSION);
        e.field(1, &self.pdq);
        e.field(2, &self.quality);
        e.field(3, &self.grid_bytes());
        e.field(4, &self.portrait);
    }
}

impl<'a> Decode<'a> for FingerprintBundle {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        let mut f = d.fields()?;
        f.version(d, Self::VERSION)?;
        let pdq: Vec<PdqShape> = f.req(d, 1)?;
        let quality = f.req(d, 2)?;
        let raw: [u8; GRID_BYTES] = f.req(d, 3)?;
        let portrait = f.req(d, 4)?;
        f.finish(d)?;
        let mut grid = [0u16; GRID_CELLS];
        for (dst, src) in grid.iter_mut().zip(raw.chunks_exact(2)) {
            *dst = u16::from_be_bytes([src[0], src[1]]);
        }
        Self::checked(pdq, quality, grid, portrait)
    }
}

#[cfg(test)]
mod tests;
