use core::fmt;
use witness_c2pa::{Action, Generator, ManifestSpec, Signer, TrustAnchor, data_hash, seal_jpeg};
use witness_fingerprint::{FingerprintError, fingerprint};
use witness_format::{
    CaptureId, CertSerial, Checks, Dims, FingerprintBundle, Geohash5, KeyRef, Kind,
    LocationEvidence, RecordId, Submission, TimeEvidence, WitnessClaim,
};
use witness_image::{ImageError, Orientation, decode, encode_jpeg, orient, pixel_hash_of};
use witness_watermark::{Watermarker, encode as watermark_payload};

pub const SEAL_QUALITY: u8 = 92;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealError {
    Image(ImageError),
    Fingerprint(FingerprintError),
    Claim(witness_format::Error),
    C2pa(witness_c2pa::Error),
    WatermarkMismatch,
}

impl fmt::Display for SealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image(e) => write!(f, "seal image: {e}"),
            Self::Fingerprint(e) => write!(f, "seal fingerprint: {e}"),
            Self::Claim(e) => write!(f, "seal claim: {e:?}"),
            Self::C2pa(e) => write!(f, "seal manifest: {e:?}"),
            Self::WatermarkMismatch => f.write_str("watermark capture id differs from claim"),
        }
    }
}

impl std::error::Error for SealError {}

impl From<ImageError> for SealError {
    fn from(e: ImageError) -> Self {
        Self::Image(e)
    }
}

impl From<FingerprintError> for SealError {
    fn from(e: FingerprintError) -> Self {
        Self::Fingerprint(e)
    }
}

impl From<witness_format::Error> for SealError {
    fn from(e: witness_format::Error) -> Self {
        Self::Claim(e)
    }
}

impl From<witness_c2pa::Error> for SealError {
    fn from(e: witness_c2pa::Error) -> Self {
        Self::C2pa(e)
    }
}

#[derive(Clone, Copy)]
pub struct Mark<'a> {
    pub watermarker: &'a dyn Watermarker,
    pub capture_id: CaptureId,
}

#[derive(Clone, Copy)]
pub struct Capture<'a> {
    pub image: &'a [u8],
    pub orientation: Orientation,
    pub quality: u8,
    pub mark: Option<Mark<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    pub jpeg: Vec<u8>,
    pub dims: Dims,
    pub asset_hash: [u8; 32],
    pub pixel_hash: [u8; 32],
    pub fingerprints: FingerprintBundle,
    pub watermark: Option<CaptureId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimTemplate {
    pub capture_id: CaptureId,
    pub kind: Kind,
    pub time: TimeEvidence,
    pub loc: LocationEvidence,
    pub checks: Checks,
    pub key: KeyRef,
    pub cert_serial: CertSerial,
    pub coarse: Option<Geohash5>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Manifest<'a> {
    pub id: [u8; 16],
    pub generator: Generator<'a>,
    pub actions: &'a [Action<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedCapture {
    pub jpeg: Vec<u8>,
    pub claim: WitnessClaim,
    pub record_id: RecordId,
    pub submission: Submission,
    pub fingerprints: FingerprintBundle,
    pub watermarked: bool,
}

pub fn prepare(capture: &Capture<'_>) -> Result<Prepared, SealError> {
    let raw = decode(capture.image)?;
    let mut image = match capture.orientation {
        Orientation::Normal => raw,
        o => orient(raw.view(), o),
    };
    let watermark = capture
        .mark
        .filter(|m| {
            m.watermarker
                .embed(&mut image, &watermark_payload(m.capture_id))
                .is_ok()
        })
        .map(|m| m.capture_id);
    let jpeg = encode_jpeg(&image, capture.quality)?;
    drop(image);
    let decoded = decode(&jpeg)?;
    let fingerprints = fingerprint(decoded.view())?;
    Ok(Prepared {
        dims: Dims {
            width: decoded.width(),
            height: decoded.height(),
        },
        pixel_hash: pixel_hash_of(decoded.view()),
        asset_hash: data_hash(&jpeg)?,
        fingerprints,
        watermark,
        jpeg,
    })
}

impl Prepared {
    pub fn claim(&self, t: ClaimTemplate) -> Result<WitnessClaim, SealError> {
        let root = match &t.kind {
            Kind::Original => t.capture_id,
            Kind::Derivative { root, .. } => *root,
        };
        if self.watermark.is_some_and(|id| id != root) {
            return Err(SealError::WatermarkMismatch);
        }
        let claim = WitnessClaim {
            capture_id: t.capture_id,
            kind: t.kind,
            asset_hash: self.asset_hash,
            pixel_hash: self.pixel_hash,
            fp_commit: self.fingerprints.fp_commit(),
            dims: self.dims,
            time: t.time,
            loc: t.loc,
            checks: t.checks,
            key: t.key,
            cert_serial: t.cert_serial,
            coarse: t.coarse,
        };
        claim.validate()?;
        Ok(claim)
    }
}

