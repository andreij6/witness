use crate::linalg::solve;
use witness_image::{Rgb8, RgbView};

pub(crate) const KNOTS: usize = 16;
const BIN_WIDTH: usize = 256 / KNOTS;
pub(crate) const GAIN_GRID: usize = 4;
const GAIN_CELLS: usize = GAIN_GRID * GAIN_GRID;
const GAIN_MIN: f64 = 0.2;
const GAIN_MAX: f64 = 5.0;
const GAIN_STEP: usize = 2;

pub(crate) type Lut = [f64; 256];
pub(crate) type Gains = [f64; GAIN_CELLS];

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Model {
    pub luts: [Lut; 3],
    pub gains: [Gains; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FitParams {
    pub ridge: f64,
    pub trim: f64,
    pub passes: u32,
}

pub(crate) fn isotonic(points: &[(f64, f64)]) -> Vec<f64> {
    let mut blocks: Vec<(f64, f64, usize)> = Vec::with_capacity(points.len());
    for &(value, weight) in points {
        blocks.push((value * weight, weight, 1));
        while blocks.len() >= 2 {
            let (s2, w2, n2) = blocks[blocks.len() - 1];
            let (s1, w1, n1) = blocks[blocks.len() - 2];
            if s1 * w2 <= s2 * w1 {
                break;
            }
            blocks.pop();
            *blocks.last_mut().expect("two blocks") = (s1 + s2, w1 + w2, n1 + n2);
        }
    }
    blocks
        .iter()
        .flat_map(|&(s, w, n)| core::iter::repeat_n(s / w, n))
        .collect()
}

fn identity_lut() -> Lut {
    core::array::from_fn(|q| q as f64)
}

fn lut_from_knots(knots: &[(f64, f64)]) -> Lut {
    if knots.is_empty() {
        return identity_lut();
    }
    let mut lut = [0.0; 256];
    let mut k = 0;
    for (q, v) in lut.iter_mut().enumerate() {
        let x = q as f64;
        while k + 1 < knots.len() && knots[k + 1].0 <= x {
            k += 1;
        }
        let (x0, y0) = knots[k];
        *v = if x <= x0 || k + 1 == knots.len() {
            y0
        } else {
            let (x1, y1) = knots[k + 1];
            y0 + (y1 - y0) * (x - x0) / (x1 - x0)
        };
    }
    lut
}

fn lut_from_bins(bins: &[(f64, f64, f64)]) -> Lut {
    let present: Vec<&(f64, f64, f64)> = bins.iter().filter(|b| b.2 > 0.0).collect();
    let fitted = isotonic(&present.iter().map(|b| (b.1, b.2)).collect::<Vec<_>>());
    let knots: Vec<(f64, f64)> = present.iter().map(|b| b.0).zip(fitted).collect();
    lut_from_knots(&knots)
}

pub(crate) struct Axis {
    index: Vec<u8>,
    frac: Vec<f64>,
}

pub(crate) fn axis(len: u32) -> Axis {
    let (mut index, mut frac) = (
        Vec::with_capacity(len as usize),
        Vec::with_capacity(len as usize),
    );
    let span = (GAIN_GRID - 1) as f64;
    for i in 0..len {
        let g = (i as f64 + 0.5) / len as f64 * span;
        let i0 = (g.floor() as usize).min(GAIN_GRID - 2);
        index.push(i0 as u8);
        frac.push(g - i0 as f64);
    }
    Axis { index, frac }
}

struct GainSampler {
    ax: Axis,
    ay: Axis,
}

impl GainSampler {
    fn new(width: u32, height: u32) -> Self {
        Self {
            ax: axis(width),
            ay: axis(height),
        }
    }

    fn row(&self, gains: &[Gains; 3], y: usize) -> [[f64; GAIN_GRID]; 3] {
        let (j0, fy) = (self.ay.index[y] as usize, self.ay.frac[y]);
        core::array::from_fn(|c| {
            core::array::from_fn(|i| {
                (1.0 - fy) * gains[c][j0 * GAIN_GRID + i] + fy * gains[c][(j0 + 1) * GAIN_GRID + i]
            })
        })
    }

    fn at(&self, row: &[[f64; GAIN_GRID]; 3], x: usize) -> [f64; 3] {
        let (i0, fx) = (self.ax.index[x] as usize, self.ax.frac[x]);
        core::array::from_fn(|c| (1.0 - fx) * row[c][i0] + fx * row[c][i0 + 1])
    }

    fn weights(&self, x: usize, y: usize) -> ([usize; 4], [f64; 4]) {
        let (i0, fx) = (self.ax.index[x] as usize, self.ax.frac[x]);
        let (j0, fy) = (self.ay.index[y] as usize, self.ay.frac[y]);
        (
            [
                j0 * GAIN_GRID + i0,
                j0 * GAIN_GRID + i0 + 1,
                (j0 + 1) * GAIN_GRID + i0,
                (j0 + 1) * GAIN_GRID + i0 + 1,
            ],
            [
                (1.0 - fx) * (1.0 - fy),
                fx * (1.0 - fy),
                (1.0 - fx) * fy,
                fx * fy,
            ],
        )
    }
}

fn is_inlier(model: &Model, q: &[u8], r: &[u8], g: &[f64; 3], trim: f64) -> bool {
    (0..3).all(|c| (r[c] as f64 - g[c] * model.luts[c][q[c] as usize]).abs() <= trim)
}

fn median_luts(warped: &Rgb8, valid: &[bool], reference: RgbView<'_>) -> [Lut; 3] {
    let mut hist = vec![[0u32; 256]; 3 * KNOTS];
    let mut q_sum = [[0u64; KNOTS]; 3];
    let width = warped.width() as usize;
    for y in 0..warped.height() {
        let (q_row, r_row) = (warped.row(y), reference.row(y));
        for x in (0..width).filter(|&x| valid[y as usize * width + x]) {
            for c in 0..3 {
                let q = q_row[x * 3 + c] as usize;
                hist[c * KNOTS + q / BIN_WIDTH][r_row[x * 3 + c] as usize] += 1;
                q_sum[c][q / BIN_WIDTH] += q as u64;
            }
        }
    }
    core::array::from_fn(|c| {
        let bins: Vec<(f64, f64, f64)> = (0..KNOTS)
            .map(|k| {
                let h = &hist[c * KNOTS + k];
                let n: u64 = h.iter().map(|&v| v as u64).sum();
                if n == 0 {
                    return (0.0, 0.0, 0.0);
                }
                let mut seen = 0u64;
                let median = h.iter().position(|&v| {
                    seen += v as u64;
                    seen * 2 >= n
                });
                (
                    q_sum[c][k] as f64 / n as f64,
                    median.unwrap_or(0) as f64,
                    n as f64,
                )
            })
            .collect();
        lut_from_bins(&bins)
    })
}

fn trimmed_luts(
    warped: &Rgb8,
    valid: &[bool],
    reference: RgbView<'_>,
    model: &Model,
    sampler: &GainSampler,
    trim: f64,
) -> [Lut; 3] {
    let mut acc = [[(0.0f64, 0.0f64, 0.0f64); KNOTS]; 3];
    let width = warped.width() as usize;
    for y in 0..warped.height() as usize {
        let (q_row, r_row) = (warped.row(y as u32), reference.row(y as u32));
        let gain_row = sampler.row(&model.gains, y);
        for x in (0..width).filter(|&x| valid[y * width + x]) {
            let (q, r) = (&q_row[x * 3..x * 3 + 3], &r_row[x * 3..x * 3 + 3]);
            let g = sampler.at(&gain_row, x);
            if !is_inlier(model, q, r, &g, trim) {
                continue;
            }
            for c in 0..3 {
                let bin = &mut acc[c][q[c] as usize / BIN_WIDTH];
                bin.0 += q[c] as f64;
                bin.1 += r[c] as f64 / g[c];
                bin.2 += 1.0;
            }
        }
    }
    core::array::from_fn(|c| {
        let bins: Vec<(f64, f64, f64)> = acc[c]
            .iter()
            .map(|&(sq, sr, n)| {
                if n > 0.0 {
                    (sq / n, sr / n, n)
                } else {
                    (0.0, 0.0, 0.0)
                }
            })
            .collect();
        lut_from_bins(&bins)
    })
}

fn fit_gains(
    warped: &Rgb8,
    valid: &[bool],
    reference: RgbView<'_>,
    model: &Model,
    sampler: &GainSampler,
    trim: Option<f64>,
    ridge: f64,
) -> [Gains; 3] {
    let mut a = [[[0.0f64; GAIN_CELLS]; GAIN_CELLS]; 3];
    let mut b = [[0.0f64; GAIN_CELLS]; 3];
    let width = warped.width() as usize;
    for y in (0..warped.height() as usize).step_by(GAIN_STEP) {
        let (q_row, r_row) = (warped.row(y as u32), reference.row(y as u32));
        let gain_row = sampler.row(&model.gains, y);
        for x in (0..width)
            .step_by(GAIN_STEP)
            .filter(|&x| valid[y * width + x])
        {
            let (q, r) = (&q_row[x * 3..x * 3 + 3], &r_row[x * 3..x * 3 + 3]);
            if trim.is_some_and(|t| !is_inlier(model, q, r, &sampler.at(&gain_row, x), t)) {
                continue;
            }
            let (cells, weights) = sampler.weights(x, y);
            for c in 0..3 {
                let v = model.luts[c][q[c] as usize];
                let target = r[c] as f64;
                for (k, &wk) in weights.iter().enumerate() {
                    let wv = wk * v;
                    b[c][cells[k]] += wv * target;
                    for (l, &wl) in weights.iter().enumerate() {
                        a[c][cells[k]][cells[l]] += wv * wl * v;
                    }
                }
            }
        }
    }
    core::array::from_fn(|c| {
        let mut m = a[c];
        let mut rhs = b[c];
        let trace: f64 = (0..GAIN_CELLS).map(|k| m[k][k]).sum();
        let lambda = ridge * trace / GAIN_CELLS as f64 + 1e-6;
        for k in 0..GAIN_CELLS {
            m[k][k] += lambda;
            rhs[k] += lambda;
        }
        solve(m, rhs).map_or([1.0; GAIN_CELLS], |g| {
            g.map(|v| v.clamp(GAIN_MIN, GAIN_MAX))
        })
    })
}

pub(crate) fn fit(
    warped: &Rgb8,
    valid: &[bool],
    reference: RgbView<'_>,
    params: &FitParams,
) -> Model {
    let sampler = GainSampler::new(warped.width(), warped.height());
    let mut model = Model {
        luts: median_luts(warped, valid, reference),
        gains: [[1.0; GAIN_CELLS]; 3],
    };
    for pass in 0..params.passes.max(1) {
        if pass > 0 {
            model.luts = trimmed_luts(warped, valid, reference, &model, &sampler, params.trim);
        }
        let trim = (pass > 0).then_some(params.trim);
        model.gains = fit_gains(
            warped,
            valid,
            reference,
            &model,
            &sampler,
            trim,
            params.ridge,
        );
    }
    model
}

pub(crate) fn apply(warped: &mut Rgb8, valid: &[bool], model: &Model) {
    let (width, height) = warped.dimensions();
    let sampler = GainSampler::new(width, height);
    for y in 0..height as usize {
        let gain_row = sampler.row(&model.gains, y);
        let flags = &valid[y * width as usize..][..width as usize];
        let row = warped.row_mut(y as u32);
        for x in (0..width as usize).filter(|&x| flags[x]) {
            let g = sampler.at(&gain_row, x);
            for c in 0..3 {
                let v = (g[c] * model.luts[c][row[x * 3 + c] as usize]).round();
                row[x * 3 + c] = v.clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARAMS: FitParams = FitParams {
        ridge: 1e-3,
        trim: 40.0,
        passes: 3,
    };

    fn textured(w: u32, h: u32, seed: u64) -> Rgb8 {
        let mut s = seed;
        Rgb8::from_fn(w, h, |x, y| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let base = ((x * 3 + y * 2) % 200) as u8 + 20;
            [base, base.wrapping_add((s % 30) as u8), 255 - base]
        })
        .unwrap()
    }

    fn map(img: &Rgb8, f: impl Fn(u32, u32, usize, u8) -> u8) -> Rgb8 {
        let mut out = img.clone();
        for y in 0..img.height() {
            let row = out.row_mut(y);
            for x in 0..img.width() {
                for c in 0..3 {
                    let i = x as usize * 3 + c;
                    row[i] = f(x, y, c, row[i]);
                }
            }
        }
        out
    }

    fn mean_error(a: &Rgb8, b: &Rgb8) -> f64 {
        a.data()
            .iter()
            .zip(b.data())
            .map(|(x, y)| x.abs_diff(*y) as u64)
            .sum::<u64>() as f64
            / a.data().len() as f64
    }

    fn corrected(query: &Rgb8, reference: &Rgb8, valid: &[bool]) -> Rgb8 {
        let model = fit(query, valid, reference.view(), &PARAMS);
        let mut out = query.clone();
        apply(&mut out, valid, &model);
        out
    }

    #[test]
    fn isotonic_keeps_monotone_input() {
        assert_eq!(
            isotonic(&[(1.0, 1.0), (2.0, 1.0), (5.0, 2.0)]),
            vec![1.0, 2.0, 5.0]
        );
    }

    #[test]
    fn isotonic_pools_violators() {
        assert_eq!(
            isotonic(&[(1.0, 1.0), (3.0, 1.0), (2.0, 1.0)]),
            vec![1.0, 2.5, 2.5]
        );
        assert_eq!(isotonic(&[(5.0, 1.0), (1.0, 3.0)]), vec![2.0, 2.0]);
        assert_eq!(
            isotonic(&[(4.0, 1.0), (3.0, 1.0), (2.0, 1.0), (1.0, 1.0)]),
            vec![2.5; 4]
        );
        assert!(isotonic(&[]).is_empty());
    }

    #[test]
    fn isotonic_property_monotone_and_mean_preserving() {
        let mut s = 0x77u64;
        for _ in 0..100 {
            let pts: Vec<(f64, f64)> = (0..16)
                .map(|_| {
                    s ^= s << 13;
                    s ^= s >> 7;
                    s ^= s << 17;
                    ((s % 256) as f64, 1.0 + (s >> 32) as f64 % 50.0)
                })
                .collect();
            let fit = isotonic(&pts);
            assert!(fit.windows(2).all(|w| w[0] <= w[1]));
            let before: f64 = pts.iter().map(|(v, w)| v * w).sum();
            let after: f64 = pts.iter().zip(&fit).map(|((_, w), f)| f * w).sum();
            assert!((before - after).abs() < 1e-6);
        }
    }

    #[test]
    fn lut_identity_when_no_knots() {
        let lut = lut_from_knots(&[]);
        assert_eq!((lut[0], lut[255]), (0.0, 255.0));
        assert_eq!(lut_from_bins(&[(0.0, 0.0, 0.0); 16]), identity_lut());
    }

    #[test]
    fn lut_interpolates_and_clamps_ends() {
        let lut = lut_from_knots(&[(10.0, 20.0), (20.0, 40.0)]);
        assert_eq!((lut[0], lut[15], lut[200]), (20.0, 30.0, 40.0));
    }

    #[test]
    fn lut_from_bins_skips_empty_and_enforces_monotone() {
        let mut bins = [(0.0, 0.0, 0.0); 16];
        bins[2] = (40.0, 100.0, 5.0);
        bins[5] = (90.0, 80.0, 5.0);
        let lut = lut_from_bins(&bins);
        assert_eq!(lut[0], 90.0);
        assert_eq!(lut[255], 90.0);
    }

    #[test]
    fn identical_images_stay_identical() {
        let img = textured(64, 48, 1);
        let valid = vec![true; 64 * 48];
        assert!(mean_error(&corrected(&img, &img, &valid), &img) < 1.0);
    }

    #[test]
    fn inverts_monotone_filter() {
        let reference = textured(96, 64, 2);
        let filtered = map(&reference, |_, _, c, v| {
            if c == 0 {
                (v as u32 * (v as u32 + 255) / 510) as u8
            } else {
                v / 2 + 60
            }
        });
        let out = corrected(&filtered, &reference, &vec![true; 96 * 64]);
        assert!(
            mean_error(&out, &reference) < 2.0,
            "{}",
            mean_error(&out, &reference)
        );
    }

    #[test]
    fn absorbs_vignette() {
        let reference = textured(128, 96, 3);
        let vignette = |x: u32, y: u32| {
            let (dx, dy) = ((x as f64 - 64.0) / 64.0, (y as f64 - 48.0) / 48.0);
            1.0 - 0.45 * (dx * dx + dy * dy) / 2.0
        };
        let query = map(&reference, |x, y, _, v| {
            (v as f64 * vignette(x, y)).round() as u8
        });
        let out = corrected(&query, &reference, &vec![true; 128 * 96]);
        assert!(
            mean_error(&out, &reference) < 3.0,
            "{} vs {}",
            mean_error(&out, &reference),
            mean_error(&query, &reference)
        );
    }

    #[test]
    fn local_outliers_do_not_bias_fit() {
        let reference = textured(128, 96, 4);
        let mut query = reference.clone();
        for y in 30..60 {
            for x in 40..80 {
                let v = if (x / 4) % 2 == 0 {
                    [10, 10, 10]
                } else {
                    [250, 250, 250]
                };
                query.row_mut(y)[x as usize * 3..x as usize * 3 + 3].copy_from_slice(&v);
            }
        }
        let out = corrected(&query, &reference, &vec![true; 128 * 96]);
        let mut worst = 0u8;
        for y in 0..96u32 {
            for x in 0..128u32 {
                if (35..85).contains(&x) && (25..65).contains(&y) {
                    continue;
                }
                let (a, b) = (out.pixel(x, y).unwrap(), reference.pixel(x, y).unwrap());
                worst = worst.max((0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap());
            }
        }
        assert!(worst <= 12, "{worst}");
    }

    #[test]
    fn no_valid_pixels_gives_identity_model() {
        let img = textured(16, 16, 5);
        let model = fit(&img, &vec![false; 256], img.view(), &PARAMS);
        assert_eq!(model.luts[0], identity_lut());
        for g in model.gains.iter().flatten() {
            assert!((g - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn apply_skips_invalid_pixels() {
        let mut img = Rgb8::filled(4, 1, [100, 100, 100]).unwrap();
        let model = Model {
            luts: [[50.0; 256]; 3],
            gains: [[1.0; GAIN_CELLS]; 3],
        };
        apply(&mut img, &[true, false, true, false], &model);
        assert_eq!(
            img.data(),
            &[50, 50, 50, 100, 100, 100, 50, 50, 50, 100, 100, 100]
        );
    }

    #[test]
    fn axis_covers_grid() {
        let a = axis(9);
        assert_eq!(a.index[0], 0);
        assert_eq!(a.index[8], 2);
        assert!(a.frac.iter().all(|&f| (0.0..=1.0).contains(&f)));
        assert!(axis(0).index.is_empty());
    }

    #[test]
    fn fit_is_deterministic() {
        let reference = textured(64, 64, 6);
        let query = map(&reference, |x, _, _, v| v / 2 + (x / 4) as u8);
        let valid = vec![true; 64 * 64];
        assert_eq!(
            fit(&query, &valid, reference.view(), &PARAMS),
            fit(&query, &valid, reference.view(), &PARAMS)
        );
    }
}
