use witness_canister_common::Submission;
use witness_core::c2pa::TrustAnchor;
use witness_core::format::{CaptureId, FingerprintBundle, Kind, RecordId, SecurityLevel, cbor};
use witness_core::image::{Orientation, Rgb8, decode, encode_jpeg, resize_area};
use witness_core::seal::SealError;
use witness_core::{Capture, ClaimTemplate, Manifest, SEAL_QUALITY, SealedCapture};
use witness_verdict::testkit::{CAPTURED, GENERATOR, TestDevice};

pub const MAX_PROBE_HASHES: usize = witness_canister_common::limits::MAX_PROBE_PDQ;

pub fn template(
    dev: &TestDevice,
    capture: u64,
    kind: Kind,
    counter: u64,
    now_ms: u64,
) -> ClaimTemplate {
    let mut c = dev.claim(capture, kind, counter);
    c.time.wall_ms = now_ms;
    c.checks.key_security = SecurityLevel::Dev;
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

pub fn seal(
    dev: &TestDevice,
    anchor: &TrustAnchor,
    image: &Rgb8,
    template: ClaimTemplate,
) -> Result<SealedCapture, SealError> {
    let jpeg = encode_jpeg(image, 95)?;
    let manifest = Manifest {
        id: [0x5e; 16],
        generator: GENERATOR,
        actions: CAPTURED,
    };
    let capture = Capture {
        image: &jpeg,
        orientation: Orientation::Normal,
        quality: SEAL_QUALITY,
        mark: None,
    };
    witness_core::seal(&capture, template, &manifest, dev, anchor)
}

pub fn submission(s: &SealedCapture) -> Submission {
    Submission::from_parts(s.submission.clone(), Some(cbor::to_vec(&s.fingerprints)))
}

pub fn derived_from(parent: &RecordId, root: CaptureId) -> Kind {
    Kind::Derivative {
        parent: *parent,
        root,
        edits: vec![],
    }
}

pub fn recompress(jpeg: &[u8], quality: u8) -> Result<Vec<u8>, SealError> {
    Ok(encode_jpeg(&decode(jpeg)?, quality)?)
}

pub fn downscale(image: &Rgb8, width: u32, height: u32) -> Result<Rgb8, SealError> {
    Ok(resize_area(image.view(), width, height)?)
}

pub fn preview(image: &Rgb8, quality: u8) -> Result<Vec<u8>, SealError> {
    Ok(encode_jpeg(&downscale(image, 320, 240)?, quality)?)
}

pub fn probe_hashes(b: &FingerprintBundle) -> Vec<Vec<u8>> {
    b.pdq()
        .iter()
        .take(MAX_PROBE_HASHES)
        .map(|s| s.hash.to_vec())
        .collect()
}

pub fn exact_hashes(s: &SealedCapture) -> Vec<Vec<u8>> {
    vec![s.claim.asset_hash.to_vec(), s.claim.pixel_hash.to_vec()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_core::c2pa::verify_seal;
    use witness_core::image::sniff;
    use witness_verdict::testkit::{TestCa, photo};

    fn device() -> (TestDevice, TrustAnchor) {
        let ca = TestCa::new(3);
        (ca.device(4, 77), ca.anchor.clone())
    }

    #[test]
    fn template_carries_time_and_dev_security() {
        let (d, _) = device();
        let t = template(&d, 5, Kind::Original, 9, 1_800_000_000_000);
        assert_eq!(t.time.wall_ms, 1_800_000_000_000);
        assert_eq!(t.checks.key_security, SecurityLevel::Dev);
        assert_eq!(t.key.counter, 9);
        assert_eq!(t.capture_id, CaptureId::new(5, 0).unwrap());
        assert_eq!(t, template(&d, 5, Kind::Original, 9, 1_800_000_000_000));
    }

    #[test]
    fn template_masks_oversized_capture_ids() {
        let (d, _) = device();
        let t = template(&d, u64::MAX, Kind::Original, 1, 1);
        assert_eq!(t.capture_id.random(), CaptureId::RANDOM_MASK);
    }

    #[test]
    fn sealed_original_verifies_and_matches_submission() {
        let (d, anchor) = device();
        let img = photo(160, 120, 1);
        let s = seal(
            &d,
            &anchor,
            &img,
            template(&d, 7, Kind::Original, 1, 1_763_510_400_000),
        )
        .unwrap();
        let sub = submission(&s);
        assert_eq!(sub.record_id(), s.record_id);
        assert!(sub.within_limits());
        let seal = verify_seal(&sub.claim, &sub.c2pa_claim, &sub.cose, &anchor, None).unwrap();
        assert_eq!(seal.claim, s.claim);
        let fp: FingerprintBundle =
            cbor::from_slice_canonical(sub.fingerprints.as_ref().unwrap()).unwrap();
        assert_eq!(fp.fp_commit(), s.claim.fp_commit);
        assert_eq!(
            exact_hashes(&s),
            vec![s.claim.asset_hash.to_vec(), s.claim.pixel_hash.to_vec()]
        );
    }

    #[test]
    fn sealing_is_deterministic() {
        let (d, anchor) = device();
        let img = photo(96, 72, 2);
        let t = template(&d, 8, Kind::Original, 2, 1_763_510_400_000);
        let a = seal(&d, &anchor, &img, t.clone()).unwrap();
        let b = seal(&d, &anchor, &img, t).unwrap();
        assert_eq!(a.jpeg, b.jpeg);
        assert_eq!(a.record_id, b.record_id);
    }

    #[test]
    fn derivative_kind_points_at_parent_and_root() {
        let id = RecordId::from_claim_cbor(b"x");
        let root = CaptureId::new(3, 0).unwrap();
        let Kind::Derivative {
            parent,
            root: r,
            edits,
        } = derived_from(&id, root)
        else {
            panic!("derivative")
        };
        assert_eq!((parent, r), (id, root));
        assert!(edits.is_empty());
    }

    #[test]
    fn recompress_strips_credentials_and_keeps_size() {
        let (d, anchor) = device();
        let img = photo(160, 120, 3);
        let s = seal(
            &d,
            &anchor,
            &img,
            template(&d, 9, Kind::Original, 3, 1_763_510_400_000),
        )
        .unwrap();
        let r = recompress(&s.jpeg, 70).unwrap();
        assert_ne!(r, s.jpeg);
        assert!(!r.windows(4).any(|w| w == b"jumb"));
        let back = decode(&r).unwrap();
        assert_eq!(back.dimensions(), (160, 120));
        assert_eq!(r, recompress(&s.jpeg, 70).unwrap());
    }

    #[test]
    fn recompress_rejects_garbage() {
        assert!(recompress(b"not a jpeg", 80).is_err());
        assert!(recompress(&[], 80).is_err());
    }

    #[test]
    fn preview_is_small_jpeg() {
        let p = preview(&photo(640, 480, 4), 80).unwrap();
        assert!(sniff(&p).is_some());
        assert!(p.len() <= witness_canister_common::limits::MAX_PREVIEW_BYTES as usize);
        assert_eq!(decode(&p).unwrap().dimensions(), (320, 240));
        assert_eq!(
            downscale(&photo(64, 48, 1), 32, 24).unwrap().dimensions(),
            (32, 24)
        );
        assert!(downscale(&photo(64, 48, 1), 0, 24).is_err());
    }

    #[test]
    fn probe_hashes_are_capped_32_byte_hashes() {
        let (d, anchor) = device();
        let s = seal(
            &d,
            &anchor,
            &photo(160, 120, 5),
            template(&d, 1, Kind::Original, 4, 1),
        )
        .unwrap();
        let h = probe_hashes(&s.fingerprints);
        assert!(!h.is_empty() && h.len() <= MAX_PROBE_HASHES);
        assert!(h.iter().all(|x| x.len() == 32));
    }
}
