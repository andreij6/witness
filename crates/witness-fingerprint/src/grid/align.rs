use super::compare::{GridDiff, compare_present};
use super::sampler::{Frame, Q16, Sampler};
use super::{Grid, grid_dims};
use crate::shape::{Anchor, Aspect, ShapeTag, stored_shape_tags};
use witness_image::LumaView;

pub const ALIGN_LONG_EDGE: u32 = 512;
pub const ALIGN_SCALES_PERCENT: [u8; 3] = [90, 100, 110];
pub const TAU_ALIGN_Q16: u32 = 49_152;
pub const ALIGN_MARGIN_Q16: u32 = 6_554;
pub const MIN_ALIGN_CELLS: u32 = 48;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hypothesis {
    pub tag: ShapeTag,
    pub scale_percent: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scored {
    pub hypothesis: Hypothesis,
    pub agreement_q16: u32,
    pub compared_cells: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alignment {
    pub best: Scored,
    pub runner_up: Option<Scored>,
    pub diff: GridDiff,
    pub accepted: bool,
}

pub fn hypotheses(width: u32, height: u32) -> Vec<Hypothesis> {
    stored_shape_tags(width, height)
        .into_iter()
        .flat_map(|tag| ALIGN_SCALES_PERCENT.map(|scale_percent| Hypothesis { tag, scale_percent }))
        .collect()
}

fn scaled_axis(start: i64, len: i64, percent: i64, anchor: Anchor) -> (i64, i64) {
    let scaled = len * percent / 100;
    let start = match anchor {
        Anchor::Start => start,
        Anchor::Center => start + (len - scaled) / 2,
        Anchor::End => start + len - scaled,
    };
    (start, scaled)
}

pub(crate) fn frame_in_copy(
    hypothesis: Hypothesis,
    original: (u32, u32),
    copy: (usize, usize),
) -> Option<Frame> {
    let placement = hypothesis.tag.placement(original.0, original.1)?;
    let r = placement.rect;
    let percent = hypothesis.scale_percent as i64;
    let full = hypothesis.tag.aspect() == Aspect::Full;
    let anchor_x = if placement.horizontal && !full {
        hypothesis.tag.anchor()
    } else {
        Anchor::Center
    };
    let anchor_y = if !placement.horizontal && !full {
        hypothesis.tag.anchor()
    } else {
        Anchor::Center
    };
    let (rx, rw) = scaled_axis(r.x as i64 * Q16, r.w as i64 * Q16, percent, anchor_x);
    let (ry, rh) = scaled_axis(r.y as i64 * Q16, r.h as i64 * Q16, percent, anchor_y);
    if rw <= 0 || rh <= 0 {
        return None;
    }
    let map = |offset: i64, len: i64, total: u32, copy_len: usize| -> Option<(i64, i64)> {
        let scale = copy_len as i128 * Q16 as i128;
        let origin = (-(offset as i128) * scale).div_euclid(len as i128);
        let extent = ((total as i128 * Q16 as i128) * scale).div_euclid(len as i128);
        Some((i64::try_from(origin).ok()?, i64::try_from(extent).ok()?))
    };
    let (x, w) = map(rx, rw, original.0, copy.0)?;
    let (y, h) = map(ry, rh, original.1, copy.1)?;
    Some(Frame { x, y, w, h })
}

fn distinct(a: &Frame, b: &Frame, (cols, rows): (usize, usize)) -> bool {
    let shift = |da: i64, db: i64, len: i64, cells: usize| (da - db).abs() * 2 * cells as i64 > len;
    let resize = |la: i64, lb: i64| (la - lb).abs() * 20 > la.max(lb);
    shift(a.x, b.x, a.w, cols) || shift(a.y, b.y, a.h, rows) || resize(a.w, b.w) || resize(a.h, b.h)
}

fn better(a: &Scored, b: &Scored) -> bool {
    (
        a.agreement_q16,
        a.compared_cells,
        core::cmp::Reverse(a.hypothesis),
    ) > (
        b.agreement_q16,
        b.compared_cells,
        core::cmp::Reverse(b.hypothesis),
    )
}

pub fn align(reference: &Grid, original: (u32, u32), copy: LumaView<'_>) -> Option<Alignment> {
    let (width, height) = original;
    if width == 0
        || height == 0
        || grid_dims(reference.portrait()) != grid_dims(super::is_portrait(width, height))
    {
        return None;
    }
    let sampler = Sampler::new(copy, ALIGN_LONG_EDGE);
    let mut best: Option<(Scored, GridDiff, Frame)> = None;
    let mut scores: Vec<(Scored, Frame)> = Vec::new();
    for hypothesis in hypotheses(width, height) {
        let Some(frame) = frame_in_copy(hypothesis, original, sampler.dimensions()) else {
            continue;
        };
        let (regridded, present) = sampler.grid(frame, reference.portrait());
        let diff = compare_present(reference, &regridded, &present);
        let compared_cells = diff.compared_cells();
        if compared_cells < MIN_ALIGN_CELLS {
            continue;
        }
        let scored = Scored {
            hypothesis,
            agreement_q16: diff.agreement_q16(),
            compared_cells,
        };
        scores.push((scored, frame));
        if best
            .as_ref()
            .is_none_or(|current| better(&scored, &current.0))
        {
            best = Some((scored, diff, frame));
        }
    }
    let (best, diff, best_frame) = best?;
    let dims = grid_dims(reference.portrait());
    let runner_up = scores
        .iter()
        .filter(|(_, frame)| distinct(frame, &best_frame, dims))
        .map(|(scored, _)| *scored)
        .max_by_key(|s| {
            (
                s.agreement_q16,
                s.compared_cells,
                core::cmp::Reverse(s.hypothesis),
            )
        });
    let margin = best
        .agreement_q16
        .saturating_sub(runner_up.map_or(0, |r| r.agreement_q16));
    Some(Alignment {
        best,
        runner_up,
        diff,
        accepted: best.agreement_q16 >= TAU_ALIGN_Q16 && margin >= ALIGN_MARGIN_Q16,
    })
}
