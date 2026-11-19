use super::align::frame_in_copy;
use super::sampler::{Frame, Q16, Sampler, cell_coefficients, signature};
use super::*;
use crate::shape::{Anchor, Aspect, ShapeTag};
use crate::testkit::{Rng, grey, jpeg_round_trip, map_pixels, paste, scene};
use witness_format::fingerprint::{GRID_CELLS, GRID_LONG, GRID_SHORT};
use witness_image::{Luma8, Rgb8, resize_area};

fn grid_of(rgb: &Rgb8) -> Grid {
    grid(grey(rgb).view())
}

fn diff(a: &Rgb8, b: &Rgb8) -> GridDiff {
    compare(&grid_of(a), &grid_of(b)).unwrap()
}

fn q16(fraction: f64) -> u32 {
    (fraction * 65536.0) as u32
}

fn random_grid(rng: &mut Rng, portrait: bool) -> Grid {
    let mut cells = [0u16; GRID_CELLS];
    cells.iter_mut().for_each(|c| *c = rng.next() as u16);
    Grid::new(cells, portrait)
}

#[test]
fn pack_round_trips() {
    for signs in [0u8, 1, 0x5A, 0xFF] {
        for stable in [0u8, 0x80, 0x0F, 0xFF] {
            let cell = Grid::pack(signs, stable);
            assert_eq!((Grid::signs(cell), Grid::stable(cell)), (signs, stable));
        }
    }
}

#[test]
fn dims_follow_orientation() {
    assert_eq!(grid_dims(false), (GRID_LONG, GRID_SHORT));
    assert_eq!(grid_dims(true), (GRID_SHORT, GRID_LONG));
    assert!(!grid(Luma8::filled(300, 300, [1]).unwrap().view()).portrait());
    assert!(grid(Luma8::filled(300, 301, [1]).unwrap().view()).portrait());
    assert_eq!(
        grid(Luma8::filled(30, 300, [1]).unwrap().view()).dims(),
        (12, 16)
    );
}

#[test]
fn signature_of_equal_coefficients_is_empty() {
    assert_eq!(signature(&[7; 8]), 0);
    assert_eq!(signature(&[0; 8]), 0);
}

#[test]
fn signature_sets_signs_above_median() {
    let big = 1i64 << 45;
    let coefficients = [big, -big, big, -big, big, -big, big, -big];
    let cell = signature(&coefficients);
    assert_eq!(Grid::signs(cell), 0b0101_0101);
    assert_eq!(Grid::stable(cell), 0xFF);
}

#[test]
fn signature_marks_weak_deviations_unstable() {
    let big = 1i64 << 45;
    let coefficients = [big, -big, big, -big, 1, -1, big / 40, -big / 40];
    let stable = Grid::stable(signature(&coefficients));
    assert_eq!(stable & 0x0F, 0x0F);
    assert_eq!(stable & 0xF0, 0);
}

#[test]
fn signature_floor_rejects_faint_texture() {
    let faint = [9i64, -9, 9, -9, 9, -9, 9, -9].map(|v| v << 30);
    assert_eq!(Grid::stable(signature(&faint)), 0);
}

#[test]
fn signature_is_scale_invariant_above_floor() {
    let mut rng = Rng::new(3);
    for _ in 0..200 {
        let coefficients: [i64; 8] = core::array::from_fn(|_| (rng.next() as i32 as i64) << 16);
        let scaled = coefficients.map(|c| c * 3);
        assert_eq!(signature(&coefficients), signature(&scaled));
    }
}

#[test]
fn constant_cell_has_no_coefficients() {
    assert_eq!(cell_coefficients(&[[12_345; 8]; 8]), [0; 8]);
}

#[test]
fn coefficients_separate_axes() {
    let horizontal: [[i64; 8]; 8] =
        core::array::from_fn(|_| core::array::from_fn(|x| x as i64 * 1000));
    let vertical: [[i64; 8]; 8] = core::array::from_fn(|y| [y as i64 * 1000; 8]);
    let h = cell_coefficients(&horizontal);
    let v = cell_coefficients(&vertical);
    assert!(h[0] < 0 && h[2] == 0 && h[3] == 0);
    assert!(v[2] < 0 && v[0] == 0 && v[1] == 0);
    assert_eq!(h[0], v[2]);
}

