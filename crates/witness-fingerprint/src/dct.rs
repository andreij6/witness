pub const PDQ_SIDE: usize = 64;
pub const PDQ_HALF: usize = PDQ_SIDE / 2;
pub const PDQ_FREQS: usize = 16;

pub const PDQ_COS: [[i32; PDQ_HALF]; PDQ_FREQS] = [
    [
        65516, 65358, 65043, 64571, 63944, 63162, 62228, 61145, 59914, 58538, 57022, 55368, 53581,
        51665, 49624, 47464, 45190, 42806, 40320, 37736, 35062, 32303, 29466, 26558, 23586, 20557,
        17479, 14359, 11204, 8022, 4821, 1608,
    ],
    [
        65457, 64827, 63572, 61705, 59244, 56212, 52639, 48559, 44011, 39040, 33692, 28020, 22078,
        15924, 9616, 3216, -3216, -9616, -15924, -22078, -28020, -33692, -39040, -44011, -48559,
        -52639, -56212, -59244, -61705, -63572, -64827, -65457,
    ],
    [
        65358, 63944, 61145, 57022, 51665, 45190, 37736, 29466, 20557, 11204, 1608, -8022, -17479,
        -26558, -35062, -42806, -49624, -55368, -59914, -63162, -65043, -65516, -64571, -62228,
        -58538, -53581, -47464, -40320, -32303, -23586, -14359, -4821,
    ],
    [
        65220, 62714, 57798, 50660, 41576, 30893, 19024, 6424, -6424, -19024, -30893, -41576,
        -50660, -57798, -62714, -65220, -65220, -62714, -57798, -50660, -41576, -30893, -19024,
        -6424, 6424, 19024, 30893, 41576, 50660, 57798, 62714, 65220,
    ],
    [
        65043, 61145, 53581, 42806, 29466, 14359, -1608, -17479, -32303, -45190, -55368, -62228,
        -65358, -64571, -59914, -51665, -40320, -26558, -11204, 4821, 20557, 35062, 47464, 57022,
        63162, 65516, 63944, 58538, 49624, 37736, 23586, 8022,
    ],
    [
        64827, 59244, 48559, 33692, 15924, -3216, -22078, -39040, -52639, -61705, -65457, -63572,
        -56212, -44011, -28020, -9616, 9616, 28020, 44011, 56212, 63572, 65457, 61705, 52639,
        39040, 22078, 3216, -15924, -33692, -48559, -59244, -64827,
    ],
    [
        64571, 57022, 42806, 23586, 1608, -20557, -40320, -55368, -63944, -65043, -58538, -45190,
        -26558, -4821, 17479, 37736, 53581, 63162, 65358, 59914, 47464, 29466, 8022, -14359,
        -35062, -51665, -62228, -65516, -61145, -49624, -32303, -11204,
    ],
    [
        64277, 54491, 36410, 12785, -12785, -36410, -54491, -64277, -64277, -54491, -36410, -12785,
        12785, 36410, 54491, 64277, 64277, 54491, 36410, 12785, -12785, -36410, -54491, -64277,
        -64277, -54491, -36410, -12785, 12785, 36410, 54491, 64277,
    ],
    [
        63944, 51665, 29466, 1608, -26558, -49624, -63162, -64571, -53581, -32303, -4821, 23586,
        47464, 62228, 65043, 55368, 35062, 8022, -20557, -45190, -61145, -65358, -57022, -37736,
        -11204, 17479, 42806, 59914, 65516, 58538, 40320, 14359,
    ],
    [
        63572, 48559, 22078, -9616, -39040, -59244, -65457, -56212, -33692, -3216, 28020, 52639,
        64827, 61705, 44011, 15924, -15924, -44011, -61705, -64827, -52639, -28020, 3216, 33692,
        56212, 65457, 59244, 39040, 9616, -22078, -48559, -63572,
    ],
    [
        63162, 45190, 14359, -20557, -49624, -64571, -61145, -40320, -8022, 26558, 53581, 65358,
        58538, 35062, 1608, -32303, -57022, -65516, -55368, -29466, 4821, 37736, 59914, 65043,
        51665, 23586, -11204, -42806, -62228, -63944, -47464, -17479,
    ],
    [
        62714, 41576, 6424, -30893, -57798, -65220, -50660, -19024, 19024, 50660, 65220, 57798,
        30893, -6424, -41576, -62714, -62714, -41576, -6424, 30893, 57798, 65220, 50660, 19024,
        -19024, -50660, -65220, -57798, -30893, 6424, 41576, 62714,
    ],
    [
        62228, 37736, -1608, -40320, -63162, -61145, -35062, 4821, 42806, 63944, 59914, 32303,
        -8022, -45190, -64571, -58538, -29466, 11204, 47464, 65043, 57022, 26558, -14359, -49624,
        -65358, -55368, -23586, 17479, 51665, 65516, 53581, 20557,
    ],
    [
        61705, 33692, -9616, -48559, -65457, -52639, -15924, 28020, 59244, 63572, 39040, -3216,
        -44011, -64827, -56212, -22078, 22078, 56212, 64827, 44011, 3216, -39040, -63572, -59244,
        -28020, 15924, 52639, 65457, 48559, 9616, -33692, -61705,
    ],
    [
        61145, 29466, -17479, -55368, -64571, -40320, 4821, 47464, 65516, 49624, 8022, -37736,
        -63944, -57022, -20557, 26558, 59914, 62228, 32303, -14359, -53581, -65043, -42806, 1608,
        45190, 65358, 51665, 11204, -35062, -63162, -58538, -23586,
    ],
    [
        60547, 25080, -25080, -60547, -60547, -25080, 25080, 60547, 60547, 25080, -25080, -60547,
        -60547, -25080, 25080, 60547, 60547, 25080, -25080, -60547, -60547, -25080, 25080, 60547,
        60547, 25080, -25080, -60547, -60547, -25080, 25080, 60547,
    ],
];

