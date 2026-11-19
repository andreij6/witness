mod blur;
mod photometric;
mod structure;
mod warp;

pub use blur::{blur_holds, high_frequency_energy};

use crate::classify::{Assessment, ClassifyParams, GridShape, StateMap, classify};
use crate::geometry::{Rect, Similarity};
use crate::mask::OverlayMask;
use photometric::FitParams;
use structure::WindowRule;
use witness_image::RgbView;

pub const MIN_WINDOW_RADIUS: u32 = 4;
pub const MAX_WINDOW_RADIUS: u32 = 5;
pub const MIN_CHANGE_AREA_PX: u64 = 256;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiffParams {
    pub window_radius: u32,
    pub edge_margin_percent: f64,
    pub edge_margin_min: f64,
    pub min_window_coverage_percent: u32,
    pub structure_threshold: f64,
    pub mean_threshold: f64,
    pub color_threshold: f64,
    pub gain_ridge: f64,
    pub photometric_trim: f64,
    pub photometric_passes: u32,
    pub blur_floor: f64,
    pub blur_ratio: f64,
    pub classify: ClassifyParams,
}

impl Default for DiffParams {
    fn default() -> Self {
        Self {
            window_radius: MIN_WINDOW_RADIUS,
            edge_margin_percent: 1.5,
            edge_margin_min: 2.0,
            min_window_coverage_percent: 60,
            structure_threshold: 0.5,
            mean_threshold: 20.0,
            color_threshold: 40.0,
            gain_ridge: 1e-3,
            photometric_trim: 40.0,
            photometric_passes: 3,
            blur_floor: 3.0,
            blur_ratio: 0.35,
            classify: ClassifyParams {
                min_area_px: MIN_CHANGE_AREA_PX,
                ..ClassifyParams::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlurCheck {
    pub rect: Rect,
    pub energy: Option<f64>,
    pub reference_energy: Option<f64>,
    pub holds: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PixelDiff {
    pub map: StateMap,
    pub overlap_fraction: f64,
    pub blurs: Vec<BlurCheck>,
    pub assessment: Assessment,
}

pub fn diff(
    query: RgbView<'_>,
    reference: RgbView<'_>,
    transform: &Similarity,
    overlay: Option<&OverlayMask>,
    declared_blurs: &[Rect],
    params: &DiffParams,
) -> PixelDiff {
    let (width, height) = reference.dimensions();
    let margin = params
        .edge_margin_min
        .max(query.width().max(query.height()) as f64 * params.edge_margin_percent / 100.0);
    let mut warped = warp::warp(query, transform, (width, height), margin, overlay);
    let fit = FitParams {
        ridge: params.gain_ridge,
        trim: params.photometric_trim,
        passes: params.photometric_passes,
    };
    let model = photometric::fit(&warped.image, &warped.valid, reference, &fit);
    photometric::apply(&mut warped.image, &warped.valid, &model);
    let radius = params
        .window_radius
        .clamp(MIN_WINDOW_RADIUS, MAX_WINDOW_RADIUS);
    let rule = WindowRule {
        radius,
        min_coverage_percent: params.min_window_coverage_percent.min(100),
        structure_threshold: params.structure_threshold,
        mean_threshold: params.mean_threshold,
        color_threshold: params.color_threshold,
    };
    let map = structure::change_map(&warped.image, &warped.valid, reference, &rule);
    let blurs: Vec<BlurCheck> = declared_blurs
        .iter()
        .map(|&rect| {
            let energy = blur::energy_masked(warped.image.view(), Some(&warped.valid), rect);
            let reference_energy = high_frequency_energy(reference, rect);
            let holds = energy.is_some_and(|e| {
                blur_holds(e, reference_energy, params.blur_floor, params.blur_ratio)
            });
            BlurCheck {
                rect,
                energy,
                reference_energy,
                holds,
            }
        })
        .collect();
    let margin = radius + 1;
    let explained: Vec<Rect> = blurs
        .iter()
        .filter(|b| b.holds)
        .map(|b| b.rect.expand(margin, margin, width, height))
        .collect();
    let assessment = classify(
        &map,
        GridShape::for_frame(width, height),
        &explained,
        &params.classify,
    );
    let frame = u64::from(width) * u64::from(height);
    let overlap_fraction = if frame == 0 {
        0.0
    } else {
        warped.overlap as f64 / frame as f64
    };
    PixelDiff {
        map,
        overlap_fraction,
        blurs,
        assessment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{CellState, Classification};
    use witness_image::Rgb8;

    fn scene(w: u32, h: u32, seed: u64) -> Rgb8 {
        let mut s = seed;
        let mut shapes = Vec::new();
        for _ in 0..80 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            shapes.push((
                (s % w as u64) as u32,
                ((s >> 12) % h as u64) as u32,
                4 + ((s >> 24) % 20) as u32,
                4 + ((s >> 32) % 20) as u32,
                [(s >> 40) as u8, (s >> 48) as u8, (s >> 56) as u8],
            ));
        }
        Rgb8::from_fn(w, h, |x, y| {
            let mut p = [
                ((x * 2) % 256) as u8 / 2 + 40,
                ((y * 3) % 256) as u8 / 2 + 40,
                120,
            ];
            for &(sx, sy, sw, sh, c) in &shapes {
                if x >= sx && x < sx + sw && y >= sy && y < sy + sh {
                    p = c;
                }
            }
            p
        })
        .unwrap()
    }

    #[test]
    fn default_params_floor_changes_at_one_codec_block() {
        assert_eq!(
            DiffParams::default().classify.min_area_px,
            MIN_CHANGE_AREA_PX
        );
        assert_eq!(ClassifyParams::default().min_area_px, 0);
        assert_eq!(MIN_CHANGE_AREA_PX, 16 * 16);
    }

    #[test]
    fn identical_copy_is_consistent() {
        let img = scene(160, 120, 1);
        let exact = DiffParams {
            edge_margin_min: 0.0,
            edge_margin_percent: 0.0,
            ..DiffParams::default()
        };
        let d = diff(
            img.view(),
            img.view(),
            &Similarity::IDENTITY,
            None,
            &[],
            &exact,
        );
        assert_eq!(d.assessment.classification, Classification::Consistent);
        assert_eq!(d.assessment.changed_fraction, 0.0);
        assert_eq!(d.map.count(CellState::Agree), 160 * 120);
        let trimmed = diff(
            img.view(),
            img.view(),
            &Similarity::IDENTITY,
            None,
            &[],
            &DiffParams::default(),
        );
        assert_eq!(
            trimmed.assessment.classification,
            Classification::Consistent
        );
        assert!(trimmed.map.count(CellState::Uncompared) >= 160 * 120 - 154 * 114);
        assert_eq!(trimmed.map.count(CellState::Changed), 0);
    }

    #[test]
    fn non_overlapping_transform_cannot_compare() {
        let img = scene(64, 48, 2);
        let t = Similarity::scale_translate(1.0, 1000.0, 0.0);
        let d = diff(
            img.view(),
            img.view(),
            &t,
            None,
            &[],
            &DiffParams::default(),
        );
        assert_eq!(d.assessment.classification, Classification::CouldNotCompare);
        assert_eq!(d.assessment.compared_fraction, 0.0);
        assert_eq!(d.overlap_fraction, 0.0);
    }

    #[test]
    fn overlap_covers_the_untested_border_of_a_full_frame_copy() {
        let img = scene(160, 120, 4);
        let d = diff(
            img.view(),
            img.view(),
            &Similarity::IDENTITY,
            None,
            &[],
            &DiffParams::default(),
        );
        assert!(d.assessment.compared_fraction < 0.95);
        assert_eq!(d.overlap_fraction, 1.0);
    }

    #[test]
    fn overlap_measures_a_crop_not_its_margin() {
        let reference = scene(200, 100, 5);
        let crop = reference.crop(50, 0, 150, 100).unwrap().to_image();
        let t = Similarity::scale_translate(1.0, -50.0, 0.0);
        let d = diff(
            crop.view(),
            reference.view(),
            &t,
            None,
            &[],
            &DiffParams::default(),
        );
        assert_eq!(d.overlap_fraction, 0.75);
        assert!(d.assessment.compared_fraction < 0.7);
        assert_eq!(d.assessment.classification, Classification::Consistent);
    }

    #[test]
    fn window_radius_is_clamped() {
        let img = scene(64, 48, 3);
        let wide = DiffParams {
            window_radius: 50,
            ..DiffParams::default()
        };
        let narrow = DiffParams {
            window_radius: 0,
            ..DiffParams::default()
        };
        assert_eq!(
            diff(
                img.view(),
                img.view(),
                &Similarity::IDENTITY,
                None,
                &[],
                &wide
            )
            .assessment
            .classification,
            Classification::Consistent
        );
        assert_eq!(
            diff(
                img.view(),
                img.view(),
                &Similarity::IDENTITY,
                None,
                &[],
                &narrow
            )
            .assessment
            .classification,
            Classification::Consistent
        );
    }

    #[test]
    fn out_of_frame_blur_rect_does_not_hold() {
        let img = scene(64, 48, 4);
        let d = diff(
            img.view(),
            img.view(),
            &Similarity::IDENTITY,
            None,
            &[Rect::new(500, 500, 10, 10)],
            &DiffParams::default(),
        );
        assert_eq!(d.blurs.len(), 1);
        assert!(!d.blurs[0].holds);
        assert_eq!(d.blurs[0].energy, None);
    }
}
