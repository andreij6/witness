use witness_core::format::{
    BlurClass, CaptureId, Edit, ImageFormat, KeyHistory, Kind, Outcome, RecordId, Rect,
    VerdictFlags,
};
use witness_core::image::{Orientation, Rgb8, decode, encode_jpeg, resize_area};
use witness_core::verdict::testkit::{
    CAPTURED, ENROLLED_NS, FixedWatermarker, GENERATOR, MemoryOracle, TestCa, TestDevice, photo,
};
use witness_core::verdict::{Evidence, Reason, Verdict};
use witness_core::{
    Capture, ClaimTemplate, Manifest, Mark, Options, SEAL_QUALITY, SealedCapture, judge, seal,
};

const W: u32 = 480;
const H: u32 = 360;
const MANIFEST: Manifest<'static> = Manifest {
    id: [0x22; 16],
    generator: GENERATOR,
    actions: CAPTURED,
};

struct World {
    ca: TestCa,
    dev: TestDevice,
    oracle: MemoryOracle,
    wm: FixedWatermarker,
    counter: u64,
}

fn template(dev: &TestDevice, capture: u64, kind: Kind, counter: u64) -> ClaimTemplate {
    let c = dev.claim(capture, kind, counter);
    ClaimTemplate {
        capture_id: c.capture_id,
        kind: c.kind,
        time: c.time,
        loc: c.loc,
        checks: c.checks,
        key: c.key,
        cert_serial: c.cert_serial,
        coarse: c.coarse,
    }
}

fn jpeg(img: &Rgb8, q: u8) -> Vec<u8> {
    encode_jpeg(img, q).unwrap()
}

fn recompress(bytes: &[u8], q: u8) -> Vec<u8> {
    jpeg(&decode(bytes).unwrap(), q)
}

fn paste(img: &Rgb8, r: Rect, from: &Rgb8) -> Rgb8 {
    Rgb8::from_fn(img.width(), img.height(), |x, y| {
        let inside = x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h;
        img.pixel(x, y)
            .filter(|_| !inside)
            .or_else(|| from.pixel(x, y))
            .unwrap()
    })
    .unwrap()
}

impl World {
    fn new() -> Self {
        let ca = TestCa::new(21);
        let dev = ca.device(0x33, 0x0A0B);
        let mut oracle = MemoryOracle::new();
        oracle.add_key(dev.key_info(KeyHistory::new(ENROLLED_NS)));
        let capture = CaptureId::new(0x5EED, 0).unwrap();
        Self {
            ca,
            dev,
            oracle,
            wm: FixedWatermarker::reading(capture),
            counter: 0,
        }
    }

    fn capture_id(&self) -> CaptureId {
        self.wm.capture_id.unwrap()
    }

    fn seal(&mut self, image: &[u8], orientation: Orientation, kind: Kind) -> SealedCapture {
        self.counter += 1;
        let mark = kind.is_original().then_some(Mark {
            watermarker: &self.wm,
            capture_id: self.capture_id(),
        });
        let capture = Capture {
            image,
            orientation,
            quality: SEAL_QUALITY,
            mark,
        };
        let t = template(&self.dev, self.capture_id().to_u64(), kind, self.counter);
        seal(&capture, t, &MANIFEST, &self.dev, &self.ca.anchor).unwrap()
    }

    fn anchor(&mut self, s: &SealedCapture, reference: bool) -> u64 {
        let record = self
            .oracle
            .anchor(s.claim.clone(), ENROLLED_NS + 1_000 * self.counter);
        assert_eq!(record.record_id, s.record_id);
        self.oracle
            .set_fingerprints(record.record_ix, s.fingerprints.clone());
        if reference {
            self.oracle
                .set_reference(record.record_ix, recompress(&s.jpeg, 85));
        }
        record.record_ix
    }

    fn original(&mut self, seed: u64, reference: bool) -> SealedCapture {
        let camera = jpeg(&photo(W, H, seed), 95);
        let s = self.seal(&camera, Orientation::Normal, Kind::Original);
        self.anchor(&s, reference);
        s
    }

    fn android(&self) -> Options<'_> {
        Options::android(&self.ca.anchor).with_watermarker(&self.wm)
    }

    fn web(&self) -> Options<'_> {
        Options::web(&self.ca.anchor).with_watermarker(&self.wm)
    }

    fn judge(&self, input: &[u8]) -> Verdict {
        let android = judge(input, &self.oracle, &self.android());
        let web = judge(input, &self.oracle, &self.web());
        assert_eq!(android.outcome, web.outcome);
        assert_eq!(android.regions, web.regions);
        android
    }
}

