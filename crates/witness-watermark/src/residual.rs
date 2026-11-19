use crate::error::WatermarkError;
use crate::tensor::{MODEL_SIZE, PLANE_LEN, TENSOR_LEN};
use witness_image::Rgb8;

pub const DEFAULT_STRENGTH: f32 = 1.0;
pub const RESIDUAL_LIMIT: f32 = 0.2;
const BORDER: usize = 2;
const HALF_RANGE: f32 = 127.5;
const CHANNELS: usize = 3;

#[derive(Clone, Debug, PartialEq)]
pub struct Residual {
    data: Vec<f32>,
}

impl Residual {
    pub fn from_model(
        mut stego: Vec<f32>,
        cover: &[f32],
        strength: f32,
    ) -> Result<Self, WatermarkError> {
        if stego.len() != TENSOR_LEN || cover.len() != TENSOR_LEN {
            return Err(WatermarkError::Inference);
        }
        for (s, &c) in stego.iter_mut().zip(cover) {
            let delta = strength * (*s - c);
            *s = if delta.is_nan() {
                0.0
            } else {
                delta.clamp(-RESIDUAL_LIMIT, RESIDUAL_LIMIT)
            };
        }
        Ok(Self { data: stego })
    }

    pub fn as_nchw(&self) -> &[f32] {
        &self.data
    }

    pub fn apply(&self, image: &mut Rgb8) {
        let (width, height) = image.dimensions();
        let grid = Grid::for_image(&self.data, width, height);
        let x_taps: Vec<Tap> = (0..width as usize)
            .map(|x| Tap::new(grid.width, width as usize, x))
            .collect();
        let row_len = width as usize * CHANNELS;
        let mut cache = [
            (usize::MAX, vec![0.0f32; row_len]),
            (usize::MAX, vec![0.0f32; row_len]),
        ];
        for y in 0..height {
            let tap = Tap::new(grid.height, height as usize, y as usize);
            let top = load(&mut cache, tap.lo, tap.hi, |gy, out| {
                grid.fill_row(gy, &x_taps, out)
            });
            let bottom = load(&mut cache, tap.hi, tap.lo, |gy, out| {
                grid.fill_row(gy, &x_taps, out)
            });
            let (wt, wb) = (1.0 - tap.l1, tap.l1);
            let (a, b) = (&cache[top].1, &cache[bottom].1);
            for ((p, &t), &u) in image.row_mut(y).iter_mut().zip(a).zip(b) {
                *p = add(*p, t * wt + u * wb);
            }
        }
    }
}

fn add(pixel: u8, residual: f32) -> u8 {
    ((pixel as f32 + residual * HALF_RANGE).clamp(0.0, 255.0) + 0.5) as u8
}

fn load(
    cache: &mut [(usize, Vec<f32>); 2],
    wanted: usize,
    keep: usize,
    fill: impl Fn(usize, &mut [f32]),
) -> usize {
    if let Some(slot) = cache.iter().position(|(index, _)| *index == wanted) {
        return slot;
    }
    let slot = usize::from(cache[0].0 == keep);
    fill(wanted, &mut cache[slot].1);
    cache[slot].0 = wanted;
    slot
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Tap {
    lo: usize,
    hi: usize,
    l1: f32,
}

impl Tap {
    fn new(in_size: usize, out_size: usize, i: usize) -> Self {
        let scale = in_size as f32 / out_size as f32;
        let src = (scale * (i as f32 + 0.5) - 0.5).max(0.0);
        let lo = (src as usize).min(in_size - 1);
        let l1 = (src - lo as f32).clamp(0.0, 1.0);
        let hi = if lo < in_size - 1 { lo + 1 } else { lo };
        Self { lo, hi, l1 }
    }
}

struct Grid<'a> {
    data: &'a [f32],
    width: usize,
    height: usize,
    x0: usize,
    y0: usize,
    border: usize,
    mean: [f32; CHANNELS],
}

