use crate::geometry::Rect;

pub(crate) const FLAT: u8 = 0;
pub(crate) const NATURAL: u8 = 1;
pub(crate) const STRONG: u8 = 2;
pub(crate) const OTHER: u8 = 3;

pub(crate) const NATURAL_MAX_STEP: u8 = 48;
pub(crate) const STRONG_MIN_STEP: u8 = 64;

pub(crate) fn pixel_classes(luma: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(width * height);
    for y in 0..height {
        let row = &luma[y * width..][..width];
        let below = (y + 1 < height).then(|| &luma[(y + 1) * width..][..width]);
        for x in 0..width {
            let right = if x + 1 < width {
                row[x].abs_diff(row[x + 1])
            } else {
                0
            };
            let down = below.map_or(0, |b| row[x].abs_diff(b[x]));
            let natural = |s: u8| (1..=NATURAL_MAX_STEP).contains(&s);
            out.push(match right.max(down) {
                _ if natural(right) || natural(down) => NATURAL,
                0 => FLAT,
                STRONG_MIN_STEP.. => STRONG,
                _ => OTHER,
            });
        }
    }
    out
}

pub(crate) struct Integral {
    width: usize,
    sums: Vec<u32>,
}

impl Integral {
    pub(crate) fn new(classes: &[u8], width: usize, height: usize, class: u8) -> Self {
        let stride = width + 1;
        let mut sums = vec![0u32; stride * (height + 1)];
        for y in 0..height {
            let mut running = 0u32;
            for x in 0..width {
                running += (classes[y * width + x] == class) as u32;
                sums[(y + 1) * stride + x + 1] = sums[y * stride + x + 1] + running;
            }
        }
        Self { width, sums }
    }

    pub(crate) fn sum(&self, rect: &Rect) -> u32 {
        let stride = self.width + 1;
        let (x0, y0, x1, y1) = (
            rect.x as usize,
            rect.y as usize,
            rect.right() as usize,
            rect.bottom() as usize,
        );
        self.sums[y1 * stride + x1] + self.sums[y0 * stride + x0]
            - self.sums[y0 * stride + x1]
            - self.sums[y1 * stride + x0]
    }
}

pub(crate) fn close_gaps(flags: &mut [bool], max_gap: usize) {
    let mut last_true: Option<usize> = None;
    for i in 0..flags.len() {
        if !flags[i] {
            continue;
        }
        if let Some(prev) = last_true {
            let gap = i - prev - 1;
            if gap > 0 && gap <= max_gap {
                flags[prev + 1..i].fill(true);
            }
        }
        last_true = Some(i);
    }
}

pub(crate) fn runs(flags: &[bool], min_len: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, &f) in flags.iter().chain(core::iter::once(&false)).enumerate() {
        match (f, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if i - s >= min_len.max(1) {
                    out.push((s, i));
                }
                start = None;
            }
            _ => {}
        }
    }
    out
}

pub(crate) fn longest(runs: &[(usize, usize)]) -> Option<(usize, usize)> {
    runs.iter()
        .copied()
        .fold(None, |best: Option<(usize, usize)>, r| match best {
            Some(b) if b.1 - b.0 >= r.1 - r.0 => Some(b),
            _ => Some(r),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_follow_steps() {
        let luma = [10u8, 10, 12, 200, 200, 150];
        let classes = pixel_classes(&luma, 6, 1);
        assert_eq!(classes, vec![FLAT, NATURAL, STRONG, FLAT, OTHER, FLAT]);
    }

    #[test]
    fn classes_use_vertical_neighbor() {
        let luma = [10u8, 10, 10, 90];
        assert_eq!(pixel_classes(&luma, 2, 2), vec![FLAT, STRONG, STRONG, FLAT]);
    }

    #[test]
    fn integral_matches_naive_counts() {
        let (w, h) = (13, 9);
        let classes: Vec<u8> = (0..w * h).map(|i| ((i * 7 + i / 5) % 4) as u8).collect();
        let integral = Integral::new(&classes, w, h, NATURAL);
        for rect in [
            Rect::new(0, 0, 13, 9),
            Rect::new(2, 3, 5, 4),
            Rect::new(12, 8, 1, 1),
            Rect::new(4, 4, 0, 3),
        ] {
            let mut naive = 0;
            for y in rect.y..rect.bottom() {
                for x in rect.x..rect.right() {
                    naive += (classes[y as usize * w + x as usize] == NATURAL) as u32;
                }
            }
            assert_eq!(integral.sum(&rect), naive, "{rect:?}");
        }
    }

    #[test]
    fn gaps_close_only_when_short() {
        let mut flags = [true, false, true, false, false, false, true, false];
        close_gaps(&mut flags, 2);
        assert_eq!(flags, [true, true, true, false, false, false, true, false]);
        let mut all = [false; 4];
        close_gaps(&mut all, 10);
        assert_eq!(all, [false; 4]);
    }

    #[test]
    fn runs_respect_min_length() {
        let flags = [true, true, false, true, false, true, true, true];
        assert_eq!(runs(&flags, 1), vec![(0, 2), (3, 4), (5, 8)]);
        assert_eq!(runs(&flags, 2), vec![(0, 2), (5, 8)]);
        assert_eq!(runs(&flags, 0), vec![(0, 2), (3, 4), (5, 8)]);
        assert!(runs(&[], 1).is_empty());
    }

    #[test]
    fn longest_prefers_first_on_ties() {
        assert_eq!(longest(&[(0, 2), (5, 7), (9, 10)]), Some((0, 2)));
        assert_eq!(longest(&[(0, 2), (5, 9)]), Some((5, 9)));
        assert_eq!(longest(&[]), None);
    }
}
