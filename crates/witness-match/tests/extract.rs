mod common;

use common::*;
use witness_image::Rgb8;
use witness_match::{Candidate, CandidateKind, ExtractParams, Rect, Rng, extract_photo_regions};

const W: u32 = 540;
const H: u32 = 1170;

struct Canvas {
    img: Rgb8,
}

impl Canvas {
    fn new(w: u32, h: u32, bg: [u8; 3]) -> Self {
        Self {
            img: Rgb8::filled(w, h, bg).unwrap(),
        }
    }

    fn fill(&mut self, rect: Rect, color: [u8; 3]) {
        for y in rect.y..rect.bottom().min(self.img.height()) {
            let row = self.img.row_mut(y);
            for x in rect.x..rect.right().min(row.len() as u32 / 3) {
                row[x as usize * 3..x as usize * 3 + 3].copy_from_slice(&color);
            }
        }
    }

    fn photo(&mut self, rect: Rect, seed: u64) {
        let content = scene(rect.width, rect.height, seed);
        for y in 0..rect.height {
            let src = content.row(y);
            let row = self.img.row_mut(rect.y + y);
            row[rect.x as usize * 3..rect.right() as usize * 3].copy_from_slice(src);
        }
    }

    fn text(&mut self, rect: Rect, color: [u8; 3], seed: u64) {
        let mut r = rng(seed);
        let mut x = rect.x;
        while x + 4 < rect.right() {
            let glyph = 3 + r.below(5);
            if r.below(6) != 0 {
                self.fill(Rect::new(x, rect.y, 2, rect.height), color);
                self.fill(
                    Rect::new(
                        x,
                        rect.y + r.below(rect.height.max(3) - 2),
                        glyph.min(rect.right() - x),
                        2,
                    ),
                    color,
                );
            }
            x += glyph + 2;
        }
    }

    fn icons(&mut self, y: u32, h: u32, count: u32, color: [u8; 3]) {
        let step = self.img.width() / (count + 1);
        for i in 1..=count {
            self.fill(Rect::new(i * step - h / 2, y, h, h), color);
        }
    }
}

fn status_and_nav(c: &mut Canvas, fg: [u8; 3]) {
    let (w, h) = c.img.dimensions();
    c.text(Rect::new(20, 12, 60, 14), fg, 1);
    c.text(Rect::new(w - 90, 12, 70, 14), fg, 2);
    c.icons(h - 45, 24, 3, fg);
}

fn best(candidates: &[Candidate]) -> &Candidate {
    &candidates[0]
}

fn assert_found(candidates: &[Candidate], photo: Rect, min_iou: f64) {
    let top = best(candidates);
    assert_eq!(top.kind, CandidateKind::Natural);
    assert!(
        iou(&top.rect, &photo) > min_iou,
        "{:?} vs {photo:?} (iou {})",
        top.rect,
        iou(&top.rect, &photo)
    );
    assert!(top.confidence > 0.3, "{}", top.confidence);
}

fn feed(dark: bool) -> (Rgb8, Rect) {
    let (bg, fg) = if dark {
        ([0, 0, 0], [245, 245, 245])
    } else {
        ([255, 255, 255], [20, 20, 20])
    };
    let mut c = Canvas::new(W, H, bg);
    status_and_nav(&mut c, fg);
    c.fill(Rect::new(16, 60, 40, 40), [200, 80, 120]);
    c.text(Rect::new(70, 72, 160, 14), fg, 3);
    let photo = Rect::new(0, 120, W, 540);
    c.photo(photo, 11);
    c.icons(680, 26, 4, fg);
    for (i, y) in [730u32, 752, 774].into_iter().enumerate() {
        c.text(Rect::new(16, y, 480, 12), fg, 10 + i as u64);
    }
    c.fill(Rect::new(16, 830, 40, 40), [90, 160, 200]);
    c.text(Rect::new(70, 842, 140, 14), fg, 20);
    (c.img, photo)
}

