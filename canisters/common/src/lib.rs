mod error;
pub mod index;
pub mod init;
pub mod inspect;
pub mod limits;
pub mod media;
pub mod memory;
pub mod peer;
pub mod registry;
pub mod runtime;
pub mod status;
pub mod timers;

pub use candid::Principal;
pub use error::ContractError;
pub use index::{Fingerprints, PdqHit, Posting, ProbeResult};
pub use init::{Origins, PeerInit, RegistryInit};
pub use inspect::{MethodLimit, arg_allowed};
pub use media::{BeginUpload, CaseInfo, Chunk, EntitlementGrant, MediaErr, Tier, UploadTarget};
pub use memory::{Memory, memory};
pub use peer::{Allow, IndexPeer, MediaPeer, PageErr, PageResult, PagesPeer, Removal, Shell};
pub use registry::{
    Anchored, CertifiedKey, CertifiedRecord, EraseAuth, EraseRequest, Fresh, KeyInfo, KeyStatus,
    Prov, Security, ShardInfo, Shards, StatusChange, Submission, SubmitErr, SubmitResult,
};
pub use status::{Status, TimerStatus};
pub use timers::{Tick, TimerBook, TimerSpec, validate_specs};

pub const DID: &str = include_str!("../common.did");
pub const UNPINNED_ANCHOR_DER: &[u8] = include_bytes!("../unpinned_anchor.der");
