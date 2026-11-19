use crate::claim::{CaptureMeta, ClaimView};
use crate::error::WitnessError;
use crate::trustmark::Trustmark;
use std::sync::{Arc, Mutex, PoisonError};
use witness_canister_common::Submission;
use witness_core::c2pa::assertions::CREATED;
use witness_core::c2pa::{Action, Generator, Signer, TrustAnchor};
use witness_core::format::crypto::es256_normalize_low_s;
use witness_core::format::{Kind, RecordId, cbor};
use witness_core::image::Orientation;
use witness_core::{Capture, Manifest, Mark, Prepared, SEAL_QUALITY};

pub(crate) const GENERATOR_NAME: &str = "Witness";
const ORIGINAL_ACTIONS: &[Action<'static>] = &[Action::CAPTURED];
const DERIVATIVE_ACTIONS: &[Action<'static>] = &[Action {
    action: CREATED,
    digital_source_type: None,
}];

#[uniffi::export(with_foreign)]
pub trait KeystoreSigner: Send + Sync {
    fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, WitnessError>;
    fn cert_chain(&self) -> Vec<Vec<u8>>;
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SealedCapture {
    pub jpeg: Vec<u8>,
    pub record_id: RecordId,
    pub claim: ClaimView,
    pub submission: Submission,
    pub watermarked: bool,
}

#[derive(uniffi::Object)]
pub struct PreparedHandle {
    meta: CaptureMeta,
    prepared: Mutex<Option<Prepared>>,
}

pub(crate) fn sign_es256(
    signer: &dyn KeystoreSigner,
    message: &[u8],
) -> Result<Vec<u8>, WitnessError> {
    let sig = signer.sign(message.to_vec())?;
    es256_normalize_low_s(&sig)
        .map(Vec::from)
        .map_err(|e| WitnessError::Signer {
            reason: format!("signature encoding: {e:?}"),
        })
}

struct Bridge<'a> {
    signer: &'a dyn KeystoreSigner,
    failure: Mutex<Option<WitnessError>>,
}

impl Signer for Bridge<'_> {
    fn sign(&self, data: &[u8]) -> Vec<u8> {
        sign_es256(self.signer, data).unwrap_or_else(|e| {
            *self.failure.lock().unwrap_or_else(PoisonError::into_inner) = Some(e);
            Vec::new()
        })
    }

    fn cert_chain(&self) -> Vec<Vec<u8>> {
        self.signer.cert_chain()
    }
}

#[uniffi::export]
pub fn prepare(
    capture: Vec<u8>,
    meta: CaptureMeta,
    watermark: Option<Arc<Trustmark>>,
) -> Result<Arc<PreparedHandle>, WitnessError> {
    let orientation = Orientation::from_exif(meta.exif_orientation)
        .ok_or_else(|| WitnessError::invalid("exif orientation"))?;
    meta.manifest_id()?;
    meta.checked_app_version()?;
    let template = meta.template(0)?;
    let mark = match (&meta.kind, watermark.as_deref()) {
        (Kind::Original, Some(t)) => Some(Mark {
            watermarker: t.watermarker(),
            capture_id: meta.capture_id,
        }),
        _ => None,
    };
    let prepared = witness_core::prepare(&Capture {
        image: &capture,
        orientation,
        quality: SEAL_QUALITY,
        mark,
    })?;
    prepared.claim(template)?;
    Ok(Arc::new(PreparedHandle {
        meta,
        prepared: Mutex::new(Some(prepared)),
    }))
}

impl PreparedHandle {
    fn with<T>(&self, f: impl FnOnce(&Prepared) -> T) -> Result<T, WitnessError> {
        self.prepared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(f)
            .ok_or(WitnessError::Consumed)
    }
}

#[uniffi::export]
impl PreparedHandle {
    pub fn width(&self) -> Result<u32, WitnessError> {
        self.with(|p| p.dims.width)
    }

    pub fn height(&self) -> Result<u32, WitnessError> {
        self.with(|p| p.dims.height)
    }

    pub fn watermarked(&self) -> Result<bool, WitnessError> {
        self.with(|p| p.watermark.is_some())
    }

    pub fn asset_hash(&self) -> Result<Vec<u8>, WitnessError> {
        self.with(|p| p.asset_hash.to_vec())
    }

