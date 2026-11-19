mod common;

use common::*;
use witness_image::{Rgb8, to_luma};
use witness_match::{
    AlignParams, CellState, ChangeMap, Classification, DiffParams, Inconclusive, OverlayMask,
    PixelDiff, Rect, Similarity, align, diff,
};

const W: u32 = 512;
const H: u32 = 384;

fn aligned_diff(query: &Rgb8, reference: &Rgb8, blurs: &[Rect]) -> PixelDiff {
    let a = align(
        to_luma(query.view()).view(),
        to_luma(reference.view()).view(),
        &AlignParams::default(),
        &mut rng(7),
    )
    .expect("alignment");
    diff(
        query.view(),
        reference.view(),
        &a.transform,
        None,
        blurs,
        &DiffParams::default(),
    )
}

fn identity_diff(query: &Rgb8, reference: &Rgb8, blurs: &[Rect]) -> PixelDiff {
    diff(
        query.view(),
        reference.view(),
        &Similarity::IDENTITY,
        None,
        blurs,
        &DiffParams::default(),
    )
}

fn assert_local(d: &PixelDiff, edits: &[Rect]) {
    let Classification::Local(regions) = &d.assessment.classification else {
        panic!(
            "expected local, got {:?} (changed {})",
            d.assessment.classification, d.assessment.changed_fraction
        );
    };
    assert_eq!(regions.len(), edits.len(), "{regions:?}");
    for edit in edits {
        assert!(
            regions
                .iter()
                .any(|r| iou(&r.rect, edit) > 0.5 && r.rect.contains_rect(edit)),
            "{edit:?} not in {regions:?}"
        );
    }
}

fn assert_consistent(d: &PixelDiff) {
    assert_eq!(
        d.assessment.classification,
        Classification::Consistent,
        "changed {}",
        d.assessment.changed_fraction
    );
}

#[test]
fn identical_copy_has_no_change() {
    let img = scene(W, H, 1);
    let d = identity_diff(&img, &img, &[]);
    assert_consistent(&d);
    assert_eq!(d.assessment.changed_fraction, 0.0);
    assert!(d.assessment.compared_fraction > 0.9);
}

#[test]
fn aligned_identical_copy_has_no_change() {
    let img = scene(W, H, 2);
    assert_consistent(&aligned_diff(&img, &img, &[]));
}

#[test]
fn recompressed_copy_is_not_flagged() {
    let img = scene(W, H, 3);
    for quality in [60, 75, 90] {
        let d = identity_diff(&jpeg(&img, quality), &img, &[]);
        assert_consistent(&d);
        assert!(
            d.assessment.changed_fraction < 0.02,
            "{quality} {}",
            d.assessment.changed_fraction
        );
    }
}

#[test]
fn filtered_copy_is_not_flagged() {
    let img = scene(W, H, 4);
    assert_consistent(&identity_diff(&filter(&img), &img, &[]));
}

#[test]
fn vignetted_copy_is_not_flagged() {
    let img = scene(W, H, 5);
    assert_consistent(&identity_diff(&vignette(&img, 0.45), &img, &[]));
}

#[test]
fn filtered_vignetted_recompressed_copy_is_not_flagged() {
    let img = scene(W, H, 6);
    assert_consistent(&identity_diff(
        &jpeg(&vignette(&filter(&img), 0.35), 80),
        &img,
        &[],
    ));
}

#[test]
fn scaled_copy_is_not_flagged_after_alignment() {
    let img = scene(W, H, 7);
    let query = jpeg(&crop_scale(&img, Rect::new(0, 0, W, H), 400, 300), 85);
    assert_consistent(&aligned_diff(&query, &img, &[]));
}

