use crate::buffer::{Image, Luma8, Rgb8, RgbView};

const WEIGHT_R: u32 = 19_595;
const WEIGHT_G: u32 = 38_470;
const WEIGHT_B: u32 = 7_471;
const HALF: u32 = 1 << 15;

pub fn luma_bt601(r: u8, g: u8, b: u8) -> u8 {
    ((WEIGHT_R * r as u32 + WEIGHT_G * g as u32 + WEIGHT_B * b as u32 + HALF) >> 16) as u8
}

pub fn to_luma(src: RgbView<'_>) -> Luma8 {
    let mut out = Vec::with_capacity(src.width() as usize * src.height() as usize);
    for row in src.rows() {
        out.extend(row.chunks_exact(3).map(|p| luma_bt601(p[0], p[1], p[2])));
    }
    Image::from_parts_unchecked(src.width(), src.height(), out)
}

pub fn luma_to_rgb(src: &Luma8) -> Rgb8 {
    let data = src.data().iter().flat_map(|&v| [v, v, v]).collect();
    Image::from_parts_unchecked(src.width(), src.height(), data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_sum_to_unity() {
        assert_eq!(WEIGHT_R + WEIGHT_G + WEIGHT_B, 1 << 16);
    }

    #[test]
    fn grey_is_preserved_exactly() {
        for v in 0..=255u8 {
            assert_eq!(luma_bt601(v, v, v), v);
        }
    }

    #[test]
    fn primaries_match_bt601() {
        assert_eq!(luma_bt601(255, 0, 0), 76);
        assert_eq!(luma_bt601(0, 255, 0), 150);
        assert_eq!(luma_bt601(0, 0, 255), 29);
        assert_eq!(luma_bt601(255, 255, 255), 255);
        assert_eq!(luma_bt601(0, 0, 0), 0);
    }

    #[test]
    fn matches_float_reference_within_half_unit() {
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..20_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let [r, g, b, ..] = state.to_le_bytes();
            let exact = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
            assert!(
                (luma_bt601(r, g, b) as f64 - exact).abs() <= 0.51,
                "{r} {g} {b}"
            );
        }
    }

    #[test]
    fn monotone_in_each_channel() {
        for v in 0..255u8 {
            assert!(luma_bt601(v + 1, 40, 90) >= luma_bt601(v, 40, 90));
            assert!(luma_bt601(40, v + 1, 90) >= luma_bt601(40, v, 90));
            assert!(luma_bt601(40, 90, v + 1) >= luma_bt601(40, 90, v));
        }
    }

    #[test]
    fn to_luma_converts_every_pixel() {
        let rgb = Rgb8::new(2, 2, vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 10, 10, 10]).unwrap();
        let luma = to_luma(rgb.view());
        assert_eq!(luma.dimensions(), (2, 2));
        assert_eq!(luma.data(), &[76, 150, 29, 10]);
    }

    #[test]
    fn to_luma_respects_crop_stride() {
        let rgb = Rgb8::from_fn(10, 6, |x, y| [(x * 20) as u8, (y * 30) as u8, 5]).unwrap();
        let view = rgb.crop(2, 1, 5, 3).unwrap();
        assert_eq!(to_luma(view), to_luma(view.to_image().view()));
        assert_eq!(to_luma(view).pixel(0, 0), Some([luma_bt601(40, 30, 5)]));
    }

    #[test]
    fn luma_to_rgb_round_trips_through_luma() {
        let luma = Luma8::from_fn(7, 3, |x, y| [(x * 37 + y * 11) as u8]).unwrap();
        let rgb = luma_to_rgb(&luma);
        assert_eq!(rgb.dimensions(), (7, 3));
        assert_eq!(to_luma(rgb.view()), luma);
    }
}
