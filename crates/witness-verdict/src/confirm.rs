use crate::candidates::{Candidate, Pool};
use crate::exact::key_flags;
use crate::options::Options;
use crate::oracle::Oracle;
use crate::query::{Query, working};
use crate::verdict::{Anchor, Evidence, Notes, Q16_ONE, Reason, Reference, Region, Verdict};
use core::cmp::Reverse;
use witness_fingerprint::{Grid, GridDiff, MIN_STABLE_BITS, align as grid_align};
use witness_format::{Dims, Edit, FingerprintBundle, Kind, Outcome, RecordId, VerdictFlags};
use witness_image::{Rgb8, decode_oriented_with_limit, to_luma};
use witness_match::{
    AlignParams, CellState, Classification, ClassifyParams, DiffParams, GridShape, Rect,
    Similarity, SplitMix64, StateMap, align as pixel_align, classify, diff,
};

pub const TAU_PIXEL_Q16: u32 = 19_661;
pub const MIN_COMPARED_Q16: u32 = 19_661;
pub const FULL_COMPARED_Q16: u32 = 49_152;
pub const GRID_CHANGED_DISTANCE: u8 = 128;
pub const FOREIGN_MAX_ERRORS: u8 = 2;
pub const FOREIGN_MIN_DISTANCE: u32 = 90;
pub const SNAP_PERCENT: f64 = 2.0;
pub const SNAP_MIN_PX: f64 = 3.0;
pub const CODEC_MCU_PX: u32 = 16;
pub const UNIT_SCALE_SNAP: f64 = 0.01;
const RNG_SEED: u64 = 0x5749_544E_4553_5331;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Comparison {
    pub index: usize,
    pub evidence: Evidence,
    pub confidence_q16: u32,
    pub compared_q16: u32,
    pub covered_q16: u32,
    pub changed_q16: u32,
    pub explained: u32,
    pub classification: Classification,
    pub frame: (u32, u32),
}

impl Comparison {
    fn fit(&self) -> (u32, u32, Reverse<u32>) {
        (self.changed_q16, self.explained, Reverse(self.compared_q16))
    }

    fn partly_compared(&self) -> bool {
        self.covered_q16 < FULL_COMPARED_Q16
            || u64::from(self.compared_q16) * u64::from(Q16_ONE)
                < u64::from(FULL_COMPARED_Q16) * u64::from(self.covered_q16)
    }

    fn rank(&self, pool: &Pool) -> (u32, u32, Reverse<u32>, u64) {
        (
            self.changed_q16,
            self.explained,
            Reverse(self.confidence_q16),
            pool.candidates[self.index].record.record_ix,
        )
    }
}

pub(crate) fn changed_area_q16(
    classification: &Classification,
    changed_fraction: f64,
    frame: (u32, u32),
) -> u32 {
    match classification {
        Classification::Consistent => 0,
        Classification::Local(components) => {
            let area: u64 = components.iter().map(|c| c.area).sum();
            let frame = u64::from(frame.0) * u64::from(frame.1);
            (area.saturating_mul(u64::from(Q16_ONE)) / frame.max(1)).min(u64::from(Q16_ONE)) as u32
        }
        Classification::CouldNotCompare | Classification::Inconclusive(_) => q16(changed_fraction),
    }
}

pub(crate) fn q16(v: f64) -> u32 {
    if v.is_nan() {
        0
    } else {
        (v.clamp(0.0, 1.0) * f64::from(Q16_ONE)) as u32
    }
}

fn scale(v: u32, from: u32, to: u32) -> u32 {
    (u64::from(v) * u64::from(to) / u64::from(from.max(1))) as u32
}

