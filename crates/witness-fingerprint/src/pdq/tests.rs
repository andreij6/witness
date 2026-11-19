use super::*;
use crate::testkit::{Rng, grey, jpeg_round_trip, map_pixels, noise_luma, scene};
use pdqhash::image::{DynamicImage, GrayImage, RgbImage};
use witness_image::{
    Luma8, Rgb8, flip_horizontal, flip_vertical, resize_area, rotate90, rotate180, rotate270,
    transpose,
};

fn oracle(image: DynamicImage) -> (PdqHash, f32) {
    let (hash, quality) = pdqhash::generate_pdq_full_size(&image);
    (PdqHash(hash), quality)
}

fn oracle_rgb(rgb: &Rgb8) -> (PdqHash, f32) {
    oracle(DynamicImage::ImageRgb8(
        RgbImage::from_raw(rgb.width(), rgb.height(), rgb.data().to_vec()).unwrap(),
    ))
}

fn oracle_luma(luma: &Luma8) -> (PdqHash, f32) {
    oracle(DynamicImage::ImageLuma8(
        GrayImage::from_raw(luma.width(), luma.height(), luma.data().to_vec()).unwrap(),
    ))
}

fn hash_of(luma: &Luma8) -> PdqHash {
    pdq(luma.view()).unwrap().hash
}

fn anti_transpose(luma: &Luma8) -> Luma8 {
    rotate180(transpose(luma.view()).view())
}

#[test]
fn oracle_agrees_exactly_on_64px_luma() {
    let mut exact = 0;
    for seed in 0..60 {
        let luma = if seed % 2 == 0 {
            noise_luma(64, 64, seed)
        } else {
            grey(&scene(64, 64, seed))
        };
        let (expected, _) = oracle_luma(&luma);
        let distance = hash_of(&luma).distance(&expected);
        assert!(distance <= 2, "seed {seed} distance {distance}");
        exact += usize::from(distance == 0);
    }
    assert!(exact >= 55, "exact {exact}/60");
}

#[test]
fn oracle_agrees_closely_on_synthetic_colour_scenes() {
    let mut rng = Rng::new(99);
    let mut total = 0;
    let cases = 40;
    for seed in 0..cases {
        let (w, h) = (64 + rng.below(600) as u32, 64 + rng.below(600) as u32);
        let rgb = scene(w, h, seed);
        let (expected, oracle_quality) = oracle_rgb(&rgb);
        let ours = pdq(grey(&rgb).view()).unwrap();
        let distance = ours.hash.distance(&expected);
        assert!(distance <= 12, "seed {seed} {w}x{h} distance {distance}");
        let reference_quality = (oracle_quality * 100.0).round() as i32;
        assert!(
            (ours.quality as i32 - reference_quality).abs() <= 8,
            "q {} vs {reference_quality}",
            ours.quality
        );
        total += distance;
    }
    assert!(
        total as f64 / (cases as f64) <= 4.0,
        "mean distance {}",
        total as f64 / cases as f64
    );
}

#[test]
fn oracle_agrees_on_noise_at_odd_sizes() {
    for (seed, (w, h)) in [
        (65u32, 64u32),
        (127, 129),
        (200, 75),
        (5, 300),
        (333, 5),
        (1000, 999),
    ]
    .into_iter()
    .enumerate()
    {
        let luma = noise_luma(w, h, seed as u64 + 500);
        let (expected, _) = oracle_luma(&luma);
        let distance = hash_of(&luma).distance(&expected);
        assert!(distance <= 40, "{w}x{h} distance {distance}");
    }
}

