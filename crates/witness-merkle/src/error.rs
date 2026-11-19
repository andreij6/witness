use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    MissingNode(u64),
    CorruptStore,
    OutOfRange,
    Full,
    Malformed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(pos) => write!(f, "missing mmr node at position {pos}"),
            Self::CorruptStore => f.write_str("node store size is not a valid mmr size"),
            Self::OutOfRange => f.write_str("index or size out of range"),
            Self::Full => f.write_str("mmr leaf capacity reached"),
            Self::Malformed => f.write_str("malformed proof encoding"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_distinct_per_variant() {
        let all = [
            Error::MissingNode(7),
            Error::CorruptStore,
            Error::OutOfRange,
            Error::Full,
            Error::Malformed,
        ];
        let texts: Vec<String> = all.iter().map(ToString::to_string).collect();
        for (i, a) in texts.iter().enumerate() {
            assert!(!a.is_empty());
            for b in &texts[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert!(texts[0].contains('7'));
    }
}
