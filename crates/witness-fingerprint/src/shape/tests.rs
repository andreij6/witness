use super::*;
use crate::pdq::pdq;
use crate::testkit::{grey, jpeg_round_trip, scene};
use witness_image::{Rgb8, flip_horizontal};

fn rect(x: u32, y: u32, w: u32, h: u32) -> Rect {
    Rect { x, y, w, h }
}

#[test]
fn full_tag_is_stable() {
    assert_eq!(ShapeTag::FULL.get(), 1);
    assert_eq!(ShapeTag::FULL.aspect(), Aspect::Full);
    assert_eq!(ShapeTag::FULL.anchor(), Anchor::Center);
    assert_eq!(ShapeTag::new(Aspect::Square, Anchor::Start).get(), 4);
    assert_eq!(
        ShapeTag::new(Aspect::Landscape191x100, Anchor::End).get(),
        26
    );
}

#[test]
fn from_u8_accepts_exactly_the_valid_tags() {
    let valid: Vec<u8> = (0..=255u8)
        .filter(|&v| ShapeTag::from_u8(v).is_some())
        .collect();
    assert_eq!(valid.len(), 1 + 6 * 3);
    for value in valid {
        let tag = ShapeTag::from_u8(value).unwrap();
        assert_eq!(ShapeTag::new(tag.aspect(), tag.anchor()), tag);
    }
    assert!(ShapeTag::from_u8(0).is_none());
    assert!(ShapeTag::from_u8(2).is_none());
    assert!(ShapeTag::from_u8(7).is_none());
    assert!(ShapeTag::from_u8(28).is_none());
}

