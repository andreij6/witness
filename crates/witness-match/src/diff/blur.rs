use crate::geometry::Rect;
use witness_image::{RgbView, luma_bt601};

fn luma(view: &RgbView<'_>, x: u32, y: u32) -> i32 {
    let p = &view.row(y)[x as usize * 3..x as usize * 3 + 3];
    luma_bt601(p[0], p[1], p[2]) as i32
}

pub(crate) fn energy_masked(view: RgbView<'_>, valid: Option<&[bool]>, rect: Rect) -> Option<f64> {
    let rect = rect.clamp_to(view.width(), view.height())?;
    if rect.width < 3 || rect.height < 3 {
        return None;
    }
    let width = view.width() as usize;
    let ok = |x: u32, y: u32| valid.is_none_or(|v| v[y as usize * width + x as usize]);
    let (mut total, mut count) = (0u64, 0u64);
    for y in rect.y + 1..rect.bottom() - 1 {
        for x in rect.x + 1..rect.right() - 1 {
            if !(ok(x, y) && ok(x - 1, y) && ok(x + 1, y) && ok(x, y - 1) && ok(x, y + 1)) {
                continue;
            }
            let lap = 4 * luma(&view, x, y)
                - luma(&view, x - 1, y)
                - luma(&view, x + 1, y)
                - luma(&view, x, y - 1)
                - luma(&view, x, y + 1);
            total += lap.unsigned_abs() as u64;
            count += 1;
        }
    }
    (count > 0).then(|| total as f64 / count as f64)
}

pub fn high_frequency_energy(view: RgbView<'_>, rect: Rect) -> Option<f64> {
    energy_masked(view, None, rect)
}

pub fn blur_holds(energy: f64, reference_energy: Option<f64>, floor: f64, ratio: f64) -> bool {
    energy <= floor.max(reference_energy.map_or(0.0, |r| r * ratio))
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_image::Rgb8;

    fn checker(w: u32, h: u32) -> Rgb8 {
        Rgb8::from_fn(w, h, |x, y| {
            if (x / 2 + y / 2) % 2 == 0 {
                [220, 200, 210]
            } else {
                [30, 40, 20]
            }
        })
        .unwrap()
    }

    fn box_blur(img: &Rgb8, rect: Rect, radius: u32) -> Rgb8 {
        let mut out = img.clone();
        for y in rect.y..rect.bottom() {
            for x in rect.x..rect.right() {
                let mut acc = [0u32; 3];
                let mut n = 0;
                for yy in y.saturating_sub(radius)..=(y + radius).min(img.height() - 1) {
                    for xx in x.saturating_sub(radius)..=(x + radius).min(img.width() - 1) {
                        let p = img.pixel(xx, yy).unwrap();
                        for c in 0..3 {
                            acc[c] += p[c] as u32;
                        }
                        n += 1;
                    }
                }
                let p = acc.map(|a| ((a + n / 2) / n) as u8);
                out.row_mut(y)[x as usize * 3..x as usize * 3 + 3].copy_from_slice(&p);
            }
        }
        out
    }

    #[test]
    fn flat_region_has_zero_energy() {
        let img = Rgb8::filled(20, 20, [90, 90, 90]).unwrap();
        assert_eq!(
            high_frequency_energy(img.view(), Rect::new(0, 0, 20, 20)),
            Some(0.0)
        );
    }

    #[test]
    fn texture_has_high_energy_and_blur_lowers_it() {
        let img = checker(60, 60);
        let rect = Rect::new(10, 10, 40, 40);
        let sharp = high_frequency_energy(img.view(), rect).unwrap();
        let blurred = box_blur(&img, Rect::new(0, 0, 60, 60), 4);
        let soft = high_frequency_energy(blurred.view(), rect).unwrap();
        assert!(sharp > 100.0, "{sharp}");
        assert!(soft < 5.0, "{soft}");
        assert!(blur_holds(soft, Some(sharp), 2.0, 0.35));
        assert!(!blur_holds(sharp, Some(sharp), 2.0, 0.35));
    }

    #[test]
    fn blur_holds_uses_floor_without_reference() {
        assert!(blur_holds(1.0, None, 2.0, 0.35));
        assert!(!blur_holds(3.0, None, 2.0, 0.35));
        assert!(blur_holds(3.0, Some(10.0), 2.0, 0.35));
        assert!(!blur_holds(f64::NAN, Some(10.0), 2.0, 0.35));
    }

    #[test]
    fn degenerate_rects_have_no_energy() {
        let img = checker(10, 10);
        assert_eq!(
            high_frequency_energy(img.view(), Rect::new(0, 0, 2, 10)),
            None
        );
        assert_eq!(
            high_frequency_energy(img.view(), Rect::new(20, 20, 5, 5)),
            None
        );
        assert!(high_frequency_energy(img.view(), Rect::new(8, 8, 10, 10)).is_none());
        assert!(high_frequency_energy(img.view(), Rect::new(7, 7, 10, 10)).is_some());
    }

    #[test]
    fn mask_excludes_pixels() {
        let img = checker(10, 10);
        assert_eq!(
            energy_masked(img.view(), Some(&[false; 100]), Rect::new(0, 0, 10, 10)),
            None
        );
        let mut valid = [true; 100];
        valid[55] = false;
        assert!(energy_masked(img.view(), Some(&valid), Rect::new(0, 0, 10, 10)).is_some());
    }
}
