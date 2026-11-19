mod components;
mod map;

pub use components::{Component, components};
pub use map::{CellState, ChangeMap, GridShape, StateMap};

use crate::geometry::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClassifyParams {
    pub max_changed_fraction: f64,
    pub min_cell_fraction: f64,
    pub min_frame_fraction: f64,
    pub max_components: u32,
    pub ring_cells: f64,
    pub min_ring_agreement: f64,
    pub border_cells: f64,
    pub min_area_px: u64,
}

impl Default for ClassifyParams {
    fn default() -> Self {
        Self {
            max_changed_fraction: 0.35,
            min_cell_fraction: 0.5,
            min_frame_fraction: 0.002,
            max_components: 6,
            ring_cells: 0.5,
            min_ring_agreement: 0.8,
            border_cells: 1.0,
            min_area_px: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inconclusive {
    TooManyComponents(u32),
    InBorderBand,
    NotRinged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Classification {
    CouldNotCompare,
    Consistent,
    Inconclusive(Inconclusive),
    Local(Vec<Component>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Assessment {
    pub compared_fraction: f64,
    pub changed_fraction: f64,
    pub explained: u32,
    pub classification: Classification,
}

struct Census {
    agree: u64,
    changed: u64,
    extent: Option<Rect>,
}

fn census<M: ChangeMap>(map: &M) -> Census {
    let (mut agree, mut changed) = (0u64, 0u64);
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for y in 0..map.height() {
        let mut row_compared = false;
        for x in 0..map.width() {
            match map.state(x, y) {
                CellState::Uncompared => continue,
                CellState::Agree => agree += 1,
                CellState::Changed => changed += 1,
            }
            row_compared = true;
            x0 = x0.min(x);
            x1 = x1.max(x + 1);
        }
        if row_compared {
            y0 = y0.min(y);
            y1 = y + 1;
        }
    }
    let extent = (x1 > x0 && y1 > y0).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0));
    Census {
        agree,
        changed,
        extent,
    }
}

fn band(cells: f64, size: f64) -> u32 {
    let v = (cells * size).round();
    if v > 0.0 { v as u32 } else { 0 }
}

fn in_border_band(rect: &Rect, extent: &Rect, bx: u32, by: u32) -> bool {
    rect.x < extent.x.saturating_add(bx)
        || rect.y < extent.y.saturating_add(by)
        || rect.right() > extent.right().saturating_sub(bx)
        || rect.bottom() > extent.bottom().saturating_sub(by)
}

fn ring_agreement<M: ChangeMap>(map: &M, rect: &Rect, rx: u32, ry: u32) -> f64 {
    let outer = rect.expand(rx, ry, map.width(), map.height());
    let (mut agree, mut total) = (0u64, 0u64);
    for y in outer.y..outer.bottom() {
        let inside_rows = y >= rect.y && y < rect.bottom();
        let mut x = outer.x;
        while x < outer.right() {
            if inside_rows && x == rect.x {
                x = rect.right();
                continue;
            }
            agree += (map.state(x, y) == CellState::Agree) as u64;
            total += 1;
            x += 1;
        }
    }
    if total == 0 {
        0.0
    } else {
        agree as f64 / total as f64
    }
}

pub fn classify<M: ChangeMap>(
    map: &M,
    grid: GridShape,
    explained: &[Rect],
    params: &ClassifyParams,
) -> Assessment {
    let frame = map.width() as u64 * map.height() as u64;
    let census = census(map);
    let compared = census.agree + census.changed;
    let compared_fraction = if frame == 0 {
        0.0
    } else {
        compared as f64 / frame as f64
    };
    let changed_fraction = if compared == 0 {
        0.0
    } else {
        census.changed as f64 / compared as f64
    };
    let assessment = |explained: u32, classification| Assessment {
        compared_fraction,
        changed_fraction,
        explained,
        classification,
    };
    let Some(extent) = census.extent else {
        return assessment(0, Classification::CouldNotCompare);
    };
    if changed_fraction > params.max_changed_fraction {
        return assessment(0, Classification::CouldNotCompare);
    }
    let (cell_w, cell_h) = grid.cell_size(map.width(), map.height());
    let min_area = (params.min_cell_fraction * cell_w * cell_h)
        .max(params.min_frame_fraction * frame as f64)
        .max(params.min_area_px as f64);
    let mut explained_count = 0;
    let mut significant: Vec<Component> = Vec::new();
    for c in components(map) {
        if (c.area as f64) < min_area {
            continue;
        }
        if explained.iter().any(|e| e.contains_rect(&c.rect)) {
            explained_count += 1;
            continue;
        }
        significant.push(c);
    }
    if significant.is_empty() {
        return assessment(explained_count, Classification::Consistent);
    }
    if significant.len() > params.max_components as usize {
        return assessment(
            explained_count,
            Classification::Inconclusive(Inconclusive::TooManyComponents(significant.len() as u32)),
        );
    }
    let (bx, by) = (
        band(params.border_cells, cell_w),
        band(params.border_cells, cell_h),
    );
    if significant
        .iter()
        .any(|c| in_border_band(&c.rect, &extent, bx, by))
    {
        return assessment(
            explained_count,
            Classification::Inconclusive(Inconclusive::InBorderBand),
        );
    }
    let (rx, ry) = (
        band(params.ring_cells, cell_w).max(1),
        band(params.ring_cells, cell_h).max(1),
    );
    if significant
        .iter()
        .any(|c| ring_agreement(map, &c.rect, rx, ry) < params.min_ring_agreement)
    {
        return assessment(
            explained_count,
            Classification::Inconclusive(Inconclusive::NotRinged),
        );
    }
    assessment(explained_count, Classification::Local(significant))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRID: GridShape = GridShape::LANDSCAPE;

    fn map_with(width: u32, height: u32, edits: &[Rect]) -> StateMap {
        StateMap::from_fn(width, height, |x, y| {
            if edits.iter().any(|r| r.contains(x, y)) {
                CellState::Changed
            } else {
                CellState::Agree
            }
        })
    }

    fn run(map: &StateMap) -> Assessment {
        classify(map, GRID, &[], &ClassifyParams::default())
    }

    #[test]
    fn clean_map_is_consistent() {
        let a = run(&map_with(160, 120, &[]));
        assert_eq!(a.classification, Classification::Consistent);
        assert_eq!((a.compared_fraction, a.changed_fraction), (1.0, 0.0));
    }

    #[test]
    fn single_interior_edit_is_local() {
        let edit = Rect::new(60, 50, 12, 10);
        let a = run(&map_with(160, 120, &[edit]));
        assert_eq!(
            a.classification,
            Classification::Local(vec![Component {
                rect: edit,
                area: 120
            }])
        );
    }

    #[test]
    fn small_noise_is_ignored() {
        let a = run(&map_with(
            160,
            120,
            &[Rect::new(60, 50, 4, 4), Rect::new(100, 30, 2, 9)],
        ));
        assert_eq!(a.classification, Classification::Consistent);
        assert!(a.changed_fraction > 0.0);
    }

    #[test]
    fn size_thresholds_use_cell_and_frame() {
        let half_cell = Rect::new(60, 50, 10, 5);
        assert!(matches!(
            run(&map_with(160, 120, &[half_cell])).classification,
            Classification::Local(_)
        ));
        let below = Rect::new(60, 50, 7, 7);
        assert_eq!(
            run(&map_with(160, 120, &[below])).classification,
            Classification::Consistent
        );
        let params = ClassifyParams {
            min_frame_fraction: 0.01,
            ..ClassifyParams::default()
        };
        assert_eq!(
            classify(&map_with(160, 120, &[half_cell]), GRID, &[], &params).classification,
            Classification::Consistent
        );
    }

    #[test]
    fn absolute_area_floor_suppresses_codec_sized_components() {
        let block = Rect::new(60, 50, 12, 12);
        assert!(matches!(
            run(&map_with(160, 120, &[block])).classification,
            Classification::Local(_)
        ));
        let floored = ClassifyParams {
            min_area_px: 256,
            ..ClassifyParams::default()
        };
        let a = classify(&map_with(160, 120, &[block]), GRID, &[], &floored);
        assert_eq!(a.classification, Classification::Consistent);
        assert!(a.changed_fraction > 0.0);
        let large = Rect::new(60, 50, 16, 16);
        assert!(matches!(
            classify(&map_with(160, 120, &[large]), GRID, &[], &floored).classification,
            Classification::Local(v) if v.len() == 1 && v[0].area == 256
        ));
    }

    #[test]
    fn area_floor_never_lowers_relative_minimum() {
        let half_cell = Rect::new(60, 50, 10, 5);
        let tiny_floor = ClassifyParams {
            min_area_px: 1,
            ..ClassifyParams::default()
        };
        let below = Rect::new(60, 50, 7, 7);
        assert_eq!(
            classify(&map_with(160, 120, &[below]), GRID, &[], &tiny_floor).classification,
            Classification::Consistent
        );
        assert!(matches!(
            classify(&map_with(160, 120, &[half_cell]), GRID, &[], &tiny_floor).classification,
            Classification::Local(_)
        ));
        let huge_floor = ClassifyParams {
            min_area_px: u64::MAX,
            ..ClassifyParams::default()
        };
        assert_eq!(
            classify(
                &map_with(160, 120, &[Rect::new(40, 30, 80, 60)]),
                GRID,
                &[],
                &huge_floor
            )
            .classification,
            Classification::Consistent
        );
    }

    #[test]
    fn up_to_six_components_allowed() {
        let edits: Vec<Rect> = (0..6).map(|i| Rect::new(20 + i * 20, 40, 8, 8)).collect();
        assert!(
            matches!(run(&map_with(160, 120, &edits)).classification, Classification::Local(v) if v.len() == 6)
        );
        let mut more = edits.clone();
        more.push(Rect::new(40, 80, 8, 8));
        assert_eq!(
            run(&map_with(160, 120, &more)).classification,
            Classification::Inconclusive(Inconclusive::TooManyComponents(7))
        );
    }

    #[test]
    fn border_band_rejects() {
        for edit in [
            Rect::new(2, 50, 10, 10),
            Rect::new(150, 50, 10, 10),
            Rect::new(60, 0, 10, 10),
            Rect::new(60, 112, 10, 8),
        ] {
            assert_eq!(
                run(&map_with(160, 120, &[edit])).classification,
                Classification::Inconclusive(Inconclusive::InBorderBand),
                "{edit:?}"
            );
        }
        assert!(matches!(
            run(&map_with(160, 120, &[Rect::new(10, 10, 10, 10)])).classification,
            Classification::Local(_)
        ));
    }

    #[test]
    fn border_band_is_relative_to_compared_extent() {
        let edit = Rect::new(50, 40, 10, 10);
        let map = StateMap::from_fn(160, 120, |x, y| {
            if x < 45 || y < 35 {
                CellState::Uncompared
            } else if edit.contains(x, y) {
                CellState::Changed
            } else {
                CellState::Agree
            }
        });
        assert_eq!(
            run(&map).classification,
            Classification::Inconclusive(Inconclusive::InBorderBand)
        );
    }

    #[test]
    fn unringed_component_is_inconclusive() {
        let edit = Rect::new(60, 50, 10, 10);
        let map = StateMap::from_fn(160, 120, |x, y| {
            if edit.contains(x, y) {
                CellState::Changed
            } else if (55..75).contains(&x) && (45..65).contains(&y) {
                CellState::Uncompared
            } else {
                CellState::Agree
            }
        });
        assert_eq!(
            run(&map).classification,
            Classification::Inconclusive(Inconclusive::NotRinged)
        );
    }

    #[test]
    fn excessive_change_cannot_compare() {
        let a = run(&map_with(160, 120, &[Rect::new(0, 0, 160, 50)]));
        assert_eq!(a.classification, Classification::CouldNotCompare);
        assert!(a.changed_fraction > 0.35);
        let just_below = run(&map_with(100, 100, &[Rect::new(0, 0, 100, 35)]));
        assert_ne!(just_below.classification, Classification::CouldNotCompare);
    }

    #[test]
    fn nothing_compared_cannot_compare() {
        let a = run(&StateMap::filled(50, 50, CellState::Uncompared));
        assert_eq!(a.classification, Classification::CouldNotCompare);
        assert_eq!(a.compared_fraction, 0.0);
        assert_eq!(
            run(&StateMap::filled(0, 0, CellState::Agree)).classification,
            Classification::CouldNotCompare
        );
    }

    #[test]
    fn explained_regions_are_removed() {
        let edit = Rect::new(60, 50, 12, 10);
        let map = map_with(160, 120, &[edit]);
        let a = classify(
            &map,
            GRID,
            &[Rect::new(55, 45, 30, 30)],
            &ClassifyParams::default(),
        );
        assert_eq!(a.classification, Classification::Consistent);
        assert_eq!(a.explained, 1);
        let partial = classify(
            &map,
            GRID,
            &[Rect::new(62, 50, 30, 30)],
            &ClassifyParams::default(),
        );
        assert!(matches!(partial.classification, Classification::Local(_)));
    }

    #[test]
    fn grid_resolution_maps_work() {
        let map = StateMap::from_fn(16, 12, |x, y| {
            if (x, y) == (7, 5) {
                CellState::Changed
            } else {
                CellState::Agree
            }
        });
        assert!(
            matches!(run(&map).classification, Classification::Local(v) if v[0].rect == Rect::new(7, 5, 1, 1))
        );
        let edge = StateMap::from_fn(16, 12, |x, y| {
            if (x, y) == (0, 5) {
                CellState::Changed
            } else {
                CellState::Agree
            }
        });
        assert_eq!(
            run(&edge).classification,
            Classification::Inconclusive(Inconclusive::InBorderBand)
        );
    }

    #[test]
    fn ring_agreement_counts_only_ring() {
        let map = map_with(20, 20, &[Rect::new(8, 8, 4, 4)]);
        assert_eq!(ring_agreement(&map, &Rect::new(8, 8, 4, 4), 2, 2), 1.0);
        assert!(ring_agreement(&map, &Rect::new(6, 6, 4, 4), 1, 1) < 1.0);
        let full = StateMap::filled(4, 4, CellState::Changed);
        assert_eq!(ring_agreement(&full, &Rect::new(0, 0, 4, 4), 1, 1), 0.0);
    }

    #[test]
    fn property_random_single_edits_are_local() {
        let mut s = 0x5EEDu64;
        let mut next = |m: u32| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s % m as u64) as u32
        };
        for _ in 0..40 {
            let (w, h) = (8 + next(6), 8 + next(6));
            let edit = Rect::new(12 + next(160 - 26 - w), 12 + next(120 - 26 - h), w, h);
            let a = run(&map_with(160, 120, &[edit]));
            assert_eq!(
                a.classification,
                Classification::Local(vec![Component {
                    rect: edit,
                    area: edit.area()
                }]),
                "{edit:?}"
            );
        }
    }
}
