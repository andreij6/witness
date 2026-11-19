use crate::buffer::{Image, Luma8, LumaView};
use crate::error::ImageError;

pub const Q8_ONE: u32 = 1 << 8;

pub fn jarosz_window(src_len: u32, dst_len: u32) -> u32 {
    let doubled = 2 * dst_len.max(1) as u64;
    (src_len as u64).div_ceil(doubled).max(1) as u32
}

pub fn jarosz_sample_index(index: u32, src_len: u32, dst_len: u32) -> u32 {
    ((2 * index as u64 + 1) * src_len as u64 / (2 * dst_len as u64)) as u32
}

pub(crate) fn box_filter(src: &[u32], dst: &mut [u32], window: usize) {
    let n = src.len();
    if n == 0 {
        return;
    }
    let window = window.clamp(1, n);
    let half = (window + 2) / 2;
    let ahead = half - 1;
    let behind = window - half;
    let mut sum = 0u64;
    let (mut lo, mut hi) = (0usize, 0usize);
    for (i, out) in dst.iter_mut().enumerate().take(n) {
        let new_hi = (i + ahead + 1).min(n);
        while hi < new_hi {
            sum += src[hi] as u64;
            hi += 1;
        }
        let new_lo = i.saturating_sub(behind);
        while lo < new_lo {
            sum -= src[lo] as u64;
            lo += 1;
        }
        let count = (hi - lo) as u64;
        *out = ((sum + count / 2) / count) as u32;
    }
}

fn filter_twice(line: &mut [u32], scratch: &mut [u32], window: usize) {
    box_filter(line, scratch, window);
    box_filter(scratch, line, window);
}

fn sample_positions(src_len: u32, dst_len: u32) -> Vec<usize> {
    (0..dst_len)
        .map(|i| jarosz_sample_index(i, src_len, dst_len) as usize)
        .collect()
}

pub fn jarosz_downsample_q8(
    src: LumaView<'_>,
    dst_width: u32,
    dst_height: u32,
) -> Result<Vec<u16>, ImageError> {
    if dst_width == 0 || dst_height == 0 {
        return Err(ImageError::InvalidDimensions);
    }
    let (src_w, src_h) = (src.width() as usize, src.height() as usize);
    let window_x = jarosz_window(src.width(), dst_width) as usize;
    let window_y = jarosz_window(src.height(), dst_height) as usize;
    let xs = sample_positions(src.width(), dst_width);
    let ys = sample_positions(src.height(), dst_height);

    let mut line = vec![0u32; src_w];
    let mut scratch = vec![0u32; src_w.max(src_h)];
    let mut columns = vec![0u32; xs.len() * src_h];
    for (y, row) in src.rows().enumerate() {
        line.iter_mut()
            .zip(row)
            .for_each(|(dst, &v)| *dst = v as u32 * Q8_ONE);
        filter_twice(&mut line, &mut scratch[..src_w], window_x);
        for (column, &x) in xs.iter().enumerate() {
            columns[column * src_h + y] = line[x];
        }
    }

    let out_w = xs.len();
    let mut out = vec![0u16; out_w * ys.len()];
    for (column_index, column) in columns.chunks_exact_mut(src_h).enumerate() {
        filter_twice(column, &mut scratch[..src_h], window_y);
        for (row_index, &y) in ys.iter().enumerate() {
            out[row_index * out_w + column_index] = column[y] as u16;
        }
    }
    Ok(out)
}

