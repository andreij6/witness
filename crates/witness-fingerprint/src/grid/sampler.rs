use super::{CellMask, Grid, grid_dims, mask_set};
use crate::dct::{CELL_COS, CELL_FREQS, CELL_SIDE};
use crate::shape::fit_long_edge;
use witness_format::fingerprint::GRID_CELLS;
use witness_image::{LumaView, resize_area};

pub(crate) const Q16: i64 = 1 << 16;
const COEFFICIENTS: usize = CELL_FREQS * CELL_FREQS - 1;
const LUMA_AMPLITUDE_UNIT: i64 = 1 << 37;
const STABILITY_FLOOR: i64 = LUMA_AMPLITUDE_UNIT;
const STABILITY_KAPPA_RECIPROCAL: i64 = 4;
const MAX_SAMPLER_EDGE: u32 = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Frame {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl Frame {
    pub fn whole((width, height): (usize, usize)) -> Self {
        Self {
            x: 0,
            y: 0,
            w: width as i64 * Q16,
            h: height as i64 * Q16,
        }
    }
}

pub(crate) struct Sampler {
    width: usize,
    height: usize,
    table: Vec<u32>,
}

fn boundaries(start: i64, len: i64, cells: usize, out: &mut [i64]) {
    let steps = (cells * CELL_SIDE) as i64;
    for (t, value) in out.iter_mut().enumerate() {
        let position = start + (len as i128 * t as i128).div_euclid(steps as i128) as i64;
        *value = (position + Q16 / 2) >> 16;
    }
}

fn block(edges: &[i64], index: usize) -> (i64, i64) {
    let (lo, hi) = (edges[index], edges[index + 1]);
    if hi > lo {
        return (lo, hi);
    }
    let lo = lo.min(edges[edges.len() - 1] - 1).max(edges[0]);
    (lo, lo + 1)
}

pub(crate) fn signature(coefficients: &[i64; COEFFICIENTS]) -> u16 {
    let mut sorted = *coefficients;
    sorted.sort_unstable();
    let middle = sorted[COEFFICIENTS / 2 - 1] + sorted[COEFFICIENTS / 2];
    let deviations = coefficients.map(|c| 2 * c - middle);
    let total: i64 = deviations.iter().map(|d| d.abs()).sum();
    let (mut signs, mut stable) = (0u8, 0u8);
    for (bit, &d) in deviations.iter().enumerate() {
        signs |= u8::from(d > 0) << bit;
        let strong = d.abs() * COEFFICIENTS as i64 * STABILITY_KAPPA_RECIPROCAL > total;
        stable |= u8::from(strong && d.abs() > 2 * STABILITY_FLOOR) << bit;
    }
    Grid::pack(signs, stable)
}

pub(crate) fn cell_coefficients(means: &[[i64; CELL_SIDE]; CELL_SIDE]) -> [i64; COEFFICIENTS] {
    let mut rows = [[0i64; CELL_FREQS]; CELL_SIDE];
    for (row, line) in rows.iter_mut().zip(means) {
        for (v, value) in row.iter_mut().enumerate() {
            *value = CELL_COS[v]
                .iter()
                .zip(line)
                .map(|(&c, &m)| c as i64 * m)
                .sum();
        }
    }
    let mut out = [0i64; COEFFICIENTS];
    for (slot, index) in out.iter_mut().zip(1..CELL_FREQS * CELL_FREQS) {
        let (u, v) = (index / CELL_FREQS, index % CELL_FREQS);
        *slot = CELL_COS[u]
            .iter()
            .zip(&rows)
            .map(|(&c, row)| c as i64 * row[v])
            .sum();
    }
    out
}

impl Sampler {
    pub fn new(luma: LumaView<'_>, long_edge: u32) -> Self {
        let (w, h) = fit_long_edge(luma.width(), luma.height(), long_edge.min(MAX_SAMPLER_EDGE));
        let resized;
        let view = if (w, h) == luma.dimensions() {
            luma
        } else {
            resized = resize_area(luma, w, h).ok();
            resized.as_ref().map_or(luma, |image| image.view())
        };
        let (width, height) = (view.width() as usize, view.height() as usize);
        let stride = width + 1;
        let mut table = vec![0u32; stride * (height + 1)];
        for (y, row) in view.rows().enumerate() {
            let mut running = 0u32;
            for (x, &value) in row.iter().enumerate() {
                running += value as u32;
                table[(y + 1) * stride + x + 1] = table[y * stride + x + 1] + running;
            }
        }
        Self {
            width,
            height,
            table,
        }
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub(super) fn sum(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {
        let stride = self.width + 1;
        let at = |x: usize, y: usize| self.table[y * stride + x] as i64;
        at(x1, y1) - at(x0, y1) - at(x1, y0) + at(x0, y0)
    }

    pub fn grid(&self, frame: Frame, portrait: bool) -> (Grid, CellMask) {
        let (cols, rows) = grid_dims(portrait);
        let mut xs = [0i64; 16 * CELL_SIDE + 1];
        let mut ys = [0i64; 16 * CELL_SIDE + 1];
        let (xs, ys) = (
            &mut xs[..cols * CELL_SIDE + 1],
            &mut ys[..rows * CELL_SIDE + 1],
        );
        boundaries(frame.x, frame.w, cols, xs);
        boundaries(frame.y, frame.h, rows, ys);
        let mut cells = [0u16; GRID_CELLS];
        let mut present = [0u64; super::CELL_MASK_WORDS];
        let mut means = [[0i64; CELL_SIDE]; CELL_SIDE];
        for row in 0..rows {
            let y_range = (
                block(ys, row * CELL_SIDE).0,
                block(ys, row * CELL_SIDE + CELL_SIDE - 1).1,
            );
            if y_range.0 < 0 || y_range.1 > self.height as i64 {
                continue;
            }
            for col in 0..cols {
                let x_range = (
                    block(xs, col * CELL_SIDE).0,
                    block(xs, col * CELL_SIDE + CELL_SIDE - 1).1,
                );
                if x_range.0 < 0 || x_range.1 > self.width as i64 {
                    continue;
                }
                for (j, line) in means.iter_mut().enumerate() {
                    let (y0, y1) = block(ys, row * CELL_SIDE + j);
                    for (i, mean) in line.iter_mut().enumerate() {
                        let (x0, x1) = block(xs, col * CELL_SIDE + i);
                        let area = (x1 - x0) * (y1 - y0);
                        let sum = self.sum(x0 as usize, y0 as usize, x1 as usize, y1 as usize);
                        *mean = (sum * 256 + area / 2) / area;
                    }
                }
                let index = row * cols + col;
                cells[index] = signature(&cell_coefficients(&means));
                mask_set(&mut present, index);
            }
        }
        (Grid::new(cells, portrait), present)
    }
}
