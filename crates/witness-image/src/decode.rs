use crate::buffer::{Image, Rgb8};
use crate::error::ImageError;
use crate::exif::jpeg_orientation;
use crate::format::{Format, sniff};
use crate::orient::{Orientation, orient};
use std::io::Cursor;
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;

pub const DEFAULT_MAX_PIXELS: u64 = 1 << 26;
const ZUNE_MAX_DIMENSION: usize = 1 << 24;

pub fn decode(bytes: &[u8]) -> Result<Rgb8, ImageError> {
    decode_with_limit(bytes, DEFAULT_MAX_PIXELS)
}

pub fn decode_with_limit(bytes: &[u8], max_pixels: u64) -> Result<Rgb8, ImageError> {
    match sniff(bytes).ok_or(ImageError::UnsupportedFormat)? {
        Format::Jpeg => decode_jpeg(bytes, max_pixels),
        Format::Png => decode_png(bytes, max_pixels),
        Format::WebP => decode_webp(bytes, max_pixels),
    }
}

pub fn decode_oriented(bytes: &[u8]) -> Result<Rgb8, ImageError> {
    decode_oriented_with_limit(bytes, DEFAULT_MAX_PIXELS)
}

pub fn decode_oriented_with_limit(bytes: &[u8], max_pixels: u64) -> Result<Rgb8, ImageError> {
    let image = decode_with_limit(bytes, max_pixels)?;
    match orientation_of(bytes) {
        Orientation::Normal => Ok(image),
        orientation => Ok(orient(image.view(), orientation)),
    }
}

pub fn orientation_of(bytes: &[u8]) -> Orientation {
    match sniff(bytes) {
        Some(Format::Jpeg) => jpeg_orientation(bytes).unwrap_or_default(),
        _ => Orientation::Normal,
    }
}

fn check_size(width: usize, height: usize, max_pixels: u64) -> Result<(u32, u32), ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::InvalidDimensions);
    }
    let w = u32::try_from(width).map_err(|_| ImageError::TooLarge)?;
    let h = u32::try_from(height).map_err(|_| ImageError::TooLarge)?;
    if w as u64 * h as u64 > max_pixels {
        return Err(ImageError::TooLarge);
    }
    crate::buffer::packed_len(w, h, 4)?;
    Ok((w, h))
}

fn safe_options() -> DecoderOptions {
    DecoderOptions::default()
        .set_use_unsafe(false)
        .set_max_width(ZUNE_MAX_DIMENSION)
        .set_max_height(ZUNE_MAX_DIMENSION)
}

fn decode_jpeg(bytes: &[u8], max_pixels: u64) -> Result<Rgb8, ImageError> {
    let fail = |_| ImageError::Decode(Format::Jpeg);
    let options = safe_options().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    decoder.decode_headers().map_err(fail)?;
    let info = decoder.info().ok_or(ImageError::Decode(Format::Jpeg))?;
    let (width, height) = check_size(info.width as usize, info.height as usize, max_pixels)?;
    let channels = decoder
        .output_colorspace()
        .ok_or(ImageError::Decode(Format::Jpeg))?
        .num_components();
    let len = decoder
        .output_buffer_size()
        .ok_or(ImageError::Decode(Format::Jpeg))?;
    let mut raw = vec![0u8; len];
    decoder.decode_into(&mut raw).map_err(fail)?;
    into_rgb(raw, channels, width, height, Format::Jpeg)
}

fn decode_png(bytes: &[u8], max_pixels: u64) -> Result<Rgb8, ImageError> {
    let fail = |_| ImageError::Decode(Format::Png);
    let options = safe_options()
        .png_set_strip_to_8bit(true)
        .png_set_decode_animated(false);
    let mut decoder = zune_png::PngDecoder::new_with_options(ZCursor::new(bytes), options);
    decoder.decode_headers().map_err(fail)?;
    let (w, h) = decoder
        .dimensions()
        .ok_or(ImageError::Decode(Format::Png))?;
    let (width, height) = check_size(w, h, max_pixels)?;
    let channels = decoder
        .colorspace()
        .ok_or(ImageError::Decode(Format::Png))?
        .num_components();
    let raw = match decoder.decode().map_err(fail)? {
        zune_core::result::DecodingResult::U8(raw) => raw,
        _ => return Err(ImageError::Decode(Format::Png)),
    };
    into_rgb(raw, channels, width, height, Format::Png)
}

fn decode_webp(bytes: &[u8], max_pixels: u64) -> Result<Rgb8, ImageError> {
    let fail = |_| ImageError::Decode(Format::WebP);
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(bytes)).map_err(fail)?;
    let (w, h) = decoder.dimensions();
    let (width, height) = check_size(w as usize, h as usize, max_pixels)?;
    let channels = if decoder.has_alpha() { 4 } else { 3 };
    let len = decoder.output_buffer_size().ok_or(ImageError::TooLarge)?;
    let mut raw = vec![0u8; len];
    decoder.read_image(&mut raw).map_err(fail)?;
    into_rgb(raw, channels, width, height, Format::WebP)
}