pub fn finish(
    prepared: Prepared,
    template: ClaimTemplate,
    manifest: &Manifest<'_>,
    signer: &dyn Signer,
    anchor: &TrustAnchor,
) -> Result<SealedCapture, SealError> {
    let claim = prepared.claim(template)?;
    let claim_cbor = claim.to_cbor()?;
    let spec = ManifestSpec {
        id: manifest.id,
        generator: manifest.generator,
        actions: manifest.actions,
        witness_claim: &claim_cbor,
    };
    let sealed = seal_jpeg(&prepared.jpeg, &spec, signer, anchor)?;
    Ok(SealedCapture {
        jpeg: sealed.jpeg,
        record_id: RecordId::from_claim_cbor(&claim_cbor),
        claim,
        submission: Submission {
            claim_cbor,
            c2pa_claim_cbor: sealed.c2pa_claim,
            cose: sealed.cose_sign1,
        },
        fingerprints: prepared.fingerprints,
        watermarked: prepared.watermark.is_some(),
    })
}

pub fn seal(
    capture: &Capture<'_>,
    template: ClaimTemplate,
    manifest: &Manifest<'_>,
    signer: &dyn Signer,
    anchor: &TrustAnchor,
) -> Result<SealedCapture, SealError> {
    finish(prepare(capture)?, template, manifest, signer, anchor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_c2pa::{Credentials, verify_jpeg, verify_seal};
    use witness_format::{Edit, Kind, Rect};
    use witness_image::{encode_jpeg, pixel_hash};
    use witness_verdict::testkit::{
        CAPTURED, FixedWatermarker, GENERATOR, TestCa, TestDevice, photo,
    };
    use witness_watermark::NullWatermarker;

    const MANIFEST: Manifest<'static> = Manifest {
        id: [0x11; 16],
        generator: GENERATOR,
        actions: CAPTURED,
    };

    fn camera(seed: u64) -> Vec<u8> {
        encode_jpeg(&photo(320, 240, seed), 95).unwrap()
    }

    fn capture(image: &[u8]) -> Capture<'_> {
        Capture {
            image,
            orientation: Orientation::Normal,
            quality: SEAL_QUALITY,
            mark: None,
        }
    }

    fn template(dev: &TestDevice, kind: Kind) -> ClaimTemplate {
        let c = dev.claim(0xABCDEF, kind, 3);
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

    fn setup() -> (TestCa, TestDevice) {
        let ca = TestCa::new(7);
        let dev = ca.device(0x42, 0x0102);
        (ca, dev)
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn prepare_binds_hashes_to_the_encoded_file() {
        let input = camera(1);
        let p = prepare(&capture(&input)).unwrap();
        assert_eq!(
            p.dims,
            Dims {
                width: 320,
                height: 240
            }
        );
        assert_eq!(p.pixel_hash, pixel_hash(&p.jpeg).unwrap());
        assert_eq!(p.asset_hash, data_hash(&p.jpeg).unwrap());
        assert_eq!(
            p.fingerprints,
            fingerprint(decode(&p.jpeg).unwrap().view()).unwrap()
        );
        assert_eq!(p.watermark, None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn prepare_orients_before_everything() {
        let input = camera(2);
        let p = prepare(&Capture {
            orientation: Orientation::Rotate90,
            ..capture(&input)
        })
        .unwrap();
        assert_eq!(
            p.dims,
            Dims {
                width: 240,
                height: 320
            }
        );
        assert!(p.fingerprints.portrait());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn watermark_is_recorded_only_when_embedding_succeeds() {
        let input = camera(3);
        let id = CaptureId::new(77, 0).unwrap();
        let fixed = FixedWatermarker::reading(id);
        let marked = prepare(&Capture {
            mark: Some(Mark {
                watermarker: &fixed,
                capture_id: id,
            }),
            ..capture(&input)
        })
        .unwrap();
        assert_eq!(marked.watermark, Some(id));
        let unavailable = prepare(&Capture {
            mark: Some(Mark {
                watermarker: &NullWatermarker,
                capture_id: id,
            }),
            ..capture(&input)
        })
        .unwrap();
        assert_eq!(unavailable.watermark, None);
        assert_eq!(unavailable.jpeg, prepare(&capture(&input)).unwrap().jpeg);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn prepare_rejects_bad_input() {
        assert!(matches!(
            prepare(&capture(b"nope")),
            Err(SealError::Image(_))
        ));
        assert!(matches!(prepare(&capture(&[])), Err(SealError::Image(_))));
        let tiny = encode_jpeg(&witness_image::Rgb8::filled(3, 3, [9, 9, 9]).unwrap(), 90).unwrap();
        assert!(matches!(
            prepare(&capture(&tiny)),
            Err(SealError::Fingerprint(_))
        ));
        assert!(matches!(
            prepare(&Capture {
                quality: 0,
                ..capture(&camera(4))
            }),
            Ok(_) | Err(SealError::Image(_))
        ));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn sealed_capture_verifies_like_the_registry_would() {
        let (ca, dev) = setup();
        let input = camera(5);
        let s = seal(
            &capture(&input),
            template(&dev, Kind::Original),
            &MANIFEST,
            &dev,
            &ca.anchor,
        )
        .unwrap();
        let report = verify_jpeg(&s.jpeg, &ca.anchor, None).unwrap();
        assert!(report.binding_intact());
        let Credentials::Witness(m) = report.credentials else {
            panic!("no witness manifest")
        };
        assert_eq!(m.seal.claim, s.claim);
        assert_eq!(s.record_id, s.claim.record_id());
        assert_eq!(s.submission.record_id(), s.record_id);
        assert_eq!(s.submission.claim().unwrap(), s.claim);
        assert!(s.submission.within_limits());
        let seal = verify_seal(
            &s.submission.claim_cbor,
            &s.submission.c2pa_claim_cbor,
            &s.submission.cose,
            &ca.anchor,
            None,
        )
        .unwrap();
        assert_eq!(seal.claim, s.claim);
        assert_eq!(pixel_hash(&s.jpeg).unwrap(), s.claim.pixel_hash);
        assert_eq!(data_hash(&s.jpeg).unwrap(), s.claim.asset_hash);
        assert_eq!(s.claim.fp_commit, s.fingerprints.fp_commit());
        assert!(!s.watermarked);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn sealing_is_deterministic() {
        let (ca, dev) = setup();
        let input = camera(6);
        let a = seal(
            &capture(&input),
            template(&dev, Kind::Original),
            &MANIFEST,
            &dev,
            &ca.anchor,
        )
        .unwrap();
        let b = seal(
            &capture(&input),
            template(&dev, Kind::Original),
            &MANIFEST,
            &dev,
            &ca.anchor,
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn watermark_must_carry_the_root_capture_id() {
        let (ca, dev) = setup();
        let input = camera(7);
        let other = CaptureId::new(1, 0).unwrap();
        let fixed = FixedWatermarker::reading(other);
        let marked = Capture {
            mark: Some(Mark {
                watermarker: &fixed,
                capture_id: other,
            }),
            ..capture(&input)
        };
        assert_eq!(
            seal(
                &marked,
                template(&dev, Kind::Original),
                &MANIFEST,
                &dev,
                &ca.anchor
            ),
            Err(SealError::WatermarkMismatch)
        );
        let t = template(&dev, Kind::Original);
        let marked = Capture {
            mark: Some(Mark {
                watermarker: &fixed,
                capture_id: t.capture_id,
            }),
            ..capture(&input)
        };
        assert!(
            seal(&marked, t, &MANIFEST, &dev, &ca.anchor)
                .unwrap()
                .watermarked
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn derivative_claims_are_validated() {
        let (ca, dev) = setup();
        let input = camera(8);
        let parent = seal(
            &capture(&input),
            template(&dev, Kind::Original),
            &MANIFEST,
            &dev,
            &ca.anchor,
        )
        .unwrap();
        let kind = Kind::Derivative {
            parent: parent.record_id,
            root: parent.claim.capture_id,
            edits: vec![Edit::Crop(Rect {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            })],
        };
        let ok = seal(
            &capture(&input),
            template(&dev, kind.clone()),
            &MANIFEST,
            &dev,
            &ca.anchor,
        )
        .unwrap();
        assert_eq!(ok.claim.kind, kind);
        let mut bad = template(&dev, kind);
        bad.loc.commit = Some([1; 32]);
        assert!(matches!(
            seal(&capture(&input), bad, &MANIFEST, &dev, &ca.anchor),
            Err(SealError::Claim(_))
        ));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn wrong_trust_anchor_or_signer_fails() {
        let (ca, dev) = setup();
        let input = camera(9);
        let foreign = TestCa::new(8);
        assert!(matches!(
            seal(
                &capture(&input),
                template(&dev, Kind::Original),
                &MANIFEST,
                &dev,
                &foreign.anchor
            ),
            Err(SealError::C2pa(_))
        ));
        let other = ca.device(0x43, 0x0102);
        assert!(matches!(
            seal(
                &capture(&input),
                template(&dev, Kind::Original),
                &MANIFEST,
                &other,
                &ca.anchor
            ),
            Err(SealError::C2pa(_))
        ));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn errors_display() {
        for e in [
            SealError::Image(ImageError::Encode),
            SealError::Fingerprint(FingerprintError::TooSmall),
            SealError::Claim(witness_format::Error::Invalid),
            SealError::C2pa(witness_c2pa::Error::Signer),
            SealError::WatermarkMismatch,
        ] {
            assert!(!e.to_string().is_empty());
        }
    }
}
