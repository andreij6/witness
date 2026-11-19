#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

#[cfg(test)]
extern crate self as witness_attest;

pub mod chain;
pub mod challenge;
pub mod der;
mod error;
pub mod keydesc;
pub mod oid;
pub mod policy;
pub mod pubkey;
pub mod revocation;
pub mod roots;
pub mod serial;
pub mod status_json;
pub mod time;
pub mod x509;

#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod testkit;

pub use challenge::{FRESH_LEN, LATE_LEN, issue_fresh, late_challenge};
pub use error::Error;
pub use keydesc::{AppId, AuthList, KeyDescription, KmSecurity, RootOfTrust, VerifiedBoot};
pub use policy::{Attested, Context, Enrollment, LateOpening, Policy, verify};
pub use pubkey::{PublicKey, SigAlg};
pub use revocation::{Revocation, RevocationList, RevocationReason, RevocationStatus};
pub use roots::{ANCHORS, RootKind, TrustAnchor};
pub use serial::Serial;
pub use status_json::parse_status_list;
pub use x509::Certificate;
