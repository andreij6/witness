mod common;

use common::*;
use witness_image::{Rgb8, rotate90, to_luma};
use witness_match::{AlignParams, Alignment, Rect, Similarity, align};

fn run(query: &Rgb8, reference: &Rgb8, seed: u64) -> Option<Alignment> {
    align(
        to_luma(query.view()).view(),
        to_luma(reference.view()).view(),
        &AlignParams::default(),
        &mut rng(seed),
    )
}

fn check(query: &Rgb8, reference: &Rgb8, truth: &Similarity, tolerance: f64) -> Alignment {
    let a = run(query, reference, 1).expect("alignment");
    let error = max_point_error(
        &a.transform,
        truth,
        reference.width() as f64,
        reference.height() as f64,
    );
    assert!(
        error < tolerance,
        "error {error} transform {:?} truth {truth:?} inliers {} / {}",
        a.transform,
        a.inliers,
        a.matches
    );
    assert_eq!(a.transform.mirrored, truth.mirrored);
    a
}

#[test]
fn identical_copy_aligns_to_identity() {
    let img = scene(512, 384, 1);
    let a = check(&img, &img, &Similarity::IDENTITY, 1.0);
    assert!(a.confidence > 0.8, "{}", a.confidence);
    assert!(a.inliers >= 50);
}

#[test]
fn downscaled_copy_is_recovered() {
    let img = scene(640, 480, 2);
    let rect = Rect::new(0, 0, 640, 480);
    let query = crop_scale(&img, rect, 400, 300);
    check(&query, &img, &crop_scale_truth(rect, 400), 2.0);
}

#[test]
fn upscaled_copy_is_recovered() {
    let img = scene(400, 300, 3);
    let rect = Rect::new(0, 0, 400, 300);
    let query = crop_scale(&img, rect, 560, 420);
    check(&query, &img, &crop_scale_truth(rect, 560), 3.0);
}

#[test]
fn center_crop_is_recovered() {
    let img = scene(640, 480, 4);
    let rect = Rect::new(120, 80, 400, 300);
    let query = crop_scale(&img, rect, 400, 300);
    check(&query, &img, &crop_scale_truth(rect, 400), 2.0);
}

#[test]
fn offset_crop_with_scale_is_recovered() {
    let img = scene(640, 480, 5);
    let rect = Rect::new(200, 40, 360, 360);
    let query = crop_scale(&img, rect, 480, 480);
    check(&query, &img, &crop_scale_truth(rect, 480), 3.0);
}

#[test]
fn horizontal_flip_is_recovered_as_mirrored() {
    let img = scene(512, 384, 6);
    let query = flip(&img);
    let truth = Similarity::IDENTITY.mirror_x(512.0);
    let a = check(&query, &img, &truth, 1.5);
    assert!(a.confidence > 0.5);
}

#[test]
fn flipped_crop_is_recovered() {
    let img = scene(640, 480, 7);
    let rect = Rect::new(100, 60, 420, 360);
    let query = flip(&crop_scale(&img, rect, 420, 360));
    let truth = crop_scale_truth(rect, 420).mirror_x(420.0);
    check(&query, &img, &truth, 2.0);
}

#[test]
fn quarter_rotation_is_recovered() {
    let img = scene(480, 360, 8);
    let query = rotate90(img.view());
    let truth = Similarity::new(0.0, 1.0, 360.0, 0.0, false);
    let (x, y) = truth.apply(0.5, 0.5);
    assert_eq!((x, y), (359.5, 0.5));
    check(&query, &img, &truth, 1.5);
}

#[test]
fn recompressed_copy_is_recovered() {
    let img = scene(512, 384, 9);
    let query = jpeg(&crop_scale(&img, Rect::new(0, 0, 512, 384), 448, 336), 70);
    check(
        &query,
        &img,
        &crop_scale_truth(Rect::new(0, 0, 512, 384), 448),
        2.0,
    );
}

#[test]
fn filtered_and_vignetted_copy_is_recovered() {
    let img = scene(512, 384, 10);
    let query = vignette(&filter(&img), 0.3);
    check(&query, &img, &Similarity::IDENTITY, 1.5);
}

#[test]
fn large_reference_uses_working_resolution() {
    let img = scene(1200, 900, 11);
    let rect = Rect::new(300, 200, 600, 450);
    let query = crop_scale(&img, rect, 600, 450);
    check(&query, &img, &crop_scale_truth(rect, 600), 4.0);
}

#[test]
fn unrelated_images_do_not_confirm() {
    let a = scene(512, 384, 12);
    let b = scene(512, 384, 13);
    let result = run(&a, &b, 1);
    assert!(
        result.is_none_or(|r| r.confidence < 0.2 && r.inliers < 20),
        "{result:?}"
    );
}

#[test]
fn alignment_is_deterministic() {
    let img = scene(512, 384, 14);
    let rect = Rect::new(50, 30, 400, 300);
    let query = flip(&crop_scale(&img, rect, 400, 300));
    assert_eq!(run(&query, &img, 5), run(&query, &img, 5));
}

#[test]
fn tiny_inputs_do_not_panic() {
    let tiny = Rgb8::filled(3, 2, [9, 9, 9]).unwrap();
    let img = scene(256, 192, 15);
    assert_eq!(run(&tiny, &img, 1), None);
    assert_eq!(run(&img, &tiny, 1), None);
    let strip = scene(2000, 4, 16);
    assert_eq!(run(&strip, &img, 1), None);
}

#[test]
fn mirrored_hypothesis_can_be_disabled() {
    let img = scene(512, 384, 17);
    let query = flip(&img);
    let params = AlignParams {
        try_mirrored: false,
        ..AlignParams::default()
    };
    let result = align(
        to_luma(query.view()).view(),
        to_luma(img.view()).view(),
        &params,
        &mut rng(1),
    );
    assert!(result.is_none_or(|r| !r.transform.mirrored && r.confidence < 0.5));
}