#[test]
fn cropped_copy_compares_only_overlap() {
    let img = scene(W, H, 8);
    let rect = Rect::new(96, 64, 320, 240);
    let query = crop_scale(&img, rect, 320, 240);
    let d = aligned_diff(&query, &img, &[]);
    assert_consistent(&d);
    let expected = rect.area() as f64 / (W * H) as f64;
    assert!(
        d.assessment.compared_fraction < expected
            && d.assessment.compared_fraction > expected * 0.85,
        "{}",
        d.assessment.compared_fraction
    );
    assert_eq!(d.map.state(10, 10), CellState::Uncompared);
}

#[test]
fn flipped_copy_is_not_flagged() {
    let img = scene(W, H, 9);
    assert_consistent(&aligned_diff(&flip(&img), &img, &[]));
}

#[test]
fn pasted_local_edit_is_flagged_with_region() {
    let img = scene(W, H, 10);
    let edit = Rect::new(220, 150, 48, 40);
    let d = identity_diff(&paste_edit(&img, edit, 1), &img, &[]);
    assert_local(&d, &[edit]);
}

#[test]
fn small_plate_sized_edit_is_flagged() {
    let img = scene(W, H, 11);
    let edit = Rect::new(300, 200, 36, 18);
    assert_local(
        &identity_diff(&paste_edit(&img, edit, 2), &img, &[]),
        &[edit],
    );
}

#[test]
fn edit_survives_recompression_and_filter() {
    let img = scene(W, H, 12);
    let edit = Rect::new(150, 120, 50, 40);
    let query = jpeg(&filter(&paste_edit(&img, edit, 3)), 80);
    assert_local(&identity_diff(&query, &img, &[]), &[edit]);
}

#[test]
fn edit_in_cropped_scaled_copy_maps_to_reference() {
    let img = scene(W, H, 13);
    let edit = Rect::new(240, 170, 48, 40);
    let edited = paste_edit(&img, edit, 4);
    let query = jpeg(
        &crop_scale(&edited, Rect::new(64, 48, 400, 300), 480, 360),
        85,
    );
    assert_local(&aligned_diff(&query, &img, &[]), &[edit]);
}

#[test]
fn edit_in_flipped_copy_is_located() {
    let img = scene(W, H, 14);
    let edit = Rect::new(120, 200, 48, 44);
    let query = flip(&paste_edit(&img, edit, 5));
    assert_local(&aligned_diff(&query, &img, &[]), &[edit]);
}

#[test]
fn several_edits_are_each_reported() {
    let img = scene(W, H, 15);
    let edits = [
        Rect::new(80, 80, 40, 40),
        Rect::new(300, 90, 44, 36),
        Rect::new(200, 260, 40, 40),
    ];
    let mut query = img.clone();
    for (i, e) in edits.iter().enumerate() {
        query = paste_edit(&query, *e, 10 + i as u64);
    }
    assert_local(&identity_diff(&query, &img, &[]), &edits);
}

#[test]
fn many_edits_are_inconclusive() {
    let img = scene(W, H, 16);
    let mut query = img.clone();
    for i in 0..8u32 {
        query = paste_edit(
            &query,
            Rect::new(60 + (i % 4) * 100, 80 + (i / 4) * 160, 36, 36),
            20 + i as u64,
        );
    }
    let d = identity_diff(&query, &img, &[]);
    assert_eq!(
        d.assessment.classification,
        Classification::Inconclusive(Inconclusive::TooManyComponents(8))
    );
}

#[test]
fn edit_in_border_band_is_inconclusive() {
    let img = scene(W, H, 17);
    let edit = Rect::new(4, 150, 40, 40);
    let d = identity_diff(&paste_edit(&img, edit, 6), &img, &[]);
    assert_eq!(
        d.assessment.classification,
        Classification::Inconclusive(Inconclusive::InBorderBand)
    );
}

#[test]
fn wholesale_change_cannot_compare() {
    let a = scene(W, H, 18);
    let b = scene(W, H, 19);
    let d = identity_diff(&a, &b, &[]);
    assert_eq!(d.assessment.classification, Classification::CouldNotCompare);
    assert!(d.assessment.changed_fraction > 0.35);
}

