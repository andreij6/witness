mod descriptor;
mod fast;
mod pattern;

pub use descriptor::{Descriptor, hamming};

use pattern::{CENTROID_RADIUS, PATTERN_RADIUS};
use witness_image::{Luma8, LumaView, resize_area};

pub(crate) const BORDER: usize = CENTROID_RADIUS + 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureParams {
    pub max_keypoints: u32,
    pub fast_threshold: u8,
    pub levels: u8,
}

impl Default for FeatureParams {
    fn default() -> Self {
        Self {
            max_keypoints: 600,
            fast_threshold: 18,
            levels: 5,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Features {
    pub points: Vec<(f64, f64)>,
    pub descriptors: Vec<Descriptor>,
}

impl Features {
    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

const _: () = assert!(PATTERN_RADIUS as usize + 1 + descriptor::SMOOTH_RADIUS <= BORDER);

fn level_sizes(width: u32, height: u32, levels: u8) -> Vec<(u32, u32)> {
    let min_side = 2 * BORDER as u32 + 8;
    let mut sizes = Vec::with_capacity(levels as usize);
    let (mut w, mut h) = (width, height);
    while sizes.len() < levels as usize && w >= min_side && h >= min_side {
        sizes.push((w, h));
        (w, h) = (w * 4 / 5, h * 4 / 5);
    }
    sizes
}

fn budgets(sizes: &[(u32, u32)], total: u32) -> Vec<usize> {
    let areas: Vec<u64> = sizes.iter().map(|&(w, h)| w as u64 * h as u64).collect();
    let sum: u64 = areas.iter().sum::<u64>().max(1);
    areas
        .iter()
        .map(|&a| (total as u64 * a / sum) as usize)
        .collect()
}

fn describe_level(
    level: &Luma8,
    budget: usize,
    threshold: u8,
    scale: (f64, f64),
    out: &mut Features,
) {
    let (w, h) = (level.width() as usize, level.height() as usize);
    let img = level.data();
    let mut corners = fast::detect(img, w, h, threshold, BORDER);
    corners.sort_unstable_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.y.cmp(&b.y))
            .then(a.x.cmp(&b.x))
    });
    corners.truncate(budget);
    if corners.is_empty() {
        return;
    }
    let smooth = descriptor::box_sum5(img, w, h);
    for c in corners {
        let (x, y) = (c.x as usize, c.y as usize);
        let bin = descriptor::angle_bin(img, w, x, y);
        out.descriptors
            .push(descriptor::describe(&smooth, w, x, y, bin));
        out.points
            .push(((x as f64 + 0.5) * scale.0, (y as f64 + 0.5) * scale.1));
    }
}

pub fn detect_features(
    image: LumaView<'_>,
    full_width: f64,
    full_height: f64,
    params: &FeatureParams,
) -> Features {
    let sizes = level_sizes(image.width(), image.height(), params.levels);
    let budgets = budgets(&sizes, params.max_keypoints);
    let mut features = Features::default();
    let mut current = image.to_image();
    for (k, (&(w, h), &budget)) in sizes.iter().zip(&budgets).enumerate() {
        if k > 0 {
            match resize_area(current.view(), w, h) {
                Ok(next) => current = next,
                Err(_) => break,
            }
        }
        let scale = (full_width / w as f64, full_height / h as f64);
        describe_level(
            &current,
            budget,
            params.fast_threshold,
            scale,
            &mut features,
        );
    }
    features
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks(width: u32, height: u32, seed: u64) -> Luma8 {
        let mut s = seed;
        let mut rects = Vec::new();
        for _ in 0..40 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            rects.push((
                (s % width as u64) as u32,
                ((s >> 16) % height as u64) as u32,
                6 + ((s >> 32) % 30) as u32,
                6 + ((s >> 40) % 30) as u32,
                (s >> 56) as u8,
            ));
        }
        Luma8::from_fn(width, height, |x, y| {
            let mut v = 128u8;
            for &(rx, ry, rw, rh, c) in &rects {
                if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                    v = c;
                }
            }
            [v]
        })
        .unwrap()
    }

    #[test]
    fn level_sizes_shrink_by_four_fifths() {
        assert_eq!(
            level_sizes(500, 400, 3),
            vec![(500, 400), (400, 320), (320, 256)]
        );
        assert_eq!(level_sizes(40, 60, 5), vec![]);
        assert_eq!(level_sizes(100, 1000, 9).len(), 4);
        assert!(level_sizes(500, 400, 0).is_empty());
    }

    #[test]
    fn budgets_are_area_proportional_and_bounded() {
        let sizes = level_sizes(500, 400, 4);
        let b = budgets(&sizes, 1000);
        assert!(b.iter().sum::<usize>() <= 1000);
        assert!(b.windows(2).all(|w| w[0] >= w[1]));
        assert_eq!(budgets(&[], 10), Vec::<usize>::new());
    }

    #[test]
    fn detects_bounded_number_of_features() {
        let img = blocks(320, 240, 11);
        let params = FeatureParams {
            max_keypoints: 100,
            ..FeatureParams::default()
        };
        let f = detect_features(img.view(), 320.0, 240.0, &params);
        assert!(f.len() > 20 && f.len() <= 100, "{}", f.len());
        assert_eq!(f.points.len(), f.descriptors.len());
        assert!(!f.is_empty());
        for &(x, y) in &f.points {
            assert!(x > 0.0 && x < 320.0 && y > 0.0 && y < 240.0);
        }
    }

    #[test]
    fn points_scale_to_full_resolution() {
        let img = blocks(320, 240, 12);
        let f = detect_features(img.view(), 640.0, 480.0, &FeatureParams::default());
        let g = detect_features(img.view(), 320.0, 240.0, &FeatureParams::default());
        assert_eq!(f.descriptors, g.descriptors);
        for (a, b) in f.points.iter().zip(&g.points) {
            assert!((a.0 - 2.0 * b.0).abs() < 1e-9 && (a.1 - 2.0 * b.1).abs() < 1e-9);
        }
    }

    #[test]
    fn flat_and_tiny_images_have_no_features() {
        let flat = Luma8::filled(200, 200, [77]).unwrap();
        assert!(detect_features(flat.view(), 200.0, 200.0, &FeatureParams::default()).is_empty());
        let tiny = Luma8::filled(10, 10, [0]).unwrap();
        assert!(detect_features(tiny.view(), 10.0, 10.0, &FeatureParams::default()).is_empty());
    }

    #[test]
    fn detection_is_deterministic() {
        let img = blocks(256, 256, 99);
        let a = detect_features(img.view(), 256.0, 256.0, &FeatureParams::default());
        let b = detect_features(img.view(), 256.0, 256.0, &FeatureParams::default());
        assert_eq!(a, b);
    }

    #[test]
    fn shifted_image_matches_descriptors() {
        let img = blocks(300, 240, 21);
        let shifted = img.crop(10, 6, 280, 220).unwrap();
        let params = FeatureParams {
            levels: 1,
            ..FeatureParams::default()
        };
        let a = detect_features(img.view(), 300.0, 240.0, &params);
        let b = detect_features(shifted, 280.0, 220.0, &params);
        let mut exact = 0;
        for (i, &(x, y)) in b.points.iter().enumerate() {
            if let Some(j) = a
                .points
                .iter()
                .position(|&(ax, ay)| (ax - x - 10.0).abs() < 1e-9 && (ay - y - 6.0).abs() < 1e-9)
            {
                exact += (a.descriptors[j] == b.descriptors[i]) as usize;
            }
        }
        assert!(exact > 10, "{exact}");
    }
}
