mod common;

use common::*;
use witness_image::{Rgb8, resize_area, to_luma};
use witness_match::{
    AlignParams, Classification, DiffParams, ExtractParams, PixelDiff, Rect, align, diff,
    extract_photo_regions,
};

fn screenshot(photo: &Rgb8, slot: Rect) -> Rgb8 {
    let placed = resize_area(photo.view(), slot.width, slot.height).unwrap();
    let mut shot = Rgb8::filled(540, 1170, [255, 255, 255]).unwrap();
    for y in 0..slot.height {
        let row = shot.row_mut(slot.y + y);
        row[slot.x as usize * 3..slot.right() as usize * 3].copy_from_slice(placed.row(y));
    }
    for (i, y) in [20u32, 70, 560, 600, 1130].into_iter().enumerate() {
        for x in (16..400).step_by(7) {
            if !(x + i).is_multiple_of(3) {
                for yy in y..y + 12 {
                    shot.row_mut(yy)[x * 3..x * 3 + 6].copy_from_slice(&[20, 20, 20, 20, 20, 20]);
                }
            }
        }
    }
    jpeg(&shot, 85)
}

fn check(shot: &Rgb8, reference: &Rgb8) -> PixelDiff {
    let candidates = extract_photo_regions(shot.view(), &ExtractParams::default());
    let top = &candidates[0];
    let rect = top.rect;
    let query = shot
        .crop(rect.x, rect.y, rect.width, rect.height)
        .unwrap()
        .to_image();
    let mask = top.overlay_mask.relative_to(rect.x as f64, rect.y as f64);
    let a = align(
        to_luma(query.view()).view(),
        to_luma(reference.view()).view(),
        &AlignParams::default(),
        &mut rng(3),
    )
    .expect("aligned");
    assert!(a.confidence > 0.3, "{}", a.confidence);
    diff(
        query.view(),
        reference.view(),
        &a.transform,
        Some(&mask),
        &[],
        &DiffParams::default(),
    )
}

const SLOT: Rect = Rect::new(0, 120, 540, 405);

#[test]
fn screenshot_of_unedited_photo_is_consistent() {
    for seed in 0..3 {
        let reference = scene(640, 480, 200 + seed);
        let d = check(&screenshot(&filter(&reference), SLOT), &reference);
        assert_eq!(
            d.assessment.classification,
            Classification::Consistent,
            "seed {seed}"
        );
        assert!(d.assessment.compared_fraction > 0.8);
    }
}

#[test]
fn screenshot_of_edited_photo_is_local() {
    let reference = scene(640, 480, 210);
    let edit = Rect::new(300, 200, 56, 48);
    let d = check(
        &screenshot(&paste_patch(&reference, edit, 999), SLOT),
        &reference,
    );
    let Classification::Local(regions) = &d.assessment.classification else {
        panic!("{:?}", d.assessment.classification)
    };
    assert_eq!(regions.len(), 1);
    assert!(iou(&regions[0].rect, &edit) > 0.5, "{:?}", regions[0].rect);
}

#[test]
fn screenshot_of_mirrored_crop_is_consistent() {
    let reference = scene(640, 480, 220);
    let cropped = flip(&crop_scale(
        &reference,
        Rect::new(80, 60, 480, 360),
        480,
        360,
    ));
    let d = check(&screenshot(&cropped, SLOT), &reference);
    assert_eq!(d.assessment.classification, Classification::Consistent);
    assert!(
        (0.4..0.7).contains(&d.assessment.compared_fraction),
        "{}",
        d.assessment.compared_fraction
    );
}
