use super::pattern::{ANGLE_BINS, CENTROID_RADIUS, DISC_HALF_WIDTH, ROTATED, cos_q14, sin_q14};

pub type Descriptor = [u64; 4];

pub(crate) const SMOOTH_RADIUS: usize = 2;

pub fn hamming(a: &Descriptor, b: &Descriptor) -> u32 {
    (a[0] ^ b[0]).count_ones()
        + (a[1] ^ b[1]).count_ones()
        + (a[2] ^ b[2]).count_ones()
        + (a[3] ^ b[3]).count_ones()
}

pub(crate) fn angle_bin(img: &[u8], width: usize, x: usize, y: usize) -> usize {
    let r = CENTROID_RADIUS as isize;
    let center = (y * width + x) as isize;
    let (mut m10, mut m01) = (0i64, 0i64);
    for v in -r..=r {
        let half = DISC_HALF_WIDTH[v.unsigned_abs()] as isize;
        let row = center + v * width as isize;
        let mut row_sum = 0i64;
        for u in -half..=half {
            let value = img[(row + u) as usize] as i64;
            m10 += u as i64 * value;
            row_sum += value;
        }
        m01 += v as i64 * row_sum;
    }
    let mut best = (0usize, i64::MIN);
    for bin in 0..ANGLE_BINS {
        let dot = m10 * cos_q14(bin) as i64 + m01 * sin_q14(bin) as i64;
        if dot > best.1 {
            best = (bin, dot);
        }
    }
    best.0
}

pub(crate) fn box_sum5(img: &[u8], width: usize, height: usize) -> Vec<u16> {
    let r = SMOOTH_RADIUS;
    let mut out = vec![0u16; width * height];
    if width <= 2 * r || height <= 2 * r {
        return out;
    }
    let mut columns = vec![0u16; width];
    for row in img.chunks_exact(width).take(2 * r) {
        for (c, &v) in columns.iter_mut().zip(row) {
            *c += v as u16;
        }
    }
    for y in r..height - r {
        let incoming = &img[(y + r) * width..][..width];
        for (c, &v) in columns.iter_mut().zip(incoming) {
            *c += v as u16;
        }
        let out_row = &mut out[y * width..][..width];
        let mut sum: u16 = columns[..2 * r + 1].iter().sum();
        out_row[r] = sum;
        for x in r + 1..width - r {
            sum = sum + columns[x + r] - columns[x - r - 1];
            out_row[x] = sum;
        }
        let outgoing = &img[(y - r) * width..][..width];
        for (c, &v) in columns.iter_mut().zip(outgoing) {
            *c -= v as u16;
        }
    }
    out
}

pub(crate) fn describe(smooth: &[u16], width: usize, x: usize, y: usize, bin: usize) -> Descriptor {
    let center = (y * width + x) as isize;
    let w = width as isize;
    let mut d = [0u64; 4];
    for (i, p) in ROTATED[bin].iter().enumerate() {
        let a = smooth[(center + p[1] as isize * w + p[0] as isize) as usize];
        let b = smooth[(center + p[3] as isize * w + p[2] as isize) as usize];
        d[i >> 6] |= ((a < b) as u64) << (i & 63);
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(width: usize, height: usize, seed: u64) -> Vec<u8> {
        let mut s = seed;
        (0..width * height)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                (s >> 40) as u8
            })
            .collect()
    }

    #[test]
    fn hamming_counts_bits() {
        assert_eq!(hamming(&[0; 4], &[0; 4]), 0);
        assert_eq!(hamming(&[u64::MAX; 4], &[0; 4]), 256);
        assert_eq!(hamming(&[1, 2, 4, 8], &[0; 4]), 4);
        assert_eq!(hamming(&[3, 0, 0, 0], &[1, 0, 0, 1]), 2);
    }

    #[test]
    fn box_sum_matches_naive() {
        let (w, h) = (23, 17);
        let img = noise(w, h, 77);
        let fast = box_sum5(&img, w, h);
        for y in 2..h - 2 {
            for x in 2..w - 2 {
                let mut naive = 0u16;
                for dy in 0..5 {
                    for dx in 0..5 {
                        naive += img[(y + dy - 2) * w + x + dx - 2] as u16;
                    }
                }
                assert_eq!(fast[y * w + x], naive, "{x},{y}");
            }
        }
        assert_eq!(fast[0], 0);
    }

    #[test]
    fn box_sum_tiny_images_are_zero() {
        assert_eq!(box_sum5(&[9; 16], 4, 4), vec![0; 16]);
        assert_eq!(box_sum5(&[255; 25], 5, 5)[12], 25 * 255);
    }

    #[test]
    fn angle_follows_brightness_direction() {
        let size = 41;
        for (target, bright) in [
            (0usize, (1i32, 0i32)),
            (8, (0, 1)),
            (16, (-1, 0)),
            (24, (0, -1)),
            (4, (1, 1)),
        ] {
            let img: Vec<u8> = (0..size * size)
                .map(|i| {
                    let (x, y) = ((i % size) as i32 - 20, (i / size) as i32 - 20);
                    if x * bright.0 + y * bright.1 > 0 {
                        200
                    } else {
                        20
                    }
                })
                .collect();
            assert_eq!(angle_bin(&img, size, 20, 20), target);
        }
    }

    #[test]
    fn flat_patch_angle_is_zero() {
        let img = vec![90u8; 41 * 41];
        assert_eq!(angle_bin(&img, 41, 20, 20), 0);
    }

    #[test]
    fn rotation_by_quarter_turn_shifts_bin_and_preserves_descriptor() {
        let size = 61;
        let base = noise(size, size, 5);
        let rotated: Vec<u8> = (0..size * size)
            .map(|i| {
                let (x, y) = (i % size, i / size);
                base[(size - 1 - x) * size + y]
            })
            .collect();
        let (sa, sb) = (box_sum5(&base, size, size), box_sum5(&rotated, size, size));
        let (ba, bb) = (
            angle_bin(&base, size, 30, 30),
            angle_bin(&rotated, size, 30, 30),
        );
        assert_eq!((ba + 8) % ANGLE_BINS, bb);
        let da = describe(&sa, size, 30, 30, ba);
        let db = describe(&sb, size, 30, 30, bb);
        assert!(hamming(&da, &db) <= 8, "{}", hamming(&da, &db));
    }

    #[test]
    fn different_patches_have_far_descriptors() {
        let size = 61;
        let a = noise(size, size, 1);
        let b = noise(size, size, 2);
        let da = describe(&box_sum5(&a, size, size), size, 30, 30, 0);
        let db = describe(&box_sum5(&b, size, size), size, 30, 30, 0);
        assert!(hamming(&da, &db) > 80);
    }

    #[test]
    fn brightness_offset_does_not_change_descriptor() {
        let size = 61;
        let a = noise(size, size, 3)
            .iter()
            .map(|v| v / 2)
            .collect::<Vec<u8>>();
        let b = a.iter().map(|v| v + 40).collect::<Vec<u8>>();
        let da = describe(&box_sum5(&a, size, size), size, 30, 30, 3);
        let db = describe(&box_sum5(&b, size, size), size, 30, 30, 3);
        assert_eq!(da, db);
    }
}
