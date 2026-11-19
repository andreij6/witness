#![no_std]

extern crate alloc;

#[macro_use]
mod macros;

pub mod base32;
pub mod cbor;
pub mod claim;
pub mod crypto;
pub mod edit;
pub mod evidence;
pub mod fingerprint;
pub mod ids;
pub mod key;
pub mod verdict;

#[cfg(test)]
mod testutil;

pub use cbor::{Decode, Decoder, Encode, Encoder, Error, Value};
pub use claim::{CertSerial, Dims, Geohash5, KeyRef, Kind, Submission, WitnessClaim};
pub use edit::{BlurClass, Edit, EditClass, EditKind, ImageFormat, Rect, presentation_only};
pub use evidence::{
    BootState, Checks, Confidence, GnssChecks, LocationEvidence, LocationOpening, LocationReasons,
    SecurityLevel, TimeEvidence, location_commit,
};
pub use fingerprint::{FingerprintBundle, PdqShape};
pub use ids::{CaptureId, CaseId, IdParseError, KeyId, RecordId};
pub use key::{Freshness, KeyHistory, KeyInfo, KeyStatus, Provisioning, StatusChange};
pub use verdict::{Outcome, VerdictFlags};
