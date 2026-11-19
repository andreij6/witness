use crate::buffer::{Image, View};
use crate::error::ImageError;

struct AxisWeights {
    first: Vec<u32>,
    offsets: Vec<u32>,
    weights: Vec<u64>,
}

impl AxisWeights {
    fn new(src: u32, dst: u32) -> Self {
        let (src64, dst64) = (src as u64, dst as u64);
        let mut first = Vec::with_capacity(dst as usize);
        let mut offsets = Vec::with_capacity(dst as usize + 1);
        let mut weights = Vec::with_capacity(src as usize + dst as usize);
        offsets.push(0);
        for i in 0..dst64 {
            let (lo, hi) = (i * src64, (i + 1) * src64);
            let (j0, j1) = (lo / dst64, (hi - 1) / dst64);
            first.push(j0 as u32);
            for j in j0..=j1 {
                weights.push(((j + 1) * dst64).min(hi) - (j * dst64).max(lo));
            }
            offsets.push(weights.len() as u32);
        }
        Self {
            first,
            offsets,
            weights,
        }
    }

    fn span(&self, i: usize) -> (usize, &[u64]) {
        let range = self.offsets[i] as usize..self.offsets[i + 1] as usize;
        (self.first[i] as usize, &self.weights[range])
    }
}

fn horizontal<const C: usize>(row: &[u8], axis: &AxisWeights, out: &mut [u64]) {
    for (x, acc) in out.chunks_exact_mut(C).enumerate() {
        let (start, weights) = axis.span(x);
        acc.fill(0);
        for (k, &w) in weights.iter().enumerate() {
            let pixel = &row[(start + k) * C..][..C];
            for c in 0..C {
                acc[c] += pixel[c] as u64 * w;
            }
        }
    }
}