#[test]
fn dihedral_variants_are_exact_on_square_64() {
    for seed in 0..20 {
        let luma = grey(&scene(64, 64, seed + 1000));
        let all = pdq_dihedral(luma.view()).unwrap();
        let view = luma.view();
        let cases = [
            (Dihedral::Identity, luma.clone()),
            (Dihedral::Rotate90, rotate90(view)),
            (Dihedral::Rotate180, rotate180(view)),
            (Dihedral::Rotate270, rotate270(view)),
            (Dihedral::MirrorHorizontal, flip_horizontal(view)),
            (Dihedral::MirrorVertical, flip_vertical(view)),
            (Dihedral::Transpose, transpose(view)),
            (Dihedral::AntiTranspose, anti_transpose(&luma)),
        ];
        for (transform, image) in cases {
            assert_eq!(
                all.get(transform),
                hash_of(&image),
                "seed {seed} {transform:?}"
            );
        }
    }
}

#[test]
fn dihedral_variants_are_close_on_general_sizes() {
    let (mut total, mut count) = (0, 0);
    for seed in 0..12 {
        let luma = grey(&scene(
            300 + seed as u32 * 17,
            200 + seed as u32 * 9,
            seed + 2000,
        ));
        let all = pdq_dihedral(luma.view()).unwrap();
        let view = luma.view();
        for (transform, image) in [
            (Dihedral::Rotate90, rotate90(view)),
            (Dihedral::Rotate180, rotate180(view)),
            (Dihedral::Rotate270, rotate270(view)),
            (Dihedral::MirrorHorizontal, flip_horizontal(view)),
            (Dihedral::MirrorVertical, flip_vertical(view)),
            (Dihedral::Transpose, transpose(view)),
            (Dihedral::AntiTranspose, anti_transpose(&luma)),
        ] {
            let distance = all.get(transform).distance(&hash_of(&image));
            assert!(
                distance <= 32,
                "seed {seed} {transform:?} distance {distance}"
            );
            total += distance;
            count += 1;
        }
    }
    assert!(total <= count * 20, "mean {}", total / count);
}

#[test]
fn dihedral_variants_agree_with_oracle_on_transformed_images() {
    let luma = grey(&scene(64, 64, 77));
    let all = pdq_dihedral(luma.view()).unwrap();
    let view = luma.view();
    for (transform, image) in [
        (Dihedral::Rotate90, rotate90(view)),
        (Dihedral::MirrorHorizontal, flip_horizontal(view)),
        (Dihedral::MirrorVertical, flip_vertical(view)),
        (Dihedral::Transpose, transpose(view)),
    ] {
        assert!(
            all.get(transform).distance(&oracle_luma(&image).0) <= 2,
            "{transform:?}"
        );
    }
}

#[test]
fn identity_variant_equals_pdq() {
    let luma = grey(&scene(123, 77, 3));
    let all = pdq_dihedral(luma.view()).unwrap();
    assert_eq!(all.pdq(), pdq(luma.view()).unwrap());
}

#[test]
fn hash_has_half_the_bits_set() {
    for seed in 0..30 {
        let hash = hash_of(&grey(&scene(200, 150, seed + 40)));
        assert_eq!(hash.count_ones(), 128, "seed {seed}");
    }
}

#[test]
fn jpeg_recompression_keeps_hash_close() {
    for seed in 0..10 {
        let rgb = scene(640, 480, seed + 70);
        let original = hash_of(&grey(&rgb));
        for (quality, bound) in [(90u8, 12u32), (70, 18), (40, 28)] {
            let copy = jpeg_round_trip(&rgb, quality);
            let distance = original.distance(&hash_of(&grey(&copy)));
            assert!(
                distance <= bound,
                "seed {seed} q{quality} distance {distance}"
            );
        }
    }
}

#[test]
fn double_recompression_and_downscale_stay_close() {
    let rgb = scene(1024, 768, 5);
    let original = hash_of(&grey(&rgb));
    let small = resize_area(jpeg_round_trip(&rgb, 80).view(), 400, 300).unwrap();
    let copy = jpeg_round_trip(&small, 60);
    assert!(original.distance(&hash_of(&grey(&copy))) <= 24);
}

