pub mod seal;

pub use witness_attest as attest;
pub use witness_c2pa as c2pa;
pub use witness_fingerprint as fingerprint;
pub use witness_format as format;
pub use witness_image as image;
pub use witness_match as matching;
pub use witness_merkle as merkle;
pub use witness_verdict as verdict;
pub use witness_watermark as watermark;

pub use seal::{
    Capture, ClaimTemplate, Manifest, Mark, Prepared, SEAL_QUALITY, SealError, SealedCapture,
    finish, prepare, seal,
};
pub use witness_verdict::{Options, Oracle, Verdict, judge};
