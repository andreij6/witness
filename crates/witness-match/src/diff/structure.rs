use crate::classify::{CellState, StateMap};
use witness_image::{Rgb8, RgbView, luma_bt601};

const C2: f64 = (0.03 * 255.0) * (0.03 * 255.0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WindowRule {
    pub radius: u32,
    pub min_coverage_percent: u32,
    pub structure_threshold: f64,
    pub mean_threshold: f64,
    pub color_threshold: f64,
}

#[derive(Clone, Copy, Default)]
struct Moments {
    n: u32,
    luma: [u32; 5],
    means: [[u32; 2]; 3],
}

impl Moments {
    fn add(&mut self, other: &Moments) {
        self.n += other.n;
        for (a, b) in self.luma.iter_mut().zip(&other.luma) {
            *a += b;
        }
        for (a, b) in self
            .means
            .iter_mut()
            .flatten()
            .zip(other.means.iter().flatten())
        {
            *a += b;
        }
    }

    fn sub(&mut self, other: &Moments) {
        self.n -= other.n;
        for (a, b) in self.luma.iter_mut().zip(&other.luma) {
            *a -= b;
        }
        for (a, b) in self
            .means
            .iter_mut()
            .flatten()
            .zip(other.means.iter().flatten())
        {
            *a -= b;
        }
    }

    fn pixel(q: &[u8], r: &[u8]) -> Moments {
        let x = luma_bt601(q[0], q[1], q[2]) as u32;
        let y = luma_bt601(r[0], r[1], r[2]) as u32;
        Moments {
            n: 1,
            luma: [x, y, x * x, y * y, x * y],
            means: core::array::from_fn(|c| [q[c] as u32, r[c] as u32]),
        }
    }
}

fn changed(m: &Moments, rule: &WindowRule) -> bool {
    let n = m.n as i64;
    let [sx, sy, sxx, syy, sxy] = m.luma.map(|v| v as i64);
    let vx = n * sxx - sx * sx;
    let vy = n * syy - sy * sy;
    let cov = n * sxy - sx * sy;
    let c2n2 = C2 * (n * n) as f64;
    let structure = (2.0 * cov as f64 + c2n2) / ((vx + vy) as f64 + c2n2);
    let (luma_limit, color_limit) = (
        rule.mean_threshold * n as f64,
        rule.color_threshold * n as f64,
    );
    structure < rule.structure_threshold
        || (sx - sy).abs() as f64 > luma_limit
        || m.means
            .iter()
            .any(|&[q, r]| (q as i64 - r as i64).abs() as f64 > color_limit)
}

fn row_moments(
    corrected: &Rgb8,
    valid: &[bool],
    reference: RgbView<'_>,
    y: u32,
    out: &mut [Moments],
) {
    let width = corrected.width() as usize;
    let (q, r) = (corrected.row(y), reference.row(y));
    let flags = &valid[y as usize * width..][..width];
    for (x, m) in out.iter_mut().enumerate() {
        *m = if flags[x] {
            Moments::pixel(&q[x * 3..x * 3 + 3], &r[x * 3..x * 3 + 3])
        } else {
            Moments::default()
        };
    }
}

pub(crate) fn change_map(
    corrected: &Rgb8,
    valid: &[bool],
    reference: RgbView<'_>,
    rule: &WindowRule,
) -> StateMap {
    let (width, height) = corrected.dimensions();
    let (w, h, r) = (width as usize, height as usize, rule.radius as usize);
    let mut columns = vec![Moments::default(); w];
    let mut scratch = vec![Moments::default(); w];
    let mut states = Vec::with_capacity(w * h);
    let add_row = |y: usize, columns: &mut [Moments], scratch: &mut [Moments], add: bool| {
        row_moments(corrected, valid, reference, y as u32, scratch);
        for (c, m) in columns.iter_mut().zip(scratch.iter()) {
            if add { c.add(m) } else { c.sub(m) }
        }
    };
    for y in 0..r.min(h) {
        add_row(y, &mut columns, &mut scratch, true);
    }
    for y in 0..h {
        if y + r < h {
            add_row(y + r, &mut columns, &mut scratch, true);
        }
        if y > r {
            add_row(y - r - 1, &mut columns, &mut scratch, false);
        }
        let rows_in = (y + r).min(h - 1) - y.saturating_sub(r) + 1;
        let mut window = Moments::default();
        for c in columns.iter().take(r.min(w)) {
            window.add(c);
        }
        let flags = &valid[y * w..][..w];
        for x in 0..w {
            if x + r < w {
                window.add(&columns[x + r]);
            }
            if x > r {
                window.sub(&columns[x - r - 1]);
            }
            if !flags[x] {
                states.push(CellState::Uncompared);
                continue;
            }
            let cols_in = (x + r).min(w - 1) - x.saturating_sub(r) + 1;
            let area = (rows_in * cols_in) as u64;
            if (window.n as u64) * 100 < area * rule.min_coverage_percent as u64 {
                states.push(CellState::Uncompared);
                continue;
            }
            states.push(if changed(&window, rule) {
                CellState::Changed
            } else {
                CellState::Agree
            });
        }
    }
    StateMap::new(width, height, states).expect("one state per pixel")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::ChangeMap;

    const RULE: WindowRule = WindowRule {
        radius: 4,
        min_coverage_percent: 60,
        structure_threshold: 0.5,
        mean_threshold: 20.0,
        color_threshold: 40.0,
    };

    fn textured(w: u32, h: u32, seed: u64) -> Rgb8 {
        let mut s = seed;
        Rgb8::from_fn(w, h, |x, y| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let v = (((x / 3 + y / 4) % 2) * 120) as u8 + (s % 16) as u8 + 40;
            [v, v / 2 + 30, 200 - v / 2]
        })
        .unwrap()
    }

    fn naive_changed(q: &Rgb8, valid: &[bool], r: &Rgb8, x: u32, y: u32) -> Option<bool> {
        let (w, h) = q.dimensions();
        if !valid[(y * w + x) as usize] {
            return None;
        }
        let mut m = Moments::default();
        let mut area = 0u64;
        for yy in y.saturating_sub(4)..=(y + 4).min(h - 1) {
            for xx in x.saturating_sub(4)..=(x + 4).min(w - 1) {
                area += 1;
                if valid[(yy * w + xx) as usize] {
                    m.add(&Moments::pixel(
                        &q.pixel(xx, yy).unwrap(),
                        &r.pixel(xx, yy).unwrap(),
                    ));
                }
            }
        }
        if (m.n as u64) * 100 < area * 60 {
            return None;
        }
        Some(changed(&m, &RULE))
    }

    #[test]
    fn identical_images_agree_everywhere() {
        let img = textured(40, 30, 1);
        let map = change_map(&img, &vec![true; 1200], img.view(), &RULE);
        assert_eq!(map.count(CellState::Agree), 1200);
    }

    #[test]
    fn replaced_patch_is_changed() {
        let reference = textured(60, 50, 2);
        let mut query = reference.clone();
        let other = textured(60, 50, 99);
        for y in 20..32 {
            for x in 25..37 {
                let p = other.pixel(59 - x, 49 - y).unwrap();
                query.row_mut(y)[x as usize * 3..x as usize * 3 + 3].copy_from_slice(&p);
            }
        }
        let map = change_map(&query, &vec![true; 3000], reference.view(), &RULE);
        assert_eq!(map.state(30, 25), CellState::Changed);
        assert_eq!(map.state(5, 5), CellState::Agree);
        assert_eq!(map.state(55, 45), CellState::Agree);
    }

    #[test]
    fn flat_fill_is_changed_by_mean_or_structure() {
        let reference = textured(50, 50, 3);
        let mut query = reference.clone();
        for y in 15..35 {
            for x in 15..35 {
                query.row_mut(y)[x as usize * 3..x as usize * 3 + 3].copy_from_slice(&[0, 0, 0]);
            }
        }
        let map = change_map(&query, &vec![true; 2500], reference.view(), &RULE);
        assert_eq!(map.state(25, 25), CellState::Changed);
    }

    #[test]
    fn invalid_pixels_and_sparse_windows_are_uncompared() {
        let img = textured(30, 30, 4);
        let mut valid = vec![true; 900];
        for y in 0..30 {
            for x in 0..15 {
                valid[y * 30 + x] = false;
            }
        }
        let map = change_map(&img, &valid, img.view(), &RULE);
        assert_eq!(map.state(3, 3), CellState::Uncompared);
        assert_eq!(map.state(15, 10), CellState::Uncompared);
        assert_eq!(map.state(25, 10), CellState::Agree);
    }

    #[test]
    fn matches_naive_window_computation() {
        let reference = textured(37, 29, 5);
        let mut query = textured(37, 29, 6);
        for (i, v) in query.data_mut().iter_mut().enumerate() {
            if (i / 3) % 37 < 18 {
                *v = reference.data()[i];
            }
        }
        let mut s = 0x99u64;
        let valid: Vec<bool> = (0..37 * 29)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                !s.is_multiple_of(10)
            })
            .collect();
        let map = change_map(&query, &valid, reference.view(), &RULE);
        for y in 0..29 {
            for x in 0..37 {
                let expected = match naive_changed(&query, &valid, &reference, x, y) {
                    None => CellState::Uncompared,
                    Some(true) => CellState::Changed,
                    Some(false) => CellState::Agree,
                };
                assert_eq!(map.state(x, y), expected, "{x},{y}");
            }
        }
    }

    #[test]
    fn tiny_images_work() {
        let img = textured(3, 2, 7);
        let map = change_map(&img, &[true; 6], img.view(), &RULE);
        assert_eq!(map.count(CellState::Agree), 6);
        let single = textured(1, 1, 8);
        assert_eq!(
            change_map(&single, &[true], single.view(), &RULE).count(CellState::Agree),
            1
        );
    }

    #[test]
    fn structure_constant_bounds() {
        assert!((C2 - 58.5225).abs() < 1e-9);
    }
}