impl<'a> Grid<'a> {
    fn for_image(data: &'a [f32], width: u32, height: u32) -> Self {
        let aspect = width as f32 / height as f32;
        if (0.5..=2.0).contains(&aspect) {
            return Self {
                data,
                width: MODEL_SIZE,
                height: MODEL_SIZE,
                x0: 0,
                y0: 0,
                border: 0,
                mean: [0.0; CHANNELS],
            };
        }
        let mean = core::array::from_fn(|c| {
            (data[c * PLANE_LEN..][..PLANE_LEN]
                .iter()
                .map(|&v| v as f64)
                .sum::<f64>()
                / PLANE_LEN as f64) as f32
        });
        let (gw, gh) = if width > height {
            ((aspect * MODEL_SIZE as f32) as usize, MODEL_SIZE)
        } else {
            (
                MODEL_SIZE,
                ((height as f32 / width as f32) * MODEL_SIZE as f32) as usize,
            )
        };
        Self {
            data,
            width: gw,
            height: gh,
            x0: (gw - MODEL_SIZE) / 2,
            y0: (gh - MODEL_SIZE) / 2,
            border: BORDER,
            mean,
        }
    }

    fn value(&self, c: usize, gy: usize, gx: usize) -> f32 {
        let (x, y) = (gx.wrapping_sub(self.x0), gy.wrapping_sub(self.y0));
        let inner = MODEL_SIZE - self.border;
        if x >= self.border && x < inner && y >= self.border && y < inner {
            self.data[c * PLANE_LEN + y * MODEL_SIZE + x]
        } else {
            self.mean[c]
        }
    }

