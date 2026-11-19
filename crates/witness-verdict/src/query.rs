use witness_fingerprint::{
    Anchor, MAX_QUERY_HASHES, PdqHash, ShapeTag, fit_long_edge, pdq, query_hashes,
    stored_shape_tags, working_luma,
};
use witness_format::Rect as FormatRect;
use witness_image::{
    ImageError, Luma8, Orientation, Rgb8, decode_with_limit, orient, orientation_of, pixel_hash_of,
    resize_area, to_luma,
};
use witness_match::{Candidate, ExtractParams, OverlayMask, Rect, extract_photo_regions};

pub const QUERY_LONG_EDGE: u32 = 2048;
pub const MAX_QUERY_RECTS: usize = 4;
pub const MAX_QUERY_VIEWS: usize = 2;

pub(crate) struct Decoded {
    pub pixel_hash: [u8; 32],
    pub image: Rgb8,
}

pub(crate) fn decode(input: &[u8], max_pixels: u64) -> Result<Decoded, ImageError> {
    let raw = decode_with_limit(input, max_pixels)?;
    let pixel_hash = pixel_hash_of(raw.view());
    let image = match orientation_of(input) {
        Orientation::Normal => raw,
        o => orient(raw.view(), o),
    };
    Ok(Decoded { pixel_hash, image })
}

pub(crate) struct Query {
    pub image: Rgb8,
    pub luma: Luma8,
    pub rects: Vec<Rect>,
    pub overlay: OverlayMask,
    pub hashes: Vec<PdqHash>,
}

pub(crate) fn working(image: Rgb8) -> Rgb8 {
    let (w, h) = image.dimensions();
    let (tw, th) = fit_long_edge(w, h, QUERY_LONG_EDGE);
    if (tw, th) == (w, h) {
        return image;
    }
    resize_area(image.view(), tw, th).unwrap_or(image)
}

fn to_format(r: &Rect) -> FormatRect {
    FormatRect {
        x: r.x,
        y: r.y,
        w: r.width,
        h: r.height,
    }
}

fn query_rects(full: Rect, regions: &[Candidate]) -> Vec<Rect> {
    let mut rects = Vec::with_capacity(MAX_QUERY_RECTS);
    for r in core::iter::once(full).chain(regions.iter().map(|c| c.rect)) {
        if rects.len() == MAX_QUERY_RECTS {
            break;
        }
        if !r.is_empty() && !rects.contains(&r) {
            rects.push(r);
        }
    }
    rects
}

fn recrop_order(tag: ShapeTag, width: u32, height: u32) -> (bool, u64) {
    let kept = tag
        .rect(width, height)
        .map_or(0, |r| u64::from(r.w) * u64::from(r.h));
    (tag.anchor() != Anchor::Center, u64::MAX - kept)
}

fn recrop_hashes(image: &Rgb8, hashes: &mut Vec<PdqHash>) {
    if hashes.len() >= MAX_QUERY_HASHES {
        return;
    }
    let work = working_luma(image.view());
    let (w, h) = work.dimensions();
    let mut tags: Vec<ShapeTag> = stored_shape_tags(w, h)
        .into_iter()
        .filter(|&t| t != ShapeTag::FULL)
        .collect();
    tags.sort_by_key(|&t| (recrop_order(t, w, h), t));
    for tag in tags {
        if hashes.len() >= MAX_QUERY_HASHES {
            break;
        }
        let Some(r) = tag.rect(w, h) else { continue };
        let Some(p) = work.view().crop(r.x, r.y, r.w, r.h).ok().and_then(pdq) else {
            continue;
        };
        if p.indexable() && !hashes.contains(&p.hash) {
            hashes.push(p.hash);
        }
    }
}

impl Query {
    pub fn new(image: Rgb8) -> Self {
        let image = working(image);
        let (w, h) = image.dimensions();
        let regions = extract_photo_regions(image.view(), &ExtractParams::default());
        let rects = query_rects(Rect::new(0, 0, w, h), &regions);
        let format_rects: Vec<FormatRect> = rects.iter().map(to_format).collect();
        let mut hashes: Vec<PdqHash> = query_hashes(image.view(), &format_rects)
            .into_iter()
            .map(|q| q.hash)
            .collect();
        recrop_hashes(&image, &mut hashes);
        let overlay = regions
            .into_iter()
            .next()
            .map(|c| c.overlay_mask)
            .unwrap_or_default();
        let luma = to_luma(image.view());
        Self {
            image,
            luma,
            rects,
            overlay,
            hashes,
        }
    }