#[test]
fn feed_light_finds_photo() {
    let (img, photo) = feed(false);
    assert_found(
        &extract_photo_regions(img.view(), &ExtractParams::default()),
        photo,
        0.85,
    );
}

#[test]
fn feed_dark_finds_photo() {
    let (img, photo) = feed(true);
    assert_found(
        &extract_photo_regions(img.view(), &ExtractParams::default()),
        photo,
        0.85,
    );
}

#[test]
fn feed_always_offers_fallbacks() {
    let (img, _) = feed(false);
    let c = extract_photo_regions(img.view(), &ExtractParams::default());
    assert!(c.len() >= 2 && c.len() <= 5, "{}", c.len());
    let full = c
        .iter()
        .find(|x| x.kind == CandidateKind::FullMinusBars)
        .expect("full minus bars");
    assert!(full.rect.y > 0 && full.rect.bottom() < H, "{:?}", full.rect);
    assert_eq!(full.rect.width, W);
    let distinct: std::collections::BTreeSet<Rect> = c.iter().map(|x| x.rect).collect();
    assert_eq!(distinct.len(), c.len());
}

#[test]
fn stories_masks_overlays() {
    let mut c = Canvas::new(W, H, [0, 0, 0]);
    let photo = Rect::new(0, 0, W, H);
    c.photo(photo, 12);
    for i in 0..4u32 {
        c.fill(Rect::new(10 + i * 130, 12, 120, 4), [255, 255, 255]);
    }
    let caption = Rect::new(80, 520, 380, 60);
    c.fill(caption, [255, 255, 255]);
    c.text(Rect::new(96, 535, 340, 30), [10, 10, 10], 5);
    let candidates = extract_photo_regions(c.img.view(), &ExtractParams::default());
    let top = best(&candidates);
    assert!(
        top.rect.area() as f64 > 0.7 * photo.area() as f64,
        "{:?}",
        top.rect
    );
    let mask = &top.overlay_mask;
    assert!(mask.covers(270.0, 550.0));
    assert!(!mask.covers(270.0, 900.0));
    assert!(!mask.covers(100.0, 300.0));
    let (cols, rows) = mask.dimensions();
    assert!(mask.masked_blocks() * 5 < (cols * rows) as usize);
}

#[test]
fn carousel_prefers_center_photo() {
    let mut c = Canvas::new(W, H, [255, 255, 255]);
    status_and_nav(&mut c, [20, 20, 20]);
    c.text(Rect::new(70, 72, 160, 14), [20, 20, 20], 3);
    let photo = Rect::new(40, 140, 460, 560);
    c.photo(Rect::new(0, 140, 24, 560), 30);
    c.photo(photo, 31);
    c.photo(Rect::new(516, 140, 24, 560), 32);
    for i in 0..5u32 {
        c.fill(
            Rect::new(230 + i * 18, 720, 8, 8),
            if i == 1 {
                [30, 120, 240]
            } else {
                [180, 180, 180]
            },
        );
    }
    c.text(Rect::new(16, 760, 480, 12), [20, 20, 20], 33);
    assert_found(
        &extract_photo_regions(c.img.view(), &ExtractParams::default()),
        photo,
        0.8,
    );
}

#[test]
fn x_timeline_photo_with_side_margins() {
    let mut c = Canvas::new(W, H, [255, 255, 255]);
    status_and_nav(&mut c, [15, 20, 25]);
    c.fill(Rect::new(14, 200, 48, 48), [60, 120, 200]);
    c.text(Rect::new(76, 204, 200, 14), [15, 20, 25], 40);
    for (i, y) in [230u32, 252, 274].into_iter().enumerate() {
        c.text(Rect::new(76, y, 440, 12), [15, 20, 25], 41 + i as u64);
    }
    let photo = Rect::new(76, 300, 448, 336);
    c.photo(photo, 44);
    c.icons(660, 20, 4, [110, 118, 125]);
    assert_found(
        &extract_photo_regions(c.img.view(), &ExtractParams::default()),
        photo,
        0.8,
    );
}

