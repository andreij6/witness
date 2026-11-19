pub(crate) fn solve<const N: usize>(mut a: [[f64; N]; N], mut b: [f64; N]) -> Option<[f64; N]> {
    let scale = a.iter().flatten().fold(0.0f64, |m, v| m.max(v.abs()));
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let tolerance = scale * 1e-12;
    for col in 0..N {
        let mut pivot = col;
        for row in col + 1..N {
            if a[row][col].abs() > a[pivot][col].abs() {
                pivot = row;
            }
        }
        if a[pivot][col].is_nan() || a[pivot][col].abs() <= tolerance {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..N {
            let factor = a[row][col] / a[col][col];
            if factor != 0.0 {
                let pivot_row = a[col];
                for (target, &source) in a[row][col..].iter_mut().zip(&pivot_row[col..]) {
                    *target -= factor * source;
                }
                b[row] -= factor * b[col];
            }
        }
    }
    let mut x = [0.0; N];
    for row in (0..N).rev() {
        let mut sum = b[row];
        for k in row + 1..N {
            sum -= a[row][k] * x[k];
        }
        x[row] = sum / a[row][row];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_small_system() {
        let x = solve([[2.0, 1.0], [1.0, 3.0]], [3.0, 5.0]).unwrap();
        assert!((x[0] - 0.8).abs() < 1e-12 && (x[1] - 1.4).abs() < 1e-12);
    }

    #[test]
    fn needs_pivoting() {
        let x = solve([[0.0, 1.0], [1.0, 0.0]], [2.0, 3.0]).unwrap();
        assert_eq!(x, [3.0, 2.0]);
    }

    #[test]
    fn singular_and_degenerate_are_none() {
        assert_eq!(solve([[1.0, 2.0], [2.0, 4.0]], [1.0, 2.0]), None);
        assert_eq!(solve([[0.0; 3]; 3], [0.0; 3]), None);
        assert_eq!(solve([[f64::NAN, 0.0], [0.0, 1.0]], [1.0, 1.0]), None);
        assert_eq!(solve([[f64::INFINITY, 0.0], [0.0, 1.0]], [1.0, 1.0]), None);
    }

    #[test]
    fn random_spd_systems_round_trip() {
        let mut seed = 0xDEAD_BEEFu64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 1000) as f64 / 500.0 - 1.0
        };
        for _ in 0..50 {
            let mut m = [[0.0; 16]; 16];
            for row in m.iter_mut() {
                for v in row.iter_mut() {
                    *v = next();
                }
            }
            let mut a = [[0.0; 16]; 16];
            for i in 0..16 {
                for j in 0..16 {
                    a[i][j] = (0..16).map(|k| m[k][i] * m[k][j]).sum::<f64>()
                        + if i == j { 0.5 } else { 0.0 };
                }
            }
            let x: [f64; 16] = core::array::from_fn(|_| next());
            let b: [f64; 16] = core::array::from_fn(|i| (0..16).map(|j| a[i][j] * x[j]).sum());
            let solved = solve(a, b).unwrap();
            for (s, e) in solved.iter().zip(x) {
                assert!((s - e).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn deterministic() {
        let a = [[4.0, 1.0, 0.5], [1.0, 3.0, 0.2], [0.5, 0.2, 2.0]];
        let b = [1.0, 2.0, 3.0];
        assert_eq!(solve(a, b), solve(a, b));
    }
}
