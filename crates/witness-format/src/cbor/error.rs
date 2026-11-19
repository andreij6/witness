use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Eof,
    Trailing,
    Indefinite,
    Reserved,
    NonMinimal,
    Float,
    Tag,
    Simple,
    Utf8,
    KeyOrder,
    Depth,
    Type,
    Length,
    Range,
    UnknownKey(u64),
    MissingKey(u64),
    Version(u64),
    Invalid,
    NotCanonical,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eof => f.write_str("unexpected end of input"),
            Self::Trailing => f.write_str("trailing bytes"),
            Self::Indefinite => f.write_str("indefinite length"),
            Self::Reserved => f.write_str("reserved additional information"),
            Self::NonMinimal => f.write_str("non-shortest encoding"),
            Self::Float => f.write_str("floating point value"),
            Self::Tag => f.write_str("tagged value"),
            Self::Simple => f.write_str("unsupported simple value"),
            Self::Utf8 => f.write_str("invalid utf-8"),
            Self::KeyOrder => f.write_str("duplicate or unordered map key"),
            Self::Depth => f.write_str("nesting too deep"),
            Self::Type => f.write_str("unexpected type"),
            Self::Length => f.write_str("unexpected length"),
            Self::Range => f.write_str("value out of range"),
            Self::UnknownKey(k) => write!(f, "unknown key {k}"),
            Self::MissingKey(k) => write!(f, "missing key {k}"),
            Self::Version(v) => write!(f, "unsupported version {v}"),
            Self::Invalid => f.write_str("invalid value"),
            Self::NotCanonical => f.write_str("does not re-encode identically"),
        }
    }
}

impl core::error::Error for Error {}
