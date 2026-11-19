#![allow(dead_code)]

use witness_image::{Rgb8, decode, encode_jpeg, flip_horizontal, resize_area, resize_bilinear};
use witness_match::{Rect, Rng, Similarity, SplitMix64};

pub fn rng(seed: u64) -> SplitMix64 {
    SplitMix64::new(seed)
}

fn color(r: &mut SplitMix64) -> [u8; 3] {
    let v = r.next_u64();
    [v as u8, (v >> 8) as u8, (v >> 16) as u8]
}

pub fn scene(width: u32, height: u32, seed: u64) -> Rgb8 {
    let mut r = rng(seed);
    let grid: Vec<[u8; 3]> = (0..25).map(|_| color(&mut r)).collect();
    let min = width.min(height);
    let shapes: Vec<(u32, u32, u32, u32, bool, [u8; 3])> = (0..140)
        .map(|_| {
            let w = min / 40 + r.below(min / 9);
            let h = min / 40 + r.below(min / 9);
            (
                r.below(width),
                r.below(height),
                w,
                h,
                r.below(3) == 0,
                color(&mut r),
            )
        })
        .collect();
    let mut noise = rng(seed ^ 0xA5A5);
    Rgb8::from_fn(width, height, |x, y| {
        let gx = x as f64 / width as f64 * 4.0;
        let gy = y as f64 / height as f64 * 4.0;
        let (ix, iy) = ((gx as usize).min(3), (gy as usize).min(3));
        let (fx, fy) = (gx - ix as f64, gy - iy as f64);
        let mut p = [0u8; 3];
        for c in 0..3 {
            let at = |i: usize, j: usize| grid[j * 5 + i][c] as f64;
            let top = at(ix, iy) * (1.0 - fx) + at(ix + 1, iy) * fx;
            let bottom = at(ix, iy + 1) * (1.0 - fx) + at(ix + 1, iy + 1) * fx;
            p[c] = (top * (1.0 - fy) + bottom * fy) as u8;
        }
        for &(sx, sy, sw, sh, disc, c) in &shapes {
            let inside = if disc {
                let (dx, dy) = (x as i64 - sx as i64, y as i64 - sy as i64);
                dx * dx * (sh as i64 * sh as i64) + dy * dy * (sw as i64 * sw as i64)
                    <= (sw as i64 * sh as i64).pow(2) / 4
            } else {
                x >= sx && y >= sy && x < sx + sw && y < sy + sh
            };
            if inside {
                p = c;
            }
        }
        let n = (noise.below(9) as i32) - 4;
        p.map(|v| (v as i32 + n).clamp(0, 255) as u8)
    })
    .unwrap()
}

pub fn crop_scale(img: &Rgb8, rect: Rect, out_w: u32, out_h: u32) -> Rgb8 {
    let view = img.crop(rect.x, rect.y, rect.width, rect.height).unwrap();
    if out_w <= rect.width {
        resize_area(view, out_w, out_h).unwrap()
    } else {
        resize_bilinear(view, out_w, out_h).unwrap()
    }
}

pub fn crop_scale_truth(rect: Rect, out_w: u32) -> Similarity {
    let s = out_w as f64 / rect.width as f64;
    Similarity::scale_translate(s, -(rect.x as f64) * s, -(rect.y as f64) * s)
}

pub fn flip(img: &Rgb8) -> Rgb8 {
    flip_horizontal(img.view())
}

pub fn jpeg(img: &Rgb8, quality: u8) -> Rgb8 {
    decode(&encode_jpeg(img, quality).unwrap()).unwrap()
}

pub fn map_pixels(img: &Rgb8, f: impl Fn(u32, u32, [u8; 3]) -> [u8; 3]) -> Rgb8 {
    Rgb8::from_fn(img.width(), img.height(), |x, y| {
        f(x, y, img.pixel(x, y).unwrap())
    })
    .unwrap()
}

pub fn filter(img: &Rgb8) -> Rgb8 {
    map_pixels(img, |_, _, p| {
        let curve = |v: u8, lift: u32| {
            ((v as u32 * (v as u32 + 255) / 510) * (255 - lift) / 255 + lift) as u8
        };
        [curve(p[0], 20), curve(p[1], 8), curve(p[2], 30)]
    })
}

pub fn vignette(img: &Rgb8, strength: f64) -> Rgb8 {
    let (cx, cy) = (img.width() as f64 / 2.0, img.height() as f64 / 2.0);
    map_pixels(img, |x, y, p| {
        let (dx, dy) = ((x as f64 - cx) / cx, (y as f64 - cy) / cy);
        let g = 1.0 - strength * (dx * dx + dy * dy) / 2.0;
        p.map(|v| (v as f64 * g).round().clamp(0.0, 255.0) as u8)
    })
}

pub fn paste_edit(img: &Rgb8, rect: Rect, seed: u64) -> Rgb8 {
    let mut r = rng(seed);
    let bars: Vec<(u32, u32, u32, u32)> = (0..12)
        .map(|_| {
            (
                r.below(rect.width),
                r.below(rect.height),
                2 + r.below(rect.width / 3),
                2 + r.below(rect.height / 3),
            )
        })
        .collect();
    let (dark, light) = ([15u8, 20, 25], [240u8, 235, 225]);
    map_pixels(img, |x, y, p| {
        if !rect.contains(x, y) {
            return p;
        }
        let (lx, ly) = (x - rect.x, y - rect.y);
        let hit = bars
            .iter()
            .any(|&(bx, by, bw, bh)| lx >= bx && ly >= by && lx < bx + bw && ly < by + bh);
        if hit { dark } else { light }
    })
}

pub fn box_blur(img: &Rgb8, rect: Rect, radius: u32) -> Rgb8 {
    map_pixels(img, |x, y, p| {
        if !rect.contains(x, y) {
            return p;
        }
        let mut acc = [0u32; 3];
        let mut n = 0;
        for yy in y.saturating_sub(radius)..=(y + radius).min(img.height() - 1) {
            for xx in x.saturating_sub(radius)..=(x + radius).min(img.width() - 1) {
                let q = img.pixel(xx, yy).unwrap();
                for c in 0..3 {
                    acc[c] += q[c] as u32;
                }
                n += 1;
            }
        }
        acc.map(|a| ((a + n / 2) / n) as u8)
    })
}

pub fn max_point_error(estimate: &Similarity, truth: &Similarity, width: f64, height: f64) -> f64 {
    let mut worst: f64 = 0.0;
    for i in 0..5 {
        for j in 0..5 {
            let (x, y) = (width * i as f64 / 4.0, height * j as f64 / 4.0);
            let (a, b) = estimate.apply(x, y);
            let (c, d) = truth.apply(x, y);
            worst = worst.max(((a - c) * (a - c) + (b - d) * (b - d)).sqrt());
        }
    }
    worst
}

pub fn iou(a: &Rect, b: &Rect) -> f64 {
    let inter = a.intersection(b).map_or(0, |r| r.area()) as f64;
    inter / ((a.area() + b.area()) as f64 - inter)
}

pub fn paste_patch(img: &Rgb8, rect: Rect, donor_seed: u64) -> Rgb8 {
    let donor = scene(img.width(), img.height(), donor_seed);
    map_pixels(img, |x, y, p| {
        if rect.contains(x, y) {
            donor.pixel(x, y).unwrap()
        } else {
            p
        }
    })
}
