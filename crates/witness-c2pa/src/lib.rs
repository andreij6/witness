#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate self as witness_c2pa;

pub mod app11;
pub mod assertions;
pub mod cbor;
pub mod cert;
pub mod claim;
pub mod cose;
mod error;
pub mod jpeg;
pub mod jumbf;
pub mod manifest;
pub mod seal;
pub mod verify;
pub mod x509;

#[cfg(test)]
mod testkit;

pub use app11::data_hash;
pub use assertions::Action;
pub use cert::{CertParams, build_leaf, build_root};
pub use claim::Generator;
pub use error::{ChainError, Error, Part};
pub use seal::{ManifestSpec, Sealed, Signer, seal_jpeg};
pub use verify::{Credentials, Report, Seal, WitnessManifest, verify_jpeg, verify_seal};
pub use x509::TrustAnchor;
