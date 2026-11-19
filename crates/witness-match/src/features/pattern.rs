pub(crate) const ANGLE_BINS: usize = 32;
pub(crate) const PAIRS: usize = 256;
pub(crate) const PATTERN_RADIUS: i32 = 13;
pub(crate) const CENTROID_RADIUS: usize = 15;
pub(crate) const Q14_ONE: i32 = 1 << 14;

const OCTANT_Q14: [i32; 9] = [16384, 16069, 15137, 13623, 11585, 9102, 6270, 3196, 0];

pub(crate) const fn cos_q14(bin: usize) -> i32 {
    let k = bin % ANGLE_BINS;
    match k {
        0..=8 => OCTANT_Q14[k],
        9..=16 => -OCTANT_Q14[16 - k],
        17..=24 => -OCTANT_Q14[k - 16],
        _ => OCTANT_Q14[32 - k],
    }
}

pub(crate) const fn sin_q14(bin: usize) -> i32 {
    cos_q14((bin + 24) % ANGLE_BINS)
}

const fn isqrt(n: i32) -> i32 {
    let mut r = 0;
    while (r + 1) * (r + 1) <= n {
        r += 1;
    }
    r
}

const fn build_disc() -> [i32; CENTROID_RADIUS + 1] {
    let mut out = [0; CENTROID_RADIUS + 1];
    let r2 = (CENTROID_RADIUS * CENTROID_RADIUS) as i32;
    let mut v = 0;
    while v <= CENTROID_RADIUS {
        out[v] = isqrt(r2 - (v * v) as i32);
        v += 1;
    }
    out
}

pub(crate) const DISC_HALF_WIDTH: [i32; CENTROID_RADIUS + 1] = build_disc();

const fn xorshift(mut s: u64) -> u64 {
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    s
}

const fn sample_point(mut state: u64) -> (u64, i8, i8) {
    let span = (2 * PATTERN_RADIUS + 1) as u64;
    loop {
        state = xorshift(state);
        let x = (state % span) as i32 - PATTERN_RADIUS;
        let y = ((state >> 20) % span) as i32 - PATTERN_RADIUS;
        if x * x + y * y <= PATTERN_RADIUS * PATTERN_RADIUS {
            return (state, x as i8, y as i8);
        }
    }
}

const fn build_pattern() -> [[i8; 4]; PAIRS] {
    let mut out = [[0i8; 4]; PAIRS];
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut i = 0;
    while i < PAIRS {
        let (s1, x1, y1) = sample_point(state);
        let (s2, x2, y2) = sample_point(s1);
        state = s2;
        if x1 != x2 || y1 != y2 {
            out[i] = [x1, y1, x2, y2];
            i += 1;
        }
    }
    out
}

pub(crate) const PATTERN: [[i8; 4]; PAIRS] = build_pattern();

const fn rotate(x: i8, y: i8, bin: usize) -> (i8, i8) {
    let (c, s) = (cos_q14(bin), sin_q14(bin));
    let (x, y) = (x as i32, y as i32);
    let half = Q14_ONE / 2;
    (
        ((c * x - s * y + half) >> 14) as i8,
        ((s * x + c * y + half) >> 14) as i8,
    )
}

const fn build_rotated() -> [[[i8; 4]; PAIRS]; ANGLE_BINS] {
    let mut out = [[[0i8; 4]; PAIRS]; ANGLE_BINS];
    let mut bin = 0;
    while bin < ANGLE_BINS {
        let mut i = 0;
        while i < PAIRS {
            let p = PATTERN[i];
            let (ax, ay) = rotate(p[0], p[1], bin);
            let (bx, by) = rotate(p[2], p[3], bin);
            out[bin][i] = [ax, ay, bx, by];
            i += 1;
        }
        bin += 1;
    }
    out
}

pub(crate) static ROTATED: [[[i8; 4]; PAIRS]; ANGLE_BINS] = build_rotated();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trig_table_matches_float_reference() {
        for bin in 0..ANGLE_BINS {
            let angle = bin as f64 * core::f64::consts::TAU / ANGLE_BINS as f64;
            assert!(
                (cos_q14(bin) as f64 / Q14_ONE as f64 - angle.cos()).abs() < 1e-4,
                "cos {bin}"
            );
            assert!(
                (sin_q14(bin) as f64 / Q14_ONE as f64 - angle.sin()).abs() < 1e-4,
                "sin {bin}"
            );
        }
        assert_eq!(cos_q14(ANGLE_BINS + 3), cos_q14(3));
    }

    #[test]
    fn disc_is_symmetric_circle() {
        assert_eq!(DISC_HALF_WIDTH[0], 15);
        assert_eq!(DISC_HALF_WIDTH[15], 0);
        for (v, &u) in DISC_HALF_WIDTH.iter().enumerate() {
            assert!(u * u + (v * v) as i32 <= 225);
            assert!((u + 1) * (u + 1) + (v * v) as i32 > 225);
        }
    }

    #[test]
    fn pattern_points_inside_radius_and_pairs_distinct() {
        for p in PATTERN {
            assert!((p[0] as i32).pow(2) + (p[1] as i32).pow(2) <= PATTERN_RADIUS * PATTERN_RADIUS);
            assert!((p[2] as i32).pow(2) + (p[3] as i32).pow(2) <= PATTERN_RADIUS * PATTERN_RADIUS);
            assert!(p[0] != p[2] || p[1] != p[3]);
        }
    }

    #[test]
    fn pattern_is_spread_out() {
        let mean_x: i32 = PATTERN.iter().map(|p| p[0] as i32 + p[2] as i32).sum();
        let mean_y: i32 = PATTERN.iter().map(|p| p[1] as i32 + p[3] as i32).sum();
        assert!(mean_x.abs() < 400 && mean_y.abs() < 400);
        let mut unique: Vec<[i8; 4]> = PATTERN.to_vec();
        unique.sort();
        unique.dedup();
        assert!(unique.len() > 250);
    }

    #[test]
    fn rotation_bin_zero_is_identity() {
        assert_eq!(ROTATED[0], PATTERN);
    }

    #[test]
    fn rotation_quarter_turn_is_exact() {
        for (p, r) in PATTERN.iter().zip(ROTATED[8].iter()) {
            assert_eq!(*r, [-p[1], p[0], -p[3], p[2]]);
        }
    }

    #[test]
    fn rotated_points_stay_in_patch() {
        for bin in ROTATED {
            for p in bin {
                for v in p {
                    assert!((v as i32).abs() <= PATTERN_RADIUS + 1);
                }
            }
        }
    }
}