pub const CELL_SIDE: usize = 8;
pub const CELL_FREQS: usize = 3;

pub const CELL_COS: [[i32; CELL_SIDE]; CELL_FREQS] = [
    [4096, 4096, 4096, 4096, 4096, 4096, 4096, 4096],
    [4017, 3406, 2276, 799, -799, -2276, -3406, -4017],
    [3784, 1567, -1567, -3784, -3784, -1567, 1567, 3784],
];

pub type Coefficients = [[i64; PDQ_FREQS]; PDQ_FREQS];

pub fn pdq_dct(buffer: &[u16]) -> Coefficients {
    debug_assert_eq!(buffer.len(), PDQ_SIDE * PDQ_SIDE);
    let mut folded = [[[0i32; PDQ_SIDE]; PDQ_HALF]; 2];
    for k in 0..PDQ_HALF {
        let top = &buffer[k * PDQ_SIDE..][..PDQ_SIDE];
        let bottom = &buffer[(PDQ_SIDE - 1 - k) * PDQ_SIDE..][..PDQ_SIDE];
        for j in 0..PDQ_SIDE {
            let (a, b) = (top[j] as i32, bottom[j] as i32);
            folded[0][k][j] = a - b;
            folded[1][k][j] = a + b;
        }
    }
    let mut vertical = [[0i64; PDQ_SIDE]; PDQ_FREQS];
    for (u, out) in vertical.iter_mut().enumerate() {
        let parity = &folded[u & 1];
        for (k, &c) in PDQ_COS[u].iter().enumerate() {
            let c = c as i64;
            out.iter_mut()
                .zip(&parity[k])
                .for_each(|(acc, &v)| *acc += c * v as i64);
        }
    }
    let mut out = [[0i64; PDQ_FREQS]; PDQ_FREQS];
    for (row, line) in out.iter_mut().zip(&vertical) {
        for (v, acc) in row.iter_mut().enumerate() {
            let sign = if v & 1 == 0 { -1 } else { 1 };
            *acc = PDQ_COS[v]
                .iter()
                .enumerate()
                .map(|(k, &c)| c as i64 * (line[k] + sign * line[PDQ_SIDE - 1 - k]))
                .sum();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Rng;
    use core::f64::consts::PI;

    #[test]
    fn pdq_table_matches_cosine_definition() {
        for (u, row) in PDQ_COS.iter().enumerate() {
            for (k, &v) in row.iter().enumerate() {
                let exact = (PI / 128.0 * (u + 1) as f64 * (2 * k + 1) as f64).cos() * 65536.0;
                assert_eq!(v as f64, exact.round(), "u={u} k={k}");
            }
        }
    }

    #[test]
    fn cell_table_matches_cosine_definition() {
        for (u, row) in CELL_COS.iter().enumerate() {
            for (x, &v) in row.iter().enumerate() {
                let exact = (PI * u as f64 * (2 * x + 1) as f64 / 16.0).cos() * 4096.0;
                assert_eq!(v as f64, exact.round(), "u={u} x={x}");
            }
        }
    }

    #[test]
    fn table_symmetry_used_by_folding_holds() {
        for (u, row) in PDQ_COS.iter().enumerate() {
            for (k, &v) in row.iter().enumerate() {
                let mirrored =
                    (PI / 128.0 * (u + 1) as f64 * (2 * (63 - k) + 1) as f64).cos() * 65536.0;
                let sign = if u % 2 == 0 { -1.0 } else { 1.0 };
                assert_eq!(mirrored.round(), sign * v as f64);
            }
        }
    }

    fn naive(buffer: &[u16]) -> Coefficients {
        let full = |u: usize, k: usize| -> i64 {
            if k < PDQ_HALF {
                PDQ_COS[u][k] as i64
            } else {
                let sign = if u.is_multiple_of(2) { -1 } else { 1 };
                sign * PDQ_COS[u][63 - k] as i64
            }
        };
        let mut out = [[0i64; PDQ_FREQS]; PDQ_FREQS];
        for (u, row) in out.iter_mut().enumerate() {
            for (v, slot) in row.iter_mut().enumerate() {
                let mut sum = 0i64;
                for k in 0..PDQ_SIDE {
                    let mut inner = 0i64;
                    for l in 0..PDQ_SIDE {
                        inner += full(v, l) * buffer[k * PDQ_SIDE + l] as i64;
                    }
                    sum += full(u, k) * inner;
                }
                *slot = sum;
            }
        }
        out
    }

    #[test]
    fn folded_dct_equals_naive_on_random_buffers() {
        let mut rng = Rng::new(11);
        for _ in 0..6 {
            let buffer: Vec<u16> = (0..PDQ_SIDE * PDQ_SIDE)
                .map(|_| rng.next() as u16)
                .collect();
            assert_eq!(pdq_dct(&buffer), naive(&buffer));
        }
    }

    #[test]
    fn constant_buffer_has_no_energy() {
        let out = pdq_dct(&vec![40_000u16; PDQ_SIDE * PDQ_SIDE]);
        assert!(out.iter().flatten().all(|&v| v == 0));
    }

    #[test]
    fn extreme_buffers_do_not_overflow() {
        let checker: Vec<u16> = (0..PDQ_SIDE * PDQ_SIDE)
            .map(|i| {
                if (i / 64 + i % 64) % 2 == 0 {
                    u16::MAX
                } else {
                    0
                }
            })
            .collect();
        let halves: Vec<u16> = (0..PDQ_SIDE * PDQ_SIDE)
            .map(|i| {
                if i % 64 < 32 && i / 64 < 32 {
                    u16::MAX
                } else {
                    0
                }
            })
            .collect();
        assert_eq!(pdq_dct(&checker), naive(&checker));
        assert_eq!(pdq_dct(&halves), naive(&halves));
        assert!(pdq_dct(&halves)[0][0] > 0);
    }

    #[test]
    fn single_cosine_concentrates_in_its_frequency() {
        let buffer: Vec<u16> = (0..PDQ_SIDE * PDQ_SIDE)
            .map(|i| {
                let (x, y) = ((i % 64) as f64, (i / 64) as f64);
                let horizontal = (PI / 128.0 * 3.0 * (2.0 * x + 1.0)).cos();
                let vertical = (PI / 128.0 * 2.0 * (2.0 * y + 1.0)).cos();
                (30_000.0 + 20_000.0 * horizontal * vertical).round() as u16
            })
            .collect();
        let out = pdq_dct(&buffer);
        let peak = out[1][2];
        assert!(peak > 0);
        for (u, row) in out.iter().enumerate() {
            for (v, &value) in row.iter().enumerate() {
                if (u, v) != (1, 2) {
                    assert!(value.abs() < peak / 1000, "{u} {v}");
                }
            }
        }
    }
}
