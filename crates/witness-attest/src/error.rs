use crate::revocation::RevocationStatus;
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Der,
    Time,
    Version,
    Serial,
    UnsupportedAlgorithm,
    AlgorithmMismatch,
    Key,
    Signature,
    UnknownCritical,
    DuplicateExtension,
    ChainLength,
    UntrustedRoot,
    NotCa,
    PathLen,
    KeyUsage,
    Validity,
    MissingKeyDescription,
    ExtensionPlacement,
    KeyDescription,
    SecurityLevel,
    BootState,
    DeviceLocked,
    Origin,
    Purpose,
    Algorithm,
    Curve,
    PatchLevel,
    Package,
    AppVersion,
    SignerDigest,
    Challenge,
    ChallengeExpired,
    CreationTime,
    KeyMismatch,
    Revoked(RevocationStatus),
    Json,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Revoked(RevocationStatus::Revoked) => f.write_str("certificate revoked"),
            Self::Revoked(RevocationStatus::Suspended) => f.write_str("certificate suspended"),
            other => fmt::Debug::fmt(other, f),
        }
    }
}

impl core::error::Error for Error {}

impl From<witness_format::Error> for Error {
    fn from(_: witness_format::Error) -> Self {
        Self::Der
    }
}
