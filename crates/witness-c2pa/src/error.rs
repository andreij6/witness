use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    NotJpeg,
    MalformedJpeg,
    MalformedSegments,
    MultipleStores,
    TooLarge,
    MalformedJumbf,
    MalformedCbor,
    MalformedClaim,
    MalformedAssertion,
    MalformedCose,
    MalformedCertificate,
    MissingPart(Part),
    UnsupportedClaim,
    UnsupportedAlgorithm,
    AssertionHash,
    UnresolvedAssertion,
    DuplicateAssertion,
    Signature,
    Signer,
    Chain(ChainError),
    WitnessClaim(witness_format::Error),
    KeyMismatch,
    SerialMismatch,
    AssetHashMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Manifest,
    AssertionStore,
    Claim,
    Signature,
    DataHash,
    WitnessClaim,
    X5Chain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainError {
    Malformed,
    Length,
    Anchor,
    Issuer,
    Signature,
    Algorithm,
    Version,
    Validity,
    KeyUsage,
    ExtendedKeyUsage,
    BasicConstraints,
    KeyIdentifier,
    CriticalExtension,
    DuplicateExtension,
    PublicKey,
    Serial,
}

impl From<ChainError> for Error {
    fn from(e: ChainError) -> Self {
        Self::Chain(e)
    }
}

impl From<minicbor::decode::Error> for Error {
    fn from(_: minicbor::decode::Error) -> Self {
        Self::MalformedCbor
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPart(p) => write!(f, "missing C2PA part {p:?}"),
            Self::Chain(c) => write!(f, "certificate chain rejected: {c:?}"),
            Self::WitnessClaim(e) => write!(f, "witness claim rejected: {e:?}"),
            other => write!(f, "{other:?}"),
        }
    }
}

impl core::error::Error for Error {}
