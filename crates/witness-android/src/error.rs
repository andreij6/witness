use witness_canister_common::MediaErr;
use witness_core::SealError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WitnessError {
    #[error("invalid input: {reason}")]
    InvalidInput { reason: String },
    #[error("image: {reason}")]
    Image { reason: String },
    #[error("seal: {reason}")]
    Seal { reason: String },
    #[error("signer: {reason}")]
    Signer { reason: String },
    #[error("network: {reason}")]
    Network { reason: String },
    #[error("rejected: {reason}")]
    Rejected { reason: String },
    #[error("media: {err:?}")]
    Media { err: MediaErr },
    #[error("malformed response: {reason}")]
    Malformed { reason: String },
    #[error("watermark unavailable: {reason}")]
    WatermarkUnavailable { reason: String },
    #[error("prepared capture already finished")]
    Consumed,
}

impl WitnessError {
    pub(crate) fn invalid(reason: impl Into<String>) -> Self {
        Self::InvalidInput {
            reason: reason.into(),
        }
    }

    pub(crate) fn network(reason: impl ToString) -> Self {
        Self::Network {
            reason: reason.to_string(),
        }
    }

    pub(crate) fn malformed(reason: impl ToString) -> Self {
        Self::Malformed {
            reason: reason.to_string(),
        }
    }
}

impl From<SealError> for WitnessError {
    fn from(e: SealError) -> Self {
        match e {
            SealError::Image(_) | SealError::Fingerprint(_) => Self::Image {
                reason: e.to_string(),
            },
            _ => Self::Seal {
                reason: e.to_string(),
            },
        }
    }
}

impl From<MediaErr> for WitnessError {
    fn from(err: MediaErr) -> Self {
        Self::Media { err }
    }
}

impl From<candid::Error> for WitnessError {
    fn from(e: candid::Error) -> Self {
        Self::malformed(e)
    }
}

impl From<uniffi::UnexpectedUniFFICallbackError> for WitnessError {
    fn from(e: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Signer { reason: e.reason }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_core::image::ImageError;

    #[test]
    fn seal_image_errors_map_to_image() {
        let e: WitnessError = SealError::Image(ImageError::UnsupportedFormat).into();
        assert!(matches!(e, WitnessError::Image { .. }));
    }

    #[test]
    fn other_seal_errors_map_to_seal() {
        let e: WitnessError = SealError::WatermarkMismatch.into();
        assert_eq!(
            e,
            WitnessError::Seal {
                reason: "watermark capture id differs from claim".into()
            }
        );
    }

    #[test]
    fn media_errors_keep_their_variant() {
        let e: WitnessError = MediaErr::RetryAfterNs(5).into();
        assert_eq!(
            e,
            WitnessError::Media {
                err: MediaErr::RetryAfterNs(5)
            }
        );
    }

    #[test]
    fn candid_errors_are_malformed() {
        let e: WitnessError = candid::decode_one::<u64>(b"junk").unwrap_err().into();
        assert!(matches!(e, WitnessError::Malformed { .. }));
    }

    #[test]
    fn callback_errors_are_signer_errors() {
        let e: WitnessError = uniffi::UnexpectedUniFFICallbackError::new("boom").into();
        assert_eq!(
            e,
            WitnessError::Signer {
                reason: "boom".into()
            }
        );
    }

    #[test]
    fn display_names_the_category() {
        assert_eq!(WitnessError::invalid("x").to_string(), "invalid input: x");
        assert_eq!(WitnessError::network("down").to_string(), "network: down");
        assert_eq!(
            WitnessError::Consumed.to_string(),
            "prepared capture already finished"
        );
    }
}
