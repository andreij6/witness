use super::{CELL_MASK_WORDS, CellMask, Grid, MIN_STABLE_BITS, mask_get, mask_set};
use witness_format::fingerprint::GRID_CELLS;

pub const DISTANCE_SCALE: u32 = 255;
const ALL_PRESENT: CellMask = {
    let mut mask = [u64::MAX; CELL_MASK_WORDS];
    mask[CELL_MASK_WORDS - 1] = u64::MAX >> (CELL_MASK_WORDS * 64 - GRID_CELLS);
    mask
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GridDiff {
    distances: [u8; GRID_CELLS],
    compared: CellMask,
    differing_bits: u32,
    compared_bits: u32,
    portrait: bool,
}

impl GridDiff {
    pub const fn portrait(&self) -> bool {
        self.portrait
    }

    pub const fn compared_mask(&self) -> &CellMask {
        &self.compared
    }

    pub const fn distances(&self) -> &[u8; GRID_CELLS] {
        &self.distances
    }

    pub fn is_compared(&self, index: usize) -> bool {
        index < GRID_CELLS && mask_get(&self.compared, index)
    }

    pub fn distance(&self, index: usize) -> Option<u8> {
        self.is_compared(index).then(|| self.distances[index])
    }

    pub fn compared_cells(&self) -> u32 {
        self.compared.iter().map(|w| w.count_ones()).sum()
    }

    pub const fn compared_bits(&self) -> u32 {
        self.compared_bits
    }

    pub const fn differing_bits(&self) -> u32 {
        self.differing_bits
    }

    pub fn compared_fraction_q16(&self) -> u32 {
        (self.compared_cells() << 16) / GRID_CELLS as u32
    }

    pub fn agreement_q16(&self) -> u32 {
        if self.compared_bits == 0 {
            return 0;
        }
        ((((self.compared_bits - self.differing_bits) as u64) << 16) / self.compared_bits as u64)
            as u32
    }
}

pub fn compare(reference: &Grid, copy: &Grid) -> Option<GridDiff> {
    (reference.portrait() == copy.portrait())
        .then(|| compare_present(reference, copy, &ALL_PRESENT))
}

pub(crate) fn compare_present(reference: &Grid, copy: &Grid, present: &CellMask) -> GridDiff {
    let mut diff = GridDiff {
        distances: [0; GRID_CELLS],
        compared: [0; CELL_MASK_WORDS],
        differing_bits: 0,
        compared_bits: 0,
        portrait: reference.portrait(),
    };
    for (index, (&a, &b)) in reference.cells().iter().zip(copy.cells()).enumerate() {
        if !mask_get(present, index) {
            continue;
        }
        let mask = Grid::stable(a) | Grid::stable(b);
        let bits = mask.count_ones();
        if bits < MIN_STABLE_BITS {
            continue;
        }
        let differing = ((Grid::signs(a) ^ Grid::signs(b)) & mask).count_ones();
        diff.distances[index] = ((differing * DISTANCE_SCALE + bits / 2) / bits) as u8;
        diff.differing_bits += differing;
        diff.compared_bits += bits;
        mask_set(&mut diff.compared, index);
    }
    diff
}