#[test]
fn whatsapp_viewer_letterboxed_photo() {
    let mut c = Canvas::new(W, H, [0, 0, 0]);
    c.text(Rect::new(60, 60, 200, 16), [240, 240, 240], 50);
    let photo = Rect::new(0, 300, W, 405);
    c.photo(photo, 51);
    c.text(Rect::new(20, 1080, 400, 14), [240, 240, 240], 52);
    let candidates = extract_photo_regions(c.img.view(), &ExtractParams::default());
    assert_found(&candidates, photo, 0.85);
    let band = candidates
        .iter()
        .find(|x| x.kind == CandidateKind::LargestBand || x.rect == candidates[0].rect)
        .unwrap();
    assert!(iou(&band.rect, &photo) > 0.8);
}

#[test]
fn tiktok_photo_mode_with_side_rail() {
    let mut c = Canvas::new(W, H, [0, 0, 0]);
    let photo = Rect::new(0, 150, W, 720);
    c.photo(photo, 60);
    for i in 0..4u32 {
        c.fill(Rect::new(480, 420 + i * 80, 40, 40), [255, 255, 255]);
    }
    c.text(Rect::new(20, 900, 400, 14), [250, 250, 250], 61);
    c.text(Rect::new(20, 930, 300, 14), [250, 250, 250], 62);
    let candidates = extract_photo_regions(c.img.view(), &ExtractParams::default());
    let top = best(&candidates);
    assert!(iou(&top.rect, &photo) > 0.8, "{:?}", top.rect);
    assert!(top.overlay_mask.covers(500.0, 440.0));
    assert!(!top.overlay_mask.covers(200.0, 400.0));
}

#[test]
fn plain_photo_is_its_own_candidate() {
    let img = scene(800, 600, 70);
    let c = extract_photo_regions(img.view(), &ExtractParams::default());
    assert!(c.iter().any(|x| x.rect == Rect::new(0, 0, 800, 600)));
    assert!(
        c[0].overlay_mask.masked_blocks() < 10,
        "{}",
        c[0].overlay_mask.masked_blocks()
    );
}

#[test]
fn extraction_is_deterministic() {
    let (img, _) = feed(true);
    assert_eq!(
        extract_photo_regions(img.view(), &ExtractParams::default()),
        extract_photo_regions(img.view(), &ExtractParams::default())
    );
}

#[test]
fn small_and_degenerate_inputs_do_not_panic() {
    for (w, h) in [(1, 1), (1, 2000), (2000, 1), (3, 3), (17, 9)] {
        let img = Rgb8::from_fn(w, h, |x, y| {
            [(x * 13) as u8, (y * 7) as u8, ((x + y) * 3) as u8]
        })
        .unwrap();
        let c = extract_photo_regions(img.view(), &ExtractParams::default());
        assert!(!c.is_empty());
        for cand in &c {
            assert!(
                cand.rect.right() <= w && cand.rect.bottom() <= h && !cand.rect.is_empty(),
                "{:?}",
                cand.rect
            );
            assert!((0.0..=1.0).contains(&cand.confidence));
        }
    }
}

#[test]
fn candidates_are_within_frame_and_ranked() {
    let mut r = rng(5);
    for seed in 0..6u64 {
        let top = 40 + r.below(300);
        let height = 200 + r.below(400);
        let mut c = Canvas::new(W, H, [250, 250, 250]);
        status_and_nav(&mut c, [10, 10, 10]);
        let photo = Rect::new(0, top, W, height);
        c.photo(photo, 80 + seed);
        let candidates = extract_photo_regions(c.img.view(), &ExtractParams::default());
        assert_found(&candidates, photo, 0.85);
        let naturals: Vec<f64> = candidates
            .iter()
            .filter(|x| x.kind == CandidateKind::Natural)
            .map(|x| x.confidence)
            .collect();
        assert!(naturals.windows(2).all(|w| w[0] >= w[1]));
        for cand in &candidates {
            assert!(cand.rect.right() <= W && cand.rect.bottom() <= H);
        }
    }
}