    pub fn pixel_hash(&self) -> Result<Vec<u8>, WitnessError> {
        self.with(|p| p.pixel_hash.to_vec())
    }

    pub fn finish(
        &self,
        counter: u64,
        signer: Arc<dyn KeystoreSigner>,
        ca_root: Vec<u8>,
    ) -> Result<SealedCapture, WitnessError> {
        let anchor = TrustAnchor::from_der(&ca_root)
            .map_err(|e| WitnessError::invalid(format!("ca root: {e:?}")))?;
        let template = self.meta.template(counter)?;
        let id = self.meta.manifest_id()?;
        let actions = match self.meta.kind {
            Kind::Original => ORIGINAL_ACTIONS,
            Kind::Derivative { .. } => DERIVATIVE_ACTIONS,
        };
        let manifest = Manifest {
            id,
            generator: Generator {
                name: GENERATOR_NAME,
                version: self.meta.checked_app_version()?,
            },
            actions,
        };
        let bridge = Bridge {
            signer: signer.as_ref(),
            failure: Mutex::new(None),
        };
        let mut slot = self.prepared.lock().unwrap_or_else(PoisonError::into_inner);
        let prepared = slot.as_ref().ok_or(WitnessError::Consumed)?.clone();
        let sealed = witness_core::finish(prepared, template, &manifest, &bridge, &anchor);
        if let Some(e) = bridge
            .failure
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
        {
            return Err(e);
        }
        let sealed = sealed?;
        *slot = None;
        Ok(SealedCapture {
            claim: ClaimView::from(&sealed.claim),
            submission: Submission::from_parts(
                sealed.submission,
                Some(cbor::to_vec(&sealed.fingerprints)),
            ),
            jpeg: sealed.jpeg,
            record_id: sealed.record_id,
            watermarked: sealed.watermarked,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::claim::tests::meta;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use witness_core::c2pa::{verify_jpeg, verify_seal};
    use witness_core::format::crypto::{es256_raw_to_der, es256_verify};
    use witness_core::format::{Edit, FingerprintBundle, KeyId, WitnessClaim};
    use witness_core::image::encode_jpeg;
    use witness_core::verdict::testkit::{FixedWatermarker, TestCa, TestDevice, photo};

    pub(crate) struct DeviceSigner {
        pub device: TestDevice,
        pub der: bool,
        pub fail: bool,
        pub calls: AtomicUsize,
    }

    impl DeviceSigner {
        pub(crate) fn new(device: TestDevice) -> Arc<Self> {
            Arc::new(Self {
                device,
                der: false,
                fail: false,
                calls: AtomicUsize::new(0),
            })
        }
    }

    impl KeystoreSigner for DeviceSigner {
        fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, WitnessError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(WitnessError::Signer {
                    reason: "keystore locked".into(),
                });
            }
            let raw = Signer::sign(&self.device, &message);
            if self.der {
                let raw: [u8; 64] = raw.try_into().unwrap();
                return Ok(es256_raw_to_der(&raw).unwrap().as_bytes().to_vec());
            }
            Ok(raw)
        }

        fn cert_chain(&self) -> Vec<Vec<u8>> {
            self.device.chain.clone()
        }
    }

    pub(crate) fn setup() -> (TestCa, TestDevice) {
        let ca = TestCa::new(7);
        let dev = ca.device(0x42, 0x0102);
        (ca, dev)
    }

    pub(crate) fn camera(seed: u64) -> Vec<u8> {
        encode_jpeg(&photo(320, 240, seed), 95).unwrap()
    }

    pub(crate) fn device_meta(dev: &TestDevice) -> CaptureMeta {
        meta(dev.key_id().as_bytes(), &dev.serial.to_be_bytes()[6..])
    }

    pub(crate) fn sealed_original(seed: u64) -> (TestCa, TestDevice, SealedCapture) {
        let (ca, dev) = setup();
        let handle = prepare(camera(seed), device_meta(&dev), None).unwrap();
        let sealed = handle
            .finish(3, DeviceSigner::new(dev.clone()), ca.root_der.clone())
            .unwrap();
        (ca, dev, sealed)
    }

    #[test]
    fn seal_round_trip_verifies_with_c2pa_lite() {
        let (ca, dev, sealed) = sealed_original(1);
        let report = verify_jpeg(&sealed.jpeg, &ca.anchor, None).unwrap();
        let manifest = report.witness().expect("witness credentials");
        assert_eq!(manifest.witness_claim_cbor, sealed.submission.claim);
        assert_eq!(manifest.seal.signer_spki, dev.spki);
        let s = &sealed.submission;
        let seal = verify_seal(&s.claim, &s.c2pa_claim, &s.cose, &ca.anchor, None).unwrap();
        assert_eq!(seal.signer_spki, dev.spki);
        let claim = WitnessClaim::from_cbor(&sealed.submission.claim).unwrap();
        assert_eq!(ClaimView::from(&claim), sealed.claim);
        assert_eq!(claim.key.counter, 3);
        assert_eq!(claim.key.key_id, dev.key_id());
        assert_eq!(
            sealed.record_id,
            RecordId::from_claim_cbor(&sealed.submission.claim)
        );
        assert!(!sealed.watermarked);
    }

    #[test]
    fn submission_fingerprints_match_fp_commit() {
        let (_, _, sealed) = sealed_original(2);
        let fp = sealed.submission.fingerprints.clone().unwrap();
        let bundle: FingerprintBundle = cbor::from_slice_canonical(&fp).unwrap();
        assert_eq!(bundle.fp_commit().to_vec(), sealed.claim.fp_commit);
        assert!(sealed.submission.within_limits());
    }

    #[test]
    fn sealing_is_deterministic() {
        let (_, _, a) = sealed_original(3);
        let (_, _, b) = sealed_original(3);
        assert_eq!(a, b);
    }

    #[test]
    fn der_signatures_are_accepted() {
        let (ca, dev) = setup();
        let signer = Arc::new(DeviceSigner {
            der: true,
            ..Arc::into_inner(DeviceSigner::new(dev.clone())).unwrap()
        });
        let sealed = prepare(camera(4), device_meta(&dev), None)
            .unwrap()
            .finish(1, signer, ca.root_der.clone())
            .unwrap();
        assert!(
            verify_jpeg(&sealed.jpeg, &ca.anchor, None)
                .unwrap()
                .witness()
                .is_some()
        );
    }

    #[test]
    fn handle_exposes_prepared_facts_then_is_consumed() {
        let (ca, dev) = setup();
        let handle = prepare(camera(5), device_meta(&dev), None).unwrap();
        assert_eq!(
            (handle.width().unwrap(), handle.height().unwrap()),
            (320, 240)
        );
        assert_eq!(handle.asset_hash().unwrap().len(), 32);
        assert_eq!(handle.pixel_hash().unwrap().len(), 32);
        assert!(!handle.watermarked().unwrap());
        let signer = DeviceSigner::new(dev);
        handle
            .finish(1, signer.clone(), ca.root_der.clone())
            .unwrap();
        assert_eq!(handle.width(), Err(WitnessError::Consumed));
        assert_eq!(
            handle.finish(2, signer, ca.root_der.clone()),
            Err(WitnessError::Consumed)
        );
    }

    #[test]
    fn signer_failure_is_reported_and_handle_stays_usable() {
        let (ca, dev) = setup();
        let handle = prepare(camera(6), device_meta(&dev), None).unwrap();
        let failing = Arc::new(DeviceSigner {
            fail: true,
            ..Arc::into_inner(DeviceSigner::new(dev.clone())).unwrap()
        });
        assert_eq!(
            handle.finish(1, failing.clone(), ca.root_der.clone()),
            Err(WitnessError::Signer {
                reason: "keystore locked".into()
            })
        );
        assert_eq!(failing.calls.load(Ordering::SeqCst), 1);
        assert!(
            handle
                .finish(1, DeviceSigner::new(dev), ca.root_der.clone())
                .is_ok()
        );
    }

    #[test]
    fn garbage_signature_is_a_signer_error() {
        struct Junk(TestDevice);
        impl KeystoreSigner for Junk {
            fn sign(&self, _: Vec<u8>) -> Result<Vec<u8>, WitnessError> {
                Ok(vec![1, 2, 3])
            }
            fn cert_chain(&self) -> Vec<Vec<u8>> {
                self.0.chain.clone()
            }
        }
        let (ca, dev) = setup();
        let handle = prepare(camera(7), device_meta(&dev), None).unwrap();
        assert!(matches!(
            handle.finish(1, Arc::new(Junk(dev)), ca.root_der.clone()),
            Err(WitnessError::Signer { .. })
        ));
    }

    #[test]
    fn wrong_ca_root_is_rejected() {
        let (_, dev) = setup();
        let other = TestCa::new(9);
        let handle = prepare(camera(8), device_meta(&dev), None).unwrap();
        assert!(matches!(
            handle.finish(1, DeviceSigner::new(dev.clone()), other.root_der.clone()),
            Err(WitnessError::Seal { .. })
        ));
        assert!(matches!(
            handle.finish(1, DeviceSigner::new(dev), vec![0x30, 0x00]),
            Err(WitnessError::InvalidInput { .. })
        ));
    }

    #[test]
    fn prepare_rejects_bad_inputs() {
        let (_, dev) = setup();
        let good = device_meta(&dev);
        let mut m = good.clone();
        m.exif_orientation = 9;
        assert!(matches!(
            prepare(camera(1), m, None),
            Err(WitnessError::InvalidInput { .. })
        ));
        let mut m = good.clone();
        m.manifest_id = vec![1; 3];
        assert!(prepare(camera(1), m, None).is_err());
        let mut m = good.clone();
        m.app_version = String::new();
        assert!(prepare(camera(1), m, None).is_err());
        let mut m = good.clone();
        m.coarse = Some("9q8yy".into());
        assert!(matches!(
            prepare(camera(1), m, None),
            Err(WitnessError::Seal { .. })
        ));
        assert!(matches!(
            prepare(b"not an image".to_vec(), good.clone(), None),
            Err(WitnessError::Image { .. })
        ));
        assert!(prepare(Vec::new(), good, None).is_err());
    }

    #[test]
    fn orientation_rotates_dimensions() {
        let (_, dev) = setup();
        let mut m = device_meta(&dev);
        m.exif_orientation = 6;
        let handle = prepare(camera(9), m, None).unwrap();
        assert_eq!(
            (handle.width().unwrap(), handle.height().unwrap()),
            (240, 320)
        );
    }

    #[test]
    fn watermark_is_embedded_on_originals_only() {
        let (ca, dev) = setup();
        let mark = Trustmark::with(Box::new(FixedWatermarker::blank()));
        let handle = prepare(camera(10), device_meta(&dev), Some(mark.clone())).unwrap();
        assert!(handle.watermarked().unwrap());
        let sealed = handle
            .finish(1, DeviceSigner::new(dev.clone()), ca.root_der.clone())
            .unwrap();
        assert!(sealed.watermarked);
        let mut m = device_meta(&dev);
        m.kind = Kind::Derivative {
            parent: sealed.record_id,
            root: m.capture_id,
            edits: vec![Edit::Orient { orientation: 1 }],
        };
        m.location.commit = None;
        let derivative = prepare(sealed.jpeg, m, Some(mark)).unwrap();
        assert!(!derivative.watermarked().unwrap());
        let sealed = derivative
            .finish(2, DeviceSigner::new(dev), ca.root_der.clone())
            .unwrap();
        assert!(matches!(sealed.claim.kind, Kind::Derivative { .. }));
    }

    #[test]
    fn unavailable_watermarker_seals_without_mark() {
        let (_, dev) = setup();
        let mark = Trustmark::with(Box::new(witness_core::watermark::NullWatermarker));
        let handle = prepare(camera(11), device_meta(&dev), Some(mark)).unwrap();
        assert!(!handle.watermarked().unwrap());
    }

    #[test]
    fn sign_es256_normalizes_to_low_s_raw() {
        let (_, dev) = setup();
        let signer = DeviceSigner::new(dev.clone());
        let sig = sign_es256(signer.as_ref(), b"witness/erase/v1").unwrap();
        assert_eq!(sig.len(), 64);
        es256_verify(&dev.spki, b"witness/erase/v1", &sig).unwrap();
        assert_eq!(
            sig,
            sign_es256(signer.as_ref(), b"witness/erase/v1").unwrap()
        );
        assert_eq!(KeyId::from_spki(&dev.spki), dev.key_id());
    }
}
