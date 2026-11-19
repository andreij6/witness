use crate::geometry::{CanvasRows, Placement};
use crate::resize::resize;
use witness_image::{ImageError, Rgb8, RgbView};

pub const MODEL_SIZE: usize = 256;
pub const PLANE_LEN: usize = MODEL_SIZE * MODEL_SIZE;
pub const TENSOR_LEN: usize = 3 * PLANE_LEN;

const NORMALIZED: [f32; 256] = {
    let mut table = [0.0f32; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = i as f32 / 255.0 * 2.0 - 1.0;
        i += 1;
    }
    table
};

pub fn normalize(value: u8) -> f32 {
    NORMALIZED[value as usize]
}

pub fn model_input(
    image: RgbView<'_>,
    placement: Placement,
    gray: u8,
    out: &mut [f32],
) -> Result<(), ImageError> {
    if out.len() != TENSOR_LEN {
        return Err(ImageError::BufferSizeMismatch);
    }
    let mut rows = CanvasRows::new(image, placement, gray)?;
    let resized = resize(&mut rows, MODEL_SIZE as u32, MODEL_SIZE as u32)?;
    write_nchw(&resized, out);
    Ok(())
}

fn write_nchw(image: &Rgb8, out: &mut [f32]) {
    let (r, rest) = out.split_at_mut(PLANE_LEN);
    let (g, b) = rest.split_at_mut(PLANE_LEN);
    for (i, px) in image.data().chunks_exact(3).enumerate() {
        r[i] = normalize(px[0]);
        g[i] = normalize(px[1]);
        b[i] = normalize(px[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::processing_region;
    use crate::resize::resize_pil_bilinear;
    use crate::testutil::Rng;

    fn input(image: &Rgb8, placement: Placement, gray: u8) -> Vec<f32> {
        let mut out = vec![f32::NAN; TENSOR_LEN];
        model_input(image.view(), placement, gray, &mut out).unwrap();
        out
    }

    #[test]
    fn normalization_endpoints_and_monotonic() {
        assert_eq!(normalize(0), -1.0);
        assert_eq!(normalize(255), 1.0);
        assert!((normalize(128) - 0.003_921_628).abs() < 1e-7);
        for v in 1..=255u8 {
            assert!(normalize(v) > normalize(v - 1));
        }
    }

    #[test]
    fn normalization_matches_runtime_formula() {
        for v in 0..=255u8 {
            let (value, divisor) = (
                core::hint::black_box(v as f32),
                core::hint::black_box(255.0f32),
            );
            assert_eq!(
                normalize(v).to_bits(),
                (value / divisor * 2.0 - 1.0).to_bits()
            );
        }
    }

    #[test]
    fn layout_is_nchw() {
        let img = Rgb8::from_fn(256, 256, |x, y| [x as u8, y as u8, 255 - x as u8]).unwrap();
        let t = input(&img, Placement::identity(256, 256), 0);
        for (x, y) in [(0usize, 0usize), (17, 3), (255, 255), (100, 200)] {
            assert_eq!(t[y * 256 + x], normalize(x as u8));
            assert_eq!(t[PLANE_LEN + y * 256 + x], normalize(y as u8));
            assert_eq!(t[2 * PLANE_LEN + y * 256 + x], normalize(255 - x as u8));
        }
    }

    #[test]
    fn identity_input_equals_resized_processing_region() {
        let mut rng = Rng::new(50);
        for (w, h) in [(400, 300), (300, 700), (1000, 200), (256, 256), (31, 17)] {
            let img = rng.image(w, h);
            let r = processing_region(w, h);
            let expected =
                resize_pil_bilinear(img.crop(r.x, r.y, r.width, r.height).unwrap(), 256, 256)
                    .unwrap();
            let mut want = vec![0.0; TENSOR_LEN];
            write_nchw(&expected, &mut want);
            assert_eq!(input(&img, Placement::identity(w, h), 9), want);
        }
    }

    #[test]
    fn padded_canvas_matches_materialized_canvas() {
        let mut rng = Rng::new(51);
        for _ in 0..10 {
            let (w, h) = (20 + rng.below(200) as u32, 20 + rng.below(200) as u32);
            let img = rng.image(w, h);
            let (cw, ch) = (w + rng.below(100) as u32, h + rng.below(100) as u32);
            let p = Placement {
                canvas_width: cw,
                canvas_height: ch,
                x: rng.below((cw - w + 1) as u64) as u32,
                y: rng.below((ch - h + 1) as u64) as u32,
            };
            let gray = rng.next() as u8;
            let canvas = Rgb8::from_fn(cw, ch, |x, y| {
                if x >= p.x && y >= p.y {
                    img.pixel(x - p.x, y - p.y).unwrap_or([gray; 3])
                } else {
                    [gray; 3]
                }
            })
            .unwrap();
            assert_eq!(
                input(&img, p, gray),
                input(&canvas, Placement::identity(cw, ch), 0)
            );
        }
    }

    #[test]
    fn rejects_wrong_buffer_and_bad_placement() {
        let img = Rgb8::filled(10, 10, [0; 3]).unwrap();
        let mut short = vec![0.0; TENSOR_LEN - 1];
        assert_eq!(
            model_input(img.view(), Placement::identity(10, 10), 0, &mut short),
            Err(ImageError::BufferSizeMismatch)
        );
        let mut out = vec![0.0; TENSOR_LEN];
        let bad = Placement {
            canvas_width: 5,
            canvas_height: 10,
            x: 0,
            y: 0,
        };
        assert_eq!(
            model_input(img.view(), bad, 0, &mut out),
            Err(ImageError::OutOfBounds)
        );
    }

    #[test]
    fn values_in_unit_range_and_deterministic() {
        let mut rng = Rng::new(52);
        let img = rng.image(640, 480);
        let a = input(&img, Placement::identity(640, 480), 0);
        assert!(a.iter().all(|v| (-1.0..=1.0).contains(v)));
        assert_eq!(a, input(&img, Placement::identity(640, 480), 0));
    }
}
