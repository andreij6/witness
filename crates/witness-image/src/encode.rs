use crate::buffer::Rgb8;
use crate::error::ImageError;
use jpeg_encoder::{ColorType, Encoder};

pub fn encode_jpeg(image: &Rgb8, quality: u8) -> Result<Vec<u8>, ImageError> {
    let width = u16::try_from(image.width()).map_err(|_| ImageError::TooLarge)?;
    let height = u16::try_from(image.height()).map_err(|_| ImageError::TooLarge)?;
    let mut out = Vec::with_capacity(image.data().len() / 8 + 1024);
    Encoder::new(&mut out, quality.clamp(1, 100))
        .encode(image.data(), width, height, ColorType::Rgb)
        .map_err(|_| ImageError::Encode)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{Format, sniff};

    #[test]
    fn produces_jpeg_magic_and_eoi() {
        let image = Rgb8::filled(16, 8, [10, 200, 30]).unwrap();
        let jpeg = encode_jpeg(&image, 90).unwrap();
        assert_eq!(sniff(&jpeg), Some(Format::Jpeg));
        assert_eq!(&jpeg[jpeg.len() - 2..], &[0xFF, 0xD9]);
    }

    #[test]
    fn deterministic_output() {
        let image =
            Rgb8::from_fn(33, 17, |x, y| [x as u8 * 7, y as u8 * 13, (x ^ y) as u8]).unwrap();
        assert_eq!(
            encode_jpeg(&image, 85).unwrap(),
            encode_jpeg(&image, 85).unwrap()
        );
    }

    #[test]
    fn quality_is_clamped() {
        let image = Rgb8::filled(8, 8, [1, 2, 3]).unwrap();
        assert_eq!(
            encode_jpeg(&image, 0).unwrap(),
            encode_jpeg(&image, 1).unwrap()
        );
        assert_eq!(
            encode_jpeg(&image, 255).unwrap(),
            encode_jpeg(&image, 100).unwrap()
        );
    }

    #[test]
    fn higher_quality_is_not_smaller_on_noise() {
        let mut state = 1u32;
        let image = Rgb8::from_fn(64, 64, |_, _| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let [a, b, c, _] = state.to_le_bytes();
            [a, b, c]
        })
        .unwrap();
        assert!(encode_jpeg(&image, 95).unwrap().len() > encode_jpeg(&image, 30).unwrap().len());
    }

    #[test]
    fn rejects_dimensions_beyond_jpeg_limit() {
        let image = Rgb8::new(65_536, 1, vec![0; 65_536 * 3]).unwrap();
        assert_eq!(encode_jpeg(&image, 90), Err(ImageError::TooLarge));
    }
}