#[test]
fn unrelated_images_are_far_apart() {
    let hashes: Vec<PdqHash> = (0..16)
        .map(|seed| hash_of(&grey(&scene(256, 192, seed + 300))))
        .collect();
    for i in 0..hashes.len() {
        for j in i + 1..hashes.len() {
            assert!(hashes[i].distance(&hashes[j]) >= 60, "{i} {j}");
        }
    }
}

#[test]
fn brightness_offset_without_clipping_is_nearly_invariant() {
    let rgb = map_pixels(&scene(320, 240, 8), |v| v / 2 + 40);
    let brighter = map_pixels(&rgb, |v| v + 30);
    assert!(hash_of(&grey(&rgb)).distance(&hash_of(&grey(&brighter))) <= 4);
}

#[test]
fn quality_bounds() {
    assert_eq!(
        pdq(Luma8::filled(100, 100, [128]).unwrap().view())
            .unwrap()
            .quality,
        0
    );
    let busy = pdq(noise_luma(256, 256, 9).view()).unwrap();
    assert_eq!(busy.quality, MAX_QUALITY);
    assert!(busy.indexable());
    let faint = Luma8::from_fn(256, 256, |x, _| [100 + (x / 128) as u8]).unwrap();
    let faint = pdq(faint.view()).unwrap();
    assert!(faint.quality < MIN_INDEX_QUALITY);
    assert!(!faint.indexable());
}

#[test]
fn quality_metric_matches_reference_formula() {
    let mut rng = Rng::new(4);
    for _ in 0..20 {
        let buffer: Vec<u16> = (0..PDQ_SIDE * PDQ_SIDE)
            .map(|_| (rng.below(256) * 256) as u16)
            .collect();
        let pixel = |i: usize, j: usize| (buffer[i * 64 + j] / 256) as f32;
        let mut sum = 0i32;
        for i in 0..63 {
            for j in 0..64 {
                sum += (((pixel(i, j) - pixel(i + 1, j)) * 100.0 / 255.0) as i32).abs();
                sum += (((pixel(j, i) - pixel(j, i + 1)) * 100.0 / 255.0) as i32).abs();
            }
        }
        assert_eq!(quality(&buffer) as i32, (sum / 90).min(100));
    }
}

#[test]
fn rejects_images_below_minimum_dimension() {
    assert!(pdq(Luma8::filled(4, 100, [9]).unwrap().view()).is_none());
    assert!(pdq(Luma8::filled(100, 4, [9]).unwrap().view()).is_none());
    assert!(pdq_dihedral(Luma8::filled(1, 1, [9]).unwrap().view()).is_none());
    assert!(pdq(noise_luma(5, 5, 1).view()).is_some());
}

#[test]
fn constant_image_hashes_to_zero() {
    let hash = hash_of(&Luma8::filled(80, 80, [200]).unwrap());
    assert_eq!(hash, PdqHash::default());
}

#[test]
fn crop_view_matches_materialised_crop() {
    let luma = grey(&scene(300, 300, 12));
    let view = luma.crop(17, 41, 200, 133).unwrap();
    assert_eq!(pdq(view), pdq(view.to_image().view()));
}

#[test]
fn deterministic_across_calls() {
    let luma = noise_luma(517, 391, 2);
    let first = pdq_dihedral(luma.view()).unwrap();
    for _ in 0..3 {
        assert_eq!(pdq_dihedral(luma.view()).unwrap(), first);
    }
}

#[test]
fn golden_hash_is_stable() {
    let hash = hash_of(&grey(&scene(97, 61, 4242)));
    assert_eq!(hash, hash_of(&grey(&scene(97, 61, 4242))));
    let ramp = Luma8::from_fn(64, 64, |x, y| [(x * 3 + y * y / 20) as u8]).unwrap();
    assert!(hash_of(&ramp).distance(&oracle_luma(&ramp).0) <= 2);
}
