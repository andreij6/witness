use witness_image::{ImageError, Rgb8, RgbView};

const PRECISION_BITS: u32 = 22;
const HALF: i32 = 1 << (PRECISION_BITS - 1);
const CHANNELS: usize = 3;

pub(crate) trait RowSource {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn row(&mut self, y: u32) -> &[u8];
}

pub(crate) struct ViewRows<'a>(pub(crate) RgbView<'a>);

impl RowSource for ViewRows<'_> {
    fn width(&self) -> u32 {
        self.0.width()
    }

    fn height(&self) -> u32 {
        self.0.height()
    }

    fn row(&mut self, y: u32) -> &[u8] {
        self.0.row(y)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Kernel {
    pub(crate) size: usize,
    pub(crate) bounds: Vec<(usize, usize)>,
    pub(crate) weights: Vec<i32>,
}

impl Kernel {
    fn taps(&self, i: usize) -> (usize, &[i32]) {
        let (start, len) = self.bounds[i];
        (start, &self.weights[i * self.size..][..len])
    }
}

fn triangle(x: f64) -> f64 {
    let x = x.abs();
    if x < 1.0 { 1.0 - x } else { 0.0 }
}

pub(crate) fn kernel(in_size: u32, out_size: u32) -> Kernel {
    let scale = in_size as f32 as f64 / out_size as f64;
    let filter_scale = if scale < 1.0 { 1.0 } else { scale };
    let support = filter_scale;
    let size = support.ceil() as usize * 2 + 1;
    let inverse = 1.0 / filter_scale;
    let mut bounds = Vec::with_capacity(out_size as usize);
    let mut weights = vec![0i32; out_size as usize * size];
    let mut raw = vec![0f64; size];
    for (xx, out) in weights.chunks_exact_mut(size).enumerate() {
        let center = (xx as f64 + 0.5) * scale;
        let start = ((center - support + 0.5) as i64).max(0);
        let end = ((center + support + 0.5) as i64).min(in_size as i64);
        let len = (end - start).max(0) as usize;
        let mut total = 0.0;
        for (x, w) in raw[..len].iter_mut().enumerate() {
            *w = triangle(((x as i64 + start) as f64 - center + 0.5) * inverse);
            total += *w;
        }
        for (dst, &w) in out.iter_mut().zip(&raw[..len]) {
            let normalized = if total != 0.0 { w / total } else { w };
            *dst = (0.5 + normalized * (1 << PRECISION_BITS) as f64) as i32;
        }
        bounds.push((start as usize, len));
    }
    Kernel {
        size,
        bounds,
        weights,
    }
}

fn clip8(v: i32) -> u8 {
    (v >> PRECISION_BITS).clamp(0, 255) as u8
}

fn horizontal(row: &[u8], kernel: &Kernel, out: &mut [u8]) {
    for (xx, px) in out.chunks_exact_mut(CHANNELS).enumerate() {
        let (start, taps) = kernel.taps(xx);
        let mut acc = [HALF; CHANNELS];
        for (src, &w) in row[start * CHANNELS..].chunks_exact(CHANNELS).zip(taps) {
            for c in 0..CHANNELS {
                acc[c] += src[c] as i32 * w;
            }
        }
        for c in 0..CHANNELS {
            px[c] = clip8(acc[c]);
        }
    }
}

pub(crate) fn resize<S: RowSource>(
    src: &mut S,
    out_width: u32,
    out_height: u32,
) -> Result<Rgb8, ImageError> {
    let out_len = (out_width as usize)
        .checked_mul(out_height as usize)
        .and_then(|n| n.checked_mul(CHANNELS))
        .filter(|&n| n > 0)
        .ok_or(ImageError::InvalidDimensions)?;
    let x_kernel = kernel(src.width(), out_width);
    let y_kernel = kernel(src.height(), out_height);
    let row_len = out_width as usize * CHANNELS;
    let slots = y_kernel.size;
    let mut ring = vec![0u8; slots * row_len];
    let mut acc = vec![0i32; row_len];
    let mut out = Vec::with_capacity(out_len);
    let mut next = 0usize;
    for yy in 0..out_height as usize {
        let (start, taps) = y_kernel.taps(yy);
        next = next.max(start);
        while next < start + taps.len() {
            horizontal(
                src.row(next as u32),
                &x_kernel,
                &mut ring[(next % slots) * row_len..][..row_len],
            );
            next += 1;
        }
        acc.fill(HALF);
        for (j, &w) in taps.iter().enumerate() {
            let row = &ring[((start + j) % slots) * row_len..][..row_len];
            for (a, &v) in acc.iter_mut().zip(row) {
                *a += v as i32 * w;
            }
        }
        out.extend(acc.iter().map(|&a| clip8(a)));
    }
    Rgb8::new(out_width, out_height, out)
}

pub fn resize_pil_bilinear(src: RgbView<'_>, width: u32, height: u32) -> Result<Rgb8, ImageError> {
    resize(&mut ViewRows(src), width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::Rng;

    fn pil_reference(src: &Rgb8, out_w: u32, out_h: u32) -> Rgb8 {
        let (w, h) = src.dimensions();
        let xk = kernel(w, out_w);
        let yk = kernel(h, out_h);
        let first = yk.bounds[0].0;
        let last = yk.bounds[out_h as usize - 1].0 + yk.bounds[out_h as usize - 1].1;
        let mut temp = vec![0u8; out_w as usize * 3 * (last - first)];
        for y in first..last {
            horizontal(
                src.row(y as u32),
                &xk,
                &mut temp[(y - first) * out_w as usize * 3..][..out_w as usize * 3],
            );
        }
        let mut out = vec![0u8; out_w as usize * out_h as usize * 3];
        for yy in 0..out_h as usize {
            let (start, taps) = yk.taps(yy);
            for x in 0..out_w as usize * 3 {
                let mut acc = HALF;
                for (j, &w) in taps.iter().enumerate() {
                    acc += temp[(start + j - first) * out_w as usize * 3 + x] as i32 * w;
                }
                out[yy * out_w as usize * 3 + x] = clip8(acc);
            }
        }
        Rgb8::new(out_w, out_h, out).unwrap()
    }

    fn gray(values: &[u8]) -> Rgb8 {
        Rgb8::new(
            values.len() as u32,
            1,
            values.iter().flat_map(|&v| [v, v, v]).collect(),
        )
        .unwrap()
    }

    fn channel0(img: &Rgb8) -> Vec<u8> {
        img.data().chunks_exact(3).map(|p| p[0]).collect()
    }

    #[test]
    fn kernel_identity_is_unit_weight() {
        let k = kernel(5, 5);
        assert_eq!(k.size, 3);
        for i in 0..5 {
            let (start, taps) = k.taps(i);
            assert_eq!(start, i);
            assert_eq!(taps[0], 1 << PRECISION_BITS);
            assert!(taps[1..].iter().all(|&w| w == 0));
        }
    }

    #[test]
    fn kernel_downscale_by_two_matches_hand_derived_pil_weights() {
        let k = kernel(4, 2);
        assert_eq!(k.size, 5);
        assert_eq!(k.taps(0), (0, &[1797559, 1797559, 599186][..]));
        assert_eq!(k.taps(1), (1, &[599186, 1797559, 1797559][..]));
    }

    #[test]
    fn downscale_by_two_matches_pil_antialias() {
        let out = resize_pil_bilinear(gray(&[0, 10, 20, 30]).view(), 2, 1).unwrap();
        assert_eq!(channel0(&out), vec![7, 23]);
    }

    #[test]
    fn upscale_matches_hand_derived_values() {
        let out = resize_pil_bilinear(gray(&[0, 200]).view(), 4, 1).unwrap();
        assert_eq!(channel0(&out), vec![0, 50, 150, 200]);
    }

    #[test]
    fn identity_is_exact() {
        let mut rng = Rng::new(30);
        let src = rng.image(17, 9);
        assert_eq!(resize_pil_bilinear(src.view(), 17, 9).unwrap(), src);
    }

    #[test]
    fn kernel_weights_sum_close_to_one_and_stay_in_bounds() {
        let mut rng = Rng::new(31);
        for _ in 0..300 {
            let input = 1 + rng.below(5000) as u32;
            let output = 1 + rng.below(700) as u32;
            let k = kernel(input, output);
            for i in 0..output as usize {
                let (start, taps) = k.taps(i);
                assert!(!taps.is_empty());
                assert!(taps.len() <= k.size);
                assert!(start + taps.len() <= input as usize);
                let sum: i64 = taps.iter().map(|&w| w as i64).sum();
                assert!(
                    (sum - (1 << PRECISION_BITS)).abs() <= taps.len() as i64,
                    "{input}->{output} sum {sum}"
                );
                assert!(taps.iter().all(|&w| w >= 0));
            }
            for pair in k.bounds.windows(2) {
                assert!(pair[1].0 >= pair[0].0);
                assert!(pair[1].0 + pair[1].1 >= pair[0].0 + pair[0].1);
            }
        }
    }

    #[test]
    fn streaming_matches_full_intermediate_reference() {
        let mut rng = Rng::new(32);
        for _ in 0..60 {
            let (w, h) = (1 + rng.below(90) as u32, 1 + rng.below(90) as u32);
            let (ow, oh) = (1 + rng.below(70) as u32, 1 + rng.below(70) as u32);
            let src = rng.image(w, h);
            assert_eq!(
                resize_pil_bilinear(src.view(), ow, oh).unwrap(),
                pil_reference(&src, ow, oh),
                "{w}x{h}->{ow}x{oh}"
            );
        }
    }

    #[test]
    fn large_downscale_to_model_size_matches_reference() {
        let mut rng = Rng::new(33);
        let src = rng.image(1013, 761);
        assert_eq!(
            resize_pil_bilinear(src.view(), 256, 256).unwrap(),
            pil_reference(&src, 256, 256)
        );
    }

    #[test]
    fn constant_images_stay_constant() {
        let mut rng = Rng::new(34);
        for _ in 0..40 {
            let px = [rng.next() as u8, rng.next() as u8, rng.next() as u8];
            let src =
                Rgb8::filled(1 + rng.below(400) as u32, 1 + rng.below(400) as u32, px).unwrap();
            let out = resize_pil_bilinear(
                src.view(),
                1 + rng.below(300) as u32,
                1 + rng.below(300) as u32,
            )
            .unwrap();
            assert!(out.data().chunks_exact(3).all(|p| p == px));
        }
        let white = Rgb8::filled(999, 3, [255; 3]).unwrap();
        assert!(
            resize_pil_bilinear(white.view(), 256, 256)
                .unwrap()
                .data()
                .iter()
                .all(|&v| v == 255)
        );
    }

    #[test]
    fn output_stays_within_input_range() {
        let mut rng = Rng::new(35);
        for _ in 0..40 {
            let lo = rng.below(128) as u8;
            let hi = lo + rng.below(128) as u8;
            let (w, h) = (2 + rng.below(60) as u32, 2 + rng.below(60) as u32);
            let src = Rgb8::from_fn(w, h, |_, _| {
                let v = lo + (rng.next() % (hi - lo + 1) as u64) as u8;
                [v, v, v]
            })
            .unwrap();
            let out = resize_pil_bilinear(
                src.view(),
                1 + rng.below(80) as u32,
                1 + rng.below(80) as u32,
            )
            .unwrap();
            assert!(out.data().iter().all(|&v| v >= lo && v <= hi));
        }
    }

    #[test]
    fn crop_view_matches_owned_copy() {
        let mut rng = Rng::new(36);
        let full = rng.image(120, 80);
        let view = full.crop(13, 7, 91, 55).unwrap();
        assert_eq!(
            resize_pil_bilinear(view, 33, 29).unwrap(),
            resize_pil_bilinear(view.to_image().view(), 33, 29).unwrap()
        );
    }

    #[test]
    fn channels_are_independent() {
        let mut rng = Rng::new(37);
        let src = rng.image(40, 30);
        let out = resize_pil_bilinear(src.view(), 13, 17).unwrap();
        for c in 0..3 {
            let plane: Vec<u8> = src.data().chunks_exact(3).flat_map(|p| [p[c]; 3]).collect();
            let single =
                resize_pil_bilinear(Rgb8::new(40, 30, plane).unwrap().view(), 13, 17).unwrap();
            let expect: Vec<u8> = single.data().chunks_exact(3).map(|p| p[0]).collect();
            let got: Vec<u8> = out.data().chunks_exact(3).map(|p| p[c]).collect();
            assert_eq!(got, expect);
        }
    }

    #[test]
    fn rejects_zero_target() {
        let src = Rgb8::filled(4, 4, [1, 2, 3]).unwrap();
        assert_eq!(
            resize_pil_bilinear(src.view(), 0, 4),
            Err(ImageError::InvalidDimensions)
        );
        assert_eq!(
            resize_pil_bilinear(src.view(), 4, 0),
            Err(ImageError::InvalidDimensions)
        );
    }

    #[test]
    fn single_pixel_sources_and_targets() {
        let src = Rgb8::filled(1, 1, [9, 8, 7]).unwrap();
        assert!(
            resize_pil_bilinear(src.view(), 5, 3)
                .unwrap()
                .data()
                .chunks_exact(3)
                .all(|p| p == [9, 8, 7])
        );
        let mut rng = Rng::new(38);
        let big = rng.image(300, 1);
        assert_eq!(
            resize_pil_bilinear(big.view(), 1, 1).unwrap(),
            pil_reference(&big, 1, 1)
        );
    }

    #[test]
    fn deterministic() {
        let mut rng = Rng::new(39);
        let src = rng.image(333, 222);
        assert_eq!(
            resize_pil_bilinear(src.view(), 256, 256),
            resize_pil_bilinear(src.view(), 256, 256)
        );
    }
}
