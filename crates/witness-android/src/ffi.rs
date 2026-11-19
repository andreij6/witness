use candid::Principal;
use serde_bytes::ByteBuf;
use witness_canister_common::{
    Anchored, CaseInfo, CertifiedKey, CertifiedRecord, EntitlementGrant, Fresh, KeyInfo, KeyStatus,
    MediaErr, PdqHit, ProbeResult, Prov, Security, ShardInfo, Shards, StatusChange, Submission,
    SubmitErr, SubmitResult, Tier, UploadTarget,
};
use witness_core::format::{
    BlurClass, BootState, CaptureId, Checks, Confidence, Dims, Edit, GnssChecks, ImageFormat, Kind,
    LocationReasons, Outcome, RecordId, Rect, SecurityLevel, TimeEvidence,
};
use witness_core::verdict::{Anchor, Evidence, Reason, Region};

uniffi::custom_type!(Principal, String, {
    remote,
    lower: |p| p.to_text(),
    try_lift: |s| Ok(Principal::from_text(s)?),
});

uniffi::custom_type!(ByteBuf, Vec<u8>, {
    remote,
    lower: |b| b.into_vec(),
    try_lift: |v| Ok(ByteBuf::from(v)),
});

uniffi::custom_type!(CaptureId, u64, {
    remote,
    lower: |c| c.to_u64(),
    try_lift: |v| CaptureId::from_u64(v).ok_or_else(|| uniffi::deps::anyhow::anyhow!("capture id out of range")),
});

uniffi::custom_type!(RecordId, String, {
    remote,
    lower: |r| r.to_string(),
    try_lift: |s| Ok(RecordId::parse(&s)?),
});

uniffi::custom_type!(GnssChecks, u8, {
    remote,
    lower: |g| g.bits(),
    try_lift: |v| GnssChecks::from_bits(v).ok_or_else(|| uniffi::deps::anyhow::anyhow!("unknown gnss bits")),
});

uniffi::custom_type!(LocationReasons, u16, {
    remote,
    lower: |r| r.bits(),
    try_lift: |v| LocationReasons::from_bits(v).ok_or_else(|| uniffi::deps::anyhow::anyhow!("unknown location reasons")),
});

#[uniffi::remote(Enum)]
pub enum Security {
    StrongBox,
    Tee,
    Dev,
}

#[uniffi::remote(Enum)]
pub enum Prov {
    Rkp,
    Factory,
    Dev,
}

#[uniffi::remote(Enum)]
pub enum Fresh {
    Fresh,
    Late,
}

#[uniffi::remote(Enum)]
pub enum KeyStatus {
    Active,
    Retired,
    Suspended,
    Revoked,
}

#[uniffi::remote(Record)]
pub struct StatusChange {
    pub status: KeyStatus,
    pub since_ns: u64,
}

#[uniffi::remote(Record)]
pub struct KeyInfo {
    pub key_id: Vec<u8>,
    pub pubkey: Vec<u8>,
    pub security: Security,
    pub provisioning: Prov,
    pub freshness: Fresh,
    pub patch: u32,
    pub enrolled_ns: u64,
    pub active_until_ns: u64,
    pub history: Vec<StatusChange>,
    pub chain_serials: Vec<String>,
}

#[uniffi::remote(Record)]
pub struct Submission {
    pub claim: Vec<u8>,
    pub c2pa_claim: Vec<u8>,
    pub cose: Vec<u8>,
    pub fingerprints: Option<ByteBuf>,
}

#[uniffi::remote(Record)]
pub struct Anchored {
    pub record_ix: u64,
    pub record_id: String,
    pub anchored_ns: u64,
    #[uniffi(default = 0)]
    pub media_shard: u8,
}

#[uniffi::remote(Enum)]
pub enum SubmitErr {
    Malformed(String),
    TooLarge,
    BadSignature,
    UnknownKey,
    KeyDistrusted,
    KeyExpired,
    CounterReused,
    ParentUnknown,
    FingerprintMismatch,
    RetryAfterNs(u64),
}

#[uniffi::remote(Enum)]
pub enum SubmitResult {
    Ok(Anchored),
    Dup(Anchored),
    Err(SubmitErr),
}

#[uniffi::remote(Record)]
pub struct CertifiedRecord {
    pub record_ix: u64,
    pub record_id: String,
    pub claim: Vec<u8>,
    pub c2pa_claim: Vec<u8>,
    pub cose: Vec<u8>,
    pub anchored_ns: u64,
    pub mmr_proof: Vec<ByteBuf>,
    pub leaf_count: u64,
    pub certificate: Vec<u8>,
    #[uniffi(default = 0)]
    pub media_shard: u8,
}

