uniffi::setup_scaffolding!();

mod claim;
mod client;
mod error;
mod ffi;
mod oracle;
mod seal;
mod transport;
mod trustmark;
mod verdict;
mod witness;

pub use claim::{CaptureMeta, ClaimView, LocationSpec, decode_claim};
pub use client::{Begun, Enrolled, Enrollment, LateOpening};
pub use error::WitnessError;
pub use seal::{KeystoreSigner, PreparedHandle, SealedCapture, prepare};
pub use trustmark::Trustmark;
pub use verdict::{ReferenceView, VerdictFlag, VerdictView};
pub use witness::{CaseLimits, Witness, WitnessConfig, case_info, case_limits};
