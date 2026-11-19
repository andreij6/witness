mod profile;

use crate::align::working_size;
use crate::geometry::Rect;
use crate::mask::OverlayMask;
use profile::{FLAT, Integral, NATURAL, STRONG, close_gaps, longest, pixel_classes, runs};
use witness_image::{RgbView, resize_area, to_luma};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateKind {
    Natural,
    FullMinusBars,
    LargestBand,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub rect: Rect,
    pub confidence: f64,
    pub kind: CandidateKind,
    pub overlay_mask: OverlayMask,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtractParams {
    pub work_edge: u32,
    pub natural_percent: u32,
    pub min_band_percent: u32,
    pub max_gap_percent: u32,
    pub max_bar_percent: u32,
    pub natural_candidates: usize,
    pub overlay_block: u32,
    pub overlay_flat_percent: u32,
    pub overlay_min_strong: u32,
}

impl Default for ExtractParams {
    fn default() -> Self {
        Self {
            work_edge: 512,
            natural_percent: 25,
            min_band_percent: 6,
            max_gap_percent: 1,
            max_bar_percent: 15,
            natural_candidates: 3,
            overlay_block: 4,
            overlay_flat_percent: 25,
            overlay_min_strong: 2,
        }
    }
}

struct Analysis {
    width: usize,
    height: usize,
    classes: Vec<u8>,
    natural: Integral,
}

impl Analysis {
    fn natural_fraction(&self, rect: &Rect) -> f64 {
        if rect.is_empty() {
            0.0
        } else {
            self.natural.sum(rect) as f64 / rect.area() as f64
        }
    }

    fn photo_flags(&self, count: usize, line: impl Fn(usize) -> Rect, percent: u32) -> Vec<bool> {
        (0..count)
            .map(|i| {
                let r = line(i);
                self.natural.sum(&r) as u64 * 100 >= r.area() * percent as u64 && !r.is_empty()
            })
            .collect()
    }
}

fn percent_of(len: usize, percent: u32, floor: usize) -> usize {
    (len * percent as usize / 100).max(floor)
}

fn row_flags(
    a: &Analysis,
    cols: (usize, usize),
    rows: (usize, usize),
    params: &ExtractParams,
) -> Vec<bool> {
    let mut flags = a.photo_flags(
        rows.1 - rows.0,
        |i| {
            Rect::new(
                cols.0 as u32,
                (rows.0 + i) as u32,
                (cols.1 - cols.0) as u32,
                1,
            )
        },
        params.natural_percent,
    );
    close_gaps(&mut flags, percent_of(a.height, params.max_gap_percent, 1));
    flags
}

fn col_flags(a: &Analysis, rows: (usize, usize), params: &ExtractParams) -> Vec<bool> {
    let mut flags = a.photo_flags(
        a.width,
        |i| Rect::new(i as u32, rows.0 as u32, 1, (rows.1 - rows.0) as u32),
        params.natural_percent,
    );
    close_gaps(&mut flags, percent_of(a.width, params.max_gap_percent, 1));
    flags
}

fn score(a: &Analysis, rect: &Rect) -> f64 {
    let frame = (a.width * a.height) as f64;
    let nat = a.natural_fraction(rect);
    let coverage = (rect.area() as f64 / frame / 0.2).min(1.0).sqrt();
    ((nat - 0.1) / 0.5).clamp(0.0, 1.0) * coverage
}

fn natural_rects(a: &Analysis, params: &ExtractParams) -> Vec<(Rect, f64)> {
    let min_band = percent_of(a.height, params.min_band_percent, 4);
    let min_cols = percent_of(a.width, params.min_band_percent, 4);
    let bands = runs(&row_flags(a, (0, a.width), (0, a.height), params), min_band);
    let mut out: Vec<(Rect, f64)> = Vec::new();
    for band in bands {
        for cols in runs(&col_flags(a, band, params), min_cols) {
            let refined = runs(&row_flags(a, cols, band, params), min_band);
            let Some((r0, r1)) = longest(&refined) else {
                continue;
            };
            let rows = (band.0 + r0, band.0 + r1);
            let rect = Rect::new(
                cols.0 as u32,
                rows.0 as u32,
                (cols.1 - cols.0) as u32,
                (rows.1 - rows.0) as u32,
            );
            if !out.iter().any(|(r, _)| *r == rect) {
                out.push((rect, score(a, &rect)));
            }
        }
    }
    out.sort_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
    out.truncate(params.natural_candidates);
    out
}

fn full_minus_bars(a: &Analysis, params: &ExtractParams) -> Rect {
    let flags = row_flags(a, (0, a.width), (0, a.height), params);
    let cap = percent_of(a.height, params.max_bar_percent, 0);
    let top = flags.iter().take_while(|&&f| !f).count().min(cap);
    let bottom = flags.iter().rev().take_while(|&&f| !f).count().min(cap);
    if top + bottom >= a.height {
        return Rect::new(0, 0, a.width as u32, a.height as u32);
    }
    Rect::new(
        0,
        top as u32,
        a.width as u32,
        (a.height - top - bottom) as u32,
    )
}

fn largest_band(a: &Analysis, params: &ExtractParams) -> Rect {
    let flags = row_flags(a, (0, a.width), (0, a.height), params);
    match longest(&runs(&flags, 1)) {
        Some((y0, y1)) => Rect::new(0, y0 as u32, a.width as u32, (y1 - y0) as u32),
        None => full_minus_bars(a, params),
    }
}

const SOLID_PERCENT: u64 = 90;

fn grow_into_solid(seeds: &mut [bool], solid: &[bool], cols: usize) {
    let mut stack: Vec<usize> = (0..seeds.len()).filter(|&i| seeds[i]).collect();
    while let Some(i) = stack.pop() {
        let (row, col) = (i / cols, i % cols);
        let neighbors = [
            (col > 0).then(|| i - 1),
            (col + 1 < cols).then(|| i + 1),
            (row > 0).then(|| i - cols),
            (i + cols < seeds.len()).then(|| i + cols),
        ];
        for j in neighbors.into_iter().flatten() {
            if solid[j] && !seeds[j] {
                seeds[j] = true;
                stack.push(j);
            }
        }
    }
}

fn dilate(blocks: &[bool], cols: usize, rows: usize) -> Vec<bool> {
    let mut out = blocks.to_vec();
    for (i, _) in blocks.iter().enumerate().filter(|(_, b)| **b) {
        let (row, col) = (i / cols, i % cols);
        for r in row.saturating_sub(1)..(row + 2).min(rows) {
            for c in col.saturating_sub(1)..(col + 2).min(cols) {
                out[r * cols + c] = true;
            }
        }
    }
    out
}

fn overlay_blocks(a: &Analysis, rect: &Rect, params: &ExtractParams) -> (u32, u32, Vec<bool>) {
    let b = params.overlay_block.max(1);
    let (cols, rows) = (rect.width.div_ceil(b), rect.height.div_ceil(b));
    let count = cols as usize * rows as usize;
    let (mut seeds, mut solid) = (vec![false; count], vec![false; count]);
    for row in 0..rows {
        for col in 0..cols {
            let block = Rect::new(rect.x + col * b, rect.y + row * b, b, b)
                .intersection(rect)
                .unwrap_or_default();
            let (mut flat, mut strong) = (0u32, 0u32);
            for y in block.y..block.bottom() {
                for &c in
                    &a.classes[y as usize * a.width + block.x as usize..][..block.width as usize]
                {
                    flat += (c == FLAT) as u32;
                    strong += (c == STRONG) as u32;
                }
            }
            let i = (row * cols + col) as usize;
            let area = block.area();
            seeds[i] = area > 0
                && flat as u64 * 100 >= area * params.overlay_flat_percent as u64
                && strong >= params.overlay_min_strong;
            solid[i] = area > 0 && flat as u64 * 100 >= area * SOLID_PERCENT;
        }
    }
    grow_into_solid(&mut seeds, &solid, cols as usize);
    (cols, rows, dilate(&seeds, cols as usize, rows as usize))
}

fn inset_interior(lo: u32, hi: u32, bound: u32) -> (u32, u32) {
    let inner = (
        if lo > 0 { lo + 1 } else { lo },
        if hi < bound { hi - 1 } else { hi },
    );
    if inner.1 > inner.0 { inner } else { (lo, hi) }
}

fn scale_inward(lo: u32, hi: u32, full: u32, work: u32) -> (u32, u32) {
    let (lo, hi, full, work) = (lo as u64, hi as u64, full as u64, work as u64);
    let (a, b) = ((lo * full).div_ceil(work), hi * full / work);
    let (a, b) = if b > a {
        (a, b)
    } else {
        (lo * full / work, (hi * full).div_ceil(work).min(full))
    };
    (a as u32, b as u32)
}

fn to_full(rect: &Rect, scale: (u32, u32, u32, u32)) -> Rect {
    let (fw, fh, ww, wh) = scale;
    let (x0, x1) = inset_interior(rect.x, rect.right(), ww);
    let (y0, y1) = inset_interior(rect.y, rect.bottom(), wh);
    let (x0, x1) = scale_inward(x0, x1, fw, ww);
    let (y0, y1) = scale_inward(y0, y1, fh, wh);
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

pub fn extract_photo_regions(screenshot: RgbView<'_>, params: &ExtractParams) -> Vec<Candidate> {
    let (full_w, full_h) = screenshot.dimensions();
    let (w, h) = working_size(full_w, full_h, params.work_edge);
    let resized;
    let work = if (w, h) == (full_w, full_h) {
        screenshot
    } else {
        match resize_area(screenshot, w, h) {
            Ok(img) => {
                resized = img;
                resized.view()
            }
            Err(_) => screenshot,
        }
    };
    let (w, h) = work.dimensions();
    let luma = to_luma(work);
    let classes = pixel_classes(luma.data(), w as usize, h as usize);
    let natural = Integral::new(&classes, w as usize, h as usize, NATURAL);
    let a = Analysis {
        width: w as usize,
        height: h as usize,
        classes,
        natural,
    };

    let mut picks: Vec<(Rect, f64, CandidateKind)> = natural_rects(&a, params)
        .into_iter()
        .map(|(r, s)| (r, s, CandidateKind::Natural))
        .collect();
    for (rect, kind) in [
        (full_minus_bars(&a, params), CandidateKind::FullMinusBars),
        (largest_band(&a, params), CandidateKind::LargestBand),
    ] {
        if !picks.iter().any(|(r, _, _)| *r == rect) {
            picks.push((rect, 0.5 * score(&a, &rect), kind));
        }
    }
    let scale = (full_w, full_h, w, h);
    let (sx, sy) = (full_w as f64 / w as f64, full_h as f64 / h as f64);
    let block = params.overlay_block.max(1) as f64;
    picks
        .into_iter()
        .map(|(rect, confidence, kind)| {
            let (cols, rows, blocks) = overlay_blocks(&a, &rect, params);
            let overlay_mask = OverlayMask::new(
                (rect.x as f64 * sx, rect.y as f64 * sy),
                (block * sx, block * sy),
                cols,
                rows,
                blocks,
            )
            .unwrap_or_default();
            Candidate {
                rect: to_full(&rect, scale),
                confidence,
                kind,
                overlay_mask,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_image::Rgb8;

    fn photo_pixel(x: u32, y: u32, seed: u64) -> [u8; 3] {
        let mut s = seed ^ ((x as u64) << 32) ^ y as u64;
        s = s.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        s ^= s >> 29;
        let n = (s % 13) as i32 - 6;
        let base = 90 + ((x / 7 + y / 5) % 9) as i32 * 9 + n;
        [base as u8, (base + 20) as u8, (180 - base / 2) as u8]
    }

    fn screenshot(w: u32, h: u32, photo: Rect, seed: u64) -> Rgb8 {
        Rgb8::from_fn(w, h, |x, y| {
            if photo.contains(x, y) {
                photo_pixel(x, y, seed)
            } else if (y / 6) % 3 == 0 && (x / 4) % 3 != 0 && (x % 50) < 30 {
                [30, 30, 30]
            } else {
                [250, 250, 250]
            }
        })
        .unwrap()
    }

    fn iou(a: &Rect, b: &Rect) -> f64 {
        let inter = a.intersection(b).map_or(0, |r| r.area()) as f64;
        inter / ((a.area() + b.area()) as f64 - inter)
    }

    #[test]
    fn finds_photo_between_ui_bars() {
        let photo = Rect::new(0, 60, 300, 260);
        let img = screenshot(300, 450, photo, 1);
        let c = extract_photo_regions(img.view(), &ExtractParams::default());
        assert_eq!(c[0].kind, CandidateKind::Natural);
        assert!(iou(&c[0].rect, &photo) > 0.9, "{:?}", c[0].rect);
        assert!(c[0].confidence > 0.5);
    }

    fn analysis(img: &Rgb8) -> Analysis {
        let (w, h) = (img.width() as usize, img.height() as usize);
        let classes = pixel_classes(to_luma(img.view()).data(), w, h);
        let natural = Integral::new(&classes, w, h, NATURAL);
        Analysis {
            width: w,
            height: h,
            classes,
            natural,
        }
    }

    #[test]
    fn fallbacks_trim_bars_and_pick_largest_band() {
        let img = Rgb8::from_fn(100, 200, |x, y| match y {
            0..10 | 190..200 | 100..110 => [250, 250, 250],
            70..100 => photo_pixel(x, y, 9),
            _ => photo_pixel(x, y, 3),
        })
        .unwrap();
        let a = analysis(&img);
        let p = ExtractParams::default();
        assert_eq!(full_minus_bars(&a, &p), Rect::new(0, 10, 100, 180));
        assert_eq!(largest_band(&a, &p), Rect::new(0, 10, 100, 90));
    }

    #[test]
    fn bar_trimming_is_capped() {
        let img = Rgb8::from_fn(100, 100, |x, y| {
            if y < 50 {
                [0, 0, 0]
            } else {
                photo_pixel(x, y, 4)
            }
        })
        .unwrap();
        let a = analysis(&img);
        assert_eq!(
            full_minus_bars(&a, &ExtractParams::default()),
            Rect::new(0, 15, 100, 85)
        );
    }

    #[test]
    fn featureless_screen_falls_back_to_frame() {
        let img = Rgb8::filled(120, 80, [10, 10, 10]).unwrap();
        let c = extract_photo_regions(img.view(), &ExtractParams::default());
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].rect, Rect::new(0, 13, 120, 54));
        assert_eq!(c[0].confidence, 0.0);
        assert!(c[0].overlay_mask.is_empty());
    }

    #[test]
    fn grow_fills_solid_regions_from_seeds() {
        let mut seeds = vec![true, false, false, false, false, false];
        let solid = vec![false, true, true, false, false, true];
        grow_into_solid(&mut seeds, &solid, 3);
        assert_eq!(seeds, vec![true, true, true, false, false, true]);
        let mut none = vec![false; 4];
        grow_into_solid(&mut none, &[true; 4], 2);
        assert_eq!(none, vec![false; 4]);
    }

    #[test]
    fn dilate_expands_by_one_block() {
        let mut blocks = vec![false; 16];
        blocks[5] = true;
        let d = dilate(&blocks, 4, 4);
        assert_eq!(d.iter().filter(|&&b| b).count(), 9);
        assert!(!d[15] && d[10] && d[0]);
        assert_eq!(dilate(&[], 0, 0), Vec::<bool>::new());
    }

    #[test]
    fn solid_button_interior_is_masked() {
        let img = Rgb8::from_fn(96, 96, |x, y| {
            if (24..72).contains(&x) && (24..72).contains(&y) {
                [255, 255, 255]
            } else {
                photo_pixel(x, y, 6)
            }
        })
        .unwrap();
        let a = analysis(&img);
        let (cols, _, blocks) =
            overlay_blocks(&a, &Rect::new(0, 0, 96, 96), &ExtractParams::default());
        assert!(blocks[12 * cols as usize + 12]);
        assert!(!blocks[2 * cols as usize + 2]);
    }

    #[test]
    fn overlay_blocks_flag_text_over_photo() {
        let img = Rgb8::from_fn(64, 64, |x, y| {
            if (20..30).contains(&y) && (8..56).contains(&x) && (x / 3) % 2 == 0 {
                [255, 255, 255]
            } else {
                photo_pixel(x, y, 5)
            }
        })
        .unwrap();
        let a = analysis(&img);
        let params = ExtractParams {
            overlay_block: 8,
            ..ExtractParams::default()
        };
        let (cols, rows, blocks) = overlay_blocks(&a, &Rect::new(0, 0, 64, 64), &params);
        assert_eq!((cols, rows), (8, 8));
        assert!(blocks[2 * 8 + 3] && blocks[3 * 8 + 4]);
        assert!(!blocks[7 * 8 + 7] && !blocks[0]);
    }

    #[test]
    fn iou_helper_sane() {
        let r = Rect::new(0, 0, 10, 10);
        assert!((iou(&r, &r) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn to_full_scales_inward_and_insets_interior_edges() {
        assert_eq!(
            to_full(&Rect::new(1, 1, 2, 2), (10, 10, 4, 4)),
            Rect::new(3, 3, 4, 4)
        );
        assert_eq!(
            to_full(&Rect::new(0, 0, 4, 4), (10, 10, 4, 4)),
            Rect::new(0, 0, 10, 10)
        );
        assert_eq!(
            to_full(&Rect::new(1, 0, 3, 4), (10, 10, 4, 4)),
            Rect::new(5, 0, 5, 10)
        );
        assert_eq!(
            to_full(&Rect::new(10, 10, 20, 20), (100, 100, 100, 100)),
            Rect::new(11, 11, 18, 18)
        );
    }

    #[test]
    fn to_full_never_empties_tiny_rects() {
        assert_eq!(
            to_full(&Rect::new(1, 1, 1, 1), (3, 3, 3, 3)),
            Rect::new(1, 1, 1, 1)
        );
        assert_eq!(
            to_full(&Rect::new(1, 1, 1, 1), (5, 5, 4, 4)),
            Rect::new(1, 1, 2, 2)
        );
        assert_eq!(
            to_full(&Rect::new(0, 0, 1, 1), (1, 1, 1, 1)),
            Rect::new(0, 0, 1, 1)
        );
    }
}
