mod matching;
mod ransac;

use crate::features::{FeatureParams, Features, detect_features};
use crate::geometry::Similarity;
use crate::rng::Rng;
use ransac::{Estimate, Point, RansacConfig};
use witness_image::{Luma8, LumaView, flip_horizontal, resize_area};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlignParams {
    pub work_edge: u32,
    pub features: FeatureParams,
    pub max_hamming: u32,
    pub ratio_percent: u32,
    pub ransac_iterations: u32,
    pub inlier_px: f64,
    pub min_scale: f64,
    pub max_scale: f64,
    pub min_inliers: u32,
    pub refine_rounds: u32,
    pub try_mirrored: bool,
}

impl Default for AlignParams {
    fn default() -> Self {
        Self {
            work_edge: 512,
            features: FeatureParams::default(),
            max_hamming: 64,
            ratio_percent: 85,
            ransac_iterations: 1000,
            inlier_px: 3.0,
            min_scale: 0.1,
            max_scale: 10.0,
            min_inliers: 10,
            refine_rounds: 4,
            try_mirrored: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alignment {
    pub transform: Similarity,
    pub inliers: u32,
    pub matches: u32,
    pub confidence: f64,
}

pub(crate) fn working_size(width: u32, height: u32, edge: u32) -> (u32, u32) {
    let long = width.max(height);
    if edge == 0 || long <= edge {
        return (width, height);
    }
    let scale = |v: u32| ((v as u64 * edge as u64 + long as u64 / 2) / long as u64).max(1) as u32;
    (scale(width), scale(height))
}

fn working(src: LumaView<'_>, edge: u32) -> Luma8 {
    let (w, h) = working_size(src.width(), src.height(), edge);
    if (w, h) == src.dimensions() {
        return src.to_image();
    }
    resize_area(src, w, h).unwrap_or_else(|_| src.to_image())
}

struct Hypothesis {
    estimate: Estimate,
    matches: u32,
}

fn hypothesis<R: Rng>(
    query: &Features,
    reference: &Features,
    params: &AlignParams,
    threshold: f64,
    rng: &mut R,
) -> Option<Hypothesis> {
    let matches = matching::match_descriptors(
        &query.descriptors,
        &reference.descriptors,
        params.max_hamming,
        params.ratio_percent,
    );
    let pairs: Vec<(Point, Point)> = matches
        .iter()
        .map(|m| {
            (
                reference.points[m.reference as usize],
                query.points[m.query as usize],
            )
        })
        .collect();
    let config = RansacConfig {
        iterations: params.ransac_iterations,
        threshold,
        min_scale: params.min_scale,
        max_scale: params.max_scale,
        refine_rounds: params.refine_rounds,
    };
    let estimate = ransac::estimate(&pairs, &config, rng)?;
    (estimate.inliers >= params.min_inliers.max(2)).then_some(Hypothesis {
        estimate,
        matches: pairs.len() as u32,
    })
}

pub fn align<R: Rng>(
    query: LumaView<'_>,
    reference: LumaView<'_>,
    params: &AlignParams,
    rng: &mut R,
) -> Option<Alignment> {
    let (qw, qh) = (query.width() as f64, query.height() as f64);
    let (rw, rh) = (reference.width() as f64, reference.height() as f64);
    let work_query = working(query, params.work_edge);
    let work_reference = working(reference, params.work_edge);
    let reference_features = detect_features(work_reference.view(), rw, rh, &params.features);
    if reference_features.is_empty() {
        return None;
    }
    let query_scale = query.width().max(query.height()) as f64
        / work_query.width().max(work_query.height()) as f64;
    let threshold = params.inlier_px * query_scale;

    let direct_features = detect_features(work_query.view(), qw, qh, &params.features);
    let direct = hypothesis(
        &direct_features,
        &reference_features,
        params,
        threshold,
        rng,
    );
    let mirrored = if params.try_mirrored {
        let flipped = flip_horizontal(work_query.view());
        let flipped_features = detect_features(flipped.view(), qw, qh, &params.features);
        hypothesis(
            &flipped_features,
            &reference_features,
            params,
            threshold,
            rng,
        )
        .map(|h| Hypothesis {
            estimate: Estimate {
                transform: h.estimate.transform.mirror_x(qw),
                inliers: h.estimate.inliers,
            },
            matches: h.matches,
        })
    } else {
        None
    };
    let best = match (direct, mirrored) {
        (Some(d), Some(m)) => {
            if m.estimate.inliers > d.estimate.inliers {
                m
            } else {
                d
            }
        }
        (d, m) => d.or(m)?,
    };
    Some(Alignment {
        transform: best.estimate.transform,
        inliers: best.estimate.inliers,
        matches: best.matches,
        confidence: best.estimate.inliers as f64 / best.matches.max(1) as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn working_size_preserves_aspect() {
        assert_eq!(working_size(1024, 768, 512), (512, 384));
        assert_eq!(working_size(768, 1024, 512), (384, 512));
        assert_eq!(working_size(300, 200, 512), (300, 200));
        assert_eq!(working_size(5000, 3, 512), (512, 1));
        assert_eq!(working_size(100, 100, 0), (100, 100));
    }

    #[test]
    fn featureless_images_do_not_align() {
        let flat = Luma8::filled(200, 150, [128]).unwrap();
        assert_eq!(
            align(
                flat.view(),
                flat.view(),
                &AlignParams::default(),
                &mut SplitMix64::new(1)
            ),
            None
        );
    }
}