fn reference_id(v: &Verdict) -> RecordId {
    v.reference.as_ref().expect("reference").record_id
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sealed_photo_is_original_everywhere() {
    let mut w = World::new();
    let s = w.original(1, true);
    assert!(s.watermarked);
    let v = w.judge(&s.jpeg);
    assert_eq!(v.outcome, Outcome::Original);
    assert!(v.reasons.contains(Reason::SealVerified));
    assert_eq!(reference_id(&v), s.record_id);
    assert_eq!(v.reference.unwrap().evidence, Evidence::Seal);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn stripped_credentials_still_match_exactly() {
    let mut w = World::new();
    let s = w.original(2, false);
    let stripped = witness_core::image::strip_and_normalize_jpeg(&s.jpeg).unwrap();
    let v = w.judge(&stripped);
    assert_eq!(v.outcome, Outcome::Original);
    assert!(
        v.reasons.contains(Reason::ExactAssetHash) || v.reasons.contains(Reason::ExactPixelHash)
    );
    assert_eq!(v.reference.unwrap().evidence, Evidence::ExactHash);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn recompressed_copy_matches_with_changes() {
    let mut w = World::new();
    let s = w.original(3, true);
    let v = w.judge(&recompress(&s.jpeg, 70));
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    assert_eq!(reference_id(&v), s.record_id);
    assert!(v.regions.is_empty());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn downscaled_copy_matches_with_changes() {
    let mut w = World::new();
    let s = w.original(4, true);
    let small = resize_area(decode(&s.jpeg).unwrap().view(), W * 2 / 3, H * 2 / 3).unwrap();
    let v = w.judge(&jpeg(&small, 80));
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    assert_eq!(reference_id(&v), s.record_id);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn cropped_copy_matches_with_partial_comparison() {
    let mut w = World::new();
    let s = w.original(5, true);
    let crop = decode(&s.jpeg)
        .unwrap()
        .crop(120, 60, 320, 260)
        .unwrap()
        .to_image();
    let v = w.judge(&jpeg(&crop, 85));
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    assert!(v.flags.contains(VerdictFlags::COULDNT_COMPARE));
    assert_eq!(reference_id(&v), s.record_id);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn locally_edited_copy_is_altered() {
    let mut w = World::new();
    let s = w.original(6, true);
    let edit = Rect {
        x: 210,
        y: 150,
        w: 60,
        h: 50,
    };
    let edited = paste(&decode(&s.jpeg).unwrap(), edit, &photo(W, H, 777));
    let v = w.judge(&jpeg(&edited, 85));
    assert_eq!(v.outcome, Outcome::Altered, "{v:?}");
    assert_eq!(v.regions.len(), 1);
    assert_eq!(reference_id(&v), s.record_id);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn copy_without_public_reference_is_capped() {
    let mut w = World::new();
    let s = w.original(7, false);
    let v = w.judge(&recompress(&s.jpeg, 75));
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    assert!(v.flags.contains(VerdictFlags::NO_PUBLIC_REFERENCE));
    assert_eq!(v.reference.unwrap().evidence, Evidence::Grid);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn tampered_sealed_file_is_altered() {
    let mut w = World::new();
    let s = w.original(8, true);
    let mut bytes = s.jpeg.clone();
    let at = bytes.len() - 64;
    bytes[at] = bytes[at].wrapping_add(1);
    let v = w.judge(&bytes);
    assert_eq!(v.outcome, Outcome::Altered);
    assert!(v.reasons.contains(Reason::SealBroken));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unanchored_seal_is_reported_as_such() {
    let mut w = World::new();
    let camera = jpeg(&photo(W, H, 9), 95);
    let s = w.seal(&camera, Orientation::Rotate90, Kind::Original);
    assert_eq!((s.claim.dims.width, s.claim.dims.height), (H, W));
    let v = w.judge(&s.jpeg);
    assert_eq!(v.outcome, Outcome::SealedUnanchored);
    assert_eq!(v.reference.unwrap().anchor, None);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sealed_derivatives_follow_their_edit_class() {
    let mut w = World::new();
    let parent = w.original(10, true);
    let decoded = decode(&parent.jpeg).unwrap();
    let resized = jpeg(&resize_area(decoded.view(), W / 2, H / 2).unwrap(), 90);
    let presentation = Kind::Derivative {
        parent: parent.record_id,
        root: parent.claim.capture_id,
        edits: vec![
            Edit::Resize {
                w: W / 2,
                h: H / 2,
                filter: 0,
            },
            Edit::Recompress {
                format: ImageFormat::Jpeg,
                quality: 90,
            },
        ],
    };
    let d1 = w.seal(&resized, Orientation::Normal, presentation);
    w.anchor(&d1, true);
    assert!(!d1.watermarked);
    assert_eq!(w.judge(&d1.jpeg).outcome, Outcome::Original);

    let rect = Rect {
        x: 40,
        y: 40,
        w: 80,
        h: 60,
    };
    let blurred = Rgb8::from_fn(W, H, |x, y| {
        let inside = x >= rect.x && y >= rect.y && x < rect.x + rect.w && y < rect.y + rect.h;
        if inside {
            [120, 118, 116]
        } else {
            decoded.pixel(x, y).unwrap()
        }
    })
    .unwrap();
    let edits = vec![Edit::Blur {
        rect,
        class: BlurClass::Face,
        sigma_e2: 900,
    }];
    let content = Kind::Derivative {
        parent: parent.record_id,
        root: parent.claim.capture_id,
        edits: edits.clone(),
    };
    let d2 = w.seal(&jpeg(&blurred, 95), Orientation::Normal, content);
    w.anchor(&d2, true);
    let v = w.judge(&d2.jpeg);
    assert_eq!(v.outcome, Outcome::MatchesWithChanges);
    assert_eq!(v.declared, edits);

    let copy = w.judge(&recompress(&d2.jpeg, 75));
    assert_eq!(copy.outcome, Outcome::MatchesWithChanges, "{copy:?}");
    assert_eq!(
        reference_id(&copy),
        d2.record_id,
        "{copy:?} parent {:?} d1 {:?}",
        parent.record_id,
        d1.record_id
    );
    assert_eq!(copy.declared, edits);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn watermark_on_unrelated_photo_warns() {
    let mut w = World::new();
    w.original(11, true);
    let v = w.judge(&jpeg(&photo(W, H, 12), 85));
    assert_eq!(v.outcome, Outcome::NoRecord, "{v:?}");
    assert!(v.reasons.contains(Reason::ForeignWatermark));
    assert!(v.reference.is_none());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unrelated_photo_without_watermark_has_no_record() {
    let mut w = World::new();
    w.original(13, true);
    let v = judge(
        &jpeg(&photo(W, H, 14), 85),
        &w.oracle,
        &Options::web(&w.ca.anchor),
    );
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::NoCandidates));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn facade_exposes_every_core_module() {
    let _ = witness_core::fingerprint::MAX_QUERY_HASHES;
    let _ = witness_core::matching::MIN_WINDOW_RADIUS;
    let _ = witness_core::merkle::ZERO;
    let _ = witness_core::attest::FRESH_LEN;
    let _ = witness_core::watermark::PAYLOAD_BITS;
    let _ = witness_core::c2pa::assertions::WITNESS_CLAIM;
    let _ = witness_core::verdict::VERDICT_V;
    let _ = witness_core::format::WitnessClaim::VERSION;
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn recompressed_full_frames_are_full_compares_with_or_without_reference() {
    for seed in 0..6u64 {
        for reference in [false, true] {
            let mut w = World::new();
            let camera = jpeg(&photo(640, 480, 9100 + seed), 95);
            let s = w.seal(&camera, Orientation::Normal, Kind::Original);
            w.anchor(&s, reference);
            let copy = recompress(&s.jpeg, 80);
            let v = w.judge(&copy);
            let label = format!("seed {seed} reference={reference}");
            assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{label}: {v:?}");
            assert_eq!(v.flags, judge(&copy, &w.oracle, &w.web()).flags, "{label}");
            assert!(
                !v.flags.contains(VerdictFlags::COULDNT_COMPARE),
                "{label}: {v:?}"
            );
            assert_eq!(
                v.flags.contains(VerdictFlags::NO_PUBLIC_REFERENCE),
                !reference,
                "{label}"
            );
            assert!(v.regions.is_empty(), "{label}");
            let r = v.reference.unwrap();
            assert_eq!(r.record_id, s.record_id, "{label}");
            assert_eq!(r.changed_q16, 0, "{label}");
        }
    }
}
