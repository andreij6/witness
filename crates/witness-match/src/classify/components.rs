use super::map::{CellState, ChangeMap};
use crate::geometry::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Component {
    pub rect: Rect,
    pub area: u64,
}

struct Run {
    start: u32,
    end: u32,
    y: u32,
}

fn find(parent: &mut [u32], mut i: u32) -> u32 {
    while parent[i as usize] != i {
        let grand = parent[parent[i as usize] as usize];
        parent[i as usize] = grand;
        i = grand;
    }
    i
}

fn union(parent: &mut [u32], a: u32, b: u32) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
        parent[hi as usize] = lo;
    }
}

fn row_runs<M: ChangeMap>(map: &M, y: u32, out: &mut Vec<Run>) {
    let mut x = 0;
    let width = map.width();
    while x < width {
        if map.state(x, y) != CellState::Changed {
            x += 1;
            continue;
        }
        let start = x;
        while x < width && map.state(x, y) == CellState::Changed {
            x += 1;
        }
        out.push(Run { start, end: x, y });
    }
}

pub fn components<M: ChangeMap>(map: &M) -> Vec<Component> {
    let mut runs: Vec<Run> = Vec::new();
    let mut parent: Vec<u32> = Vec::new();
    let mut previous = 0..0;
    for y in 0..map.height() {
        let first = runs.len();
        row_runs(map, y, &mut runs);
        parent.extend(first as u32..runs.len() as u32);
        let mut p = previous.start;
        for current in first..runs.len() {
            while p < previous.end && runs[p].end < runs[current].start {
                p += 1;
            }
            let mut q = p;
            while q < previous.end && runs[q].start <= runs[current].end {
                union(&mut parent, current as u32, q as u32);
                q += 1;
            }
        }
        previous = first..runs.len();
    }
    let mut slot = vec![u32::MAX; runs.len()];
    let mut out: Vec<Component> = Vec::new();
    for (i, run) in runs.iter().enumerate() {
        let root = find(&mut parent, i as u32) as usize;
        let rect = Rect::new(run.start, run.y, run.end - run.start, 1);
        if slot[root] == u32::MAX {
            slot[root] = out.len() as u32;
            out.push(Component {
                rect,
                area: rect.area(),
            });
        } else {
            let c = &mut out[slot[root] as usize];
            c.rect = c.rect.bounding_union(&rect);
            c.area += rect.area();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::map::StateMap;

    fn parse(rows: &[&str]) -> StateMap {
        let h = rows.len() as u32;
        let w = rows[0].len() as u32;
        StateMap::from_fn(w, h, |x, y| match rows[y as usize].as_bytes()[x as usize] {
            b'#' => CellState::Changed,
            b'.' => CellState::Agree,
            _ => CellState::Uncompared,
        })
    }

    fn naive(map: &StateMap) -> Vec<(Rect, u64)> {
        let (w, h) = (map.width() as i64, map.height() as i64);
        let mut seen = vec![false; (w * h) as usize];
        let mut out = Vec::new();
        for start in 0..(w * h) {
            if seen[start as usize]
                || map.state((start % w) as u32, (start / w) as u32) != CellState::Changed
            {
                continue;
            }
            let mut stack = vec![start];
            seen[start as usize] = true;
            let mut rect: Option<Rect> = None;
            let mut area = 0;
            while let Some(i) = stack.pop() {
                let (x, y) = (i % w, i / w);
                area += 1;
                let r = Rect::new(x as u32, y as u32, 1, 1);
                rect = Some(rect.map_or(r, |c| c.bounding_union(&r)));
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx >= 0 && ny >= 0 && nx < w && ny < h {
                            let j = ny * w + nx;
                            if !seen[j as usize]
                                && map.state(nx as u32, ny as u32) == CellState::Changed
                            {
                                seen[j as usize] = true;
                                stack.push(j);
                            }
                        }
                    }
                }
            }
            out.push((rect.unwrap(), area));
        }
        out.sort();
        out
    }

    fn sorted(map: &StateMap) -> Vec<(Rect, u64)> {
        let mut v: Vec<(Rect, u64)> = components(map).iter().map(|c| (c.rect, c.area)).collect();
        v.sort();
        v
    }

    #[test]
    fn empty_map_has_no_components() {
        assert!(components(&StateMap::filled(10, 10, CellState::Agree)).is_empty());
        assert!(components(&StateMap::filled(0, 0, CellState::Agree)).is_empty());
    }

    #[test]
    fn diagonal_pixels_are_connected() {
        let map = parse(&["#...", ".#..", "..#.", "...#"]);
        assert_eq!(
            components(&map),
            vec![Component {
                rect: Rect::new(0, 0, 4, 4),
                area: 4
            }]
        );
    }

    #[test]
    fn separate_blobs_are_separate() {
        let map = parse(&["##..##", "##..##", "......", "..##.."]);
        assert_eq!(components(&map).len(), 3);
    }

    #[test]
    fn u_shape_merges_late() {
        let map = parse(&["#...#", "#...#", "#...#", "#####"]);
        assert_eq!(
            components(&map),
            vec![Component {
                rect: Rect::new(0, 0, 5, 4),
                area: 11
            }]
        );
    }

    #[test]
    fn spiral_and_uncompared_cells() {
        let map = parse(&[
            "#####", "....#", "###.#", "#...#", "#####", "?????", "#?#?#",
        ]);
        let c = sorted(&map);
        assert_eq!(c.len(), 4);
        assert_eq!(c, naive(&map));
    }

    #[test]
    fn full_map_is_one_component() {
        let map = StateMap::filled(37, 23, CellState::Changed);
        assert_eq!(
            components(&map),
            vec![Component {
                rect: Rect::new(0, 0, 37, 23),
                area: 37 * 23
            }]
        );
    }

    #[test]
    fn random_maps_match_flood_fill() {
        let mut s = 0xABCDu64;
        for case in 0..60 {
            let (w, h) = (5 + case % 23, 4 + case % 17);
            let density = 20 + case % 50;
            let map = StateMap::from_fn(w, h, |_, _| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                match s % 100 {
                    v if v < density as u64 => CellState::Changed,
                    v if v < 90 => CellState::Agree,
                    _ => CellState::Uncompared,
                }
            });
            assert_eq!(sorted(&map), naive(&map), "case {case}");
        }
    }

    #[test]
    fn deterministic_order_is_raster_of_first_run() {
        let map = parse(&["..#", "#..", "..."]);
        let c = components(&map);
        assert_eq!(c[0].rect, Rect::new(2, 0, 1, 1));
        assert_eq!(c[1].rect, Rect::new(0, 1, 1, 1));
    }
}
