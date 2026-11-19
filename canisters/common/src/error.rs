use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractError {
    BadId,
    BadLength,
    TooLarge,
    WrongCaller,
    BadSignature,
    BadKey,
    Expired,
    Invalid,
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BadId => "bad id",
            Self::BadLength => "bad length",
            Self::TooLarge => "too large",
            Self::WrongCaller => "wrong caller",
            Self::BadSignature => "bad signature",
            Self::BadKey => "bad key",
            Self::Expired => "expired",
            Self::Invalid => "invalid",
        })
    }
}

impl core::error::Error for ContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_distinct_for_every_variant() {
        let all = [
            ContractError::BadId,
            ContractError::BadLength,
            ContractError::TooLarge,
            ContractError::WrongCaller,
            ContractError::BadSignature,
            ContractError::BadKey,
            ContractError::Expired,
            ContractError::Invalid,
        ];
        let mut texts: Vec<String> = all.iter().map(|e| e.to_string()).collect();
        assert!(texts.iter().all(|t| !t.is_empty()));
        texts.sort();
        texts.dedup();
        assert_eq!(texts.len(), all.len());
    }

    #[test]
    fn is_std_error() {
        let e: Box<dyn core::error::Error> = Box::new(ContractError::Expired);
        assert_eq!(e.to_string(), "expired");
    }
}
