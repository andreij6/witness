use crate::geometry::Similarity;
use crate::rng::Rng;

pub(crate) type Point = (f64, f64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RansacConfig {
    pub iterations: u32,
    pub threshold: f64,
    pub min_scale: f64,
    pub max_scale: f64,
    pub refine_rounds: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Estimate {
    pub transform: Similarity,
    pub inliers: u32,
}

pub(crate) fn from_two(p1: Point, p2: Point, q1: Point, q2: Point) -> Option<Similarity> {
    let (dpx, dpy) = (p2.0 - p1.0, p2.1 - p1.1);
    let (dqx, dqy) = (q2.0 - q1.0, q2.1 - q1.1);
    let norm = dpx * dpx + dpy * dpy;
    if norm.is_nan() || norm <= 1e-9 {
        return None;
    }
    let a = (dqx * dpx + dqy * dpy) / norm;
    let b = (dqy * dpx - dqx * dpy) / norm;
    let t = Similarity::new(a, b, 0.0, 0.0, false);
    let (mx, my) = t.apply(p1.0, p1.1);
    let out = Similarity::new(a, b, q1.0 - mx, q1.1 - my, false);
    out.is_finite().then_some(out)
}

pub(crate) fn least_squares(pairs: &[(Point, Point)], inliers: &[bool]) -> Option<Similarity> {
    let selected = || {
        pairs
            .iter()
            .zip(inliers)
            .filter(|(_, keep)| **keep)
            .map(|(pair, _)| pair)
    };
    let n = selected().count();
    if n < 2 {
        return None;
    }
    let (mut spx, mut spy, mut sqx, mut sqy) = (0.0, 0.0, 0.0, 0.0);
    for (p, q) in selected() {
        spx += p.0;
        spy += p.1;
        sqx += q.0;
        sqy += q.1;
    }
    let inv = 1.0 / n as f64;
    let (cpx, cpy, cqx, cqy) = (spx * inv, spy * inv, sqx * inv, sqy * inv);
    let (mut num_a, mut num_b, mut den) = (0.0, 0.0, 0.0);
    for (p, q) in selected() {
        let (px, py, qx, qy) = (p.0 - cpx, p.1 - cpy, q.0 - cqx, q.1 - cqy);
        num_a += px * qx + py * qy;
        num_b += px * qy - py * qx;
        den += px * px + py * py;
    }
    if den.is_nan() || den <= 1e-9 {
        return None;
    }
    let (a, b) = (num_a / den, num_b / den);
    let t = Similarity::new(a, b, 0.0, 0.0, false);
    let (mx, my) = t.apply(cpx, cpy);
    let out = Similarity::new(a, b, cqx - mx, cqy - my, false);
    out.is_finite().then_some(out)
}

fn mark_inliers(
    pairs: &[(Point, Point)],
    t: &Similarity,
    threshold_sq: f64,
    mask: &mut [bool],
) -> u32 {
    let mut count = 0;
    for ((p, q), m) in pairs.iter().zip(mask.iter_mut()) {
        let (x, y) = t.apply(p.0, p.1);
        let (dx, dy) = (x - q.0, y - q.1);
        *m = dx * dx + dy * dy <= threshold_sq;
        count += *m as u32;
    }
    count
}

fn count_inliers(pairs: &[(Point, Point)], t: &Similarity, threshold_sq: f64) -> u32 {
    pairs
        .iter()
        .filter(|(p, q)| {
            let (x, y) = t.apply(p.0, p.1);
            let (dx, dy) = (x - q.0, y - q.1);
            dx * dx + dy * dy <= threshold_sq
        })
        .count() as u32
}

pub(crate) fn estimate<R: Rng>(
    pairs: &[(Point, Point)],
    config: &RansacConfig,
    rng: &mut R,
) -> Option<Estimate> {
    let n = pairs.len();
    if n < 2 || n > u32::MAX as usize {
        return None;
    }
    let threshold_sq = config.threshold * config.threshold;
    let mut best: Option<Estimate> = None;
    for _ in 0..config.iterations {
        let i = rng.below(n as u32) as usize;
        let j = rng.below(n as u32 - 1) as usize;
        let j = if j >= i { j + 1 } else { j };
        let Some(t) = from_two(pairs[i].0, pairs[j].0, pairs[i].1, pairs[j].1) else {
            continue;
        };
        let scale = t.scale();
        if !(scale >= config.min_scale && scale <= config.max_scale) {
            continue;
        }
        let inliers = count_inliers(pairs, &t, threshold_sq);
        if best.as_ref().is_none_or(|b| inliers > b.inliers) {
            best = Some(Estimate {
                transform: t,
                inliers,
            });
            if inliers as usize == n {
                break;
            }
        }
    }
    let mut best = best?;
    let mut mask = vec![false; n];
    mark_inliers(pairs, &best.transform, threshold_sq, &mut mask);
    for _ in 0..config.refine_rounds {
        let Some(t) = least_squares(pairs, &mask) else {
            break;
        };
        let scale = t.scale();
        if !(scale >= config.min_scale && scale <= config.max_scale) {
            break;
        }
        let mut next_mask = vec![false; n];
        let inliers = mark_inliers(pairs, &t, threshold_sq, &mut next_mask);
        if inliers < best.inliers {
            break;
        }
        let stable = next_mask == mask;
        best = Estimate {
            transform: t,
            inliers,
        };
        mask = next_mask;
        if stable {
            break;
        }
    }
    Some(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn config() -> RansacConfig {
        RansacConfig {
            iterations: 300,
            threshold: 2.0,
            min_scale: 0.1,
            max_scale: 10.0,
            refine_rounds: 3,
        }
    }

    fn synth(
        t: &Similarity,
        n: usize,
        outliers: usize,
        noise: f64,
        seed: u64,
    ) -> Vec<(Point, Point)> {
        let mut rng = SplitMix64::new(seed);
        let mut u = || (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        let mut out = Vec::new();
        for k in 0..n + outliers {
            let p = (u() * 500.0, u() * 400.0);
            let q = if k < n {
                let (x, y) = t.apply(p.0, p.1);
                (x + (u() - 0.5) * noise, y + (u() - 0.5) * noise)
            } else {
                (u() * 500.0, u() * 400.0)
            };
            out.push((p, q));
        }
        out
    }

    fn assert_close(a: &Similarity, b: &Similarity, tol: f64) {
        for p in [(0.0, 0.0), (500.0, 0.0), (0.0, 400.0), (500.0, 400.0)] {
            let (x, y) = a.apply(p.0, p.1);
            let (u, v) = b.apply(p.0, p.1);
            assert!((x - u).abs() < tol && (y - v).abs() < tol, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn two_point_solution_is_exact() {
        let t = Similarity::new(0.9, 0.3, 12.0, -7.0, false);
        let (p1, p2) = ((10.0, 20.0), (200.0, 50.0));
        let s = from_two(p1, p2, t.apply(p1.0, p1.1), t.apply(p2.0, p2.1)).unwrap();
        assert_close(&s, &t, 1e-9);
    }

    #[test]
    fn coincident_points_are_degenerate() {
        assert_eq!(
            from_two((1.0, 1.0), (1.0, 1.0), (0.0, 0.0), (5.0, 5.0)),
            None
        );
        assert_eq!(
            from_two((f64::NAN, 1.0), (1.0, 1.0), (0.0, 0.0), (5.0, 5.0)),
            None
        );
    }

    #[test]
    fn least_squares_recovers_noiseless_model() {
        let t = Similarity::new(1.3, -0.4, 40.0, 3.0, false);
        let pairs = synth(&t, 30, 0, 0.0, 1);
        let s = least_squares(&pairs, &[true; 30]).unwrap();
        assert_close(&s, &t, 1e-6);
        assert_eq!(least_squares(&pairs, &[false; 30]), None);
    }

    #[test]
    fn recovers_model_with_half_outliers() {
        let t = Similarity::new(0.7, 0.2, -30.0, 15.0, false);
        let pairs = synth(&t, 60, 60, 1.0, 2);
        let est = estimate(&pairs, &config(), &mut SplitMix64::new(3)).unwrap();
        assert!(est.inliers >= 58 && est.inliers <= 64, "{}", est.inliers);
        assert_close(&est.transform, &t, 1.0);
    }

    #[test]
    fn random_models_property() {
        let mut rng = SplitMix64::new(77);
        for case in 0..25 {
            let a = 0.3 + (rng.below(1000) as f64) / 600.0;
            let b = (rng.below(1000) as f64) / 1000.0 - 0.5;
            let t = Similarity::new(
                a,
                b,
                rng.below(200) as f64 - 100.0,
                rng.below(200) as f64 - 100.0,
                false,
            );
            let pairs = synth(&t, 40, 30, 0.8, case);
            let est = estimate(&pairs, &config(), &mut SplitMix64::new(case)).unwrap();
            assert_close(&est.transform, &t, 1.5);
        }
    }

    #[test]
    fn deterministic_given_seed() {
        let t = Similarity::new(1.1, 0.1, 5.0, 5.0, false);
        let pairs = synth(&t, 20, 40, 1.0, 4);
        let a = estimate(&pairs, &config(), &mut SplitMix64::new(9));
        let b = estimate(&pairs, &config(), &mut SplitMix64::new(9));
        assert_eq!(a, b);
    }

    #[test]
    fn scale_bounds_reject_models() {
        let t = Similarity::new(20.0, 0.0, 0.0, 0.0, false);
        let pairs = synth(&t, 20, 0, 0.0, 5);
        assert_eq!(estimate(&pairs, &config(), &mut SplitMix64::new(1)), None);
    }

    #[test]
    fn too_few_pairs() {
        assert_eq!(estimate(&[], &config(), &mut SplitMix64::new(1)), None);
        assert_eq!(
            estimate(
                &[((0.0, 0.0), (1.0, 1.0))],
                &config(),
                &mut SplitMix64::new(1)
            ),
            None
        );
    }

    #[test]
    fn pure_noise_has_few_inliers() {
        let pairs = synth(&Similarity::IDENTITY, 0, 100, 0.0, 6);
        let est = estimate(&pairs, &config(), &mut SplitMix64::new(2)).unwrap();
        assert!(est.inliers < 10, "{}", est.inliers);
    }
}
