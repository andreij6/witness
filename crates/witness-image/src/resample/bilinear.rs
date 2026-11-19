use crate::buffer::{Image, View};
use crate::error::ImageError;

pub const BILINEAR_FRACTION_BITS: u32 = 11;
const ONE: u32 = 1 << BILINEAR_FRACTION_BITS;
const ROUND: u32 = 1 << (2 * BILINEAR_FRACTION_BITS - 1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tap {
    pub(crate) lo: u32,
    pub(crate) hi: u32,
    pub(crate) frac: u32,
}

pub(crate) fn taps(src: u32, dst: u32) -> Vec<Tap> {
    let (src64, den) = (src as i64, 2 * dst as i64);
    let last = src - 1;
    (0..dst as i64)
        .map(|i| {
            let num = (2 * i + 1) * src64 - dst as i64;
            if num <= 0 {
                return Tap {
                    lo: 0,
                    hi: 0,
                    frac: 0,
                };
            }
            let lo = (num / den) as u32;
            if lo >= last {
                return Tap {
                    lo: last,
                    hi: last,
                    frac: 0,
                };
            }
            let frac = (((num % den) as u64) << BILINEAR_FRACTION_BITS) / den as u64;
            Tap {
                lo,
                hi: lo + 1,
                frac: frac as u32,
            }
        })
        .collect()
}

fn horizontal<const C: usize>(row: &[u8], taps: &[Tap], out: &mut [u32]) {
    for (tap, acc) in taps.iter().zip(out.chunks_exact_mut(C)) {
        let a = &row[tap.lo as usize * C..][..C];
        let b = &row[tap.hi as usize * C..][..C];
        for c in 0..C {
            acc[c] = a[c] as u32 * (ONE - tap.frac) + b[c] as u32 * tap.frac;
        }
    }
}

struct RowCache {
    index: [u32; 2],
    rows: [Vec<u32>; 2],
}

impl RowCache {
    fn new(len: usize) -> Self {
        Self {
            index: [u32::MAX; 2],
            rows: [vec![0; len], vec![0; len]],
        }
    }

    fn load<const C: usize>(
        &mut self,
        src: &View<'_, C>,
        taps: &[Tap],
        wanted: u32,
        keep: u32,
    ) -> usize {
        if let Some(slot) = self.index.iter().position(|&i| i == wanted) {
            return slot;
        }
        let slot = usize::from(self.index[0] == keep);
        horizontal::<C>(src.row(wanted), taps, &mut self.rows[slot]);
        self.index[slot] = wanted;
        slot
    }
}

pub fn resize_bilinear<const C: usize>(
    src: View<'_, C>,
    dst_width: u32,
    dst_height: u32,
) -> Result<Image<C>, ImageError> {
    let out_len = crate::buffer::packed_len(dst_width, dst_height, C)?;
    let x_taps = taps(src.width(), dst_width);
    let y_taps = taps(src.height(), dst_height);
    let row_len = dst_width as usize * C;
    let mut cache = RowCache::new(row_len);
    let mut out = Vec::with_capacity(out_len);
    for tap in &y_taps {
        let top = cache.load(&src, &x_taps, tap.lo, tap.hi);
        let bottom = cache.load(&src, &x_taps, tap.hi, tap.lo);
        let (wt, wb) = (ONE - tap.frac, tap.frac);
        let (a, b) = (&cache.rows[top], &cache.rows[bottom]);
        out.extend(
            a.iter()
                .zip(b)
                .map(|(&a, &b)| ((a * wt + b * wb + ROUND) >> (2 * BILINEAR_FRACTION_BITS)) as u8),
        );
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

    fn float_reference(src: &Luma8, dw: u32, dh: u32) -> Vec<f64> {
        let (sw, sh) = src.dimensions();
        let coord = |i: u32, s: u32, d: u32| {
            let c = ((i as f64 + 0.5) * s as f64 / d as f64 - 0.5).max(0.0);
            let lo = (c.floor() as u32).min(s - 1);
            let hi = (lo + 1).min(s - 1);
            let t = if lo == s - 1 { 0.0 } else { c - lo as f64 };
            (lo, hi, t)
        };
        let mut out = Vec::new();
        for y in 0..dh {
            let (y0, y1, ty) = coord(y, sh, dh);
            for x in 0..dw {
                let (x0, x1, tx) = coord(x, sw, dw);
                let p = |x: u32, y: u32| src.pixel(x, y).unwrap()[0] as f64;
                let top = p(x0, y0) * (1.0 - tx) + p(x1, y0) * tx;
                let bottom = p(x0, y1) * (1.0 - tx) + p(x1, y1) * tx;
                out.push(top * (1.0 - ty) + bottom * ty);
            }
        }
        out
    }

    #[test]
    fn taps_identity() {
        for tap in taps(5, 5).iter().enumerate() {
            assert_eq!(tap.1.lo, tap.0 as u32);
            assert_eq!(tap.1.frac, 0);
        }
    }

    #[test]
    fn taps_upscale_by_two() {
        let t = taps(2, 4);
        assert_eq!(
            t[0],
            Tap {
                lo: 0,
                hi: 0,
                frac: 0
            }
        );
        assert_eq!(
            t[1],
            Tap {
                lo: 0,
                hi: 1,
                frac: ONE / 4
            }
        );
        assert_eq!(
            t[2],
            Tap {
                lo: 0,
                hi: 1,
                frac: 3 * ONE / 4
            }
        );
        assert_eq!(
            t[3],
            Tap {
                lo: 1,
                hi: 1,
                frac: 0
            }
        );
    }

    #[test]
    fn taps_downscale_by_two_hit_midpoints() {
        let t = taps(4, 2);
        assert_eq!(
            t[0],
            Tap {
                lo: 0,
                hi: 1,
                frac: ONE / 2
            }
        );
        assert_eq!(
            t[1],
            Tap {
                lo: 2,
                hi: 3,
                frac: ONE / 2
            }
        );
    }

    #[test]
    fn taps_single_source_pixel() {
        assert!(taps(1, 7).iter().all(|t| *t
            == Tap {
                lo: 0,
                hi: 0,
                frac: 0
            }));
    }

    #[test]
    fn taps_stay_in_bounds() {
        let mut rng = XorShift(17);
        for _ in 0..200 {
            let src = 1 + (rng.next() % 5000) as u32;
            let dst = 1 + (rng.next() % 600) as u32;
            for t in taps(src, dst) {
                assert!(t.lo < src && t.hi < src && t.frac < ONE && t.hi >= t.lo);
            }
        }
    }

    #[test]
    fn identity_is_exact() {
        let src = Rgb8::from_fn(11, 7, |x, y| [x as u8 * 20, y as u8 * 30, 99]).unwrap();
        assert_eq!(resize_bilinear(src.view(), 11, 7).unwrap(), src);
    }

    #[test]
    fn downscale_by_two_averages_quads() {
        let src = Luma8::new(4, 2, vec![0, 10, 20, 30, 40, 50, 60, 70]).unwrap();
        assert_eq!(resize_bilinear(src.view(), 2, 1).unwrap().data(), &[25, 45]);
    }

    #[test]
    fn upscale_known_values() {
        let src = Luma8::new(2, 1, vec![0, 200]).unwrap();
        assert_eq!(
            resize_bilinear(src.view(), 4, 1).unwrap().data(),
            &[0, 50, 150, 200]
        );
    }

    #[test]
    fn matches_float_reference_within_one() {
        let mut rng = XorShift(23);
        for _ in 0..40 {
            let sw = 1 + (rng.next() % 40) as u32;
            let sh = 1 + (rng.next() % 40) as u32;
            let dw = 1 + (rng.next() % 40) as u32;
            let dh = 1 + (rng.next() % 40) as u32;
            let src = Luma8::from_fn(sw, sh, |_, _| [(rng.next() % 256) as u8]).unwrap();
            let fixed = resize_bilinear(src.view(), dw, dh).unwrap();
            for (f, r) in fixed.data().iter().zip(float_reference(&src, dw, dh)) {
                assert!(
                    (*f as f64 - r).abs() <= 1.0,
                    "{sw}x{sh}->{dw}x{dh}: {f} vs {r}"
                );
            }
        }
    }

    #[test]
    fn constant_stays_constant() {
        let src = Rgb8::filled(33, 21, [255, 0, 128]).unwrap();
        for (w, h) in [(256, 256), (5, 3), (1, 1), (33, 21)] {
            let out = resize_bilinear(src.view(), w, h).unwrap();
            assert!(out.data().chunks_exact(3).all(|p| p == [255, 0, 128]));
        }
    }

    #[test]
    fn crop_view_equals_owned_crop() {
        let full =
            Rgb8::from_fn(60, 40, |x, y| [(x * 4) as u8, (y * 6) as u8, (x ^ y) as u8]).unwrap();
        let view = full.crop(10, 3, 37, 29).unwrap();
        assert_eq!(
            resize_bilinear(view, 256, 256).unwrap(),
            resize_bilinear(view.to_image().view(), 256, 256).unwrap()
        );
    }

    #[test]
    fn row_cache_reuses_rows() {
        let src = Luma8::from_fn(3, 3, |x, y| [(x + 3 * y) as u8 * 10]).unwrap();
        let x_taps = taps(3, 3);
        let mut cache = RowCache::new(3);
        let a = cache.load(&src.view(), &x_taps, 0, 1);
        let b = cache.load(&src.view(), &x_taps, 1, 0);
        assert_ne!(a, b);
        assert_eq!(cache.load(&src.view(), &x_taps, 0, 1), a);
        let c = cache.load(&src.view(), &x_taps, 2, 1);
        assert_eq!(c, a);
        assert_eq!(cache.load(&src.view(), &x_taps, 1, 2), b);
    }

    #[test]
    fn rejects_zero_target() {
        let src = Luma8::filled(4, 4, [0]).unwrap();
        assert_eq!(
            resize_bilinear(src.view(), 0, 4),
            Err(ImageError::InvalidDimensions)
        );
    }

    #[test]
    fn watermark_size_from_photo_shape() {
        let src = Rgb8::from_fn(1200, 900, |x, y| [(x % 251) as u8, (y % 241) as u8, 1]).unwrap();
        let out = resize_bilinear(src.view(), 256, 256).unwrap();
        assert_eq!(out.dimensions(), (256, 256));
        assert!(out.data().chunks_exact(3).all(|p| p[2] == 1));
    }
}