    pub fn views(&self) -> impl Iterator<Item = Rect> + '_ {
        self.rects.iter().copied().take(MAX_QUERY_VIEWS)
    }

    pub fn min_distance(&self, stored: &[witness_format::PdqShape]) -> Option<u32> {
        self.hashes
            .iter()
            .flat_map(|q| stored.iter().map(move |s| q.distance(&PdqHash(s.hash))))
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::photo;
    use witness_fingerprint::{Aspect, MIH_RADIUS, fingerprint, stored_shapes};
    use witness_image::encode_jpeg;

    fn crop(img: &Rgb8, x: u32, y: u32, w: u32, h: u32) -> Rgb8 {
        img.crop(x, y, w, h).unwrap().to_image()
    }

    fn base_hashes(img: &Rgb8) -> Vec<PdqHash> {
        let (w, h) = img.dimensions();
        let full = [to_format(&Rect::new(0, 0, w, h))];
        query_hashes(img.view(), &full)
            .into_iter()
            .map(|q| q.hash)
            .collect()
    }

    fn distance(hashes: &[PdqHash], stored: &[witness_format::PdqShape]) -> u32 {
        hashes
            .iter()
            .flat_map(|q| stored.iter().map(move |s| q.distance(&PdqHash(s.hash))))
            .min()
            .unwrap()
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn recrop_fills_the_budget_without_duplicates() {
        let img = photo(640, 480, 11);
        let mut hashes = base_hashes(&img);
        let base = hashes.clone();
        recrop_hashes(&img, &mut hashes);
        assert_eq!(&hashes[..base.len()], &base[..]);
        assert!(hashes.len() > base.len());
        assert!(hashes.len() <= MAX_QUERY_HASHES);
        let mut unique = hashes.clone();
        unique.sort_unstable_by_key(|h| h.0);
        unique.dedup();
        assert_eq!(unique.len(), hashes.len());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn recrop_leads_with_centered_shapes_largest_area_first() {
        let img = photo(640, 480, 12);
        let work = working_luma(img.view());
        let shapes = stored_shapes(work.view());
        let center = |aspect| {
            shapes
                .iter()
                .find(|s| s.tag == ShapeTag::new(aspect, Anchor::Center))
                .unwrap()
                .pdq
                .hash
        };
        let mut hashes = Vec::new();
        recrop_hashes(&img, &mut hashes);
        assert_eq!(hashes[0], center(Aspect::Square));
        assert_eq!(hashes[1], center(Aspect::Landscape16x9));
        assert_eq!(hashes[2], center(Aspect::Landscape191x100));
        assert_eq!(hashes.len(), MAX_QUERY_HASHES);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn recrop_respects_a_full_budget() {
        let img = photo(320, 240, 13);
        let mut hashes: Vec<PdqHash> = (0..MAX_QUERY_HASHES as u8)
            .map(|i| PdqHash([i; 32]))
            .collect();
        let before = hashes.clone();
        recrop_hashes(&img, &mut hashes);
        assert_eq!(hashes, before);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn recrop_skips_flat_and_degenerate_images() {
        for img in [
            Rgb8::filled(320, 240, [90, 90, 90]).unwrap(),
            Rgb8::filled(1, 1, [1, 2, 3]).unwrap(),
            Rgb8::filled(400, 2, [1, 2, 3]).unwrap(),
        ] {
            let mut hashes = Vec::new();
            recrop_hashes(&img, &mut hashes);
            assert!(hashes.is_empty(), "{:?}", img.dimensions());
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn anchored_one_axis_crops_reach_a_stored_shape() {
        for seed in 0..4u64 {
            let original = photo(640, 480, 20 + seed);
            let stored = fingerprint(original.view()).unwrap();
            for removed in [5u32, 10, 20, 30, 50] {
                let keep = 640 * (100 - removed) / 100;
                for x in [0, (640 - keep) / 2, 640 - keep] {
                    let copy = crop(&original, x, 0, keep, 480);
                    let label = format!("seed {seed} -{removed}% x{x}");
                    let q = Query::new(copy.clone());
                    assert!(distance(&q.hashes, stored.pdq()) <= MIH_RADIUS, "{label}");
                    assert!(q.hashes.len() <= MAX_QUERY_HASHES, "{label}");
                }
            }
            for removed in [10u32, 20, 30] {
                let keep = 480 * (100 - removed) / 100;
                let copy = crop(&original, 0, (480 - keep) / 2, 640, keep);
                let q = Query::new(copy);
                assert!(
                    distance(&q.hashes, stored.pdq()) <= MIH_RADIUS,
                    "seed {seed} vertical -{removed}%"
                );
            }
        }
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn base_hashes_alone_miss_crops_between_stored_aspects() {
        let original = photo(640, 480, 30);
        let stored = fingerprint(original.view()).unwrap();
        let copy = crop(&original, 51, 0, 538, 480);
        assert!(distance(&base_hashes(&copy), stored.pdq()) > MIH_RADIUS);
        assert!(distance(&Query::new(copy).hashes, stored.pdq()) <= MIH_RADIUS);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn decode_hashes_raw_pixels_and_orients() {
        let img = photo(64, 48, 1);
        let jpeg = encode_jpeg(&img, 90).unwrap();
        let d = decode(&jpeg, 1 << 20).unwrap();
        assert_eq!(d.pixel_hash, witness_image::pixel_hash(&jpeg).unwrap());
        assert_eq!(d.image.dimensions(), (64, 48));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn decode_rejects_garbage_and_limits() {
        assert!(decode(b"not an image", 1 << 20).is_err());
        assert!(decode(&[], 1 << 20).is_err());
        let jpeg = encode_jpeg(&photo(64, 48, 1), 90).unwrap();
        assert!(decode(&jpeg, 100).is_err());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn working_caps_long_edge_and_keeps_small_images() {
        let small = photo(40, 30, 2);
        assert_eq!(working(small.clone()), small);
        let big = Rgb8::filled(4100, 1000, [1, 2, 3]).unwrap();
        let w = working(big);
        assert_eq!(w.width(), QUERY_LONG_EDGE);
        assert!(w.height() < 1000);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn query_rects_lead_with_full_frame_and_dedupe() {
        let full = Rect::new(0, 0, 100, 80);
        let c = |r: Rect| Candidate {
            rect: r,
            confidence: 1.0,
            kind: witness_match::CandidateKind::Natural,
            overlay_mask: OverlayMask::empty(),
        };
        let regions = [
            c(full),
            c(Rect::new(0, 10, 100, 60)),
            c(Rect::new(0, 0, 0, 5)),
            c(Rect::new(1, 1, 5, 5)),
            c(Rect::new(2, 2, 5, 5)),
            c(Rect::new(3, 3, 5, 5)),
        ];
        let rects = query_rects(full, &regions);
        assert_eq!(rects[0], full);
        assert_eq!(rects.len(), MAX_QUERY_RECTS);
        assert!(rects.iter().all(|r| !r.is_empty()));
        assert_eq!(rects[1], Rect::new(0, 10, 100, 60));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn query_produces_bounded_hashes_and_views() {
        let q = Query::new(photo(320, 240, 3));
        assert!(!q.hashes.is_empty());
        assert!(q.hashes.len() <= witness_fingerprint::MAX_QUERY_HASHES);
        assert_eq!(q.rects[0], Rect::new(0, 0, 320, 240));
        assert!(q.views().count() <= MAX_QUERY_VIEWS);
        assert_eq!(q.luma.dimensions(), (320, 240));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn min_distance_is_zero_against_own_shapes() {
        let img = photo(320, 240, 4);
        let bundle = witness_fingerprint::fingerprint(img.view()).unwrap();
        let q = Query::new(img);
        assert_eq!(q.min_distance(bundle.pdq()), Some(0));
        assert_eq!(q.min_distance(&[]), None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn query_is_deterministic() {
        let a = Query::new(photo(300, 200, 5));
        let b = Query::new(photo(300, 200, 5));
        assert_eq!(a.hashes, b.hashes);
        assert_eq!(a.rects, b.rects);
    }
}