#[test]
fn tiny_edits_are_ignored() {
    let img = scene(W, H, 20);
    let d = identity_diff(&paste_edit(&img, Rect::new(250, 180, 6, 6), 7), &img, &[]);
    assert_consistent(&d);
    assert!(d.assessment.changed_fraction > 0.0);
}

#[test]
fn declared_blur_that_is_blurred_is_explained() {
    let face = Rect::new(200, 140, 64, 56);
    let img = paste_edit(&scene(W, H, 21), face, 30);
    let query = box_blur(&img, face, 6);
    let d = identity_diff(&query, &img, &[face]);
    assert!(d.blurs[0].holds, "{:?}", d.blurs[0]);
    assert_consistent(&d);
    assert_eq!(d.assessment.explained, 1);
}

#[test]
fn undeclared_blur_is_a_local_change() {
    let face = Rect::new(200, 140, 64, 56);
    let img = paste_edit(&scene(W, H, 22), face, 31);
    let d = identity_diff(&box_blur(&img, face, 6), &img, &[]);
    assert!(
        matches!(d.assessment.classification, Classification::Local(_)),
        "{:?}",
        d.assessment.classification
    );
}

#[test]
fn declared_blur_hiding_an_edit_is_not_explained() {
    let img = scene(W, H, 23);
    let rect = Rect::new(200, 140, 64, 56);
    let d = identity_diff(&paste_edit(&img, rect, 8), &img, &[rect]);
    assert!(!d.blurs[0].holds, "{:?}", d.blurs[0]);
    assert_local(&d, &[rect]);
}

#[test]
fn declared_blur_against_redacted_reference_holds() {
    let img = scene(W, H, 24);
    let face = Rect::new(150, 100, 60, 60);
    let redacted = box_blur(&img, face, 6);
    let d = identity_diff(&jpeg(&redacted, 85), &redacted, &[face]);
    assert!(d.blurs[0].holds, "{:?}", d.blurs[0]);
    assert_consistent(&d);
}

#[test]
fn blur_declared_elsewhere_does_not_explain_edit() {
    let img = scene(W, H, 25);
    let edit = Rect::new(300, 200, 48, 40);
    let blurred = Rect::new(80, 80, 60, 60);
    let query = box_blur(&paste_edit(&img, edit, 9), blurred, 6);
    let d = identity_diff(&query, &img, &[blurred]);
    assert_local(&d, &[edit]);
    assert_eq!(d.assessment.explained, 1);
}

#[test]
fn overlay_mask_excludes_story_text() {
    let img = scene(W, H, 26);
    let text = Rect::new(150, 160, 200, 40);
    let query = paste_edit(&img, text, 11);
    let unmasked = identity_diff(&query, &img, &[]);
    assert!(matches!(
        unmasked.assessment.classification,
        Classification::Local(_)
    ));
    let blocks: Vec<bool> = (0..64 * 48)
        .map(|i| {
            text.expand(8, 8, W, H)
                .contains((i % 64) * 8 + 4, (i / 64) * 8 + 4)
        })
        .collect();
    let mask = OverlayMask::new((0.0, 0.0), (8.0, 8.0), 64, 48, blocks).unwrap();
    let masked = diff(
        query.view(),
        img.view(),
        &Similarity::IDENTITY,
        Some(&mask),
        &[],
        &DiffParams::default(),
    );
    assert_consistent(&masked);
    assert_eq!(masked.map.state(250, 180), CellState::Uncompared);
}

#[test]
fn diff_is_deterministic() {
    let img = scene(W, H, 27);
    let query = jpeg(
        &paste_edit(&vignette(&img, 0.3), Rect::new(200, 150, 40, 40), 12),
        75,
    );
    assert_eq!(
        identity_diff(&query, &img, &[]),
        identity_diff(&query, &img, &[])
    );
}