#[test]
fn landscape_photo_has_nineteen_shapes() {
    let tags = stored_shape_tags(4032, 3024);
    assert_eq!(tags.len(), 19);
    assert!(tags.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn portrait_photo_skips_its_own_aspect() {
    let tags = stored_shape_tags(3024, 4032);
    assert_eq!(tags.len(), 16);
    assert!(tags.iter().all(|t| t.aspect() != Aspect::Portrait3x4));
}

#[test]
fn near_matching_aspect_keeps_only_center() {
    let tags = stored_shape_tags(1000, 590);
    let wide: Vec<_> = tags
        .iter()
        .filter(|t| t.aspect() == Aspect::Landscape16x9)
        .collect();
    assert_eq!(wide.len(), 1);
    assert_eq!(wide[0].anchor(), Anchor::Center);
}

#[test]
fn shape_counts_stay_in_documented_band_for_common_frames() {
    for (w, h) in [
        (4032, 3024),
        (3024, 4032),
        (4000, 2250),
        (2250, 4000),
        (3000, 3000),
        (1080, 1350),
        (1920, 1080),
    ] {
        let n = stored_shape_tags(w, h).len();
        assert!((13..=MAX_STORED_SHAPES).contains(&n), "{w}x{h}: {n}");
        assert!(n <= witness_format::fingerprint::MAX_PDQ_SHAPES);
    }
    assert!(stored_shape_tags(0, 10).is_empty());
    assert_eq!(stored_shape_tags(1, 1), vec![ShapeTag::FULL]);
}

#[test]
fn rects_fit_frame_and_match_aspect() {
    for (w, h) in [
        (4032u32, 3024u32),
        (3024, 4032),
        (641, 479),
        (7, 3000),
        (3000, 7),
        (1, 1),
        (512, 512),
    ] {
        for tag in stored_shape_tags(w, h) {
            let r = tag.rect(w, h).unwrap();
            assert!(
                r.w > 0 && r.h > 0 && r.x + r.w <= w && r.y + r.h <= h,
                "{w}x{h} {tag:?} {r:?}"
            );
            if let Some((num, den)) = tag.aspect().ratio() {
                let error =
                    (r.w as f64 * den as f64 - r.h as f64 * num as f64).abs() / num.max(den) as f64;
                assert!(error <= 1.0, "{w}x{h} {tag:?} {r:?}");
            }
        }
    }
}

#[test]
fn anchors_place_start_center_end() {
    let start = ShapeTag::new(Aspect::Square, Anchor::Start)
        .rect(400, 300)
        .unwrap();
    let center = ShapeTag::new(Aspect::Square, Anchor::Center)
        .rect(400, 300)
        .unwrap();
    let end = ShapeTag::new(Aspect::Square, Anchor::End)
        .rect(400, 300)
        .unwrap();
    assert_eq!(start, rect(0, 0, 300, 300));
    assert_eq!(center, rect(50, 0, 300, 300));
    assert_eq!(end, rect(100, 0, 300, 300));
    let tall = ShapeTag::new(Aspect::Landscape16x9, Anchor::End)
        .rect(900, 1600)
        .unwrap();
    assert_eq!(tall, rect(0, 1094, 900, 506));
    assert!(ShapeTag::FULL.rect(0, 5).is_none());
}

#[test]
fn fit_long_edge_preserves_aspect() {
    assert_eq!(fit_long_edge(4032, 3024, 512), (512, 384));
    assert_eq!(fit_long_edge(3024, 4032, 512), (384, 512));
    assert_eq!(fit_long_edge(300, 200, 512), (300, 200));
    assert_eq!(fit_long_edge(10_000, 1, 512), (512, 1));
    assert_eq!(fit_long_edge(512, 512, 256), (256, 256));
}

#[test]
fn working_luma_downsamples_large_images_only() {
    assert_eq!(
        working_luma(scene(1024, 600, 1).view()).dimensions(),
        (512, 300)
    );
    let small = scene(200, 100, 1);
    assert_eq!(working_luma(small.view()), grey(&small));
}

#[test]
fn stored_shapes_cover_every_tag_and_full_equals_pdq() {
    let work = working_luma(scene(1600, 1200, 9).view());
    let shapes = stored_shapes(work.view());
    let tags: Vec<_> = shapes.iter().map(|s| s.tag).collect();
    assert_eq!(tags, stored_shape_tags(512, 384));
    assert_eq!(shapes[0].pdq, pdq(work.view()).unwrap());
    let format: Vec<_> = shapes.iter().map(ShapeHash::to_format).collect();
    assert_eq!(format[0].tag, 1);
    assert_eq!(format[0].hash, shapes[0].pdq.hash.0);
    assert!(shapes.iter().all(|s| s.pdq.quality > 0));
}

#[test]
fn stored_shapes_of_tiny_image_skip_unhashable_crops() {
    let work = working_luma(scene(30, 5, 2).view());
    let shapes = stored_shapes(work.view());
    assert!(!shapes.is_empty() && shapes.len() < stored_shape_tags(30, 5).len());
    assert!(shapes.iter().all(|s| s.tag.rect(30, 5).unwrap().w >= 5));
    assert!(stored_shapes(working_luma(scene(4, 4, 2).view()).view()).is_empty());
}

#[test]
fn platform_crop_matches_its_stored_shape() {
    for seed in 0..6 {
        let original = scene(1600, 1200, seed + 30);
        let shapes = stored_shapes(working_luma(original.view()).view());
        for tag in [
            ShapeTag::new(Aspect::Square, Anchor::Center),
            ShapeTag::new(Aspect::Portrait4x5, Anchor::Start),
        ] {
            let r = tag.rect(1600, 1200).unwrap();
            let crop = jpeg_round_trip(&original.crop(r.x, r.y, r.w, r.h).unwrap().to_image(), 75);
            let full = rect(0, 0, crop.width(), crop.height());
            let base = query_hashes(crop.view(), &[full])[0].hash;
            let stored = shapes.iter().find(|s| s.tag == tag).unwrap().pdq.hash;
            assert!(
                base.distance(&stored) <= 31,
                "seed {seed} {tag:?} distance {}",
                base.distance(&stored)
            );
        }
    }
}

#[test]
fn mirrored_copy_is_found_by_mirror_variant() {
    let original = scene(800, 600, 3);
    let stored = stored_shapes(working_luma(original.view()).view())[0]
        .pdq
        .hash;
    let mirrored = flip_horizontal(original.view());
    let queries = query_hashes(mirrored.view(), &[rect(0, 0, 800, 600)]);
    let mirror = queries
        .iter()
        .find(|q| q.variant == QueryVariant::Mirror)
        .unwrap();
    let base = queries
        .iter()
        .find(|q| q.variant == QueryVariant::Base)
        .unwrap();
    assert!(
        mirror.hash.distance(&stored) <= 31,
        "{}",
        mirror.hash.distance(&stored)
    );
    assert!(base.hash.distance(&stored) > 40);
}

#[test]
fn query_emits_base_mirror_inset_outset() {
    let image = scene(1000, 800, 4);
    let queries = query_hashes(image.view(), &[rect(100, 100, 600, 500)]);
    let variants: Vec<_> = queries.iter().map(|q| q.variant).collect();
    assert_eq!(
        variants,
        vec![
            QueryVariant::Base,
            QueryVariant::Mirror,
            QueryVariant::Inset,
            QueryVariant::Outset
        ]
    );
    assert!(queries.iter().all(|q| q.rect_index == 0));
    let work = working_luma(image.view());
    let inset = pdq(work.crop(58, 57, 294, 245).unwrap()).unwrap().hash;
    let outset = pdq(work.crop(44, 45, 322, 269).unwrap()).unwrap().hash;
    assert_eq!(queries[2].hash, inset);
    assert_eq!(queries[3].hash, outset);
}

#[test]
fn full_frame_outset_is_skipped() {
    let image = scene(300, 200, 5);
    let queries = query_hashes(image.view(), &[rect(0, 0, 300, 200)]);
    assert_eq!(queries.len(), 3);
    assert!(queries.iter().all(|q| q.variant != QueryVariant::Outset));
}

#[test]
fn query_caps_at_sixteen_hashes() {
    let image = scene(1200, 900, 6);
    let rects: Vec<Rect> = (0..8).map(|i| rect(i * 60, i * 40, 500, 400)).collect();
    let queries = query_hashes(image.view(), &rects);
    assert_eq!(queries.len(), MAX_QUERY_HASHES);
    assert_eq!(queries.last().unwrap().rect_index, 3);
    for i in 0..queries.len() {
        for j in i + 1..queries.len() {
            assert_ne!(queries[i].hash, queries[j].hash);
        }
    }
}

#[test]
fn query_skips_malformed_rects() {
    let image = scene(300, 200, 7);
    let bad = [
        rect(0, 0, 0, 10),
        rect(300, 0, 10, 10),
        rect(0, 200, 5, 5),
        rect(u32::MAX, 0, 2, 2),
        rect(10, 10, 2, 2),
    ];
    assert!(query_hashes(image.view(), &bad).is_empty());
    assert!(query_hashes(image.view(), &[]).is_empty());
    let clipped = query_hashes(image.view(), &[rect(250, 150, u32::MAX - 250, 1000)]);
    assert_eq!(clipped[0].rect_index, 0);
    assert_eq!(
        clipped[0].hash,
        pdq(grey(&image).crop(250, 150, 50, 50).unwrap())
            .unwrap()
            .hash
    );
}

#[test]
fn query_rect_index_tracks_input_position() {
    let image = scene(600, 400, 8);
    let queries = query_hashes(image.view(), &[rect(0, 0, 0, 0), rect(50, 50, 300, 200)]);
    assert!(!queries.is_empty() && queries.iter().all(|q| q.rect_index == 1));
}

#[test]
fn scale_rect_maps_between_resolutions() {
    assert_eq!(
        scale_rect(rect(0, 0, 4032, 3024), (4032, 3024), (512, 384)),
        Some(rect(0, 0, 512, 384))
    );
    assert_eq!(
        scale_rect(rect(1008, 0, 2016, 3024), (4032, 3024), (512, 384)),
        Some(rect(128, 0, 256, 384))
    );
    assert_eq!(
        scale_rect(rect(1, 1, 1, 1), (4032, 3024), (512, 384)),
        Some(rect(0, 0, 1, 1))
    );
    assert_eq!(
        scale_rect(rect(10, 10, 100, 100), (50, 50), (50, 50)),
        Some(rect(10, 10, 40, 40))
    );
    assert_eq!(scale_rect(rect(50, 0, 1, 1), (50, 50), (50, 50)), None);
}

#[test]
fn resize_about_center_steps_four_percent() {
    let base = rect(100, 100, 200, 100);
    assert_eq!(
        resize_about_center(base, (1000, 1000), false),
        rect(104, 102, 192, 96)
    );
    assert_eq!(
        resize_about_center(base, (1000, 1000), true),
        rect(96, 98, 208, 104)
    );
    assert_eq!(
        resize_about_center(rect(0, 0, 50, 50), (50, 50), true),
        rect(0, 0, 50, 50)
    );
    assert_eq!(
        resize_about_center(rect(0, 0, 1, 1), (50, 50), false),
        rect(0, 0, 1, 1)
    );
}

#[test]
fn query_is_deterministic() {
    let image: Rgb8 = scene(700, 500, 11);
    let rects = [rect(10, 20, 400, 300), rect(200, 100, 480, 390)];
    assert_eq!(
        query_hashes(image.view(), &rects),
        query_hashes(image.view(), &rects)
    );
}