#[uniffi::remote(Record)]
pub struct CertifiedKey {
    pub key: KeyInfo,
    pub smt_proof: Vec<u8>,
    pub certificate: Vec<u8>,
}

#[uniffi::remote(Record)]
pub struct ShardInfo {
    pub shard: u8,
    pub canister: Principal,
    pub open: bool,
}

#[uniffi::remote(Record)]
pub struct Shards {
    pub index: Vec<ShardInfo>,
    pub media: Vec<ShardInfo>,
}

#[uniffi::remote(Record)]
pub struct PdqHit {
    pub record_ix: u64,
    pub distance: u32,
}

#[uniffi::remote(Record)]
pub struct ProbeResult {
    pub pdq: Vec<PdqHit>,
    pub watermark: Vec<u64>,
    pub exact: Vec<u64>,
    pub truncated_candidates: bool,
    pub indexed_upto: u64,
}

#[uniffi::remote(Enum)]
pub enum Tier {
    Plus,
}

#[uniffi::remote(Record)]
pub struct EntitlementGrant {
    pub ii_principal: Principal,
    pub tier: Tier,
    pub quota_bytes: u64,
    pub expires_ns: u64,
    pub signature: Vec<u8>,
}

#[uniffi::remote(Enum)]
pub enum UploadTarget {
    Preview(String),
    Og(String),
    Full(String),
    CaseChunk {
        case_id: String,
        ix: u32,
        count: u32,
    },
}

#[uniffi::remote(Record)]
pub struct CaseInfo {
    pub expires_ns: u64,
    pub bytes: u64,
    pub chunks: u32,
}

#[uniffi::remote(Enum)]
pub enum MediaErr {
    Malformed(String),
    Unauthorized,
    NotFound,
    TooLarge,
    QuotaExceeded,
    GrantInvalid,
    BadChunk,
    HashMismatch,
    Incomplete,
    RetryAfterNs(u64),
}

#[uniffi::remote(Enum)]
pub enum SecurityLevel {
    StrongBox,
    Tee,
    Dev,
}

#[uniffi::remote(Enum)]
pub enum BootState {
    Verified,
    SelfSigned,
    Unverified,
    Failed,
}

#[uniffi::remote(Enum)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[uniffi::remote(Enum)]
pub enum BlurClass {
    Face,
    Plate,
    HouseNo,
    Other,
}

#[uniffi::remote(Enum)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Avif,
    Heic,
}

#[uniffi::remote(Record)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[uniffi::remote(Enum)]
pub enum Edit {
    Resize {
        w: u32,
        h: u32,
        filter: u8,
    },
    Recompress {
        format: ImageFormat,
        quality: u8,
    },
    Orient {
        orientation: u8,
    },
    Crop(Rect),
    Rotate {
        deg_e2: i16,
    },
    Tone {
        exposure_e2: i16,
        contrast_e2: i16,
        saturation_e2: i16,
    },
    Filter {
        id: u16,
        strength_e2: u16,
    },
    Blur {
        rect: Rect,
        class: BlurClass,
        sigma_e2: u16,
    },
}

#[uniffi::remote(Enum)]
pub enum Kind {
    Original,
    Derivative {
        parent: RecordId,
        root: CaptureId,
        edits: Vec<Edit>,
    },
}

#[uniffi::remote(Record)]
pub struct Dims {
    pub width: u32,
    pub height: u32,
}

#[uniffi::remote(Record)]
pub struct TimeEvidence {
    pub wall_ms: u64,
    pub gnss_ms: Option<u64>,
    pub elapsed_ns: u64,
    pub boot_count: u32,
}

#[uniffi::remote(Record)]
pub struct Checks {
    pub key_security: SecurityLevel,
    pub boot: BootState,
    pub device_locked: bool,
    pub mock_location: bool,
    pub gnss: GnssChecks,
    pub recapture: u8,
}

#[uniffi::remote(Enum)]
pub enum Outcome {
    Original,
    MatchesWithChanges,
    Altered,
    NoRecord,
    SealedUnanchored,
}

#[uniffi::remote(Enum)]
pub enum Reason {
    SealVerified,
    SealBroken,
    UnverifiedCredentials,
    OtherCredentials,
    KeyNotTrusted,
    KeyNotTrustedAtAnchor,
    ExactAssetHash,
    ExactPixelHash,
    WatermarkFound,
    WatermarkUnavailable,
    ForeignWatermark,
    Unconfirmed,
    NoCandidates,
    Undecodable,
    IndexPending,
}

#[uniffi::remote(Enum)]
pub enum Evidence {
    Seal,
    ExactHash,
    Pixels,
    Grid,
}

#[uniffi::remote(Record)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[uniffi::remote(Record)]
pub struct Anchor {
    pub record_ix: u64,
    pub anchored_ns: u64,
}