pub(crate) fn declared_blurs(
    pool: &Pool,
    parent: &RecordId,
    dims: Dims,
    frame: (u32, u32),
) -> Vec<Rect> {
    pool.candidates
        .iter()
        .filter_map(|c| match &c.record.claim.kind {
            Kind::Derivative {
                parent: p, edits, ..
            } if p == parent => Some(edits),
            _ => None,
        })
        .flatten()
        .filter_map(|e| match e {
            Edit::Blur { rect, .. } => {
                let x = scale(rect.x, dims.width, frame.0);
                let y = scale(rect.y, dims.height, frame.1);
                let w = scale(rect.w, dims.width, frame.0).max(1);
                let h = scale(rect.h, dims.height, frame.1).max(1);
                Rect::new(x, y, w, h).clamp_to(frame.0, frame.1)
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn snap_px(width: u32, height: u32) -> f64 {
    (f64::from(width.max(height)) * SNAP_PERCENT / 100.0).max(SNAP_MIN_PX)
}

pub(crate) fn upright(aligned: Similarity, reference: (u32, u32)) -> Option<Similarity> {
    if aligned.mirrored || !aligned.is_finite() {
        return None;
    }
    let (cx, cy) = (f64::from(reference.0) / 2.0, f64::from(reference.1) / 2.0);
    let (cu, cv) = aligned.apply(cx, cy);
    let scale = aligned.scale();
    let scale = if (scale - 1.0).abs() <= UNIT_SCALE_SNAP {
        1.0
    } else {
        scale
    };
    let snapped = Similarity::scale_translate(scale, cu - scale * cx, cv - scale * cy);
    (snapped != aligned
        && snapped.max_corner_shift(&aligned, reference.0, reference.1)
            <= snap_px(reference.0, reference.1))
    .then_some(snapped)
}

pub(crate) fn hypotheses(
    aligned: Similarity,
    reference: (u32, u32),
    view: (u32, u32),
) -> impl Iterator<Item = Similarity> {
    let resize = Similarity::resize(reference, view);
    let near = resize.is_some_and(|r| {
        r.max_corner_shift(&aligned, reference.0, reference.1) <= snap_px(reference.0, reference.1)
    });
    let (first, last) = if near { (resize, None) } else { (None, resize) };
    first
        .into_iter()
        .chain(core::iter::once(aligned))
        .chain(last)
}

fn by_pixels(
    query: &Query,
    index: usize,
    reference: &Rgb8,
    blurs: &[Rect],
    seed: u64,
) -> Option<Comparison> {
    let reference_luma = to_luma(reference.view());
    let frame = reference.dimensions();
    for view in query.views() {
        let luma = query
            .luma
            .crop(view.x, view.y, view.width, view.height)
            .ok()?;
        let rgb = query
            .image
            .crop(view.x, view.y, view.width, view.height)
            .ok()?;
        let Some(a) = pixel_align(
            luma,
            reference_luma.view(),
            &AlignParams::default(),
            &mut SplitMix64::new(seed),
        ) else {
            continue;
        };
        let confidence_q16 = q16(a.confidence);
        if confidence_q16 < TAU_PIXEL_Q16 {
            continue;
        }
        let overlay = query
            .overlay
            .relative_to(f64::from(view.x), f64::from(view.y));
        let measure = |t: &Similarity| {
            let d = diff(
                rgb,
                reference.view(),
                t,
                Some(&overlay),
                blurs,
                &DiffParams::default(),
            );
            let c = Comparison {
                index,
                evidence: Evidence::Pixels,
                confidence_q16,
                compared_q16: q16(d.assessment.compared_fraction),
                covered_q16: q16(d.overlap_fraction),
                changed_q16: changed_area_q16(
                    &d.assessment.classification,
                    d.assessment.changed_fraction,
                    frame,
                ),
                explained: d.assessment.explained,
                classification: d.assessment.classification,
                frame,
            };
            (c.compared_q16 >= MIN_COMPARED_Q16).then_some(c)
        };
        let mut best: Option<Comparison> = None;
        for t in hypotheses(a.transform, frame, (view.width, view.height)) {
            let Some(c) = measure(&t) else {
                continue;
            };
            let settled = c.classification == Classification::Consistent;
            if best.as_ref().is_none_or(|b| c.fit() < b.fit()) {
                best = Some(c);
            }
            if settled {
                break;
            }
        }
        let unsettled = best
            .as_ref()
            .is_some_and(|b| b.classification != Classification::Consistent);
        if let Some(c) = upright(a.transform, frame)
            .filter(|_| unsettled)
            .and_then(|u| measure(&u))
            .filter(|c| c.classification == Classification::Consistent)
        {
            best = Some(c);
        }
        if best.is_some() {
            return best;
        }
    }
    None
}

pub(crate) fn grid_coverage_q16(reference: &Grid, diff: &GridDiff) -> u32 {
    let comparable = reference
        .cells()
        .iter()
        .enumerate()
        .filter(|&(i, &c)| Grid::stable(c).count_ones() >= MIN_STABLE_BITS || diff.is_compared(i))
        .count() as u64;
    let compared = u64::from(diff.compared_cells());
    ((compared << 16) / comparable.max(1)).min(u64::from(Q16_ONE)) as u32
}

pub(crate) fn grid_min_change_cells(
    shape: GridShape,
    original: (u32, u32),
    copy: (u32, u32),
) -> u32 {
    let side = |(w, h): (u32, u32)| (w / shape.cols.max(1)).min(h / shape.rows.max(1));
    let cell = side(original).min(side(copy)).max(1);
    let span = CODEC_MCU_PX.div_ceil(cell);
    span * span
}

fn by_grid(
    query: &Query,
    index: usize,
    bundle: &FingerprintBundle,
    dims: Dims,
) -> Option<Comparison> {
    let grid = Grid::from_bundle(bundle);
    let a = grid_align(&grid, (dims.width, dims.height), query.luma.view())?;
    let compared_q16 = grid_coverage_q16(&grid, &a.diff);
    if !a.accepted || compared_q16 < MIN_COMPARED_Q16 {
        return None;
    }
    let (cols, rows) = bundle.grid_dims();
    let (cols, rows) = (cols as u32, rows as u32);
    let shape = GridShape { cols, rows };
    let map = StateMap::from_fn(cols, rows, |x, y| {
        match a.diff.distance((y * cols + x) as usize) {
            None => CellState::Uncompared,
            Some(d) if d > GRID_CHANGED_DISTANCE => CellState::Changed,
            Some(_) => CellState::Agree,
        }
    });
    let params = ClassifyParams {
        min_cell_fraction: f64::from(grid_min_change_cells(
            shape,
            (dims.width, dims.height),
            query.luma.dimensions(),
        )),
        ..ClassifyParams::default()
    };
    let assessment = classify(&map, shape, &[], &params);
    Some(Comparison {
        index,
        evidence: Evidence::Grid,
        confidence_q16: a.best.agreement_q16,
        compared_q16,
        covered_q16: compared_q16,
        changed_q16: changed_area_q16(
            &assessment.classification,
            assessment.changed_fraction,
            (cols, rows),
        ),
        explained: assessment.explained,
        classification: assessment.classification,
        frame: (cols, rows),
    })
}

fn compare<O: Oracle + ?Sized>(
    query: &Query,
    pool: &Pool,
    index: usize,
    oracle: &O,
    max_pixels: u64,
) -> Option<Comparison> {
    let Candidate { record, bundle, .. } = &pool.candidates[index];
    let reference = oracle
        .reference_image(record)
        .and_then(|bytes| decode_oriented_with_limit(&bytes, max_pixels).ok())
        .map(working);
    if let Some(reference) = reference {
        let blurs = declared_blurs(
            pool,
            &record.record_id,
            record.claim.dims,
            reference.dimensions(),
        );
        if let Some(c) = by_pixels(
            query,
            index,
            &reference,
            &blurs,
            RNG_SEED ^ record.record_ix,
        ) {
            return Some(c);
        }
    }
    bundle
        .as_ref()
        .and_then(|b| by_grid(query, index, b, record.claim.dims))
}

fn unconfirmed(pool: &Pool, mut notes: Notes) -> Verdict {
    let foreign = pool
        .watermark
        .is_some_and(|d| d.errors <= FOREIGN_MAX_ERRORS)
        && pool
            .candidates
            .iter()
            .all(|c| c.distance > FOREIGN_MIN_DISTANCE);
    notes.reason(if foreign {
        Reason::ForeignWatermark
    } else {
        Reason::Unconfirmed
    });
    notes.no_record()
}

pub(crate) fn decide<O: Oracle + ?Sized>(
    query: &Query,
    pool: &Pool,
    oracle: &O,
    options: &Options<'_>,
    mut notes: Notes,
) -> Verdict {
    let best = (0..pool.candidates.len().min(options.k()))
        .filter_map(|i| compare(query, pool, i, oracle, options.max_pixels))
        .min_by_key(|c| c.rank(pool));
    let Some(best) = best else {
        return unconfirmed(pool, notes);
    };
    let record = &pool.candidates[best.index].record;
    key_flags(oracle, record, &mut notes);
    if best.evidence == Evidence::Grid {
        notes.flag(VerdictFlags::NO_PUBLIC_REFERENCE);
    }
    if best.partly_compared() {
        notes.flag(VerdictFlags::COULDNT_COMPARE);
    }
    let regions: Vec<Region> = match &best.classification {
        Classification::Local(components) => components
            .iter()
            .filter_map(|c| {
                Region::normalized(c.rect.x, c.rect.y, c.rect.width, c.rect.height, best.frame)
            })
            .collect(),
        Classification::Consistent => Vec::new(),
        Classification::CouldNotCompare | Classification::Inconclusive(_) => {
            notes.flag(VerdictFlags::COULDNT_COMPARE);
            Vec::new()
        }
    };
    let outcome = if regions.is_empty() {
        Outcome::MatchesWithChanges
    } else {
        Outcome::Altered
    };
    let reference = Reference {
        record_id: record.record_id,
        claim: record.claim.clone(),
        anchor: Some(Anchor {
            record_ix: record.record_ix,
            anchored_ns: record.anchored_ns,
        }),
        evidence: best.evidence,
        confidence_q16: best.confidence_q16,
        compared_q16: best.covered_q16,
        changed_q16: best.changed_q16,
    };
    let mut verdict = notes.verdict(outcome, Some(reference));
    verdict.regions = regions;
    verdict
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{MemoryOracle, TestCa};
    use witness_format::{BlurClass, Rect as FRect};

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn q16_clamps_and_handles_nan() {
        assert_eq!(q16(f64::NAN), 0);
        assert_eq!(q16(-1.0), 0);
        assert_eq!(q16(0.0), 0);
        assert_eq!(q16(0.5), 32768);
        assert_eq!(q16(1.0), Q16_ONE);
        assert_eq!(q16(7.0), Q16_ONE);
        assert_eq!(q16(f64::INFINITY), Q16_ONE);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn changed_area_ignores_noise_and_measures_components() {
        use witness_match::{Component, Inconclusive};
        assert_eq!(
            changed_area_q16(&Classification::Consistent, 0.3, (10, 10)),
            0
        );
        let local = Classification::Local(vec![
            Component {
                rect: Rect::new(0, 0, 5, 5),
                area: 25,
            },
            Component {
                rect: Rect::new(5, 5, 5, 5),
                area: 25,
            },
        ]);
        assert_eq!(changed_area_q16(&local, 0.9, (10, 10)), Q16_ONE / 2);
        assert_eq!(changed_area_q16(&local, 0.9, (0, 0)), Q16_ONE);
        assert_eq!(
            changed_area_q16(
                &Classification::Inconclusive(Inconclusive::NotRinged),
                0.25,
                (10, 10)
            ),
            Q16_ONE / 4
        );
        assert_eq!(
            changed_area_q16(&Classification::CouldNotCompare, 0.5, (10, 10)),
            Q16_ONE / 2
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn snap_tolerance_scales_with_frame_and_has_a_floor() {
        assert_eq!(snap_px(1000, 500), 20.0);
        assert_eq!(snap_px(500, 1000), 20.0);
        assert_eq!(snap_px(96, 72), SNAP_MIN_PX);
        assert_eq!(snap_px(0, 0), SNAP_MIN_PX);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn hypotheses_offer_resize_first_when_alignment_is_near() {
        let near = Similarity::new(1.0003, 0.0001, -0.05, 0.02, false);
        let h: Vec<_> = hypotheses(near, (192, 144), (192, 144)).collect();
        assert_eq!(h, vec![Similarity::IDENTITY, near]);
        let half = Similarity::scale_translate(0.5, 0.4, -0.3);
        let h: Vec<_> = hypotheses(half, (1024, 768), (512, 384)).collect();
        assert_eq!(h, vec![Similarity::scale_translate(0.5, 0.0, 0.0), half]);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn hypotheses_try_resize_last_for_far_alignments_and_never_for_reshaped_views() {
        let shifted = Similarity::scale_translate(1.0, 12.0, 0.0);
        assert_eq!(
            hypotheses(shifted, (192, 144), (192, 144)).collect::<Vec<_>>(),
            vec![shifted, Similarity::IDENTITY]
        );
        let tilted = Similarity::new(0.985, -0.055, -1.6, 3.1, false);
        assert_eq!(
            hypotheses(tilted, (96, 72), (96, 72)).collect::<Vec<_>>(),
            vec![tilted, Similarity::IDENTITY]
        );
        let near = Similarity::IDENTITY;
        assert_eq!(
            hypotheses(near, (192, 144), (150, 144)).collect::<Vec<_>>(),
            vec![near]
        );
        let mirrored = Similarity::IDENTITY.mirror_x(192.0);
        assert_eq!(
            hypotheses(mirrored, (192, 144), (192, 144)).collect::<Vec<_>>(),
            vec![mirrored, Similarity::IDENTITY]
        );
        let broken = Similarity::new(f64::NAN, 0.0, 0.0, 0.0, false);
        let h: Vec<_> = hypotheses(broken, (192, 144), (192, 144)).collect();
        assert_eq!(h.len(), 2);
        assert_eq!(h[1], Similarity::IDENTITY);
        assert_eq!(hypotheses(near, (0, 0), (192, 144)).count(), 1);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn upright_snaps_near_unit_drift_to_a_pure_translation_through_the_center() {
        let drift = Similarity::new(1.003, 0.00165, -52.5, -2.38, false);
        let u = upright(drift, (1024, 768)).unwrap();
        assert_eq!((u.a, u.b, u.mirrored), (1.0, 0.0, false));
        let (cu, cv) = drift.apply(512.0, 384.0);
        assert_eq!(u.apply(512.0, 384.0), (cu, cv));
        assert!(
            (u.tx + 51.6).abs() < 0.1 && (u.ty + 0.4).abs() < 0.1,
            "{u:?}"
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn upright_keeps_a_real_rescale_and_drops_only_rotation() {
        let tilted = Similarity::new(0.75 * 0.99999, 0.75 * 0.004, 3.0, -2.0, false);
        let u = upright(tilted, (640, 480)).unwrap();
        assert_eq!(u.b, 0.0);
        assert!((u.a - tilted.scale()).abs() < 1e-12);
        assert!(u.max_corner_shift(&tilted, 640, 480) <= snap_px(640, 480));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn upright_declines_exact_mirrored_broken_and_far_placements() {
        assert_eq!(upright(Similarity::IDENTITY, (640, 480)), None);
        let shifted = Similarity::scale_translate(1.0, -40.0, 3.0);
        assert_eq!(upright(shifted, (640, 480)), None);
        assert_eq!(
            upright(Similarity::IDENTITY.mirror_x(640.0), (640, 480)),
            None
        );
        let broken = Similarity::new(f64::NAN, 0.0, 0.0, 0.0, false);
        assert_eq!(upright(broken, (640, 480)), None);
        let rotated = Similarity::new(0.966, 0.259, 0.0, 0.0, false);
        assert_eq!(upright(rotated, (640, 480)), None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn upright_is_deterministic() {
        let t = Similarity::new(0.9947, -0.0021, 0.46, 1.61, false);
        assert_eq!(upright(t, (1024, 1536)), upright(t, (1024, 1536)));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn hypotheses_are_deterministic() {
        let t = Similarity::new(0.999, -0.003, -1.8, 2.0, false);
        let a: Vec<_> = hypotheses(t, (1024, 768), (1024, 768)).collect();
        let b: Vec<_> = hypotheses(t, (1024, 768), (1024, 768)).collect();
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn fit_prefers_fewer_changes_then_fewer_explained_then_more_compared() {
        let base = Comparison {
            index: 0,
            evidence: Evidence::Pixels,
            confidence_q16: 1,
            compared_q16: 50_000,
            covered_q16: 50_000,
            changed_q16: 10,
            explained: 1,
            classification: Classification::Consistent,
            frame: (1, 1),
        };
        let fewer = Comparison {
            changed_q16: 0,
            ..base.clone()
        };
        let wider = Comparison {
            compared_q16: 60_000,
            ..base.clone()
        };
        let unexplained = Comparison {
            explained: 0,
            ..base.clone()
        };
        assert!(fewer.fit() < base.fit());
        assert!(wider.fit() < base.fit());
        assert!(unexplained.fit() < base.fit());
        assert_eq!(base.fit(), base.clone().fit());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn partly_compared_needs_a_crop_or_an_unmeasured_overlap() {
        let c = |compared_q16, covered_q16| Comparison {
            index: 0,
            evidence: Evidence::Pixels,
            confidence_q16: 1,
            compared_q16,
            covered_q16,
            changed_q16: 0,
            explained: 0,
            classification: Classification::Consistent,
            frame: (1, 1),
        };
        assert!(!c(58_000, Q16_ONE).partly_compared());
        assert!(!c(FULL_COMPARED_Q16, Q16_ONE).partly_compared());
        assert!(c(FULL_COMPARED_Q16 - 1, Q16_ONE).partly_compared());
        assert!(!c(45_000, 52_000).partly_compared());
        assert!(c(FULL_COMPARED_Q16 - 1, FULL_COMPARED_Q16 - 1).partly_compared());
        assert!(c(30_000, 52_000).partly_compared());
        assert!(c(0, 0).partly_compared());
        assert!(!c(Q16_ONE, Q16_ONE).partly_compared());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn scale_is_safe_for_zero_source() {
        assert_eq!(scale(10, 0, 100), 1000);
        assert_eq!(scale(10, 20, 10), 5);
        assert_eq!(scale(u32::MAX, u32::MAX, u32::MAX), u32::MAX);
    }

    const LANDSCAPE: GridShape = GridShape { cols: 16, rows: 12 };

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn grid_change_floor_is_one_cell_once_cells_cover_a_codec_mcu() {
        assert_eq!(grid_min_change_cells(LANDSCAPE, (640, 480), (640, 480)), 1);
        assert_eq!(grid_min_change_cells(LANDSCAPE, (256, 192), (256, 192)), 1);
        assert_eq!(
            grid_min_change_cells(LANDSCAPE, (4032, 3024), (2048, 1536)),
            1
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn grid_change_floor_grows_with_the_mcu_footprint_of_small_cells() {
        assert_eq!(grid_min_change_cells(LANDSCAPE, (240, 180), (240, 180)), 4);
        assert_eq!(grid_min_change_cells(LANDSCAPE, (160, 120), (160, 120)), 4);
        assert_eq!(grid_min_change_cells(LANDSCAPE, (96, 72), (96, 72)), 9);
        assert_eq!(grid_min_change_cells(LANDSCAPE, (64, 48), (64, 48)), 16);
        assert_eq!(grid_min_change_cells(LANDSCAPE, (48, 36), (48, 36)), 36);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn grid_change_floor_uses_the_smaller_cell_of_original_and_copy() {
        assert_eq!(grid_min_change_cells(LANDSCAPE, (640, 480), (160, 120)), 4);
        assert_eq!(grid_min_change_cells(LANDSCAPE, (96, 72), (1024, 768)), 9);
        assert_eq!(grid_min_change_cells(LANDSCAPE, (640, 160), (640, 160)), 4);
        let portrait = GridShape { cols: 12, rows: 16 };
        assert_eq!(grid_min_change_cells(portrait, (480, 640), (480, 640)), 1);
        assert_eq!(grid_min_change_cells(portrait, (72, 96), (72, 96)), 9);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn grid_change_floor_survives_degenerate_inputs() {
        let max = CODEC_MCU_PX * CODEC_MCU_PX;
        assert_eq!(grid_min_change_cells(LANDSCAPE, (0, 0), (640, 480)), max);
        assert_eq!(grid_min_change_cells(LANDSCAPE, (640, 480), (0, 0)), max);
        assert_eq!(
            grid_min_change_cells(GridShape { cols: 0, rows: 0 }, (8, 8), (8, 8)),
            4
        );
        assert_eq!(
            grid_min_change_cells(LANDSCAPE, (u32::MAX, u32::MAX), (u32::MAX, u32::MAX)),
            1
        );
    }

    fn textured_grid(textured: impl Fn(usize) -> bool) -> Grid {
        let mut cells = [0u16; witness_format::fingerprint::GRID_CELLS];
        for (i, cell) in cells.iter_mut().enumerate() {
            if textured(i) {
                *cell = Grid::pack((i as u8).wrapping_mul(37), 0xFF);
            }
        }
        Grid::new(cells, false)
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn grid_coverage_ignores_cells_too_flat_to_compare() {
        let reference = textured_grid(|i| i % 3 != 0);
        let diff = witness_fingerprint::compare(&reference, &reference).unwrap();
        assert_eq!(diff.compared_cells(), 128);
        assert!(diff.compared_fraction_q16() < FULL_COMPARED_Q16);
        assert_eq!(grid_coverage_q16(&reference, &diff), Q16_ONE);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn grid_coverage_counts_texture_the_copy_added_without_exceeding_one() {
        let reference = textured_grid(|i| i < 96);
        let copy = textured_grid(|_| true);
        let diff = witness_fingerprint::compare(&reference, &copy).unwrap();
        assert_eq!(diff.compared_cells(), 192);
        assert_eq!(grid_coverage_q16(&reference, &diff), Q16_ONE);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn grid_coverage_is_zero_when_nothing_is_comparable() {
        let flat = textured_grid(|_| false);
        let diff = witness_fingerprint::compare(&flat, &flat).unwrap();
        assert_eq!(grid_coverage_q16(&flat, &diff), 0);
    }

    fn blur(x: u32, y: u32, w: u32, h: u32) -> Edit {
        Edit::Blur {
            rect: FRect { x, y, w, h },
            class: BlurClass::Face,
            sigma_e2: 500,
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn declared_blurs_come_only_from_children_and_scale_to_reference() {
        let ca = TestCa::new(1);
        let dev = ca.device(2, 2);
        let mut oracle = MemoryOracle::new();
        let parent = oracle.anchor(dev.claim(1, Kind::Original, 1), 1);
        let stranger = oracle.anchor(dev.claim(2, Kind::Original, 2), 2);
        let child = |p: &crate::oracle::Record, edits| Kind::Derivative {
            parent: p.record_id,
            root: p.claim.capture_id,
            edits,
        };
        let a = oracle.anchor(
            dev.claim(
                3,
                child(
                    &parent,
                    vec![blur(10, 10, 20, 10), Edit::Rotate { deg_e2: 5 }],
                ),
                3,
            ),
            3,
        );
        let b = oracle.anchor(dev.claim(4, child(&stranger, vec![blur(0, 0, 5, 5)]), 4), 4);
        let c = oracle.anchor(
            dev.claim(5, child(&parent, vec![blur(60, 40, 100, 100)]), 5),
            5,
        );
        let pool = Pool {
            candidates: [parent.clone(), stranger, a, b, c]
                .into_iter()
                .map(|record| Candidate {
                    record,
                    bundle: None,
                    distance: 0,
                })
                .collect(),
            watermark: None,
        };
        let dims = Dims {
            width: 64,
            height: 48,
        };
        let rects = declared_blurs(&pool, &parent.record_id, dims, (128, 96));
        assert_eq!(
            rects,
            vec![Rect::new(20, 20, 40, 20), Rect::new(120, 80, 8, 16)]
        );
        assert!(declared_blurs(&pool, &parent.record_id, dims, (0, 0)).is_empty());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn unconfirmed_distinguishes_foreign_watermark() {
        let ca = TestCa::new(1);
        let dev = ca.device(2, 2);
        let mut oracle = MemoryOracle::new();
        let r = oracle.anchor(dev.claim(1, Kind::Original, 1), 1);
        let decoded = |errors| witness_watermark::Decoded {
            capture_id: r.claim.capture_id,
            errors,
            erasures: 0,
        };
        let pool = |distance, watermark| Pool {
            candidates: vec![Candidate {
                record: r.clone(),
                bundle: None,
                distance,
            }],
            watermark,
        };
        let reason = |p: &Pool| unconfirmed(p, Notes::default()).reasons;
        assert!(reason(&pool(91, Some(decoded(2)))).contains(Reason::ForeignWatermark));
        assert!(reason(&pool(90, Some(decoded(0)))).contains(Reason::Unconfirmed));
        assert!(reason(&pool(200, Some(decoded(3)))).contains(Reason::Unconfirmed));
        assert!(reason(&pool(200, None)).contains(Reason::Unconfirmed));
        assert_eq!(
            unconfirmed(&pool(200, None), Notes::default()).outcome,
            Outcome::NoRecord
        );
    }
}
