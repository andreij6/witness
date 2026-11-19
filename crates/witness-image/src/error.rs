use crate::format::Format;
use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageError {
    UnsupportedFormat,
    InvalidDimensions,
    BufferSizeMismatch,
    TooLarge,
    OutOfBounds,
    Decode(Format),
    Encode,
    MalformedJpeg,
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat => f.write_str("unsupported image format"),
            Self::InvalidDimensions => f.write_str("invalid image dimensions"),
            Self::BufferSizeMismatch => f.write_str("buffer size does not match dimensions"),
            Self::TooLarge => f.write_str("image exceeds size limit"),
            Self::OutOfBounds => f.write_str("region out of bounds"),
            Self::Decode(format) => write!(f, "failed to decode {format:?}"),
            Self::Encode => f.write_str("failed to encode image"),
            Self::MalformedJpeg => f.write_str("malformed JPEG structure"),
        }
    }
}

impl std::error::Error for ImageError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_non_empty_for_every_variant() {
        let all = [
            ImageError::UnsupportedFormat,
            ImageError::InvalidDimensions,
            ImageError::BufferSizeMismatch,
            ImageError::TooLarge,
            ImageError::OutOfBounds,
            ImageError::Decode(Format::Jpeg),
            ImageError::Decode(Format::Png),
            ImageError::Decode(Format::WebP),
            ImageError::Encode,
            ImageError::MalformedJpeg,
        ];
        for error in all {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn decode_display_names_format() {
        assert!(ImageError::Decode(Format::Png).to_string().contains("Png"));
    }

    #[test]
    fn implements_std_error() {
        let boxed: Box<dyn std::error::Error> = Box::new(ImageError::Encode);
        assert_eq!(boxed.to_string(), "failed to encode image");
    }
}