pub fn jarosz_downsample(
    src: LumaView<'_>,
    dst_width: u32,
    dst_height: u32,
) -> Result<Luma8, ImageError> {
    let q8 = jarosz_downsample_q8(src, dst_width, dst_height)?;
    let data = q8
        .into_iter()
        .map(|v| ((v as u32 + Q8_ONE / 2) >> 8) as u8)
        .collect();
    Ok(Image::from_parts_unchecked(dst_width, dst_height, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_box(src: &[u32], window: usize) -> Vec<u32> {
        let n = src.len();
        let window = window.clamp(1, n);
        let half = (window + 2) / 2;
        let (ahead, behind) = (half - 1, window - half);
        (0..n)
            .map(|i| {
                let lo = i.saturating_sub(behind);
                let hi = (i + ahead).min(n - 1);
                let sum: u64 = src[lo..=hi].iter().map(|&v| v as u64).sum();
                let count = (hi - lo + 1) as u64;
                ((sum + count / 2) / count) as u32
            })
            .collect()
    }

    fn pdq_float_box(src: &[f64], window: usize) -> Vec<f64> {
        let n = src.len();
        let half = (window + 2) / 2;
        let phase1 = half - 1;
        let phase2 = window - half + 1;
        let phase3 = n - window;
        let phase4 = half - 1;
        let mut out = vec![0.0; n];
        let (mut li, mut ri, mut oi) = (0, 0, 0);
        let (mut sum, mut size) = (0.0, 0.0);
        for _ in 0..phase1 {
            sum += src[ri];
            size += 1.0;
            ri += 1;
        }
        for _ in 0..phase2 {
            sum += src[ri];
            size += 1.0;
            out[oi] = sum / size;
            ri += 1;
            oi += 1;
        }
        for _ in 0..phase3 {
            sum += src[ri];
            sum -= src[li];
            out[oi] = sum / size;
            li += 1;
            ri += 1;
            oi += 1;
        }
        for _ in 0..phase4 {
            sum -= src[li];
            size -= 1.0;
            out[oi] = sum / size;
            li += 1;
            oi += 1;
        }
        out
    }

    struct XorShift(u64);

    impl XorShift {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }

    #[test]
    fn window_is_ceil_of_half_ratio() {
        assert_eq!(jarosz_window(512, 64), 4);
        assert_eq!(jarosz_window(513, 64), 5);
        assert_eq!(jarosz_window(4032, 64), 32);
        assert_eq!(jarosz_window(3024, 64), 24);
        assert_eq!(jarosz_window(64, 64), 1);
        assert_eq!(jarosz_window(10, 64), 1);
        assert_eq!(jarosz_window(1, 1), 1);
        assert_eq!(jarosz_window(u32::MAX, 1), u32::MAX.div_ceil(2));
        assert_eq!(jarosz_window(100, 0), 50);
    }

    #[test]
    fn sample_index_matches_pdq_centres() {
        assert_eq!(jarosz_sample_index(0, 512, 64), 4);
        assert_eq!(jarosz_sample_index(63, 512, 64), 508);
        for (src, dst) in [(7u32, 3u32), (100, 64), (64, 64), (3, 10), (4032, 64)] {
            for i in 0..dst {
                let expected = ((i as f64 + 0.5) * src as f64 / dst as f64) as u32;
                assert_eq!(jarosz_sample_index(i, src, dst), expected);
                assert!(jarosz_sample_index(i, src, dst) < src);
            }
        }
    }

    #[test]
    fn box_filter_matches_naive_on_random_inputs() {
        let mut rng = XorShift(0xDEAD_BEEF);
        for _ in 0..300 {
            let n = 1 + (rng.next() % 90) as usize;
            let window = 1 + (rng.next() % 100) as usize;
            let src: Vec<u32> = (0..n).map(|_| (rng.next() % 65_281) as u32).collect();
            let mut dst = vec![0; n];
            box_filter(&src, &mut dst, window);
            assert_eq!(dst, naive_box(&src, window), "n={n} window={window}");
        }
    }

    #[test]
    fn box_filter_matches_pdq_reference_within_rounding() {
        let mut rng = XorShift(42);
        for _ in 0..200 {
            let n = 2 + (rng.next() % 80) as usize;
            let window = 1 + (rng.next() % n as u64) as usize;
            let src: Vec<u32> = (0..n).map(|_| (rng.next() % 256) as u32 * 256).collect();
            let float: Vec<f64> = src.iter().map(|&v| v as f64).collect();
            let mut dst = vec![0; n];
            box_filter(&src, &mut dst, window);
            for (fixed, reference) in dst.iter().zip(pdq_float_box(&float, window)) {
                assert!((*fixed as f64 - reference).abs() <= 0.5 + 1e-9);
            }
        }
    }

    #[test]
    fn box_filter_constant_is_fixed_point() {
        let src = vec![12_345u32; 33];
        for window in [1, 2, 3, 8, 33, 100] {
            let mut dst = vec![0; 33];
            box_filter(&src, &mut dst, window);
            assert!(dst.iter().all(|&v| v == 12_345));
        }
    }

    #[test]
    fn box_filter_window_one_is_identity() {
        let src: Vec<u32> = (0..17).map(|v| v * 1000).collect();
        let mut dst = vec![0; 17];
        box_filter(&src, &mut dst, 1);
        assert_eq!(dst, src);
    }

    #[test]
    fn box_filter_empty_is_noop() {
        let mut dst: [u32; 0] = [];
        box_filter(&[], &mut dst, 3);
    }

    #[test]
    fn constant_image_downsamples_to_constant() {
        let src = Luma8::filled(300, 200, [77]).unwrap();
        let out = jarosz_downsample(src.view(), 64, 64).unwrap();
        assert_eq!(out.dimensions(), (64, 64));
        assert!(out.data().iter().all(|&v| v == 77));
        let q8 = jarosz_downsample_q8(src.view(), 64, 64).unwrap();
        assert!(q8.iter().all(|&v| v == 77 * 256));
    }

    #[test]
    fn identity_size_with_window_one_is_exact() {
        let src = Luma8::from_fn(16, 16, |x, y| [(x * 13 + y * 7) as u8]).unwrap();
        assert_eq!(jarosz_downsample(src.view(), 16, 16).unwrap(), src);
    }

    #[test]
    fn known_small_case() {
        let src = Luma8::new(8, 1, vec![0, 100, 200, 100, 0, 0, 0, 0]).unwrap();
        let out = jarosz_downsample_q8(src.view(), 2, 1).unwrap();
        assert_eq!(out, vec![25_600, 0]);
    }

    #[test]
    fn matches_full_resolution_filter_then_decimate() {
        let mut rng = XorShift(7);
        for _ in 0..20 {
            let w = 1 + (rng.next() % 70) as u32;
            let h = 1 + (rng.next() % 70) as u32;
            let dw = 1 + (rng.next() % 20) as u32;
            let dh = 1 + (rng.next() % 20) as u32;
            let src = Luma8::from_fn(w, h, |_, _| [(rng.next() % 256) as u8]).unwrap();
            let (wu, hu) = (w as usize, h as usize);
            let wx = jarosz_window(w, dw) as usize;
            let wy = jarosz_window(h, dh) as usize;
            let mut plane: Vec<u32> = src.data().iter().map(|&v| v as u32 * 256).collect();
            for row in plane.chunks_exact_mut(wu) {
                let once = naive_box(row, wx);
                row.copy_from_slice(&naive_box(&once, wx));
            }
            for x in 0..wu {
                let column: Vec<u32> = (0..hu).map(|y| plane[y * wu + x]).collect();
                let filtered = naive_box(&naive_box(&column, wy), wy);
                (0..hu).for_each(|y| plane[y * wu + x] = filtered[y]);
            }
            let expected: Vec<u16> = (0..dh)
                .flat_map(|i| {
                    let plane = &plane;
                    (0..dw).map(move |j| {
                        let sy = jarosz_sample_index(i, h, dh) as usize;
                        let sx = jarosz_sample_index(j, w, dw) as usize;
                        plane[sy * wu + sx] as u16
                    })
                })
                .collect();
            assert_eq!(jarosz_downsample_q8(src.view(), dw, dh).unwrap(), expected);
        }
    }

    #[test]
    fn rejects_zero_target() {
        let src = Luma8::filled(4, 4, [0]).unwrap();
        assert_eq!(
            jarosz_downsample(src.view(), 0, 4),
            Err(ImageError::InvalidDimensions)
        );
        assert_eq!(
            jarosz_downsample_q8(src.view(), 4, 0),
            Err(ImageError::InvalidDimensions)
        );
    }

    #[test]
    fn upsampling_target_is_allowed() {
        let src = Luma8::new(2, 1, vec![10, 20]).unwrap();
        let out = jarosz_downsample(src.view(), 4, 2).unwrap();
        assert_eq!(out.data(), &[10, 10, 20, 20, 10, 10, 20, 20]);
    }

    #[test]
    fn works_on_crop_views() {
        let full = Luma8::from_fn(50, 40, |x, y| [(x * 5 + y) as u8]).unwrap();
        let view = full.crop(5, 3, 30, 20).unwrap();
        assert_eq!(
            jarosz_downsample_q8(view, 8, 8).unwrap(),
            jarosz_downsample_q8(view.to_image().view(), 8, 8).unwrap()
        );
    }

    #[test]
    fn deterministic_across_runs() {
        let mut rng = XorShift(99);
        let src = Luma8::from_fn(257, 193, |_, _| [(rng.next() % 256) as u8]).unwrap();
        let first = jarosz_downsample_q8(src.view(), 64, 64).unwrap();
        assert_eq!(first, jarosz_downsample_q8(src.view(), 64, 64).unwrap());
    }

    #[test]
    fn large_image_smoke() {
        let src = Luma8::from_fn(2016, 1512, |x, y| [((x / 9 + y / 7) % 256) as u8]).unwrap();
        let out = jarosz_downsample(src.view(), 64, 64).unwrap();
        assert_eq!(out.data().len(), 64 * 64);
    }

    #[test]
    fn output_bounded_by_input_range() {
        let mut rng = XorShift(3);
        let src = Luma8::from_fn(120, 90, |_, _| [40 + (rng.next() % 100) as u8]).unwrap();
        let out = jarosz_downsample(src.view(), 16, 16).unwrap();
        assert!(out.data().iter().all(|&v| (40..140).contains(&v)));
    }
}