#[test]
fn larger_window_radius_also_detects_edits() {
    let img = scene(W, H, 28);
    let edit = Rect::new(220, 150, 48, 40);
    let params = DiffParams {
        window_radius: 5,
        ..DiffParams::default()
    };
    let d = diff(
        paste_edit(&img, edit, 13).view(),
        img.view(),
        &Similarity::IDENTITY,
        None,
        &[],
        &params,
    );
    assert_local(&d, &[edit]);
}

#[test]
fn seeded_random_edits_property() {
    let img = scene(W, H, 29);
    let mut r = rng(99);
    use witness_match::Rng;
    for case in 0..6 {
        let (w, h) = (36 + r.below(30), 30 + r.below(30));
        let edit = Rect::new(48 + r.below(W - 96 - w), 48 + r.below(H - 96 - h), w, h);
        let query = jpeg(&paste_edit(&img, edit, 100 + case), 85);
        assert_local(&identity_diff(&query, &img, &[]), &[edit]);
    }
}

#[test]
fn seeded_unedited_copies_property() {
    for seed in 40..46 {
        let img = scene(W, H, seed);
        let query = jpeg(
            &vignette(&filter(&img), 0.1 + (seed - 40) as f64 * 0.06),
            70 + (seed as u8 - 40) * 4,
        );
        assert_consistent(&identity_diff(&query, &img, &[]));
    }
}

fn best_of_hypotheses(query: &Rgb8, reference: &Rgb8) -> PixelDiff {
    let a = align(
        to_luma(query.view()).view(),
        to_luma(reference.view()).view(),
        &AlignParams::default(),
        &mut rng(7),
    )
    .expect("alignment");
    let resize =
        Similarity::resize(reference.dimensions(), query.dimensions()).expect("same aspect");
    assert!(resize.max_corner_shift(&a.transform, reference.width(), reference.height()) < 6.0);
    [resize, a.transform]
        .iter()
        .map(|t| {
            diff(
                query.view(),
                reference.view(),
                t,
                None,
                &[],
                &DiffParams::default(),
            )
        })
        .min_by(|x, y| {
            x.assessment
                .changed_fraction
                .total_cmp(&y.assessment.changed_fraction)
        })
        .unwrap()
}

#[test]
fn small_frame_recompressions_are_consistent_across_sizes_and_qualities() {
    for (i, (w, h)) in [
        (96u32, 72u32),
        (72, 96),
        (128, 96),
        (160, 120),
        (192, 144),
        (256, 192),
    ]
    .into_iter()
    .enumerate()
    {
        for q in [50u8, 65, 80, 95] {
            let original = jpeg(&scene(w, h, 60 + i as u64), 92);
            let reference = jpeg(&original, 85);
            let query = jpeg(&original, q);
            assert_consistent(&identity_diff(&query, &reference, &[]));
            assert_consistent(&best_of_hypotheses(&query, &reference));
        }
    }
}

#[test]
fn resize_hypothesis_fixes_large_frame_alignment_jitter() {
    let original = jpeg(&scene(1024, 768, 70), 92);
    let reference = jpeg(&original, 85);
    assert_consistent(&best_of_hypotheses(&jpeg(&original, 50), &reference));
    let half = crop_scale(&original, Rect::new(0, 0, 1024, 768), 512, 384);
    assert_consistent(&best_of_hypotheses(&jpeg(&half, 60), &reference));
}

#[test]
fn small_frame_edit_above_codec_floor_is_local() {
    let img = scene(192, 144, 71);
    let edit = Rect::new(80, 56, 24, 24);
    assert_local(
        &identity_diff(&jpeg(&paste_edit(&img, edit, 3), 80), &img, &[]),
        &[edit],
    );
}

#[test]
fn small_frame_codec_sized_edit_is_not_an_accusation() {
    let img = scene(96, 72, 72);
    let d = identity_diff(
        &jpeg(&paste_edit(&img, Rect::new(40, 30, 8, 8), 4), 80),
        &img,
        &[],
    );
    assert!(!matches!(
        d.assessment.classification,
        Classification::Local(_)
    ));
}
