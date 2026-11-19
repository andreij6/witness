use crate::testkit::*;
use crate::*;
use witness_format::{
    BlurClass, CaptureId, Edit, FingerprintBundle, ImageFormat, KeyHistory, KeyStatus, Kind,
    Outcome, Provisioning, Rect as FRect, StatusChange, VerdictFlags, WitnessClaim,
};
use witness_image::{Rgb8, decode, encode_jpeg, resize_area};
use witness_watermark::NullWatermarker;

const W: u32 = 480;
const H: u32 = 360;
const T0: u64 = ENROLLED_NS + 1_000_000;

struct Fx {
    ca: TestCa,
    dev: TestDevice,
    oracle: MemoryOracle,
}

fn fx_with(history: KeyHistory) -> Fx {
    let ca = TestCa::new(7);
    let dev = ca.device(0x42, 0x01_0203_0405);
    let mut oracle = MemoryOracle::new();
    oracle.add_key(dev.key_info(history));
    Fx { ca, dev, oracle }
}

fn fx() -> Fx {
    fx_with(KeyHistory::new(ENROLLED_NS))
}

fn jpeg(img: &Rgb8, q: u8) -> Vec<u8> {
    encode_jpeg(img, q).unwrap()
}

fn recompress(bytes: &[u8], q: u8) -> Vec<u8> {
    jpeg(&decode(bytes).unwrap(), q)
}

fn paste(img: &Rgb8, rect: FRect, from: &Rgb8) -> Rgb8 {
    Rgb8::from_fn(img.width(), img.height(), |x, y| {
        let inside = x >= rect.x && y >= rect.y && x < rect.x + rect.w && y < rect.y + rect.h;
        if inside {
            from.pixel(x % from.width(), y % from.height()).unwrap()
        } else {
            img.pixel(x, y).unwrap()
        }
    })
    .unwrap()
}

fn with_comment(jpeg: &[u8]) -> Vec<u8> {
    let mut out = jpeg[..2].to_vec();
    out.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x05, b'h', b'i', b'!']);
    out.extend_from_slice(&jpeg[2..]);
    out
}

fn derivative(parent: &Record, edits: Vec<Edit>) -> Kind {
    Kind::Derivative {
        parent: parent.record_id,
        root: parent.claim.capture_id,
        edits,
    }
}