#[test]
fn summed_area_table_matches_naive_sums() {
    let mut rng = Rng::new(9);
    let luma = Luma8::from_fn(37, 23, |_, _| [rng.next() as u8]).unwrap();
    let sampler = Sampler::new(luma.view(), 512);
    assert_eq!(sampler.dimensions(), (37, 23));
    let frame = Frame::whole((37, 23));
    assert_eq!(
        frame,
        Frame {
            x: 0,
            y: 0,
            w: 37 * Q16,
            h: 23 * Q16
        }
    );
    for _ in 0..500 {
        let (x0, x1) = (rng.below(37) as usize, rng.below(37) as usize);
        let (y0, y1) = (rng.below(23) as usize, rng.below(23) as usize);
        let (x0, x1, y0, y1) = (x0.min(x1), x0.max(x1) + 1, y0.min(y1), y0.max(y1) + 1);
        let naive: i64 = (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .map(|(x, y)| luma.pixel(x as u32, y as u32).unwrap()[0] as i64)
            .sum();
        assert_eq!(sampler.sum(x0, y0, x1, y1), naive);
    }
}

#[test]
fn frame_outside_copy_marks_cells_absent() {
    let luma = grey(&scene(256, 192, 1));
    let sampler = Sampler::new(luma.view(), 256);
    let shifted = Frame {
        x: -128 * Q16,
        y: 0,
        w: 256 * Q16,
        h: 192 * Q16,
    };
    let (_, present) = sampler.grid(shifted, false);
    for row in 0..GRID_SHORT {
        for col in 0..GRID_LONG {
            assert_eq!(
                mask_get(&present, row * GRID_LONG + col),
                col >= 8,
                "{col} {row}"
            );
        }
    }
    let gone = Frame {
        x: 300 * Q16,
        ..shifted
    };
    assert_eq!(sampler.grid(gone, false).1, [0; CELL_MASK_WORDS]);
}

#[test]
fn self_comparison_is_perfect() {
    let rgb = scene(640, 480, 2);
    let d = diff(&rgb, &rgb);
    assert_eq!(d.differing_bits(), 0);
    assert_eq!(d.agreement_q16(), 65536);
    assert!(d.compared_cells() >= 150, "{}", d.compared_cells());
    assert!((0..GRID_CELLS).all(|i| d.distance(i).is_none_or(|v| v == 0)));
}

#[test]
fn flat_image_is_uncomparable_not_changed() {
    let flat = Rgb8::filled(400, 300, [90, 90, 90]).unwrap();
    let d = diff(&flat, &flat);
    assert_eq!(d.compared_cells(), 0);
    assert_eq!(d.agreement_q16(), 0);
    assert_eq!(d.compared_fraction_q16(), 0);
    assert!(grid_of(&flat).cells().iter().all(|&c| Grid::stable(c) == 0));
}

#[test]
fn recompression_keeps_grid_stable() {
    for seed in 0..6 {
        let rgb = scene(800, 600, seed + 10);
        for quality in [85u8, 60, 40] {
            let d = diff(&rgb, &jpeg_round_trip(&rgb, quality));
            assert!(
                d.agreement_q16() >= q16(0.93),
                "seed {seed} q{quality} {}",
                d.agreement_q16()
            );
            assert!(
                d.compared_fraction_q16() >= q16(0.6),
                "seed {seed} compared {}",
                d.compared_cells()
            );
        }
    }
}

#[test]
fn global_brightness_and_contrast_do_not_change_grid() {
    for seed in 0..6 {
        let rgb = map_pixels(&scene(640, 480, seed + 20), |v| {
            (v as u32 * 3 / 4 + 30) as u8
        });
        let edits = [
            map_pixels(&rgb, |v| v.saturating_add(25)),
            map_pixels(&rgb, |v| v.saturating_sub(25)),
            map_pixels(&rgb, |v| {
                ((v as f64 - 128.0) * 1.3 + 128.0).round().clamp(0.0, 255.0) as u8
            }),
            map_pixels(&rgb, |v| ((v as f64 - 128.0) * 0.6 + 128.0).round() as u8),
            map_pixels(&rgb, |v| {
                (255.0 * (v as f64 / 255.0).powf(0.8)).round() as u8
            }),
        ];
        for (i, edited) in edits.iter().enumerate() {
            let d = diff(&rgb, edited);
            assert!(
                d.agreement_q16() >= q16(0.93),
                "seed {seed} edit {i} {}",
                d.agreement_q16()
            );
            let worst = (0..GRID_CELLS).filter_map(|c| d.distance(c)).max().unwrap();
            assert!(worst <= 160, "seed {seed} edit {i} worst {worst}");
        }
    }
}

#[test]
fn downscaled_copy_matches() {
    let rgb = scene(1200, 900, 31);
    let small = resize_area(rgb.view(), 500, 375).unwrap();
    assert!(diff(&rgb, &jpeg_round_trip(&small, 70)).agreement_q16() >= q16(0.92));
}

fn patch_cells(x0: u32, y0: u32, size: u32, width: u32, height: u32) -> Vec<usize> {
    let (cw, ch) = (width / GRID_LONG as u32, height / GRID_SHORT as u32);
    let mut cells = Vec::new();
    for row in 0..GRID_SHORT as u32 {
        for col in 0..GRID_LONG as u32 {
            let inside = col * cw >= x0
                && (col + 1) * cw <= x0 + size
                && row * ch >= y0
                && (row + 1) * ch <= y0 + size;
            if inside {
                cells.push((row * GRID_LONG as u32 + col) as usize);
            }
        }
    }
    cells
}

#[test]
fn pasted_patch_is_detected_and_localised() {
    for seed in 0..6 {
        let rgb = scene(1024, 768, seed + 40);
        let donor = scene(256, 256, seed + 900);
        let edited = jpeg_round_trip(&paste(&rgb, &donor, 384, 256), 80);
        let d = diff(&rgb, &edited);
        let inside = patch_cells(384, 256, 256, 1024, 768);
        assert_eq!(inside.len(), 16);
        let changed_inside = inside
            .iter()
            .filter(|&&c| d.distance(c).is_some_and(|v| v >= 64))
            .count();
        assert!(changed_inside >= 8, "seed {seed} inside {changed_inside}");
        let border = |c: usize| {
            let (col, row) = ((c % GRID_LONG) as i32, (c / GRID_LONG) as i32);
            (5..=10).contains(&col) && (3..=8).contains(&row)
        };
        let false_alarms = (0..GRID_CELLS)
            .filter(|&c| !border(c) && d.distance(c).is_some_and(|v| v >= 64))
            .count();
        assert!(false_alarms <= 3, "seed {seed} false alarms {false_alarms}");
    }
}

#[test]
fn flat_paint_over_texture_is_detected() {
    let rgb = scene(800, 600, 50);
    let flat = Rgb8::filled(200, 200, [128, 128, 128]).unwrap();
    let d = diff(&rgb, &paste(&rgb, &flat, 300, 200));
    let inside = patch_cells(300, 200, 200, 800, 600);
    let changed = inside
        .iter()
        .filter(|&&c| d.distance(c).is_some_and(|v| v >= 64))
        .count();
    let compared = inside.iter().filter(|&&c| d.is_compared(c)).count();
    assert!(
        compared * 2 >= inside.len(),
        "compared {compared}/{}",
        inside.len()
    );
    assert!(changed * 3 >= compared, "changed {changed} of {compared}");
}

#[test]
fn unrelated_images_disagree() {
    for seed in 0..5 {
        let d = diff(&scene(640, 480, seed + 60), &scene(640, 480, seed + 160));
        assert!(
            d.agreement_q16() <= q16(0.7),
            "seed {seed} {}",
            d.agreement_q16()
        );
    }
}

#[test]
fn compare_rejects_orientation_mismatch() {
    let mut rng = Rng::new(1);
    assert!(compare(&random_grid(&mut rng, true), &random_grid(&mut rng, false)).is_none());
}

#[test]
fn compare_random_grids_is_consistent() {
    let mut rng = Rng::new(77);
    for _ in 0..300 {
        let portrait = rng.next() & 1 == 1;
        let (a, b) = (
            random_grid(&mut rng, portrait),
            random_grid(&mut rng, portrait),
        );
        let d = compare(&a, &b).unwrap();
        assert_eq!(d.portrait(), portrait);
        let (mut bits, mut differing, mut cells) = (0, 0, 0);
        for i in 0..GRID_CELLS {
            let mask = Grid::stable(a.cells()[i]) | Grid::stable(b.cells()[i]);
            let comparable = mask.count_ones() >= MIN_STABLE_BITS;
            assert_eq!(d.is_compared(i), comparable);
            if comparable {
                cells += 1;
                bits += mask.count_ones();
                differing +=
                    ((Grid::signs(a.cells()[i]) ^ Grid::signs(b.cells()[i])) & mask).count_ones();
            } else {
                assert_eq!(d.distances()[i], 0);
            }
        }
        assert_eq!(
            (d.compared_bits(), d.differing_bits(), d.compared_cells()),
            (bits, differing, cells)
        );
        assert_eq!(compare(&b, &a).unwrap().distances(), d.distances());
        assert!(!d.is_compared(GRID_CELLS) && d.distance(10_000).is_none());
    }
}

#[test]
fn opposite_signs_give_full_distance() {
    let a = Grid::new([Grid::pack(0x0F, 0xFF); GRID_CELLS], false);
    let b = Grid::new([Grid::pack(0xF0, 0xFF); GRID_CELLS], false);
    let d = compare(&a, &b).unwrap();
    assert!(d.distances().iter().all(|&v| v == 255));
    assert_eq!(d.agreement_q16(), 0);
    assert_eq!(d.compared_fraction_q16(), 65536);
}

#[test]
fn grid_round_trips_through_bundle() {
    let g = grid_of(&scene(300, 500, 70));
    let bundle =
        witness_format::FingerprintBundle::new(vec![], 50, *g.cells(), g.portrait()).unwrap();
    assert_eq!(Grid::from_bundle(&bundle), g);
}

#[test]
fn grid_is_deterministic_and_view_independent() {
    let luma = grey(&scene(900, 700, 71));
    let view = luma.crop(50, 40, 700, 500).unwrap();
    assert_eq!(grid(view), grid(view.to_image().view()));
    assert_eq!(grid(luma.view()), grid(luma.view()));
}

#[test]
fn tiny_images_still_produce_grids() {
    for (w, h) in [(1u32, 1u32), (3, 2), (16, 12), (17, 300)] {
        let g = grid(Luma8::filled(w, h, [10]).unwrap().view());
        assert!(g.cells().iter().all(|&c| c == 0));
    }
    let g = grid(grey(&scene(20, 15, 3)).view());
    assert_eq!(g.dims(), (16, 12));
}

#[test]
fn hypotheses_cover_shapes_times_scales() {
    let h = hypotheses(4032, 3024);
    assert_eq!(h.len(), 19 * 3);
    assert!(h.contains(&Hypothesis {
        tag: ShapeTag::FULL,
        scale_percent: 100
    }));
    assert!(hypotheses(0, 0).is_empty());
}

#[test]
fn identity_hypothesis_maps_to_whole_copy() {
    let frame = frame_in_copy(
        Hypothesis {
            tag: ShapeTag::FULL,
            scale_percent: 100,
        },
        (4032, 3024),
        (512, 384),
    )
    .unwrap();
    assert_eq!(frame, Frame::whole((512, 384)));
    let square = Hypothesis {
        tag: ShapeTag::new(Aspect::Square, Anchor::Start),
        scale_percent: 100,
    };
    let frame = frame_in_copy(square, (400, 300), (300, 300)).unwrap();
    assert_eq!(
        frame,
        Frame {
            x: 0,
            y: 0,
            w: 400 * Q16,
            h: 300 * Q16
        }
    );
    let end = Hypothesis {
        tag: ShapeTag::new(Aspect::Square, Anchor::End),
        scale_percent: 100,
    };
    assert_eq!(
        frame_in_copy(end, (400, 300), (300, 300)).unwrap().x,
        -100 * Q16
    );
}

#[test]
fn align_accepts_recompressed_full_copy() {
    let rgb = scene(1024, 768, 80);
    let reference = grid_of(&rgb);
    let copy = jpeg_round_trip(&resize_area(rgb.view(), 640, 480).unwrap(), 70);
    let a = align(&reference, (1024, 768), grey(&copy).view()).unwrap();
    assert_eq!(
        a.best.hypothesis,
        Hypothesis {
            tag: ShapeTag::FULL,
            scale_percent: 100
        }
    );
    assert!(a.accepted, "{a:?}");
    assert!(a.diff.compared_cells() >= 100);
}

#[test]
fn align_finds_social_crops() {
    let rgb = scene(1600, 1200, 81);
    let reference = grid_of(&rgb);
    for tag in [
        ShapeTag::new(Aspect::Square, Anchor::Center),
        ShapeTag::new(Aspect::Square, Anchor::Start),
        ShapeTag::new(Aspect::Portrait4x5, Anchor::End),
        ShapeTag::new(Aspect::Landscape16x9, Anchor::Center),
    ] {
        let r = tag.rect(1600, 1200).unwrap();
        let crop = jpeg_round_trip(&rgb.crop(r.x, r.y, r.w, r.h).unwrap().to_image(), 75);
        let a = align(&reference, (1600, 1200), grey(&crop).view()).unwrap();
        assert_eq!(
            a.best.hypothesis,
            Hypothesis {
                tag,
                scale_percent: 100
            },
            "{tag:?} {a:?}"
        );
        assert!(a.accepted, "{tag:?} {:?} {:?}", a.best, a.runner_up);
    }
}

#[test]
fn align_rejects_unrelated_image() {
    for seed in 0..4 {
        let reference = grid_of(&scene(1024, 768, seed + 90));
        let other = scene(1024, 768, seed + 190);
        let a = align(&reference, (1024, 768), grey(&other).view()).unwrap();
        assert!(!a.accepted, "seed {seed} {:?}", a.best);
    }
}

#[test]
fn align_reports_pasted_patch_in_cropped_copy() {
    let rgb = scene(1600, 1200, 82);
    let reference = grid_of(&rgb);
    let tag = ShapeTag::new(Aspect::Square, Anchor::Center);
    let r = tag.rect(1600, 1200).unwrap();
    let edited = paste(&rgb, &scene(300, 300, 999), 700, 450);
    let crop = jpeg_round_trip(&edited.crop(r.x, r.y, r.w, r.h).unwrap().to_image(), 80);
    let a = align(&reference, (1600, 1200), grey(&crop).view()).unwrap();
    assert!(a.accepted);
    assert_eq!(a.best.hypothesis.tag, tag);
    let inside = patch_cells(700, 450, 300, 1600, 1200);
    let changed = inside
        .iter()
        .filter(|&&c| a.diff.distance(c).is_some_and(|v| v >= 64))
        .count();
    assert!(
        changed * 2 >= inside.len(),
        "changed {changed}/{}",
        inside.len()
    );
    let left_edge_compared = (0..GRID_SHORT).any(|row| a.diff.is_compared(row * GRID_LONG));
    assert!(!left_edge_compared);
}

#[test]
fn align_rejects_bad_inputs() {
    let landscape = grid_of(&scene(400, 300, 1));
    let copy = grey(&scene(400, 300, 1));
    assert!(align(&landscape, (300, 400), copy.view()).is_none());
    assert!(align(&landscape, (0, 300), copy.view()).is_none());
    assert!(align(&landscape, (400, 0), copy.view()).is_none());
    let flat = Luma8::filled(400, 300, [50]).unwrap();
    assert!(align(&landscape, (400, 300), flat.view()).is_none_or(|a| !a.accepted));
    let huge = align(&landscape, (u32::MAX, u32::MAX - 1), copy.view());
    assert_eq!(
        huge,
        align(&landscape, (u32::MAX, u32::MAX - 1), copy.view())
    );
}

#[test]
fn align_is_deterministic() {
    let rgb = scene(800, 600, 83);
    let reference = grid_of(&rgb);
    let copy = grey(&jpeg_round_trip(&rgb, 60));
    assert_eq!(
        align(&reference, (800, 600), copy.view()),
        align(&reference, (800, 600), copy.view())
    );
}

#[test]
fn degenerate_sub_blocks_stay_inside_frame() {
    for (w, h) in [(1usize, 1usize), (5, 3), (16, 12), (37, 23), (130, 97)] {
        let luma = Luma8::filled(w as u32, h as u32, [77]).unwrap();
        let sampler = Sampler::new(luma.view(), 512);
        let (_, present) = sampler.grid(Frame::whole((w, h)), false);
        assert_eq!(
            present.iter().map(|m| m.count_ones()).sum::<u32>(),
            GRID_CELLS as u32,
            "{w}x{h}"
        );
    }
}
