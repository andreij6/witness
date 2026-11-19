use core::fmt;
use witness_image::ImageError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatermarkError {
    Unavailable,
    Inference,
    Image(ImageError),
}

impl From<ImageError> for WatermarkError {
    fn from(error: ImageError) -> Self {
        Self::Image(error)
    }
}

impl fmt::Display for WatermarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => f.write_str("watermark unavailable"),
            Self::Inference => f.write_str("watermark model output is invalid"),
            Self::Image(error) => write!(f, "watermark image error: {error}"),
        }
    }
}

impl std::error::Error for WatermarkError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_conversion() {
        assert_eq!(
            WatermarkError::Unavailable.to_string(),
            "watermark unavailable"
        );
        assert_eq!(
            WatermarkError::Inference.to_string(),
            "watermark model output is invalid"
        );
        let e: WatermarkError = ImageError::OutOfBounds.into();
        assert_eq!(e, WatermarkError::Image(ImageError::OutOfBounds));
        assert!(e.to_string().contains("region out of bounds"));
    }
}
