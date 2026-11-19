use crate::dihedral::Dihedral;
use crate::hash::PdqHash;
use crate::pdq::{Analysis, Pdq};
use witness_format::Rect;
use witness_image::{Luma8, LumaView, RgbView, resize_area, to_luma};

pub const WORK_LONG_EDGE: u32 = 512;
pub const MAX_QUERY_HASHES: usize = 16;
pub const MAX_STORED_SHAPES: usize = 1 + 6 * 3;
const ANCHOR_SPREAD_PERCENT: u64 = 5;
const QUERY_STEP_PERCENT: u64 = 4;
const QUERY_STEP_SIDES: u64 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Aspect {
    Full,
    Square,
    Portrait4x5,
    Portrait3x4,
    Portrait9x16,
    Landscape16x9,
    Landscape191x100,
}

impl Aspect {
    pub const ALL: [Self; 7] = [
        Self::Full,
        Self::Square,
        Self::Portrait4x5,
        Self::Portrait3x4,
        Self::Portrait9x16,
        Self::Landscape16x9,
        Self::Landscape191x100,
    ];

    pub const fn ratio(self) -> Option<(u32, u32)> {
        match self {
            Self::Full => None,
            Self::Square => Some((1, 1)),
            Self::Portrait4x5 => Some((4, 5)),
            Self::Portrait3x4 => Some((3, 4)),
            Self::Portrait9x16 => Some((9, 16)),
            Self::Landscape16x9 => Some((16, 9)),
            Self::Landscape191x100 => Some((191, 100)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Anchor {
    Start,
    Center,
    End,
}

impl Anchor {
    pub const ALL: [Self; 3] = [Self::Start, Self::Center, Self::End];

    pub const fn offset(self, slack: u32) -> u32 {
        match self {
            Self::Start => 0,
            Self::Center => slack / 2,
            Self::End => slack,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeTag(u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub rect: Rect,
    pub slack: u32,
    pub frame_len: u32,
    pub horizontal: bool,
}

impl ShapeTag {
    pub const FULL: Self = Self::new(Aspect::Full, Anchor::Center);

    pub const fn new(aspect: Aspect, anchor: Anchor) -> Self {
        Self(((aspect as u8) << 2) | anchor as u8)
    }

    pub const fn from_u8(value: u8) -> Option<Self> {
        let (aspect, anchor) = (value >> 2, value & 3);
        let valid = aspect < Aspect::ALL.len() as u8
            && anchor < 3
            && (aspect != 0 || anchor == Anchor::Center as u8);
        if valid { Some(Self(value)) } else { None }
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    pub const fn aspect(self) -> Aspect {
        Aspect::ALL[(self.0 >> 2) as usize]
    }

    pub const fn anchor(self) -> Anchor {
        Anchor::ALL[(self.0 & 3) as usize]
    }

    pub fn placement(self, width: u32, height: u32) -> Option<Placement> {
        if width == 0 || height == 0 {
            return None;
        }
        let full = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let Some((num, den)) = self.aspect().ratio() else {
            return Some(Placement {
                rect: full,
                slack: 0,
                frame_len: width,
                horizontal: true,
            });
        };
        let (w, h, num, den) = (width as u64, height as u64, num as u64, den as u64);
        let horizontal = w * den > h * num;
        let (frame_len, cross_len, ratio_num, ratio_den) = if horizontal {
            (w, h, num, den)
        } else {
            (h, w, den, num)
        };
        let crop = ((cross_len * ratio_num + ratio_den / 2) / ratio_den).clamp(1, frame_len);
        let slack = (frame_len - crop) as u32;
        let offset = self.anchor().offset(slack);
        let rect = if horizontal {
            Rect {
                x: offset,
                y: 0,
                w: crop as u32,
                h: height,
            }
        } else {
            Rect {
                x: 0,
                y: offset,
                w: width,
                h: crop as u32,
            }
        };
        Some(Placement {
            rect,
            slack,
            frame_len: frame_len as u32,
            horizontal,
        })
    }

    pub fn rect(self, width: u32, height: u32) -> Option<Rect> {
        self.placement(width, height).map(|p| p.rect)
    }
}

pub fn stored_shape_tags(width: u32, height: u32) -> Vec<ShapeTag> {
    let mut tags = Vec::with_capacity(MAX_STORED_SHAPES);
    if width == 0 || height == 0 {
        return tags;
    }
    tags.push(ShapeTag::FULL);
    for aspect in &Aspect::ALL[1..] {
        let Some(placement) = ShapeTag::new(*aspect, Anchor::Center).placement(width, height)
        else {
            continue;
        };
        if placement.slack == 0 {
            continue;
        }
        if placement.slack as u64 * 100 > placement.frame_len as u64 * ANCHOR_SPREAD_PERCENT {
            tags.extend(Anchor::ALL.map(|anchor| ShapeTag::new(*aspect, anchor)));
        } else {
            tags.push(ShapeTag::new(*aspect, Anchor::Center));
        }
    }
    tags
}

pub fn fit_long_edge(width: u32, height: u32, long_edge: u32) -> (u32, u32) {
    let long = width.max(height);
    if long <= long_edge {
        return (width, height);
    }
    let scale =
        |v: u32| ((v as u64 * long_edge as u64 + long as u64 / 2) / long as u64).max(1) as u32;
    (scale(width), scale(height))
}

pub fn working_luma(rgb: RgbView<'_>) -> Luma8 {
    let (w, h) = fit_long_edge(rgb.width(), rgb.height(), WORK_LONG_EDGE);
    if (w, h) == rgb.dimensions() {
        return to_luma(rgb);
    }
    match resize_area(rgb, w, h) {
        Ok(small) => to_luma(small.view()),
        Err(_) => to_luma(rgb),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeHash {
    pub tag: ShapeTag,
    pub pdq: Pdq,
}

impl ShapeHash {
    pub const fn to_format(&self) -> witness_format::PdqShape {
        witness_format::PdqShape {
            tag: self.tag.0,
            hash: self.pdq.hash.0,
        }
    }
}

pub fn stored_shapes(work: LumaView<'_>) -> Vec<ShapeHash> {
    stored_shape_tags(work.width(), work.height())
        .into_iter()
        .filter_map(|tag| {
            let r = tag.rect(work.width(), work.height())?;
            let analysis = Analysis::new(work.crop(r.x, r.y, r.w, r.h).ok()?)?;
            Some(ShapeHash {
                tag,
                pdq: Pdq {
                    hash: analysis.hash(Dihedral::Identity),
                    quality: analysis.quality,
                },
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum QueryVariant {
    Base,
    Mirror,
    Inset,
    Outset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryHash {
    pub rect_index: u8,
    pub variant: QueryVariant,
    pub hash: PdqHash,
}

fn scale_rect(rect: Rect, from: (u32, u32), to: (u32, u32)) -> Option<Rect> {
    if !rect.is_valid() || rect.x >= from.0 || rect.y >= from.1 {
        return None;
    }
    let axis = |start: u32, len: u32, src: u32, dst: u32| {
        let end = (start as u64 + len as u64).min(src as u64);
        let lo = start as u64 * dst as u64 / src as u64;
        let hi = (end * dst as u64).div_ceil(src as u64).max(lo + 1);
        (lo as u32, (hi - lo) as u32)
    };
    let (x, w) = axis(rect.x, rect.w, from.0, to.0);
    let (y, h) = axis(rect.y, rect.h, from.1, to.1);
    Some(Rect { x, y, w, h })
}

fn resize_about_center(rect: Rect, frame: (u32, u32), grow: bool) -> Rect {
    let axis = |start: u32, len: u32, bound: u32| {
        let step = (len as u64 * QUERY_STEP_PERCENT).div_ceil(100 * QUERY_STEP_SIDES) as i64;
        let (start, len) = (start as i64, len as i64);
        let (lo, hi) = if grow {
            (start - step, start + len + step)
        } else {
            (start + step, start + len - step)
        };
        let (lo, hi) = (lo.clamp(0, bound as i64), hi.clamp(0, bound as i64));
        if hi <= lo {
            (start as u32, len as u32)
        } else {
            (lo as u32, (hi - lo) as u32)
        }
    };
    let (x, w) = axis(rect.x, rect.w, frame.0);
    let (y, h) = axis(rect.y, rect.h, frame.1);
    Rect { x, y, w, h }
}

fn analyse_rect(work: LumaView<'_>, r: Rect) -> Option<Analysis> {
    Analysis::new(work.crop(r.x, r.y, r.w, r.h).ok()?)
}

pub fn query_hashes(rgb: RgbView<'_>, rects: &[Rect]) -> Vec<QueryHash> {
    let work = working_luma(rgb);
    let frame = work.dimensions();
    let mut out: Vec<QueryHash> = Vec::with_capacity(MAX_QUERY_HASHES);
    let push = |out: &mut Vec<QueryHash>, rect_index: u8, variant, hash: PdqHash| {
        if out.len() < MAX_QUERY_HASHES && out.iter().all(|q| q.hash != hash) {
            out.push(QueryHash {
                rect_index,
                variant,
                hash,
            });
        }
    };
    for (index, &rect) in rects.iter().enumerate().take(u8::MAX as usize) {
        if out.len() >= MAX_QUERY_HASHES {
            break;
        }
        let Some(base) = scale_rect(rect, rgb.dimensions(), frame) else {
            continue;
        };
        let Some(analysis) = analyse_rect(work.view(), base) else {
            continue;
        };
        let index = index as u8;
        push(
            &mut out,
            index,
            QueryVariant::Base,
            analysis.hash(Dihedral::Identity),
        );
        push(
            &mut out,
            index,
            QueryVariant::Mirror,
            analysis.hash(Dihedral::MirrorHorizontal),
        );
        for (variant, grow) in [(QueryVariant::Inset, false), (QueryVariant::Outset, true)] {
            let resized = resize_about_center(base, frame, grow);
            if resized == base {
                continue;
            }
            if let Some(analysis) = analyse_rect(work.view(), resized) {
                push(&mut out, index, variant, analysis.hash(Dihedral::Identity));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
