use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Json(String),
    Malformed(&'static str),
    Certificate(&'static str),
    Proof(&'static str),
    Case(&'static str),
}

pub type Result<T> = core::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(e) => write!(f, "invalid json: {e}"),
            Self::Malformed(what) => write!(f, "malformed {what}"),
            Self::Certificate(what) => write!(f, "certificate rejected: {what}"),
            Self::Proof(what) => write!(f, "proof rejected: {what}"),
            Self::Case(what) => write!(f, "case rejected: {what}"),
        }
    }
}

impl core::error::Error for Error {}

impl From<witness_proof::Error> for Error {
    fn from(e: witness_proof::Error) -> Self {
        match e {
            witness_proof::Error::Malformed(w) => Self::Malformed(w),
            witness_proof::Error::Certificate(w) => Self::Certificate(w),
            witness_proof::Error::Proof(w) => Self::Proof(w),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_failing_part() {
        assert_eq!(Error::Malformed("anchor").to_string(), "malformed anchor");
        assert_eq!(
            Error::Certificate("signature").to_string(),
            "certificate rejected: signature"
        );
        assert_eq!(Error::Proof("mmr").to_string(), "proof rejected: mmr");
        assert_eq!(Error::Case("hash").to_string(), "case rejected: hash");
        assert!(Error::Json("eof".into()).to_string().contains("eof"));
    }

    #[test]
    fn proof_errors_keep_their_kind_and_part() {
        use witness_proof::Error as P;
        assert_eq!(Error::from(P::Malformed("key")), Error::Malformed("key"));
        assert_eq!(
            Error::from(P::Certificate("stale")),
            Error::Certificate("stale")
        );
        assert_eq!(Error::from(P::Proof("smt")), Error::Proof("smt"));
    }

    #[test]
    fn json_errors_convert() {
        let e: Error = serde_json::from_str::<u8>("x").unwrap_err().into();
        assert!(matches!(e, Error::Json(_)));
    }
}
