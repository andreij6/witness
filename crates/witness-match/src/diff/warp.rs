use crate::geometry::Similarity;
use crate::mask::OverlayMask;
use witness_image::{Rgb8, RgbView};

const EPSILON: f64 = 1e-9;
const HALF_PIXEL: f64 = 0.5;

pub(crate) struct Warped {
    pub image: Rgb8,
    pub valid: Vec<bool>,
    pub overlap: u64,
}

fn within(s: f64, lo: f64, hi: f64) -> bool {
    s >= lo - EPSILON && s <= hi + EPSILON
}

fn lerp(a: u8, b: u8, f: u32) -> u32 {
    a as u32 * (256 - f) + b as u32 * f
}

pub(crate) fn warp(
    query: RgbView<'_>,
    transform: &Similarity,
    size: (u32, u32),
    margin: f64,
    overlay: Option<&OverlayMask>,
) -> Warped {
    let (width, height) = size;
    let (qw, qh) = (query.width(), query.height());
    let (max_x, max_y) = ((qw - 1) as f64, (qh - 1) as f64);
    let (lo_x, hi_x) = (margin.min(max_x / 2.0), max_x - margin.min(max_x / 2.0));
    let (lo_y, hi_y) = (margin.min(max_y / 2.0), max_y - margin.min(max_y / 2.0));
    let n = width as usize * height as usize;
    let mut data = vec![0u8; n * 3];
    let mut valid = vec![false; n];
    let mut overlap = 0u64;
    for y in 0..height {
        let cy = y as f64 + 0.5;
        for x in 0..width {
            let (u, v) = transform.apply(x as f64 + 0.5, cy);
            let (sx, sy) = (u - 0.5, v - 0.5);
            if !(within(sx, -HALF_PIXEL, max_x + HALF_PIXEL)
                && within(sy, -HALF_PIXEL, max_y + HALF_PIXEL))
            {
                continue;
            }
            overlap += 1;
            if !(within(sx, lo_x, hi_x) && within(sy, lo_y, hi_y))
                || overlay.is_some_and(|m| m.covers(u, v))
            {
                continue;
            }
            let (sx, sy) = (sx.clamp(0.0, max_x), sy.clamp(0.0, max_y));
            let (x0, y0) = (sx.floor(), sy.floor());
            let fx = ((sx - x0) * 256.0).round() as u32;
            let fy = ((sy - y0) * 256.0).round() as u32;
            let (x0, y0) = (x0 as u32, y0 as u32);
            let (x1, y1) = ((x0 + 1).min(qw - 1), (y0 + 1).min(qh - 1));
            let (top, bottom) = (query.row(y0), query.row(y1));
            let i = y as usize * width as usize + x as usize;
            for c in 0..3 {
                let t = lerp(top[x0 as usize * 3 + c], top[x1 as usize * 3 + c], fx);
                let b = lerp(bottom[x0 as usize * 3 + c], bottom[x1 as usize * 3 + c], fx);
                data[i * 3 + c] = ((t * (256 - fy) + b * fy + 32768) >> 16) as u8;
            }
            valid[i] = true;
        }
    }
    Warped {
        image: Rgb8::new(width, height, data).expect("dimensions match buffer"),
        valid,
        overlap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(w: u32, h: u32) -> Rgb8 {
        Rgb8::from_fn(w, h, |x, y| {
            [(x * 7) as u8, (y * 5) as u8, ((x + y) * 3) as u8]
        })
        .unwrap()
    }

    #[test]
    fn identity_copies_exactly() {
        let img = ramp(20, 15);
        let w = warp(img.view(), &Similarity::IDENTITY, (20, 15), 0.0, None);
        assert_eq!(w.image, img);
        assert!(w.valid.iter().all(|&v| v));
    }

    #[test]
    fn integer_translation_shifts() {
        let img = ramp(20, 15);
        let t = Similarity::scale_translate(1.0, 3.0, 2.0);
        let w = warp(img.view(), &t, (20, 15), 0.0, None);
        assert_eq!(w.image.pixel(0, 0), img.pixel(3, 2));
        assert_eq!(w.image.pixel(16, 12), img.pixel(19, 14));
        assert!(!w.valid[12 * 20 + 17]);
        assert!(!w.valid[13 * 20 + 5]);
    }

    #[test]
    fn half_pixel_interpolates() {
        let img = Rgb8::from_fn(
            2,
            1,
            |x, _| if x == 0 { [0, 100, 200] } else { [100, 200, 0] },
        )
        .unwrap();
        let t = Similarity::scale_translate(1.0, 0.5, 0.0);
        let w = warp(img.view(), &t, (1, 1), 0.0, None);
        assert_eq!(w.image.pixel(0, 0), Some([50, 150, 100]));
    }

    #[test]
    fn upscale_by_two_matches_bilinear_centers() {
        let img = Rgb8::from_fn(4, 4, |x, y| [(x * 60) as u8, (y * 60) as u8, 0]).unwrap();
        let t = Similarity::scale_translate(0.5, 0.0, 0.0);
        let w = warp(img.view(), &t, (8, 8), 0.0, None);
        assert!(!w.valid[0]);
        assert!(w.valid[9]);
        assert_eq!(w.image.pixel(1, 1), Some([15, 15, 0]));
        assert_eq!(w.image.pixel(2, 1), Some([45, 15, 0]));
    }

    #[test]
    fn mirrored_transform_flips() {
        let img = ramp(10, 4);
        let t = Similarity::IDENTITY.mirror_x(10.0);
        let w = warp(img.view(), &t, (10, 4), 0.0, None);
        for y in 0..4 {
            for x in 0..10 {
                assert_eq!(w.image.pixel(x, y), img.pixel(9 - x, y));
            }
        }
    }

    #[test]
    fn overlay_blocks_are_invalid() {
        let img = ramp(16, 16);
        let mask = OverlayMask::new(
            (0.0, 0.0),
            (8.0, 8.0),
            2,
            2,
            vec![true, false, false, false],
        )
        .unwrap();
        let w = warp(
            img.view(),
            &Similarity::IDENTITY,
            (16, 16),
            0.0,
            Some(&mask),
        );
        assert!(!w.valid[0] && !w.valid[7 * 16 + 7]);
        assert!(w.valid[8] && w.valid[8 * 16]);
    }

    #[test]
    fn edge_margin_drops_query_border() {
        let img = ramp(20, 20);
        let w = warp(img.view(), &Similarity::IDENTITY, (20, 20), 3.0, None);
        assert!(!w.valid[2 * 20 + 10] && !w.valid[10 * 20 + 17]);
        assert!(w.valid[3 * 20 + 3] && w.valid[16 * 20 + 16]);
        let tiny = ramp(3, 3);
        assert!(warp(tiny.view(), &Similarity::IDENTITY, (3, 3), 50.0, None).valid[4]);
    }

    #[test]
    fn overlap_counts_margin_and_overlay_pixels() {
        let img = ramp(16, 16);
        let mask =
            OverlayMask::new((0.0, 0.0), (8.0, 8.0), 2, 2, vec![true, false, false, true]).unwrap();
        let w = warp(
            img.view(),
            &Similarity::IDENTITY,
            (16, 16),
            3.0,
            Some(&mask),
        );
        assert_eq!(w.overlap, 256);
        assert!(w.valid.iter().filter(|&&v| v).count() < 64);
    }

    #[test]
    fn overlap_is_the_shared_area_under_translation() {
        let img = ramp(20, 15);
        let t = Similarity::scale_translate(1.0, 5.0, 2.0);
        assert_eq!(warp(img.view(), &t, (20, 15), 0.0, None).overlap, 15 * 13);
        assert_eq!(warp(img.view(), &t, (20, 15), 4.0, None).overlap, 15 * 13);
        let t = Similarity::scale_translate(1.0, -20.0, 0.0);
        assert_eq!(warp(img.view(), &t, (20, 15), 0.0, None).overlap, 0);
    }

    #[test]
    fn overlap_is_full_for_plain_resizes() {
        for (from, to) in [
            ((64, 48), (32, 24)),
            ((64, 48), (128, 96)),
            ((640, 480), (213, 160)),
        ] {
            let img = ramp(to.0, to.1);
            let t = Similarity::resize(from, to).unwrap();
            let w = warp(img.view(), &t, from, 2.0, None);
            assert_eq!(w.overlap, u64::from(from.0 * from.1), "{from:?} -> {to:?}");
        }
    }

    #[test]
    fn overlap_of_a_scaled_crop_is_its_footprint() {
        let img = ramp(30, 40);
        let t = Similarity::scale_translate(0.5, -10.0, -5.0);
        let w = warp(img.view(), &t, (100, 100), 1.0, None);
        assert_eq!(w.overlap, 60 * 80);
    }

    #[test]
    fn non_finite_transform_is_all_invalid() {
        let img = ramp(5, 5);
        let t = Similarity::new(f64::NAN, 0.0, 0.0, 0.0, false);
        let w = warp(img.view(), &t, (5, 5), 0.0, None);
        assert!(w.valid.iter().all(|&v| !v));
        assert_eq!(w.overlap, 0);
    }
}