pub(crate) fn into_rgb(
    mut raw: Vec<u8>,
    channels: usize,
    width: u32,
    height: u32,
    format: Format,
) -> Result<Rgb8, ImageError> {
    let pixels = width as usize * height as usize;
    let needed = pixels.checked_mul(channels).ok_or(ImageError::TooLarge)?;
    if raw.len() < needed {
        return Err(ImageError::Decode(format));
    }
    match channels {
        3 => {}
        4 => {
            for i in 0..pixels {
                raw.copy_within(i * 4..i * 4 + 3, i * 3);
            }
        }
        1 | 2 => {
            raw.resize(pixels * 3, 0);
            for i in (0..pixels).rev() {
                let v = raw[i * channels];
                raw[i * 3..i * 3 + 3].fill(v);
            }
        }
        _ => return Err(ImageError::Decode(format)),
    }
    raw.truncate(pixels * 3);
    Ok(Image::from_parts_unchecked(width, height, raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_rgb_passes_rgb_through() {
        let rgb = into_rgb(vec![1, 2, 3, 4, 5, 6], 3, 2, 1, Format::Png).unwrap();
        assert_eq!(rgb.data(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn into_rgb_drops_alpha() {
        let rgb = into_rgb(
            vec![1, 2, 3, 9, 4, 5, 6, 9, 7, 8, 9, 0],
            4,
            3,
            1,
            Format::Png,
        )
        .unwrap();
        assert_eq!(rgb.data(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn into_rgb_expands_grey() {
        let rgb = into_rgb(vec![10, 20, 30], 1, 3, 1, Format::Png).unwrap();
        assert_eq!(rgb.data(), &[10, 10, 10, 20, 20, 20, 30, 30, 30]);
    }

    #[test]
    fn into_rgb_expands_grey_alpha() {
        let rgb = into_rgb(vec![10, 0, 20, 255, 30, 128], 2, 1, 3, Format::Png).unwrap();
        assert_eq!(rgb.dimensions(), (1, 3));
        assert_eq!(rgb.data(), &[10, 10, 10, 20, 20, 20, 30, 30, 30]);
    }

    #[test]
    fn into_rgb_trims_padding() {
        let rgb = into_rgb(vec![1, 2, 3, 4, 5, 6, 7], 3, 2, 1, Format::Jpeg).unwrap();
        assert_eq!(rgb.data().len(), 6);
    }

    #[test]
    fn into_rgb_rejects_short_and_odd_channels() {
        assert_eq!(
            into_rgb(vec![1, 2], 3, 1, 1, Format::Jpeg),
            Err(ImageError::Decode(Format::Jpeg))
        );
        assert_eq!(
            into_rgb(vec![0; 5], 5, 1, 1, Format::WebP),
            Err(ImageError::Decode(Format::WebP))
        );
    }

    #[test]
    fn into_rgb_large_grey_expansion_matches_naive() {
        let raw: Vec<u8> = (0..10_000u32).map(|i| (i * 7) as u8).collect();
        let rgb = into_rgb(raw.clone(), 1, 100, 100, Format::Png).unwrap();
        let expected: Vec<u8> = raw.iter().flat_map(|&v| [v, v, v]).collect();
        assert_eq!(rgb.data(), expected.as_slice());
    }

    #[test]
    fn check_size_limits() {
        assert_eq!(check_size(0, 5, 100), Err(ImageError::InvalidDimensions));
        assert_eq!(check_size(5, 0, 100), Err(ImageError::InvalidDimensions));
        assert_eq!(check_size(11, 10, 100), Err(ImageError::TooLarge));
        assert_eq!(check_size(10, 10, 100), Ok((10, 10)));
        assert_eq!(
            check_size(usize::MAX, 1, u64::MAX),
            Err(ImageError::TooLarge)
        );
    }

    #[test]
    fn rejects_unknown_format() {
        assert_eq!(decode(b"GIF89a"), Err(ImageError::UnsupportedFormat));
        assert_eq!(decode(&[]), Err(ImageError::UnsupportedFormat));
        assert_eq!(decode_oriented(b"BM"), Err(ImageError::UnsupportedFormat));
    }

    #[test]
    fn truncated_magic_is_decode_error() {
        assert_eq!(
            decode(&[0xFF, 0xD8, 0xFF]),
            Err(ImageError::Decode(Format::Jpeg))
        );
        assert!(matches!(
            decode(b"\x89PNG\r\n\x1a\n"),
            Err(ImageError::Decode(Format::Png))
        ));
        assert!(matches!(
            decode(b"RIFF\0\0\0\0WEBP"),
            Err(ImageError::Decode(Format::WebP))
        ));
    }

    #[test]
    fn orientation_of_non_jpeg_is_normal() {
        assert_eq!(orientation_of(b"\x89PNG\r\n\x1a\n"), Orientation::Normal);
        assert_eq!(orientation_of(&[]), Orientation::Normal);
    }
}
