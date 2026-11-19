const CIRCLE: [(i8, i8); 16] = [
    (0, -3),
    (1, -3),
    (2, -2),
    (3, -1),
    (3, 0),
    (3, 1),
    (2, 2),
    (1, 3),
    (0, 3),
    (-1, 3),
    (-2, 2),
    (-3, 1),
    (-3, 0),
    (-3, -1),
    (-2, -2),
    (-1, -3),
];

pub(crate) const FAST_RADIUS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Corner {
    pub x: u32,
    pub y: u32,
    pub score: u16,
}

fn has_arc_of_nine(mask: u32) -> bool {
    let mut run = mask | (mask << 16);
    for _ in 0..8 {
        run &= run >> 1;
    }
    run != 0
}

fn arc_score(values: &[i16; 16], mask: u32, sign: i16, reference: i16) -> u16 {
    if !has_arc_of_nine(mask) {
        return 0;
    }
    (0..16)
        .filter(|i| mask >> i & 1 == 1)
        .map(|i| (sign * (values[i] - reference)) as u16)
        .sum()
}

fn corner_score(img: &[u8], center: usize, offsets: &[isize; 16], threshold: i16) -> u16 {
    let at = |i: usize| img[center.wrapping_add_signed(offsets[i])] as i16;
    let p = img[center] as i16;
    let (hi, lo) = (p + threshold, p - threshold);
    let compass = [at(0), at(4), at(8), at(12)];
    let bright = compass.iter().filter(|&&v| v > hi).count();
    let dark = compass.iter().filter(|&&v| v < lo).count();
    if bright < 2 && dark < 2 {
        return 0;
    }
    let values: [i16; 16] = core::array::from_fn(at);
    let (mut bright_mask, mut dark_mask) = (0u32, 0u32);
    for (i, &v) in values.iter().enumerate() {
        bright_mask |= ((v > hi) as u32) << i;
        dark_mask |= ((v < lo) as u32) << i;
    }
    arc_score(&values, bright_mask, 1, hi).max(arc_score(&values, dark_mask, -1, lo))
}

fn score_row(
    img: &[u8],
    width: usize,
    y: usize,
    border: usize,
    offsets: &[isize; 16],
    threshold: i16,
    out: &mut [u16],
) {
    out.fill(0);
    for (x, slot) in out.iter_mut().enumerate().take(width - border).skip(border) {
        *slot = corner_score(img, y * width + x, offsets, threshold);
    }
}