    fn fill_row(&self, gy: usize, taps: &[Tap], out: &mut [f32]) {
        for (px, tap) in out.chunks_exact_mut(CHANNELS).zip(taps) {
            for (c, v) in px.iter_mut().enumerate() {
                *v =
                    self.value(c, gy, tap.lo) * (1.0 - tap.l1) + self.value(c, gy, tap.hi) * tap.l1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::Rng;

    fn residual(rng: &mut Rng) -> Residual {
        let stego: Vec<f32> = (0..TENSOR_LEN).map(|_| rng.unit() * 0.8 - 0.4).collect();
        Residual::from_model(stego, &vec![0.0; TENSOR_LEN], 1.0).unwrap()
    }

    fn constant(value: f32) -> Residual {
        Residual::from_model(vec![value; TENSOR_LEN], &vec![0.0; TENSOR_LEN], 1.0).unwrap()
    }

    fn torch_bilinear(grid: &Grid<'_>, c: usize, x: usize, y: usize, w: usize, h: usize) -> f32 {
        let coord = |i: usize, input: usize, output: usize| {
            let scale = input as f32 / output as f32;
            let real = scale * (i as f32 + 0.5) - 0.5;
            let real = if real < 0.0 { 0.0 } else { real };
            let index = (real.floor() as usize).min(input - 1);
            let lambda = (real - index as f32).clamp(0.0, 1.0);
            (index, index + usize::from(index < input - 1), lambda)
        };
        let (y0, y1, ly) = coord(y, grid.height, h);
        let (x0, x1, lx) = coord(x, grid.width, w);
        let row = |gy| grid.value(c, gy, x0) * (1.0 - lx) + grid.value(c, gy, x1) * lx;
        row(y0) * (1.0 - ly) + row(y1) * ly
    }

    #[test]
    fn from_model_scales_and_clamps() {
        let mut stego = vec![0.5f32; TENSOR_LEN];
        stego[0] = 0.6;
        stego[1] = 0.0;
        stego[2] = 0.55;
        stego[3] = f32::NAN;
        stego[4] = f32::INFINITY;
        let cover = vec![0.5f32; TENSOR_LEN];
        let r = Residual::from_model(stego, &cover, 0.5).unwrap();
        let d = r.as_nchw();
        assert!((d[0] - 0.05).abs() < 1e-6);
        assert_eq!(d[1], -0.2);
        assert!((d[2] - 0.025).abs() < 1e-6);
        assert_eq!(d[3], 0.0);
        assert_eq!(d[4], 0.2);
        assert!(d[5..].iter().all(|&v| v == 0.0));
    }

    #[test]
    fn from_model_rejects_wrong_lengths_and_nan_strength() {
        assert_eq!(
            Residual::from_model(vec![0.0; 3], &vec![0.0; TENSOR_LEN], 1.0),
            Err(WatermarkError::Inference)
        );
        assert_eq!(
            Residual::from_model(vec![0.0; TENSOR_LEN], &[0.0; 3], 1.0),
            Err(WatermarkError::Inference)
        );
        let r =
            Residual::from_model(vec![1.0; TENSOR_LEN], &vec![0.0; TENSOR_LEN], f32::NAN).unwrap();
        assert!(r.as_nchw().iter().all(|&v| v == 0.0));
    }

    #[test]
    fn zero_residual_leaves_image_untouched() {
        let mut rng = Rng::new(60);
        for (w, h) in [(400, 300), (1000, 100), (100, 1000), (1, 1), (3, 700)] {
            let original = rng.image(w, h);
            let mut img = original.clone();
            constant(0.0).apply(&mut img);
            assert_eq!(img, original);
        }
    }

    #[test]
    fn constant_residual_shifts_and_clamps() {
        let mut img = Rgb8::from_fn(300, 200, |x, _| [x as u8, 128, 250]).unwrap();
        constant(0.1).apply(&mut img);
        for px in img.data().chunks_exact(3) {
            assert_eq!(px[1], 141);
            assert_eq!(px[2], 255);
        }
        let mut dark = Rgb8::filled(256, 256, [5, 30, 0]).unwrap();
        constant(-0.2).apply(&mut dark);
        assert!(dark.data().chunks_exact(3).all(|p| p == [0, 5, 0]));
    }

    #[test]
    fn add_rounds_half_up_and_saturates() {
        assert_eq!(add(10, 0.0), 10);
        assert_eq!(add(10, 0.2), 36);
        assert_eq!(add(10, -0.2), 0);
        assert_eq!(add(10, -1.0 / HALF_RANGE * 0.49), 10);
        assert_eq!(add(255, 0.2), 255);
        assert_eq!(add(0, -0.2), 0);
        assert_eq!(add(200, 1.0), 255);
    }

    #[test]
    fn changes_are_bounded_by_residual_limit() {
        let mut rng = Rng::new(61);
        let limit = (RESIDUAL_LIMIT * HALF_RANGE).ceil() as i32;
        for (w, h) in [(640, 480), (257, 255), (90, 40), (1200, 300)] {
            let original = rng.image(w, h);
            let mut img = original.clone();
            residual(&mut rng).apply(&mut img);
            for (&a, &b) in original.data().iter().zip(img.data()) {
                assert!((a as i32 - b as i32).abs() <= limit);
            }
        }
    }

    #[test]
    fn upsample_matches_torch_reference_exactly() {
        let mut rng = Rng::new(62);
        let r = residual(&mut rng);
        for (w, h) in [
            (517, 389),
            (256, 256),
            (100, 77),
            (1100, 300),
            (60, 300),
            (1, 1),
            (2, 900),
        ] {
            let base = Rgb8::filled(w, h, [128; 3]).unwrap();
            let mut img = base.clone();
            r.apply(&mut img);
            let grid = Grid::for_image(r.as_nchw(), w, h);
            for y in (0..h as usize).step_by(7) {
                for x in (0..w as usize).step_by(5) {
                    for c in 0..3 {
                        let expected =
                            add(128, torch_bilinear(&grid, c, x, y, w as usize, h as usize));
                        assert_eq!(
                            img.pixel(x as u32, y as u32).unwrap()[c],
                            expected,
                            "{w}x{h} at {x},{y},{c}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn same_size_is_pointwise() {
        let mut rng = Rng::new(63);
        let r = residual(&mut rng);
        let mut img = Rgb8::filled(256, 256, [100; 3]).unwrap();
        r.apply(&mut img);
        for (i, px) in img.data().chunks_exact(3).enumerate() {
            for (c, &v) in px.iter().enumerate() {
                assert_eq!(v, add(100, r.as_nchw()[c * PLANE_LEN + i]));
            }
        }
    }

    #[test]
    fn taps_follow_half_pixel_convention() {
        assert_eq!(
            Tap::new(2, 4, 0),
            Tap {
                lo: 0,
                hi: 1,
                l1: 0.0
            }
        );
        assert_eq!(
            Tap::new(2, 4, 1),
            Tap {
                lo: 0,
                hi: 1,
                l1: 0.25
            }
        );
        assert_eq!(
            Tap::new(2, 4, 2),
            Tap {
                lo: 0,
                hi: 1,
                l1: 0.75
            }
        );
        assert_eq!(
            Tap::new(2, 4, 3),
            Tap {
                lo: 1,
                hi: 1,
                l1: 0.25
            }
        );
        assert_eq!(
            Tap::new(5, 5, 3),
            Tap {
                lo: 3,
                hi: 4,
                l1: 0.0
            }
        );
        assert_eq!(
            Tap::new(1, 9, 0),
            Tap {
                lo: 0,
                hi: 0,
                l1: 0.0
            }
        );
    }

    #[test]
    fn taps_stay_in_bounds() {
        let mut rng = Rng::new(64);
        for _ in 0..500 {
            let (input, output) = (1 + rng.below(3000) as usize, 1 + rng.below(9000) as usize);
            for i in [0, output / 3, output / 2, output - 1] {
                let t = Tap::new(input, output, i);
                assert!(
                    t.lo < input && t.hi < input && t.hi >= t.lo && (0.0..=1.0).contains(&t.l1)
                );
            }
        }
    }

    #[test]
    fn extreme_aspect_pads_with_channel_mean_and_softens_border() {
        let mut stego = vec![0.0f32; TENSOR_LEN];
        stego[..PLANE_LEN].iter_mut().for_each(|v| *v = 0.1);
        stego[PLANE_LEN..2 * PLANE_LEN]
            .iter_mut()
            .enumerate()
            .for_each(|(i, v)| *v = if i % 2 == 0 { 0.2 } else { -0.2 });
        let r = Residual::from_model(stego, &vec![0.0; TENSOR_LEN], 1.0).unwrap();
        let grid = Grid::for_image(r.as_nchw(), 1200, 300);
        assert_eq!(
            (grid.width, grid.height, grid.x0, grid.y0),
            (1024, 256, 384, 0)
        );
        assert_eq!(grid.mean, [0.1, 0.0, 0.0]);
        assert_eq!(grid.value(0, 0, 0), 0.1);
        assert_eq!(grid.value(1, 0, 0), 0.0);
        assert_eq!(grid.value(1, 10, 384 + 1), 0.0);
        assert_eq!(grid.value(1, 10, 384 + 2), 0.2);
        assert_eq!(grid.value(1, 10, 384 + 3), -0.2);
        assert_eq!(grid.value(1, 254, 384 + 3), 0.0);
        assert_eq!(grid.value(1, 10, 384 + 254), 0.0);
        let mut img = Rgb8::filled(1200, 300, [100; 3]).unwrap();
        r.apply(&mut img);
        assert_eq!(img.pixel(0, 150).unwrap(), [113, 100, 100]);
        assert_eq!(img.pixel(1199, 150).unwrap(), [113, 100, 100]);
    }

    #[test]
    fn tall_extreme_aspect_grid() {
        let r = constant(0.0);
        let grid = Grid::for_image(r.as_nchw(), 100, 1000);
        assert_eq!(
            (grid.width, grid.height, grid.x0, grid.y0),
            (256, 2560, 0, 1152)
        );
        let plain = Grid::for_image(r.as_nchw(), 200, 100);
        assert_eq!((plain.width, plain.height, plain.border), (256, 256, 0));
        let edge = Grid::for_image(r.as_nchw(), 201, 100);
        assert_eq!((edge.width, edge.border), (514, BORDER));
    }

    #[test]
    fn row_cache_reuses_and_evicts_correctly() {
        let mut cache = [(usize::MAX, vec![0.0f32; 1]), (usize::MAX, vec![0.0f32; 1])];
        let fill = |gy: usize, out: &mut [f32]| out[0] = gy as f32;
        let a = load(&mut cache, 3, 4, fill);
        let b = load(&mut cache, 4, 3, fill);
        assert_ne!(a, b);
        assert_eq!(load(&mut cache, 3, 4, fill), a);
        let c = load(&mut cache, 5, 4, fill);
        assert_eq!(c, a);
        assert_eq!(cache[c].1[0], 5.0);
        assert_eq!(cache[b].1[0], 4.0);
    }

    #[test]
    fn apply_is_deterministic() {
        let mut rng = Rng::new(65);
        let r = residual(&mut rng);
        let original = rng.image(333, 251);
        let (mut a, mut b) = (original.clone(), original);
        r.apply(&mut a);
        r.apply(&mut b);
        assert_eq!(a, b);
    }
}