pub fn resize_area<const C: usize>(
    src: View<'_, C>,
    dst_width: u32,
    dst_height: u32,
) -> Result<Image<C>, ImageError> {
    let out_len = crate::buffer::packed_len(dst_width, dst_height, C)?;
    let x_axis = AxisWeights::new(src.width(), dst_width);
    let y_axis = AxisWeights::new(src.height(), dst_height);
    let row_len = dst_width as usize * C;
    let denominator = src.width() as u64 * src.height() as u64;
    let half = denominator / 2;

    let mut cached_row = usize::MAX;
    let mut cached = vec![0u64; row_len];
    let mut acc = vec![0u64; row_len];
    let mut out = Vec::with_capacity(out_len);
    for y in 0..dst_height as usize {
        let (start, weights) = y_axis.span(y);
        acc.fill(0);
        for (k, &wy) in weights.iter().enumerate() {
            let source_row = start + k;
            if source_row != cached_row {
                horizontal::<C>(src.row(source_row as u32), &x_axis, &mut cached);
                cached_row = source_row;
            }
            acc.iter_mut().zip(&cached).for_each(|(a, &h)| *a += h * wy);
        }
        out.extend(acc.iter().map(|&a| ((a + half) / denominator) as u8));
    }
    Ok(Image::from_parts_unchecked(dst_width, dst_height, out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Luma8, Rgb8};

    struct XorShift(u64);

    impl XorShift {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }

    fn reference(src: &Luma8, dw: u32, dh: u32) -> Luma8 {
        let (sw, sh) = (src.width() as u64, src.height() as u64);
        let (dw64, dh64) = (dw as u64, dh as u64);
        let overlap = |a0: u64, a1: u64, b0: u64, b1: u64| a1.min(b1).saturating_sub(a0.max(b0));
        Luma8::from_fn(dw, dh, |x, y| {
            let mut total = 0u64;
            for sy in 0..sh {
                let wy = overlap(
                    y as u64 * sh,
                    (y as u64 + 1) * sh,
                    sy * dh64,
                    (sy + 1) * dh64,
                );
                for sx in 0..sw {
                    let wx = overlap(
                        x as u64 * sw,
                        (x as u64 + 1) * sw,
                        sx * dw64,
                        (sx + 1) * dw64,
                    );
                    total += src.pixel(sx as u32, sy as u32).unwrap()[0] as u64 * wx * wy;
                }
            }
            let den = sw * sh;
            [((total + den / 2) / den) as u8]
        })
        .unwrap()
    }

    #[test]
    fn weights_sum_to_source_length() {
        for (src, dst) in [(10u32, 3u32), (3, 10), (7, 7), (1, 5), (4032, 256), (13, 1)] {
            let axis = AxisWeights::new(src, dst);
            for i in 0..dst as usize {
                let (start, weights) = axis.span(i);
                assert_eq!(weights.iter().sum::<u64>(), src as u64);
                assert!(start + weights.len() <= src as usize);
                assert!(weights.iter().all(|&w| w > 0));
            }
            assert!(axis.weights.len() <= (src + dst) as usize);
        }
    }

    #[test]
    fn identity_size_is_exact() {
        let src = Rgb8::from_fn(13, 9, |x, y| [x as u8 * 19, y as u8 * 23, 200]).unwrap();
        assert_eq!(resize_area(src.view(), 13, 9).unwrap(), src);
    }

    #[test]
    fn integer_factor_averages_blocks() {
        let src = Luma8::new(4, 2, vec![0, 10, 20, 30, 40, 50, 60, 71]).unwrap();
        let out = resize_area(src.view(), 2, 1).unwrap();
        assert_eq!(out.data(), &[25, 45]);
    }

    #[test]
    fn rounding_is_half_up() {
        let src = Luma8::new(2, 1, vec![0, 1]).unwrap();
        assert_eq!(resize_area(src.view(), 1, 1).unwrap().data(), &[1]);
        let src = Luma8::new(4, 1, vec![0, 0, 0, 1]).unwrap();
        assert_eq!(resize_area(src.view(), 1, 1).unwrap().data(), &[0]);
    }

    #[test]
    fn fractional_ratio_matches_reference() {
        let mut rng = XorShift(11);
        for _ in 0..40 {
            let sw = 1 + (rng.next() % 30) as u32;
            let sh = 1 + (rng.next() % 30) as u32;
            let dw = 1 + (rng.next() % 30) as u32;
            let dh = 1 + (rng.next() % 30) as u32;
            let src = Luma8::from_fn(sw, sh, |_, _| [(rng.next() % 256) as u8]).unwrap();
            assert_eq!(
                resize_area(src.view(), dw, dh).unwrap(),
                reference(&src, dw, dh),
                "{sw}x{sh}->{dw}x{dh}"
            );
        }
    }

    #[test]
    fn upscale_replicates_by_area() {
        let src = Luma8::new(2, 1, vec![10, 30]).unwrap();
        assert_eq!(
            resize_area(src.view(), 4, 2).unwrap().data(),
            &[10, 10, 30, 30, 10, 10, 30, 30]
        );
        assert_eq!(resize_area(src.view(), 3, 1).unwrap().data(), &[10, 20, 30]);
    }

    #[test]
    fn constant_stays_constant() {
        let src = Rgb8::filled(97, 61, [3, 128, 255]).unwrap();
        let out = resize_area(src.view(), 17, 11).unwrap();
        assert!(out.data().chunks_exact(3).all(|p| p == [3, 128, 255]));
    }

    #[test]
    fn preserves_mean_on_exact_division() {
        let mut rng = XorShift(5);
        let src = Luma8::from_fn(64, 48, |_, _| [(rng.next() % 256) as u8]).unwrap();
        let out = resize_area(src.view(), 1, 1).unwrap();
        let sum: u64 = src.data().iter().map(|&v| v as u64).sum();
        assert_eq!(out.data()[0] as u64, (sum + 64 * 48 / 2) / (64 * 48));
    }

    #[test]
    fn channels_are_independent() {
        let src = Rgb8::from_fn(8, 8, |x, y| [x as u8 * 30, y as u8 * 30, 7]).unwrap();
        let out = resize_area(src.view(), 2, 2).unwrap();
        let r = Luma8::from_fn(8, 8, |x, _| [x as u8 * 30]).unwrap();
        let r_out = resize_area(r.view(), 2, 2).unwrap();
        for (rgb, luma) in out.data().chunks_exact(3).zip(r_out.data()) {
            assert_eq!(rgb[0], *luma);
            assert_eq!(rgb[2], 7);
        }
    }

    #[test]
    fn crop_view_equals_owned_crop() {
        let full = Rgb8::from_fn(40, 30, |x, y| [x as u8 * 6, y as u8 * 8, (x + y) as u8]).unwrap();
        let view = full.crop(4, 5, 25, 17).unwrap();
        assert_eq!(
            resize_area(view, 7, 5).unwrap(),
            resize_area(view.to_image().view(), 7, 5).unwrap()
        );
    }

    #[test]
    fn rejects_zero_target() {
        let src = Luma8::filled(4, 4, [0]).unwrap();
        assert_eq!(
            resize_area(src.view(), 0, 1),
            Err(ImageError::InvalidDimensions)
        );
        assert_eq!(
            resize_area(src.view(), 1, 0),
            Err(ImageError::InvalidDimensions)
        );
    }

    #[test]
    fn large_downscale_smoke() {
        let src = Rgb8::from_fn(1600, 1200, |x, y| [(x % 256) as u8, (y % 256) as u8, 9]).unwrap();
        let out = resize_area(src.view(), 256, 192).unwrap();
        assert_eq!(out.dimensions(), (256, 192));
        assert!(out.data().chunks_exact(3).all(|p| p[2] == 9));
    }
}