pub(crate) fn detect(
    img: &[u8],
    width: usize,
    height: usize,
    threshold: u8,
    border: usize,
) -> Vec<Corner> {
    let border = border.max(FAST_RADIUS);
    if width <= 2 * border + 2 || height <= 2 * border + 2 || img.len() < width * height {
        return Vec::new();
    }
    let offsets: [isize; 16] = CIRCLE.map(|(dx, dy)| dy as isize * width as isize + dx as isize);
    let threshold = threshold.max(1) as i16;
    let mut rows = [vec![0u16; width], vec![0u16; width], vec![0u16; width]];
    score_row(
        img,
        width,
        border,
        border,
        &offsets,
        threshold,
        &mut rows[border % 3],
    );
    let mut corners = Vec::new();
    for y in border..height - border {
        let next = (y + 1) % 3;
        if y + 1 < height - border {
            score_row(
                img,
                width,
                y + 1,
                border,
                &offsets,
                threshold,
                &mut rows[next],
            );
        } else {
            rows[next].fill(0);
        }
        let (prev, cur, below) = (&rows[(y + 2) % 3], &rows[y % 3], &rows[next]);
        for x in border..width - border {
            let s = cur[x];
            if s == 0 {
                continue;
            }
            let earlier = s > prev[x - 1] && s > prev[x] && s > prev[x + 1] && s > cur[x - 1];
            let later = s >= cur[x + 1] && s >= below[x - 1] && s >= below[x] && s >= below[x + 1];
            if earlier && later {
                corners.push(Corner {
                    x: x as u32,
                    y: y as u32,
                    score: s,
                });
            }
        }
    }
    corners
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_image(size: usize, x0: usize, y0: usize, side: usize, fg: u8, bg: u8) -> Vec<u8> {
        let mut img = vec![bg; size * size];
        for y in y0..y0 + side {
            for x in x0..x0 + side {
                img[y * size + x] = fg;
            }
        }
        img
    }

    #[test]
    fn arc_detection_handles_wraparound() {
        assert!(has_arc_of_nine(0x01FF));
        assert!(!has_arc_of_nine(0x00FF));
        assert!(has_arc_of_nine(0xF01F));
        assert!(!has_arc_of_nine(0xF00F));
        assert!(has_arc_of_nine(0xFFFF));
        assert!(!has_arc_of_nine(0x5555));
        assert!(!has_arc_of_nine(0));
    }

    #[test]
    fn flat_image_has_no_corners() {
        let img = vec![128u8; 64 * 64];
        assert!(detect(&img, 64, 64, 20, 3).is_empty());
    }

    #[test]
    fn square_corners_are_found() {
        let img = square_image(64, 20, 20, 24, 220, 30);
        let corners = detect(&img, 64, 64, 30, 3);
        assert_eq!(corners.len(), 4, "{corners:?}");
        let expected = [(20, 20), (43, 20), (20, 43), (43, 43)];
        for (ex, ey) in expected {
            assert!(
                corners
                    .iter()
                    .any(|c| (c.x as i32 - ex).abs() <= 1 && (c.y as i32 - ey).abs() <= 1),
                "{ex},{ey} {corners:?}"
            );
        }
    }

    #[test]
    fn dark_square_corners_are_found() {
        let img = square_image(64, 16, 16, 30, 10, 200);
        assert_eq!(detect(&img, 64, 64, 30, 3).len(), 4);
    }

    #[test]
    fn straight_edge_is_not_a_corner() {
        let mut img = vec![30u8; 64 * 64];
        for y in 0..64 {
            for x in 32..64 {
                img[y * 64 + x] = 220;
            }
        }
        assert!(detect(&img, 64, 64, 30, 3).is_empty());
    }

    #[test]
    fn threshold_above_contrast_suppresses() {
        let img = square_image(64, 20, 20, 24, 100, 80);
        assert!(detect(&img, 64, 64, 30, 3).is_empty());
        assert!(!detect(&img, 64, 64, 10, 3).is_empty());
    }

    #[test]
    fn border_excludes_edges() {
        let img = square_image(64, 2, 2, 20, 220, 30);
        let corners = detect(&img, 64, 64, 30, 10);
        assert!(
            corners
                .iter()
                .all(|c| c.x >= 10 && c.y >= 10 && c.x < 54 && c.y < 54)
        );
    }

    #[test]
    fn tiny_or_short_buffers_yield_nothing() {
        assert!(detect(&[0u8; 9], 3, 3, 10, 3).is_empty());
        assert!(detect(&[0u8; 10], 64, 64, 10, 3).is_empty());
        assert!(detect(&[], 0, 0, 10, 3).is_empty());
    }

    #[test]
    fn nms_keeps_one_per_plateau() {
        let mut img = vec![0u8; 40 * 40];
        img[20 * 40 + 20] = 255;
        let corners = detect(&img, 40, 40, 20, 3);
        assert_eq!(
            corners,
            vec![Corner {
                x: 20,
                y: 20,
                score: 16 * (255 - 20)
            }]
        );
    }

    #[test]
    fn deterministic_on_noise() {
        let mut s = 0x1234u64;
        let img: Vec<u8> = (0..96 * 80)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                s as u8
            })
            .collect();
        let a = detect(&img, 96, 80, 25, 4);
        assert_eq!(a, detect(&img, 96, 80, 25, 4));
        assert!(!a.is_empty());
        for w in a.windows(2) {
            assert!((w[0].y, w[0].x) < (w[1].y, w[1].x));
        }
    }
}