impl Fx {
    fn options(&self) -> Options<'_> {
        Options::android(&self.ca.anchor)
    }

    fn sealed(&self, img: &Rgb8, kind: Kind, counter: u64) -> (Vec<u8>, WitnessClaim) {
        let claim = self.dev.claim(0xC0FFEE, kind, counter);
        self.dev.seal(&self.ca.anchor, &jpeg(img, 92), claim)
    }

    fn anchored_photo(
        &mut self,
        seed: u64,
        reference: bool,
        fingerprints: bool,
    ) -> (Vec<u8>, Record) {
        self.anchored_sized(seed, (W, H), reference, fingerprints)
    }

    fn anchored_sized(
        &mut self,
        seed: u64,
        (width, height): (u32, u32),
        reference: bool,
        fingerprints: bool,
    ) -> (Vec<u8>, Record) {
        let mut claim = self.dev.claim(0xC0FFEE ^ seed, Kind::Original, seed);
        claim.dims = witness_format::Dims { width, height };
        let (bytes, claim) = self.dev.seal(
            &self.ca.anchor,
            &jpeg(&photo(width, height, seed), 92),
            claim,
        );
        let record = self.oracle.anchor(claim, T0 + seed);
        if fingerprints {
            let decoded = decode(&bytes).unwrap();
            self.oracle.set_fingerprints(
                record.record_ix,
                witness_fingerprint::fingerprint(decoded.view()).unwrap(),
            );
        }
        if reference {
            self.oracle
                .set_reference(record.record_ix, recompress(&bytes, 85));
        }
        (bytes, record)
    }

    fn judge(&self, input: &[u8]) -> Verdict {
        judge(input, &self.oracle, &self.options())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn sealed_original_is_original() {
    let mut f = fx();
    let (bytes, record) = f.anchored_photo(1, false, false);
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::Original);
    assert!(v.reasons.contains(Reason::SealVerified));
    assert_eq!(v.flags, VerdictFlags::EMPTY);
    let r = v.reference.unwrap();
    assert_eq!(r.evidence, Evidence::Seal);
    assert_eq!(r.record_id, record.record_id);
    assert_eq!(
        r.anchor,
        Some(Anchor {
            record_ix: record.record_ix,
            anchored_ns: record.anchored_ns
        })
    );
    assert!(v.declared.is_empty() && v.regions.is_empty());
    assert_eq!(v.version, VERDICT_V);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn presentation_only_derivative_is_original_with_edits_listed() {
    let mut f = fx();
    let (_, parent) = f.anchored_photo(2, false, false);
    let edits = vec![
        Edit::Resize {
            w: 240,
            h: 180,
            filter: 0,
        },
        Edit::Recompress {
            format: ImageFormat::Jpeg,
            quality: 80,
        },
        Edit::Orient { orientation: 1 },
    ];
    let (bytes, claim) = f.sealed(&photo(240, 180, 2), derivative(&parent, edits.clone()), 9);
    f.oracle.anchor(claim, T0 + 50);
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::Original);
    assert_eq!(v.declared, edits);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn content_edited_derivative_matches_with_declared_changes() {
    let mut f = fx();
    let (_, parent) = f.anchored_photo(3, false, false);
    let edits = vec![
        Edit::Crop(FRect {
            x: 10,
            y: 10,
            w: 200,
            h: 150,
        }),
        Edit::Blur {
            rect: FRect {
                x: 20,
                y: 20,
                w: 30,
                h: 30,
            },
            class: BlurClass::Face,
            sigma_e2: 800,
        },
    ];
    let (bytes, claim) = f.sealed(&photo(200, 150, 3), derivative(&parent, edits.clone()), 10);
    f.oracle.anchor(claim, T0 + 60);
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::MatchesWithChanges);
    assert_eq!(v.declared, edits);
    assert_eq!(v.reference.unwrap().evidence, Evidence::Seal);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn modified_pixels_break_the_seal() {
    let mut f = fx();
    let (bytes, record) = f.anchored_photo(4, false, false);
    let mut tampered = bytes.clone();
    let at = tampered.len() - 40;
    tampered[at] ^= 0x10;
    let v = f.judge(&tampered);
    assert_eq!(v.outcome, Outcome::Altered);
    assert!(v.reasons.contains(Reason::SealBroken));
    assert!(v.declared.is_empty());
    assert_eq!(v.reference.unwrap().record_id, record.record_id);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn broken_seal_of_unanchored_capture_is_still_altered() {
    let f = fx();
    let (bytes, claim) = f.sealed(&photo(W, H, 5), Kind::Original, 1);
    let mut tampered = bytes;
    let at = tampered.len() - 30;
    tampered[at] ^= 0x01;
    let v = f.judge(&tampered);
    assert_eq!(v.outcome, Outcome::Altered);
    let r = v.reference.unwrap();
    assert_eq!(r.anchor, None);
    assert_eq!(r.claim, claim);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn enrolled_but_unanchored_is_sealed_unanchored() {
    let f = fx();
    let (bytes, claim) = f.sealed(&photo(W, H, 6), Kind::Original, 1);
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::SealedUnanchored);
    assert!(v.reasons.contains(Reason::SealVerified));
    let r = v.reference.unwrap();
    assert_eq!(r.anchor, None);
    assert_eq!(r.record_id, claim.record_id());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unanchored_seal_from_revoked_key_is_not_trusted() {
    let mut history = KeyHistory::new(ENROLLED_NS);
    history
        .push(StatusChange {
            status: KeyStatus::Revoked,
            since_ns: T0,
        })
        .unwrap();
    let f = fx_with(history);
    let (bytes, _) = f.sealed(&photo(W, H, 7), Kind::Original, 1);
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::KeyNotTrusted));
    assert!(v.reasons.contains(Reason::NoCandidates));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn key_revoked_after_anchor_keeps_verdict_with_flag() {
    let mut history = KeyHistory::new(ENROLLED_NS);
    history
        .push(StatusChange {
            status: KeyStatus::Revoked,
            since_ns: T0 + 1_000_000,
        })
        .unwrap();
    let mut f = fx_with(history);
    let (bytes, _) = f.anchored_photo(8, false, false);
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::Original);
    assert!(v.flags.contains(VerdictFlags::KEY_REVOKED_AFTER_ANCHOR));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn key_untrusted_at_anchor_time_falls_through_and_skips_exact() {
    let mut history = KeyHistory::new(ENROLLED_NS);
    history
        .push(StatusChange {
            status: KeyStatus::Suspended,
            since_ns: T0 - 10,
        })
        .unwrap();
    history
        .push(StatusChange {
            status: KeyStatus::Active,
            since_ns: T0 + 500,
        })
        .unwrap();
    let mut f = fx_with(history);
    let (bytes, _) = f.anchored_photo(9, false, false);
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::KeyNotTrustedAtAnchor));
    assert!(!v.reasons.contains(Reason::ExactAssetHash));
    assert!(!v.flags.contains(VerdictFlags::KEY_REVOKED_AFTER_ANCHOR));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn record_flags_and_dev_keys_are_surfaced() {
    let ca = TestCa::new(7);
    let dev = ca.device(0x42, 0x01_0203_0405);
    let mut oracle = MemoryOracle::new();
    let mut info = dev.key_info(KeyHistory::new(ENROLLED_NS));
    info.provisioning = Provisioning::Dev;
    oracle.add_key(info);
    let mut f = Fx { ca, dev, oracle };
    let (bytes, record) = f.anchored_photo(10, false, false);
    f.oracle.set_flags(
        record.record_ix,
        VerdictFlags::LATE_ANCHOR | VerdictFlags::CLOCK_MISMATCH,
    );
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::Original);
    assert_eq!(
        v.flags,
        VerdictFlags::LATE_ANCHOR | VerdictFlags::CLOCK_MISMATCH | VerdictFlags::DEV_ATTESTATION
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unenrolled_key_is_unverified_and_ignored() {
    let f = fx();
    let stranger = f.ca.device(0x55, 77);
    let claim = stranger.claim(1, Kind::Original, 1);
    let (bytes, _) = stranger.seal(&f.ca.anchor, &jpeg(&photo(W, H, 11), 90), claim);
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::UnverifiedCredentials));
    assert!(v.reference.is_none());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn oracle_key_with_mismatched_pubkey_is_unverified() {
    let mut f = fx();
    let other = f.ca.device(0x56, 78);
    let mut info = other.key_info(KeyHistory::new(ENROLLED_NS));
    info.key_id = f.dev.key_id();
    f.oracle.add_key(info);
    let (bytes, _) = f.sealed(&photo(W, H, 12), Kind::Original, 1);
    let v = f.judge(&bytes);
    assert!(v.reasons.contains(Reason::UnverifiedCredentials));
    assert_ne!(v.outcome, Outcome::SealedUnanchored);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn foreign_ca_credentials_never_decide_but_exact_hash_still_does() {
    let mut f = fx();
    let foreign = TestCa::new(9);
    let dev = foreign.device(0x42, 0x01_0203_0405);
    let base = jpeg(&photo(W, H, 13), 90);
    let (bytes, claim) = dev.seal(&foreign.anchor, &base, dev.claim(3, Kind::Original, 1));
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::UnverifiedCredentials));
    f.oracle.anchor(claim, T0);
    let v = f.judge(&bytes);
    assert_eq!(v.outcome, Outcome::Original);
    assert!(v.reasons.contains(Reason::ExactAssetHash));
    assert!(v.reasons.contains(Reason::UnverifiedCredentials));
    assert_eq!(v.reference.unwrap().evidence, Evidence::ExactHash);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn exact_asset_hash_of_content_edited_derivative_matches_with_changes() {
    let mut f = fx();
    let (_, parent) = f.anchored_photo(14, false, false);
    let plain = jpeg(&photo(W, H, 15), 90);
    let edits = vec![Edit::Tone {
        exposure_e2: 30,
        contrast_e2: 0,
        saturation_e2: -10,
    }];
    let mut claim = f.dev.claim(15, derivative(&parent, edits.clone()), 4);
    claim.asset_hash = witness_c2pa::data_hash(&plain).unwrap();
    f.oracle.anchor(claim, T0 + 99);
    let v = f.judge(&plain);
    assert_eq!(v.outcome, Outcome::MatchesWithChanges);
    assert_eq!(v.declared, edits);
    assert!(v.reasons.contains(Reason::ExactAssetHash));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn exact_pixel_hash_survives_metadata_changes() {
    let mut f = fx();
    let plain = jpeg(&photo(W, H, 16), 90);
    let mut claim = f.dev.claim(16, Kind::Original, 1);
    claim.pixel_hash = witness_image::pixel_hash(&plain).unwrap();
    claim.asset_hash = [7; 32];
    f.oracle.anchor(claim, T0);
    let v = f.judge(&with_comment(&plain));
    assert_eq!(v.outcome, Outcome::Original);
    assert!(v.reasons.contains(Reason::ExactPixelHash));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn exact_hits_resolve_to_earliest_anchor() {
    let mut f = fx();
    let plain = jpeg(&photo(W, H, 17), 90);
    let hash = witness_c2pa::data_hash(&plain).unwrap();
    let mut late = f.dev.claim(1, Kind::Original, 1);
    late.asset_hash = hash;
    let mut early = f.dev.claim(2, Kind::Original, 2);
    early.asset_hash = hash;
    f.oracle.anchor(late, T0 + 100);
    let e = f.oracle.anchor(early, T0 + 1);
    let v = f.judge(&plain);
    assert_eq!(v.reference.unwrap().record_id, e.record_id);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn exact_hits_from_a_lying_oracle_are_ignored() {
    struct Liar(MemoryOracle, Record);
    impl Oracle for Liar {
        fn get_key(&self, k: &witness_format::KeyId) -> Option<witness_format::KeyInfo> {
            self.0.get_key(k)
        }
        fn get_record(&self, id: &witness_format::RecordId) -> Option<Record> {
            self.0.get_record(id)
        }
        fn exact(&self, _: &[u8; 32], _: Option<&[u8; 32]>) -> Vec<Record> {
            vec![self.1.clone()]
        }
        fn by_watermark(&self, c: CaptureId) -> Vec<Record> {
            self.0.by_watermark(c)
        }
        fn pdq_probe(&self, h: &[witness_fingerprint::PdqHash]) -> PdqProbe {
            self.0.pdq_probe(h)
        }
        fn derivatives(&self, r: &Record) -> Vec<Record> {
            self.0.derivatives(r)
        }
        fn reference_image(&self, r: &Record) -> Option<Vec<u8>> {
            self.0.reference_image(r)
        }
        fn fingerprints(&self, r: &Record) -> Option<FingerprintBundle> {
            self.0.fingerprints(r)
        }
    }
    let mut f = fx();
    let (_, record) = f.anchored_photo(18, false, false);
    let liar = Liar(f.oracle.clone(), record);
    let v = judge(&jpeg(&photo(W, H, 19), 90), &liar, &f.options());
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(!v.reasons.contains(Reason::ExactAssetHash));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn undecodable_input_is_no_record_unless_exact_asset_hit() {
    let mut f = fx();
    let garbage = b"definitely not an image".to_vec();
    let v = f.judge(&garbage);
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::Undecodable));
    let mut claim = f.dev.claim(1, Kind::Original, 1);
    claim.asset_hash = witness_format::crypto::sha256(&garbage);
    f.oracle.anchor(claim, T0);
    assert_eq!(f.judge(&garbage).outcome, Outcome::Original);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn empty_and_truncated_inputs_never_panic() {
    let f = fx();
    let (bytes, _) = f.sealed(&photo(W, H, 20), Kind::Original, 1);
    for cut in [0, 1, 2, 3, 4, 10, 100, bytes.len() / 2, bytes.len() - 1] {
        let v = f.judge(&bytes[..cut]);
        assert_ne!(v.outcome, Outcome::Original, "cut {cut}");
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn recompressed_copy_with_reference_matches_with_changes() {
    let mut f = fx();
    let (bytes, record) = f.anchored_photo(21, true, true);
    let copy = recompress(&bytes, 70);
    let v = f.judge(&copy);
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    let r = v.reference.unwrap();
    assert_eq!(r.record_id, record.record_id);
    assert_eq!(r.evidence, Evidence::Pixels);
    assert!(r.confidence_q16 >= TAU_PIXEL_Q16);
    assert!(v.regions.is_empty());
    assert!(!v.flags.contains(VerdictFlags::NO_PUBLIC_REFERENCE));
    assert!(v.reasons.contains(Reason::WatermarkUnavailable));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn local_edit_is_altered_with_regions() {
    let mut f = fx();
    let (bytes, _) = f.anchored_photo(22, true, true);
    let edit = FRect {
        x: 200,
        y: 140,
        w: 56,
        h: 48,
    };
    let edited = paste(&decode(&bytes).unwrap(), edit, &photo(W, H, 999));
    let v = f.judge(&jpeg(&edited, 85));
    assert_eq!(v.outcome, Outcome::Altered, "{v:?}");
    assert_eq!(v.regions.len(), 1);
    let r = v.regions[0];
    let cx = ((r.x + r.w / 2) as u64 * W as u64) >> 16;
    let cy = ((r.y + r.h / 2) as u64 * H as u64) >> 16;
    assert!(
        (200..256).contains(&cx) && (140..188).contains(&cy),
        "{r:?}"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn no_reference_is_capped_and_flagged() {
    let mut f = fx();
    let (bytes, record) = f.anchored_photo(23, false, true);
    let v = f.judge(&recompress(&bytes, 75));
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    assert!(v.flags.contains(VerdictFlags::NO_PUBLIC_REFERENCE));
    let r = v.reference.unwrap();
    assert_eq!(r.evidence, Evidence::Grid);
    assert_eq!(r.record_id, record.record_id);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn heavy_changes_cannot_be_compared() {
    let mut f = fx();
    let (bytes, record) = f.anchored_photo(24, true, true);
    let original = decode(&bytes).unwrap();
    let edited = paste(
        &original,
        FRect {
            x: 0,
            y: 0,
            w: W * 45 / 100,
            h: H,
        },
        &photo(W, H, 4242),
    );
    let wm = FixedWatermarker::reading(record.claim.capture_id);
    let v = judge(
        &jpeg(&edited, 85),
        &f.oracle,
        &f.options().with_watermarker(&wm),
    );
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    assert!(v.flags.contains(VerdictFlags::COULDNT_COMPARE));
    assert!(v.regions.is_empty());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn truncated_search_is_flagged() {
    let mut f = fx();
    let (bytes, _) = f.anchored_photo(25, false, true);
    f.oracle.truncated = true;
    let v = f.judge(&recompress(&bytes, 80));
    assert!(v.flags.contains(VerdictFlags::TRUNCATED_CANDIDATES));
    let v = f.judge(&jpeg(&photo(W, H, 26), 80));
    assert!(v.flags.contains(VerdictFlags::TRUNCATED_CANDIDATES));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unconfirmed_pdq_candidate_gives_a_hint() {
    let mut f = fx();
    f.anchored_photo(27, false, true);
    f.oracle.pdq_radius = 256;
    let v = f.judge(&jpeg(&photo(W, H, 28), 85));
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::Unconfirmed));
    assert!(v.reference.is_none());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn transplanted_watermark_warns_without_reference() {
    let mut f = fx();
    let (_, record) = f.anchored_photo(29, true, true);
    let wm = FixedWatermarker::reading(record.claim.capture_id);
    let options = f.options().with_watermarker(&wm);
    let v = judge(&jpeg(&photo(W, H, 30), 85), &f.oracle, &options);
    assert_eq!(v.outcome, Outcome::NoRecord, "{v:?}");
    assert!(v.reasons.contains(Reason::WatermarkFound));
    assert!(v.reasons.contains(Reason::ForeignWatermark));
    assert!(v.reference.is_none() && v.regions.is_empty());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn noisy_watermark_on_unrelated_content_is_only_a_hint() {
    let mut f = fx();
    let (_, record) = f.anchored_photo(31, false, true);
    let wm = FixedWatermarker {
        capture_id: Some(record.claim.capture_id),
        flipped: 3,
    };
    let v = judge(
        &jpeg(&photo(W, H, 32), 85),
        &f.oracle,
        &f.options().with_watermarker(&wm),
    );
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::Unconfirmed));
    assert!(!v.reasons.contains(Reason::ForeignWatermark));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn watermark_finds_a_crop_that_pdq_misses() {
    let mut f = fx();
    let (bytes, record) = f.anchored_photo(33, true, true);
    let original = decode(&bytes).unwrap();
    let crop = original.crop(150, 40, 300, 280).unwrap().to_image();
    let copy = jpeg(&crop, 85);
    let without = f.judge(&copy);
    let wm = FixedWatermarker::reading(record.claim.capture_id);
    let with = judge(&copy, &f.oracle, &f.options().with_watermarker(&wm));
    assert_eq!(with.outcome, Outcome::MatchesWithChanges, "{with:?}");
    assert_eq!(with.reference.unwrap().record_id, record.record_id);
    assert!(with.flags.contains(VerdictFlags::COULDNT_COMPARE));
    assert!(matches!(
        without.outcome,
        Outcome::NoRecord | Outcome::MatchesWithChanges
    ));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unavailable_watermarker_is_reported() {
    let mut f = fx();
    f.anchored_photo(34, false, true);
    let input = jpeg(&photo(W, H, 35), 85);
    let v = judge(
        &input,
        &f.oracle,
        &f.options().with_watermarker(&NullWatermarker),
    );
    assert!(v.reasons.contains(Reason::WatermarkUnavailable));
    let blank = FixedWatermarker::blank();
    let v = judge(&input, &f.oracle, &f.options().with_watermarker(&blank));
    assert!(!v.reasons.contains(Reason::WatermarkUnavailable));
    assert!(!v.reasons.contains(Reason::WatermarkFound));
    assert!(v.reasons.contains(Reason::NoCandidates));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn pending_index_is_noted_on_no_record() {
    let mut f = fx();
    f.oracle.pending = true;
    let v = f.judge(&jpeg(&photo(W, H, 36), 85));
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::IndexPending));
    f.anchored_photo(37, false, true);
    f.oracle.pdq_radius = 256;
    let v = f.judge(&jpeg(&photo(W, H, 38), 85));
    assert!(v.reasons.contains(Reason::Unconfirmed) && v.reasons.contains(Reason::IndexPending));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn top_k_limits_confirmation_attempts() {
    let mut f = fx();
    let (bytes, real) = f.anchored_photo(39, true, false);
    let copy = recompress(&bytes, 80);
    let copy_bundle = witness_fingerprint::fingerprint(decode(&copy).unwrap().view()).unwrap();
    let unrelated = witness_fingerprint::fingerprint(photo(W, H, 40).view()).unwrap();
    let decoy_bundle = FingerprintBundle::new(
        copy_bundle.pdq().to_vec(),
        copy_bundle.quality(),
        *unrelated.grid(),
        unrelated.portrait(),
    )
    .unwrap();
    let mut decoy = f.dev.claim(0xC0FFEE, Kind::Original, 99);
    decoy.capture_id = real.claim.capture_id;
    decoy.asset_hash = [1; 32];
    let decoy = f.oracle.anchor(decoy, T0 + 5);
    f.oracle.set_fingerprints(decoy.record_ix, decoy_bundle);
    let wm = FixedWatermarker::reading(real.claim.capture_id);
    let narrow = Options::new(&f.ca.anchor, 1).with_watermarker(&wm);
    let wide = f.options().with_watermarker(&wm);
    let v1 = judge(&copy, &f.oracle, &narrow);
    assert_eq!(v1.outcome, Outcome::NoRecord, "{v1:?}");
    let v8 = judge(&copy, &f.oracle, &wide);
    assert_eq!(v8.outcome, Outcome::MatchesWithChanges, "{v8:?}");
    assert_eq!(v8.reference.unwrap().record_id, real.record_id);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn public_derivative_that_best_explains_pixels_is_chosen() {
    let mut f = fx();
    let (bytes, parent) = f.anchored_photo(41, true, true);
    let original = decode(&bytes).unwrap();
    let rect = FRect {
        x: 180,
        y: 120,
        w: 80,
        h: 70,
    };
    let blurred = Rgb8::from_fn(W, H, |x, y| {
        let inside = x >= rect.x && y >= rect.y && x < rect.x + rect.w && y < rect.y + rect.h;
        if inside {
            [128, 120, 110]
        } else {
            original.pixel(x, y).unwrap()
        }
    })
    .unwrap();
    let edits = vec![Edit::Blur {
        rect,
        class: BlurClass::Plate,
        sigma_e2: 1200,
    }];
    let mut claim = f.dev.claim(41, derivative(&parent, edits.clone()), 2);
    claim.asset_hash = [3; 32];
    let d = f.oracle.anchor(claim, T0 + 900);
    f.oracle.set_reference(d.record_ix, jpeg(&blurred, 90));
    let v = f.judge(&jpeg(&blurred, 80));
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    assert_eq!(v.reference.unwrap().record_id, d.record_id);
    assert_eq!(v.declared, edits);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn verdicts_are_deterministic() {
    let mut f = fx();
    let (bytes, _) = f.anchored_photo(42, true, true);
    let edited = paste(
        &decode(&bytes).unwrap(),
        FRect {
            x: 100,
            y: 100,
            w: 60,
            h: 50,
        },
        &photo(W, H, 43),
    );
    for input in [bytes.clone(), recompress(&bytes, 60), jpeg(&edited, 85)] {
        assert_eq!(f.judge(&input), f.judge(&input));
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn web_and_android_options_differ_only_in_k() {
    let ca = TestCa::new(1);
    let (a, w) = (Options::android(&ca.anchor), Options::web(&ca.anchor));
    assert_eq!((a.top_k, w.top_k), (ANDROID_TOP_K, WEB_TOP_K));
    assert_eq!((a.at_s, a.max_pixels), (w.at_s, w.max_pixels));
    assert!(a.watermarker.is_none());
    assert_eq!(Options::new(&ca.anchor, 0).top_k, 1);
    assert_eq!(Options::new(&ca.anchor, 1000).top_k, MAX_TOP_K);
    assert_eq!(a.at(NOW_S).at_s, Some(NOW_S));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn certificate_validity_uses_injected_time() {
    let mut f = fx();
    let (bytes, _) = f.anchored_photo(44, false, false);
    let expired = judge(&bytes, &f.oracle, &f.options().at(NOT_AFTER_S + 1));
    assert!(expired.reasons.contains(Reason::UnverifiedCredentials));
    assert!(!expired.reasons.contains(Reason::SealVerified));
    assert_eq!(expired.reference.unwrap().evidence, Evidence::ExactHash);
    let valid = judge(&bytes, &f.oracle, &f.options().at(NOW_S));
    assert_eq!(valid.outcome, Outcome::Original);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn random_mutations_never_forge_an_original() {
    let mut f = fx();
    let (bytes, record) = f.anchored_photo(45, false, false);
    let mut s = 0x9E37_79B9_7F4A_7C15_u64;
    for i in 0..24 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let mut m = bytes.clone();
        for k in 0..1 + (s % 4) as usize {
            let at = ((s >> (8 * k)) as usize) % m.len();
            m[at] ^= 1 << ((s >> 40) % 8);
        }
        if m == bytes {
            continue;
        }
        let v = f.judge(&m);
        if v.outcome == Outcome::Original {
            let same_asset = witness_c2pa::data_hash(&m).ok() == Some(record.claim.asset_hash);
            let same_pixels = witness_image::pixel_hash(&m).ok() == Some(record.claim.pixel_hash);
            assert!(same_asset || same_pixels, "mutation {i} forged an original");
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn random_bytes_never_panic() {
    let f = fx();
    let mut s = 1u64;
    for len in [0usize, 1, 2, 8, 64, 512, 4096] {
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                (s >> 56) as u8
            })
            .collect();
        let mut jpegish = vec![0xFF, 0xD8, 0xFF];
        jpegish.extend_from_slice(&bytes);
        for input in [bytes, jpegish] {
            let v = f.judge(&input);
            assert_eq!(v.outcome, Outcome::NoRecord);
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn downscaled_previews_still_confirm() {
    let mut f = fx();
    let (bytes, record) = f.anchored_photo(46, false, true);
    let small = resize_area(decode(&bytes).unwrap().view(), W / 2, H / 2).unwrap();
    f.oracle.set_reference(record.record_ix, jpeg(&small, 85));
    let v = f.judge(&recompress(&bytes, 75));
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    assert_eq!(v.reference.unwrap().evidence, Evidence::Pixels);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn seeded_unedited_copies_are_never_accused() {
    let mut f = fx();
    for seed in 100..106 {
        let (bytes, record) = f.anchored_photo(seed, seed % 2 == 0, true);
        for copy in [
            recompress(&bytes, 60 + (seed % 5) as u8 * 8),
            jpeg(
                &resize_area(decode(&bytes).unwrap().view(), W * 3 / 4, H * 3 / 4).unwrap(),
                80,
            ),
        ] {
            let v = f.judge(&copy);
            assert_eq!(v.outcome, Outcome::MatchesWithChanges, "seed {seed}: {v:?}");
            assert_eq!(
                v.reference.unwrap().record_id,
                record.record_id,
                "seed {seed}"
            );
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn seeded_local_edits_are_caught_with_a_reference() {
    let mut f = fx();
    let mut caught = 0;
    for seed in 200..206 {
        let (bytes, record) = f.anchored_photo(seed, true, true);
        let x = 120 + (seed as u32 % 5) * 40;
        let y = 100 + (seed as u32 % 3) * 40;
        let edited = paste(
            &decode(&bytes).unwrap(),
            FRect { x, y, w: 56, h: 48 },
            &photo(W, H, seed * 31),
        );
        let v = f.judge(&jpeg(&edited, 85));
        assert_ne!(v.outcome, Outcome::NoRecord, "seed {seed}");
        assert_eq!(v.reference.as_ref().unwrap().record_id, record.record_id);
        caught += usize::from(v.outcome == Outcome::Altered);
    }
    assert!(caught >= 5, "caught {caught}/6");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn grid_only_catches_large_local_edits() {
    let mut f = fx();
    let (bytes, record) = f.anchored_photo(300, false, true);
    let edited = paste(
        &decode(&bytes).unwrap(),
        FRect {
            x: 180,
            y: 120,
            w: 90,
            h: 90,
        },
        &photo(W, H, 301),
    );
    let wm = FixedWatermarker::reading(record.claim.capture_id);
    let v = judge(
        &jpeg(&edited, 85),
        &f.oracle,
        &f.options().with_watermarker(&wm),
    );
    assert_eq!(v.outcome, Outcome::Altered, "{v:?}");
    assert!(v.flags.contains(VerdictFlags::NO_PUBLIC_REFERENCE));
    assert_eq!(v.reference.unwrap().evidence, Evidence::Grid);
    assert!(!v.regions.is_empty());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn foreign_content_credentials_are_noted_but_ignored() {
    use witness_c2pa::{app11, assertions, claim, manifest};
    let mut f = fx();
    let plain = jpeg(&photo(W, H, 400), 90);
    let label = claim::manifest_label(&[3; 16]);
    let actions = assertions::cbor_assertion_box(
        assertions::ACTIONS,
        &assertions::actions_cbor(CAPTURED).unwrap(),
    )
    .unwrap();
    let mut store = Vec::new();
    manifest::write_store(&mut store, &label, &[&actions], &[0xA0], &[0x80]).unwrap();
    let mut foreign = plain[..2].to_vec();
    app11::write_packets(&mut foreign, &store, 1, app11::MAX_CHUNK).unwrap();
    foreign.extend_from_slice(&plain[2..]);
    let v = f.judge(&foreign);
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reasons.contains(Reason::OtherCredentials));
    assert!(!v.reasons.contains(Reason::UnverifiedCredentials));
    let mut c = f.dev.claim(400, Kind::Original, 1);
    c.asset_hash = witness_c2pa::data_hash(&plain).unwrap();
    f.oracle.anchor(c, T0);
    let v = f.judge(&foreign);
    assert_eq!(v.outcome, Outcome::Original);
    assert!(
        v.reasons.contains(Reason::OtherCredentials) && v.reasons.contains(Reason::ExactAssetHash)
    );
}

const SIZES: [(u32, u32); 8] = [
    (96, 72),
    (72, 96),
    (128, 96),
    (160, 120),
    (192, 144),
    (256, 192),
    (640, 480),
    (1024, 768),
];

fn assert_full_compare(v: &Verdict, record: &Record, label: &str) {
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{label}: {v:?}");
    assert!(
        !v.flags.contains(VerdictFlags::COULDNT_COMPARE),
        "{label}: {v:?}"
    );
    assert!(v.regions.is_empty(), "{label}: {v:?}");
    let r = v.reference.as_ref().unwrap();
    assert_eq!(r.record_id, record.record_id, "{label}");
    assert_eq!(r.evidence, Evidence::Pixels, "{label}");
    assert_eq!(r.changed_q16, 0, "{label}: {v:?}");
    assert_eq!(r.compared_q16, Q16_ONE, "{label}: {v:?}");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn qa_192x144_recompress_is_a_full_compare() {
    let mut f = fx();
    let (bytes, record) = f.anchored_sized(7003, (192, 144), true, true);
    for q in [50, 60, 70, 80, 95] {
        assert_full_compare(&f.judge(&recompress(&bytes, q)), &record, &format!("q{q}"));
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn plain_recompress_is_a_full_compare_across_sizes_and_qualities() {
    let mut f = fx();
    for (i, &(w, h)) in SIZES.iter().enumerate() {
        let (bytes, record) = f.anchored_sized(500 + i as u64, (w, h), true, true);
        for q in [50, 65, 80, 95] {
            let v = f.judge(&recompress(&bytes, q));
            assert_full_compare(&v, &record, &format!("{w}x{h} q{q}"));
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn seeded_small_recompresses_are_never_couldnt_compare() {
    let mut f = fx();
    for seed in 0..12u64 {
        let (w, h) = SIZES[seed as usize % 6];
        let (bytes, record) = f.anchored_sized(600 + seed, (w, h), true, true);
        let q = 50 + (seed % 4) as u8 * 15;
        let v = f.judge(&recompress(&bytes, q));
        assert_full_compare(&v, &record, &format!("seed {seed} {w}x{h} q{q}"));
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn small_frame_local_edit_above_codec_floor_is_still_altered() {
    let mut f = fx();
    let (bytes, record) = f.anchored_sized(700, (192, 144), true, true);
    let edit = FRect {
        x: 80,
        y: 56,
        w: 28,
        h: 28,
    };
    let edited = paste(&decode(&bytes).unwrap(), edit, &photo(192, 144, 701));
    let v = f.judge(&jpeg(&edited, 80));
    assert_eq!(v.outcome, Outcome::Altered, "{v:?}");
    assert_eq!(v.reference.unwrap().record_id, record.record_id);
    assert_eq!(v.regions.len(), 1);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn small_frame_recompress_verdict_is_deterministic() {
    let mut f = fx();
    let (bytes, _) = f.anchored_sized(710, (160, 120), true, true);
    let copy = recompress(&bytes, 50);
    assert_eq!(f.judge(&copy), f.judge(&copy));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn plain_recompress_at_the_query_cap_is_a_full_compare() {
    let mut f = fx();
    let (bytes, record) = f.anchored_sized(520, (2048, 1536), true, true);
    assert_full_compare(&f.judge(&recompress(&bytes, 50)), &record, "2048x1536 q50");
}

fn assert_full_grid_compare(v: &Verdict, record: &Record, label: &str) {
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{label}: {v:?}");
    assert!(
        !v.flags.contains(VerdictFlags::COULDNT_COMPARE),
        "{label}: {v:?}"
    );
    assert!(
        v.flags.contains(VerdictFlags::NO_PUBLIC_REFERENCE),
        "{label}: {v:?}"
    );
    assert!(v.regions.is_empty(), "{label}: {v:?}");
    let r = v.reference.as_ref().unwrap();
    assert_eq!(r.record_id, record.record_id, "{label}");
    assert_eq!(r.evidence, Evidence::Grid, "{label}");
    assert_eq!(r.changed_q16, 0, "{label}: {v:?}");
    assert_eq!(r.compared_q16, Q16_ONE, "{label}: {v:?}");
}

fn assert_full_compare_either_way(v: &Verdict, record: &Record, reference: bool, label: &str) {
    if reference {
        assert_full_compare(v, record, label);
    } else {
        assert_full_grid_compare(v, record, label);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn grid_only_640x480_q80_recompresses_are_full_compares() {
    let mut f = fx();
    for seed in 0..12u64 {
        let (bytes, record) = f.anchored_sized(1100 + seed, (640, 480), false, true);
        let v = f.judge(&recompress(&bytes, 80));
        assert_full_grid_compare(&v, &record, &format!("seed {seed}"));
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn recompress_is_a_full_compare_with_and_without_reference() {
    let mut f = fx();
    for (i, &(w, h)) in SIZES.iter().enumerate() {
        for reference in [false, true] {
            let seed = 1200 + i as u64 * 2 + u64::from(reference);
            let (bytes, record) = f.anchored_sized(seed, (w, h), reference, true);
            for q in [50, 95] {
                let v = f.judge(&recompress(&bytes, q));
                let label = format!("{w}x{h} q{q} reference={reference}");
                assert_full_compare_either_way(&v, &record, reference, &label);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn assert_unchanged_full_compare(v: &Verdict, record: &Record, label: &str) {
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{label}: {v:?}");
    assert!(
        !v.flags.contains(VerdictFlags::COULDNT_COMPARE),
        "{label}: {v:?}"
    );
    assert!(v.regions.is_empty(), "{label}: {v:?}");
    let r = v.reference.as_ref().unwrap();
    assert_eq!(r.record_id, record.record_id, "{label}");
    assert_eq!(r.changed_q16, 0, "{label}: {v:?}");
    assert_eq!(r.compared_q16, Q16_ONE, "{label}: {v:?}");
}

#[cfg(not(target_arch = "wasm32"))]
fn recompress_matrix(sizes: &[(u32, u32)], base_seed: u64) {
    let mut f = fx();
    for (i, &(w, h)) in sizes.iter().enumerate() {
        for (j, q) in (50..=95).step_by(5).enumerate() {
            for reference in [false, true].into_iter().take(1 + (j + 1) % 2) {
                let seed = base_seed + i as u64 * 100 + j as u64 * 2 + u64::from(reference);
                let (bytes, record) = f.anchored_sized(seed, (w, h), reference, true);
                let v = f.judge(&recompress(&bytes, q));
                let label = format!("{w}x{h} seed {seed} q{q} reference={reference}");
                if !reference {
                    assert_full_grid_compare(&v, &record, &label);
                } else if SIZES.contains(&(w, h)) {
                    assert_full_compare(&v, &record, &label);
                } else {
                    assert_unchanged_full_compare(&v, &record, &label);
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn seeded_recompress_matrix_small_frames() {
    recompress_matrix(
        &[
            (48, 36),
            (96, 72),
            (72, 96),
            (112, 84),
            (128, 96),
            (136, 102),
        ],
        1300,
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn seeded_recompress_matrix_medium_frames() {
    recompress_matrix(
        &[
            (160, 120),
            (176, 132),
            (192, 144),
            (192, 256),
            (256, 192),
            (400, 300),
        ],
        2300,
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn seeded_recompress_matrix_640x480() {
    recompress_matrix(&[(640, 480)], 3300);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn seeded_recompress_matrix_1024x768() {
    recompress_matrix(&[(1024, 768)], 4300);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn seeded_grid_only_downscales_are_never_accused() {
    let mut f = fx();
    for seed in 0..8u64 {
        let (w, h) = [(640, 480), (1024, 768), (480, 360), (256, 192)][seed as usize % 4];
        let (bytes, record) = f.anchored_sized(1500 + seed, (w, h), false, true);
        let decoded = decode(&bytes).unwrap();
        for divisor in [2, 3, 4] {
            let small = resize_area(decoded.view(), w / divisor, h / divisor).unwrap();
            for q in [55, 75, 90] {
                let v = f.judge(&jpeg(&small, q));
                let label = format!("{w}x{h}/{divisor} seed {seed} q{q}");
                assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{label}: {v:?}");
                assert!(v.regions.is_empty(), "{label}: {v:?}");
                assert_eq!(v.reference.unwrap().record_id, record.record_id, "{label}");
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn grid_only_local_edits_on_full_frames_are_still_caught() {
    let mut f = fx();
    let mut caught = 0;
    for seed in 0..6u64 {
        let (bytes, record) = f.anchored_sized(1600 + seed, (640, 480), false, true);
        let edit = FRect {
            x: 160 + (seed as u32 % 3) * 80,
            y: 120 + (seed as u32 % 2) * 80,
            w: 120,
            h: 120,
        };
        let edited = paste(
            &decode(&bytes).unwrap(),
            edit,
            &photo(640, 480, 1700 + seed),
        );
        let wm = FixedWatermarker::reading(record.claim.capture_id);
        let v = judge(
            &jpeg(&edited, 85),
            &f.oracle,
            &f.options().with_watermarker(&wm),
        );
        assert!(
            matches!(v.outcome, Outcome::Altered | Outcome::MatchesWithChanges),
            "seed {seed}: {v:?}"
        );
        assert_eq!(v.reference.as_ref().unwrap().evidence, Evidence::Grid);
        assert_eq!(v.outcome == Outcome::Altered, !v.regions.is_empty());
        caught += usize::from(v.outcome == Outcome::Altered);
    }
    assert!(caught >= 2, "caught {caught}/6");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn grid_only_crop_is_not_a_full_compare() {
    let mut f = fx();
    let (bytes, record) = f.anchored_sized(1800, (640, 480), false, true);
    let crop = decode(&bytes)
        .unwrap()
        .crop(128, 0, 384, 480)
        .unwrap()
        .to_image();
    let v = f.judge(&jpeg(&crop, 85));
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{v:?}");
    assert!(v.flags.contains(VerdictFlags::COULDNT_COMPARE), "{v:?}");
    let r = v.reference.unwrap();
    assert_eq!(r.record_id, record.record_id);
    assert_eq!(r.evidence, Evidence::Grid);
    assert!(r.compared_q16 < FULL_COMPARED_Q16, "{r:?}");
}

fn removed_percent(r: &Reference) -> i64 {
    100 - ((u64::from(r.compared_q16) * 100 + u64::from(Q16_ONE / 2)) / u64::from(Q16_ONE)) as i64
}

fn anchored_crop(img: &Rgb8, removed: u32, horizontal: bool, anchor: u32) -> Rgb8 {
    let (w, h) = img.dimensions();
    let keep = |len: u32| len * (100 - removed) / 100;
    let offset = |len: u32, kept: u32| (len - kept) * anchor / 2;
    let (cw, ch) = if horizontal {
        (keep(w), h)
    } else {
        (w, keep(h))
    };
    img.crop(offset(w, cw), offset(h, ch), cw, ch)
        .unwrap()
        .to_image()
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn uncropped_copy_reports_no_crop() {
    let mut f = fx();
    let (bytes, record) = f.anchored_sized(5000, (1024, 768), true, true);
    let v = f.judge(&recompress(&bytes, 75));
    assert_full_compare(&v, &record, "1024x768 q75");
    assert_eq!(removed_percent(v.reference.as_ref().unwrap()), 0);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn uncropped_resizes_report_full_coverage_across_sizes_qualities_and_seeds() {
    let mut f = fx();
    for (i, &(w, h)) in [(256, 192), (480, 360), (640, 480), (480, 640)]
        .iter()
        .enumerate()
    {
        for seed in 0..1u64 {
            let s = 5100 + i as u64 * 10 + seed;
            let (bytes, record) = f.anchored_sized(s, (w, h), true, true);
            let decoded = decode(&bytes).unwrap();
            for (num, den) in [(1, 1), (3, 4), (1, 2), (5, 4)] {
                let resized = resize_area(decoded.view(), w * num / den, h * num / den).unwrap();
                for q in [55, 90] {
                    let v = f.judge(&jpeg(&resized, q));
                    let label = format!("{w}x{h} seed {s} x{num}/{den} q{q}");
                    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{label}: {v:?}");
                    assert!(v.regions.is_empty(), "{label}: {v:?}");
                    let r = v.reference.as_ref().unwrap();
                    assert_eq!(r.record_id, record.record_id, "{label}");
                    assert_eq!(r.evidence, Evidence::Pixels, "{label}");
                    assert_eq!(r.compared_q16, Q16_ONE, "{label}: {v:?}");
                    assert!(!v.flags.contains(VerdictFlags::COULDNT_COMPARE), "{label}");
                }
            }
        }
    }
}

fn assert_crop_reported(v: &Verdict, record: &Record, removed: u32, label: &str) {
    assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{label}: {v:?}");
    assert!(v.regions.is_empty(), "{label}: {v:?}");
    let r = v.reference.as_ref().unwrap();
    assert_eq!(r.record_id, record.record_id, "{label}");
    assert_eq!(r.evidence, Evidence::Pixels, "{label}: {v:?}");
    assert!(r.compared_q16 < Q16_ONE, "{label}: {v:?}");
    let reported = removed_percent(r);
    assert!(
        (reported - i64::from(removed)).abs() <= 2,
        "{label}: reported {reported}%"
    );
    let flagged = v.flags.contains(VerdictFlags::COULDNT_COMPARE);
    if removed > 25 {
        assert!(flagged, "{label}: {v:?}");
    } else if r.changed_q16 == 0 {
        assert!(!flagged, "{label}: {v:?}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn real_crops_report_their_removed_share_across_sizes_and_seeds() {
    let mut f = fx();
    for (i, &(w, h)) in [(256, 192), (480, 360), (640, 480)].iter().enumerate() {
        for seed in [i as u64 % 2] {
            let s = 5200 + i as u64 * 10 + seed;
            let (bytes, record) = f.anchored_sized(s, (w, h), true, true);
            let decoded = decode(&bytes).unwrap();
            let wm = FixedWatermarker::reading(record.claim.capture_id);
            let options = f.options().with_watermarker(&wm);
            for removed in [5u32, 10, 20, 30, 40, 50, 60] {
                for (horizontal, anchor) in [(true, 1), (true, (seed as u32) * 2), (false, 1)] {
                    if !horizontal && removed > 30 {
                        continue;
                    }
                    let crop = anchored_crop(&decoded, removed, horizontal, anchor);
                    let v = judge(&jpeg(&crop, 85), &f.oracle, &options);
                    let label = format!("{w}x{h} seed {s} -{removed}% h={horizontal} a={anchor}");
                    assert_crop_reported(&v, &record, removed, &label);
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn two_axis_crops_report_the_removed_area() {
    let mut f = fx();
    let (bytes, record) = f.anchored_sized(5300, (640, 480), true, true);
    let decoded = decode(&bytes).unwrap();
    let wm = FixedWatermarker::reading(record.claim.capture_id);
    let options = f.options().with_watermarker(&wm);
    for (x, y, cw, ch, removed) in [
        (32, 24, 576, 432, 19),
        (64, 48, 512, 384, 36),
        (0, 0, 448, 336, 51),
    ] {
        let crop = decoded.crop(x, y, cw, ch).unwrap().to_image();
        let v = judge(&jpeg(&crop, 85), &f.oracle, &options);
        assert_crop_reported(&v, &record, removed, &format!("{cw}x{ch}+{x}+{y}"));
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn anchored_one_axis_crops_are_found_without_a_watermark() {
    let mut f = fx();
    let (bytes, record) = f.anchored_sized(5400, (640, 480), true, true);
    let decoded = decode(&bytes).unwrap();
    for removed in [5u32, 10, 20, 30, 50] {
        for anchor in [0, 1, 2] {
            let crop = anchored_crop(&decoded, removed, true, anchor);
            let v = f.judge(&jpeg(&crop, 85));
            let label = format!("-{removed}% anchor {anchor}");
            assert_crop_reported(&v, &record, removed, &label);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn crops_of_unrelated_photos_never_match_a_record() {
    let mut f = fx();
    for seed in 0..4u64 {
        f.anchored_sized(5500 + seed, (640, 480), seed % 2 == 0, true);
    }
    for seed in 0..4u64 {
        let other = photo(640, 480, 5600 + seed);
        for removed in [10u32, 30, 50] {
            for anchor in [0, 1, 2] {
                let crop = anchored_crop(&other, removed, true, anchor);
                let v = f.judge(&jpeg(&crop, 85));
                assert_eq!(
                    v.outcome,
                    Outcome::NoRecord,
                    "seed {seed} -{removed}% a{anchor}"
                );
                assert!(v.reference.is_none());
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn unedited_crops_that_misregistered_are_not_accused() {
    for (ix, seed, (w, h), removed, horizontal, anchor) in [
        (1, 7201, (256, 192), 60, true, 1),
        (6, 7212, (480, 360), 40, true, 0),
        (13, 7241, (2048, 1536), 50, true, 0),
        (14, 7242, (2048, 1536), 20, false, 0),
        (5, 5221, (1024, 768), 5, true, 2),
    ] {
        let mut f = fx();
        for pad in 0..ix {
            f.oracle
                .anchor(f.dev.claim(pad, Kind::Original, 90_000 + pad), T0);
        }
        let s = seed;
        let (bytes, record) = f.anchored_sized(s, (w, h), true, true);
        assert_eq!(record.record_ix, ix);
        let crop = anchored_crop(&decode(&bytes).unwrap(), removed, horizontal, anchor);
        let wm = FixedWatermarker::reading(record.claim.capture_id);
        let v = judge(
            &jpeg(&crop, 85),
            &f.oracle,
            &f.options().with_watermarker(&wm),
        );
        let label = format!("{w}x{h} seed {s} -{removed}% h={horizontal} a={anchor}");
        assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{label}: {v:?}");
        assert!(v.regions.is_empty(), "{label}: {v:?}");
        assert_eq!(v.reference.unwrap().record_id, record.record_id, "{label}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn rotated(img: &Rgb8, degrees: f64) -> Rgb8 {
    let (w, h) = img.dimensions();
    let (s, c) = degrees.to_radians().sin_cos();
    let (cx, cy) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
    let at = |x: f64, y: f64| {
        let x = x.clamp(0.0, f64::from(w - 1));
        let y = y.clamp(0.0, f64::from(h - 1));
        let (x0, y0) = (x.floor() as u32, y.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
        let (fx, fy) = (x - f64::from(x0), y - f64::from(y0));
        let p = |px, py| img.pixel(px, py).unwrap().map(f64::from);
        let (a, b, cc, d) = (p(x0, y0), p(x1, y0), p(x0, y1), p(x1, y1));
        core::array::from_fn(|i| {
            let top = a[i] + (b[i] - a[i]) * fx;
            let bottom = cc[i] + (d[i] - cc[i]) * fx;
            (top + (bottom - top) * fy).round() as u8
        })
    };
    Rgb8::from_fn(w, h, |x, y| {
        let (dx, dy) = (f64::from(x) + 0.5 - cx, f64::from(y) + 0.5 - cy);
        at(c * dx + s * dy + cx - 0.5, -s * dx + c * dy + cy - 0.5)
    })
    .unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
fn assert_never_clean(v: &Verdict, record: &Record, label: &str) {
    assert_eq!(
        v.reference.as_ref().map(|r| r.record_id),
        Some(record.record_id),
        "{label}: {v:?}"
    );
    assert!(
        v.outcome == Outcome::Altered || v.flags.contains(VerdictFlags::COULDNT_COMPARE),
        "{label}: {v:?}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn local_edits_inside_crops_are_never_reported_clean() {
    let mut f = fx();
    let mut altered = 0;
    let mut total = 0;
    for s in [8000u64, 8002] {
        let (bytes, record) = f.anchored_sized(s, (640, 480), true, true);
        let decoded = decode(&bytes).unwrap();
        let wm = FixedWatermarker::reading(record.claim.capture_id);
        let options = f.options().with_watermarker(&wm);
        for removed in [10u32, 30] {
            for horizontal in [true, false] {
                for anchor in [0, 2] {
                    let crop = anchored_crop(&decoded, removed, horizontal, anchor);
                    let (cw, ch) = crop.dimensions();
                    let edit = FRect {
                        x: cw * 2 / 5,
                        y: ch * 2 / 5,
                        w: 64,
                        h: 64,
                    };
                    let edited = paste(&crop, edit, &photo(640, 480, s * 7 + 1));
                    let v = judge(&jpeg(&edited, 85), &f.oracle, &options);
                    let label = format!("seed {s} -{removed}% h={horizontal} a={anchor}");
                    assert_never_clean(&v, &record, &label);
                    altered += usize::from(v.outcome == Outcome::Altered);
                    total += 1;
                }
            }
        }
    }
    assert!(altered * 5 >= total * 4, "altered {altered}/{total}");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn rotation_drift_neither_accuses_clean_crops_nor_clears_edited_ones() {
    let mut f = fx();
    for s in [8001u64, 8002] {
        let (bytes, record) = f.anchored_sized(s, (640, 480), true, true);
        let decoded = decode(&bytes).unwrap();
        let wm = FixedWatermarker::reading(record.claim.capture_id);
        let options = f.options().with_watermarker(&wm);
        for degrees in [0.2f64, 0.5] {
            let turned = rotated(&anchored_crop(&decoded, 20, true, 1), degrees);
            let label = format!("seed {s} {degrees}deg");
            let v = judge(&jpeg(&turned, 85), &f.oracle, &options);
            assert_eq!(v.outcome, Outcome::MatchesWithChanges, "{label}: {v:?}");
            assert!(v.regions.is_empty(), "{label}: {v:?}");
            assert_eq!(
                removed_percent(v.reference.as_ref().unwrap()),
                20,
                "{label}"
            );
            let (cw, ch) = turned.dimensions();
            let edit = FRect {
                x: cw / 3,
                y: ch / 3,
                w: 64,
                h: 64,
            };
            let edited = paste(&turned, edit, &photo(640, 480, s * 7 + 2));
            let v = judge(&jpeg(&edited, 85), &f.oracle, &options);
            assert_never_clean(&v, &record, &format!("{label} edited"));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn crops_resized_or_padded_back_to_full_frame_do_not_hide_the_crop() {
    let mut f = fx();
    for s in [8000u64, 8001] {
        let (bytes, record) = f.anchored_sized(s, (640, 480), true, true);
        let decoded = decode(&bytes).unwrap();
        let wm = FixedWatermarker::reading(record.claim.capture_id);
        let options = f.options().with_watermarker(&wm);
        for (keep, x, y, removed) in [(90u32, 32, 24, 19), (80, 0, 0, 36)] {
            let crop = decoded
                .crop(x, y, 640 * keep / 100, 480 * keep / 100)
                .unwrap()
                .to_image();
            let zoomed = resize_area(crop.view(), 640, 480).unwrap();
            let v = judge(&jpeg(&zoomed, 85), &f.oracle, &options);
            assert_crop_reported(&v, &record, removed, &format!("seed {s} zoom {keep}%"));
        }
        let crop = anchored_crop(&decoded, 15, true, 0);
        let cw = crop.width();
        let padded = Rgb8::from_fn(640, 480, |x, y| {
            if x < cw {
                crop.pixel(x, y).unwrap()
            } else {
                [128, 128, 128]
            }
        })
        .unwrap();
        let v = judge(&jpeg(&padded, 85), &f.oracle, &options);
        assert_never_clean(&v, &record, &format!("seed {s} padded"));
    }
}
