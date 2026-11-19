use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Malformed(&'static str),
    Certificate(&'static str),
    Proof(&'static str),
}

pub type Result<T> = core::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(what) => write!(f, "malformed {what}"),
            Self::Certificate(what) => write!(f, "certificate rejected: {what}"),
            Self::Proof(what) => write!(f, "proof rejected: {what}"),
        }
    }
}

impl core::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_failing_part() {
        assert_eq!(Error::Malformed("key").to_string(), "malformed key");
        assert_eq!(
            Error::Certificate("stale").to_string(),
            "certificate rejected: stale"
        );
        assert_eq!(Error::Proof("mmr").to_string(), "proof rejected: mmr");
    }
}
