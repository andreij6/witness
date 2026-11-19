mod align;
mod compare;
mod sampler;

pub use align::{
    ALIGN_LONG_EDGE, ALIGN_MARGIN_Q16, ALIGN_SCALES_PERCENT, Alignment, Hypothesis,
    MIN_ALIGN_CELLS, Scored, TAU_ALIGN_Q16, align, hypotheses,
};
pub use compare::{DISTANCE_SCALE, GridDiff, compare};

use sampler::{Frame, Sampler};
use witness_format::FingerprintBundle;
use witness_format::fingerprint::{GRID_CELLS, GRID_LONG, GRID_SHORT};
use witness_image::LumaView;

pub const GRID_LONG_EDGE: u32 = 256;
pub const MIN_STABLE_BITS: u32 = 4;
pub const CELL_MASK_WORDS: usize = GRID_CELLS.div_ceil(64);

pub type CellMask = [u64; CELL_MASK_WORDS];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Grid {
    cells: [u16; GRID_CELLS],
    portrait: bool,
}

impl Grid {
    pub const fn new(cells: [u16; GRID_CELLS], portrait: bool) -> Self {
        Self { cells, portrait }
    }

    pub const fn from_bundle(bundle: &FingerprintBundle) -> Self {
        Self::new(*bundle.grid(), bundle.portrait())
    }

    pub const fn cells(&self) -> &[u16; GRID_CELLS] {
        &self.cells
    }

    pub const fn portrait(&self) -> bool {
        self.portrait
    }

    pub const fn dims(&self) -> (usize, usize) {
        grid_dims(self.portrait)
    }

    pub const fn signs(cell: u16) -> u8 {
        cell as u8
    }

    pub const fn stable(cell: u16) -> u8 {
        (cell >> 8) as u8
    }

    pub const fn pack(signs: u8, stable: u8) -> u16 {
        ((stable as u16) << 8) | signs as u16
    }
}

pub const fn grid_dims(portrait: bool) -> (usize, usize) {
    if portrait {
        (GRID_SHORT, GRID_LONG)
    } else {
        (GRID_LONG, GRID_SHORT)
    }
}

pub const fn is_portrait(width: u32, height: u32) -> bool {
    height > width
}

pub fn grid(luma: LumaView<'_>) -> Grid {
    grid_oriented(luma, is_portrait(luma.width(), luma.height()))
}

pub(crate) fn grid_oriented(luma: LumaView<'_>, portrait: bool) -> Grid {
    let sampler = Sampler::new(luma, GRID_LONG_EDGE);
    sampler.grid(Frame::whole(sampler.dimensions()), portrait).0
}

pub(crate) fn mask_get(mask: &CellMask, index: usize) -> bool {
    mask[index / 64] >> (index % 64) & 1 == 1
}

pub(crate) fn mask_set(mask: &mut CellMask, index: usize) {
    mask[index / 64] |= 1 << (index % 64);
}

#[cfg(test)]
mod tests;
