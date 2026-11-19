#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod certificate;
mod error;
pub mod proof;
#[cfg(feature = "registry")]
pub mod registry;
#[cfg(any(test, feature = "testkit"))]
pub mod testkit;

pub use certificate::{Certified, DEFAULT_MAX_AGE_NS, IC_ROOT_KEY_DER};
pub use error::{Error, Result};
pub use proof::{RecordParts, ShardEntry, Trust};
