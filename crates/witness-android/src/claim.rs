use crate::error::WitnessError;
use witness_core::ClaimTemplate;
use witness_core::format::{
    CaptureId, CertSerial, Checks, Confidence, Dims, Geohash5, KeyId, KeyRef, Kind,
    LocationEvidence, LocationReasons, TimeEvidence, WitnessClaim,
};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LocationSpec {
    pub commit: Option<Vec<u8>>,
    pub confidence: Confidence,
    pub reasons: LocationReasons,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CaptureMeta {
    pub capture_id: CaptureId,
    pub exif_orientation: u32,
    pub kind: Kind,
    pub time: TimeEvidence,
    pub location: LocationSpec,
    pub checks: Checks,
    pub key_id: Vec<u8>,
    pub cert_serial: Vec<u8>,
    pub coarse: Option<String>,
    pub manifest_id: Vec<u8>,
    pub app_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ClaimView {
    pub capture_id: CaptureId,
    pub kind: Kind,
    pub asset_hash: Vec<u8>,
    pub pixel_hash: Vec<u8>,
    pub fp_commit: Vec<u8>,
    pub dims: Dims,
    pub time: TimeEvidence,
    pub location: LocationSpec,
    pub checks: Checks,
    pub key_id: Vec<u8>,
    pub counter: u64,
    pub cert_serial: Vec<u8>,
    pub coarse: Option<String>,
}

pub(crate) const MAX_APP_VERSION: usize = 64;

impl From<LocationEvidence> for LocationSpec {
    fn from(l: LocationEvidence) -> Self {
        Self {
            commit: l.commit.map(Vec::from),
            confidence: l.confidence,
            reasons: l.reasons,
        }
    }
}

impl TryFrom<&LocationSpec> for LocationEvidence {
    type Error = WitnessError;

    fn try_from(l: &LocationSpec) -> Result<Self, WitnessError> {
        let commit = l
            .commit
            .as_deref()
            .map(<[u8; 32]>::try_from)
            .transpose()
            .map_err(|_| WitnessError::invalid("location commit must be 32 bytes"))?;
        Ok(Self {
            commit,
            confidence: l.confidence,
            reasons: l.reasons,
        })
    }
}

impl CaptureMeta {
    pub(crate) fn template(&self, counter: u64) -> Result<ClaimTemplate, WitnessError> {
        let key_id = KeyId::try_from(self.key_id.as_slice())
            .map_err(|_| WitnessError::invalid("key id must be 16 bytes"))?;
        let cert_serial = CertSerial::new(&self.cert_serial)
            .ok_or_else(|| WitnessError::invalid("certificate serial"))?;
        let coarse = self
            .coarse
            .as_deref()
            .map(|c| Geohash5::new(c).ok_or_else(|| WitnessError::invalid("coarse geohash")))
            .transpose()?;
        Ok(ClaimTemplate {
            capture_id: self.capture_id,
            kind: self.kind.clone(),
            time: self.time,
            loc: LocationEvidence::try_from(&self.location)?,
            checks: self.checks,
            key: KeyRef { key_id, counter },
            cert_serial,
            coarse,
        })
    }

    pub(crate) fn manifest_id(&self) -> Result<[u8; 16], WitnessError> {
        <[u8; 16]>::try_from(self.manifest_id.as_slice())
            .map_err(|_| WitnessError::invalid("manifest id must be 16 bytes"))
    }

    pub(crate) fn checked_app_version(&self) -> Result<&str, WitnessError> {
        if self.app_version.is_empty() || self.app_version.len() > MAX_APP_VERSION {
            return Err(WitnessError::invalid("app version"));
        }
        Ok(&self.app_version)
    }
}

impl From<&WitnessClaim> for ClaimView {
    fn from(c: &WitnessClaim) -> Self {
        Self {
            capture_id: c.capture_id,
            kind: c.kind.clone(),
            asset_hash: c.asset_hash.to_vec(),
            pixel_hash: c.pixel_hash.to_vec(),
            fp_commit: c.fp_commit.to_vec(),
            dims: c.dims,
            time: c.time,
            location: c.loc.into(),
            checks: c.checks,
            key_id: c.key.key_id.as_bytes().to_vec(),
            counter: c.key.counter,
            cert_serial: c.cert_serial.as_bytes().to_vec(),
            coarse: c.coarse.map(|g| g.as_str().to_owned()),
        }
    }
}

#[uniffi::export]
pub fn decode_claim(cbor: Vec<u8>) -> Result<ClaimView, WitnessError> {
    WitnessClaim::from_cbor(&cbor)
        .map(|c| ClaimView::from(&c))
        .map_err(|e| WitnessError::malformed(format!("claim: {e:?}")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use witness_core::format::{BootState, Edit, GnssChecks, RecordId, SecurityLevel};

    pub(crate) fn meta(key_id: &[u8], serial: &[u8]) -> CaptureMeta {
        CaptureMeta {
            capture_id: CaptureId::new(0x00AB_CDEF, 0).unwrap(),
            exif_orientation: 1,
            kind: Kind::Original,
            time: TimeEvidence {
                wall_ms: 1_763_510_400_000,
                gnss_ms: Some(1_763_510_400_100),
                elapsed_ns: 77,
                boot_count: 12,
            },
            location: LocationSpec {
                commit: Some(vec![7; 32]),
                confidence: Confidence::High,
                reasons: LocationReasons::RAW_GNSS_PASSED,
            },
            checks: Checks {
                key_security: SecurityLevel::Tee,
                boot: BootState::Verified,
                device_locked: true,
                mock_location: false,
                gnss: GnssChecks::RAW_AVAILABLE,
                recapture: 3,
            },
            key_id: key_id.to_vec(),
            cert_serial: serial.to_vec(),
            coarse: None,
            manifest_id: vec![0x11; 16],
            app_version: "1.0.0".into(),
        }
    }

    #[test]
    fn template_carries_every_field_and_counter() {
        let m = meta(&[3; 16], &[1, 2]);
        let t = m.template(41).unwrap();
        assert_eq!(t.capture_id, m.capture_id);
        assert_eq!(t.kind, Kind::Original);
        assert_eq!(t.time, m.time);
        assert_eq!(t.loc.commit, Some([7; 32]));
        assert_eq!(t.loc.confidence, Confidence::High);
        assert_eq!(t.checks, m.checks);
        assert_eq!(t.key.key_id.as_bytes(), &[3; 16]);
        assert_eq!(t.key.counter, 41);
        assert_eq!(t.cert_serial.as_bytes(), &[1, 2]);
        assert_eq!(t.coarse, None);
    }

    #[test]
    fn template_is_deterministic() {
        let m = meta(&[3; 16], &[9]);
        assert_eq!(m.template(5).unwrap(), m.template(5).unwrap());
    }

    #[test]
    fn template_rejects_bad_key_id() {
        for len in [0, 15, 17, 32] {
            let m = meta(&vec![1; len], &[1]);
            assert!(matches!(
                m.template(0),
                Err(WitnessError::InvalidInput { .. })
            ));
        }
    }

    #[test]
    fn template_rejects_bad_serial() {
        for serial in [vec![], vec![0, 1], vec![1; 21]] {
            assert!(meta(&[1; 16], &serial).template(0).is_err());
        }
    }

    #[test]
    fn template_rejects_bad_location_commit() {
        let mut m = meta(&[1; 16], &[1]);
        m.location.commit = Some(vec![1; 31]);
        assert!(m.template(0).is_err());
        m.location.commit = None;
        assert_eq!(m.template(0).unwrap().loc.commit, None);
    }

    #[test]
    fn template_parses_coarse() {
        let mut m = meta(&[1; 16], &[1]);
        m.coarse = Some("9q8yy".into());
        assert_eq!(m.template(0).unwrap().coarse.unwrap().as_str(), "9q8yy");
        for bad in ["9q8y", "9q8yya", "AAAAA", "9q8ya"] {
            m.coarse = Some(bad.into());
            assert!(m.template(0).is_err(), "{bad}");
        }
    }

    #[test]
    fn template_keeps_derivative_kind() {
        let mut m = meta(&[1; 16], &[1]);
        m.kind = Kind::Derivative {
            parent: RecordId::from_claim_cbor(b"p"),
            root: CaptureId::new(5, 0).unwrap(),
            edits: vec![Edit::Orient { orientation: 3 }],
        };
        assert_eq!(m.template(0).unwrap().kind, m.kind);
    }

    #[test]
    fn manifest_id_must_be_16_bytes() {
        let mut m = meta(&[1; 16], &[1]);
        assert_eq!(m.manifest_id().unwrap(), [0x11; 16]);
        for len in [0, 15, 17] {
            m.manifest_id = vec![0; len];
            assert!(m.manifest_id().is_err());
        }
    }

    #[test]
    fn app_version_is_bounded() {
        let mut m = meta(&[1; 16], &[1]);
        assert_eq!(m.checked_app_version().unwrap(), "1.0.0");
        m.app_version = String::new();
        assert!(m.checked_app_version().is_err());
        m.app_version = "1".repeat(MAX_APP_VERSION + 1);
        assert!(m.checked_app_version().is_err());
        m.app_version = "1".repeat(MAX_APP_VERSION);
        assert!(m.checked_app_version().is_ok());
    }

    #[test]
    fn location_round_trips() {
        let spec = meta(&[1; 16], &[1]).location;
        let ev = LocationEvidence::try_from(&spec).unwrap();
        assert_eq!(LocationSpec::from(ev), spec);
    }

    fn claim() -> WitnessClaim {
        let t = meta(&[4; 16], &[7, 7]).template(9).unwrap();
        WitnessClaim {
            capture_id: t.capture_id,
            kind: t.kind,
            asset_hash: [1; 32],
            pixel_hash: [2; 32],
            fp_commit: [3; 32],
            dims: Dims {
                width: 640,
                height: 480,
            },
            time: t.time,
            loc: t.loc,
            checks: t.checks,
            key: t.key,
            cert_serial: t.cert_serial,
            coarse: None,
        }
    }

    #[test]
    fn claim_view_mirrors_claim() {
        let c = claim();
        let v = ClaimView::from(&c);
        assert_eq!(v.capture_id, c.capture_id);
        assert_eq!(v.asset_hash, vec![1; 32]);
        assert_eq!(v.pixel_hash, vec![2; 32]);
        assert_eq!(v.fp_commit, vec![3; 32]);
        assert_eq!(v.dims, c.dims);
        assert_eq!(v.key_id, vec![4; 16]);
        assert_eq!(v.counter, 9);
        assert_eq!(v.cert_serial, vec![7, 7]);
        assert_eq!(v.location.commit, Some(vec![7; 32]));
    }

    #[test]
    fn decode_claim_round_trips_cbor() {
        let c = claim();
        let v = decode_claim(c.to_cbor().unwrap()).unwrap();
        assert_eq!(v, ClaimView::from(&c));
    }

    #[test]
    fn decode_claim_rejects_garbage_and_non_canonical() {
        assert!(matches!(
            decode_claim(vec![]),
            Err(WitnessError::Malformed { .. })
        ));
        assert!(decode_claim(vec![0xff; 40]).is_err());
        let mut bytes = claim().to_cbor().unwrap();
        bytes.push(0);
        assert!(decode_claim(bytes).is_err());
        let truncated = claim().to_cbor().unwrap();
        assert!(decode_claim(truncated[..truncated.len() - 1].to_vec()).is_err());
    }
}
