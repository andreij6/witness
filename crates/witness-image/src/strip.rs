use crate::error::ImageError;
use crate::jpeg_markers::{APP0, APP14, EOI, SOI, Segment, segments};

fn keep_marker(marker: u8, payload: &[u8]) -> bool {
    match marker {
        APP0 => payload.starts_with(b"JFIF\0"),
        APP14 => payload.starts_with(b"Adobe"),
        0xE1..=0xEF | 0xFE => false,
        _ => true,
    }
}

pub fn strip_and_normalize_jpeg(bytes: &[u8]) -> Result<Vec<u8>, ImageError> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut saw_frame = false;
    for segment in segments(bytes) {
        match segment? {
            Segment::Marker {
                marker,
                bytes: range,
                payload,
            } => {
                if marker == SOI {
                    out.extend_from_slice(&[0xFF, SOI]);
                } else if keep_marker(marker, &bytes[payload]) {
                    saw_frame |= matches!(marker, 0xC0..=0xCF if marker != 0xC4 && marker != 0xC8 && marker != 0xCC);
                    out.extend_from_slice(&bytes[range]);
                }
            }
            Segment::Entropy(range) => out.extend_from_slice(&bytes[range]),
            Segment::End(_) => {
                if !saw_frame {
                    return Err(ImageError::MalformedJpeg);
                }
                out.extend_from_slice(&[0xFF, EOI]);
                return Ok(out);
            }
        }
    }
    Err(ImageError::MalformedJpeg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg_markers::{APP1, SOS};

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, marker];
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn skeleton(extra: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![0xFF, SOI];
        out.extend(segment(APP0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"));
        extra.iter().for_each(|s| out.extend(s));
        out.extend(segment(0xC0, &[8, 0, 1, 0, 1, 1, 1, 0x11, 0]));
        out.extend(segment(SOS, &[1, 1, 0, 0, 63, 0]));
        out.extend([0xAB, 0xFF, 0x00, 0xCD]);
        out.extend([0xFF, EOI]);
        out
    }

    #[test]
    fn removes_metadata_segments() {
        let input = skeleton(&[
            segment(APP1, b"Exif\0\0MM\0\x2a\0\0\0\x08"),
            segment(APP1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>"),
            segment(0xE2, b"ICC_PROFILE\0\x01\x01data"),
            segment(0xE2, b"MPF\0MM\0\x2a"),
            segment(0xEB, b"JP\0\x01jumb"),
            segment(0xED, b"Photoshop 3.0\0"),
            segment(0xFE, b"comment"),
            segment(0xEE, b"Adobe\0\x64\0\0\0\0\x01"),
        ]);
        let stripped = strip_and_normalize_jpeg(&input).unwrap();
        let markers: Vec<u8> = segments(&stripped)
            .filter_map(|s| match s.unwrap() {
                Segment::Marker { marker, .. } => Some(marker),
                _ => None,
            })
            .collect();
        assert_eq!(markers, vec![SOI, APP0, APP14, 0xC0, SOS]);
        assert_eq!(&stripped[stripped.len() - 2..], &[0xFF, EOI]);
    }

    #[test]
    fn clean_input_is_unchanged() {
        let input = skeleton(&[]);
        assert_eq!(strip_and_normalize_jpeg(&input).unwrap(), input);
    }

    #[test]
    fn idempotent() {
        let input = skeleton(&[segment(APP1, b"Exif\0\0junk"), segment(0xE2, b"MPF\0")]);
        let once = strip_and_normalize_jpeg(&input).unwrap();
        assert_eq!(strip_and_normalize_jpeg(&once).unwrap(), once);
    }

    #[test]
    fn drops_trailing_gain_map_image() {
        let mut input = skeleton(&[segment(0xE2, b"MPF\0")]);
        let clean_len = strip_and_normalize_jpeg(&input).unwrap().len();
        input.extend(skeleton(&[segment(
            APP1,
            b"http://ns.adobe.com/xap/1.0/\0hdrgm",
        )]));
        input.extend([0, 0, 0]);
        assert_eq!(strip_and_normalize_jpeg(&input).unwrap().len(), clean_len);
    }

    #[test]
    fn drops_non_jfif_app0_and_non_adobe_app14() {
        let input = skeleton(&[segment(APP0, b"JFXX\0\x10thumb"), segment(APP14, b"Other")]);
        let stripped = strip_and_normalize_jpeg(&input).unwrap();
        assert_eq!(stripped, skeleton(&[]));
    }

    #[test]
    fn removes_fill_bytes() {
        let mut input = skeleton(&[]);
        input.splice(2..2, [0xFF, 0xFF]);
        assert_eq!(strip_and_normalize_jpeg(&input).unwrap(), skeleton(&[]));
    }

    #[test]
    fn preserves_entropy_data_with_restart_markers() {
        let mut input = skeleton(&[]);
        let at = input.len() - 2;
        input.splice(at..at, [0xFF, 0xD0, 0x11, 0xFF, 0xD7, 0x22]);
        assert_eq!(strip_and_normalize_jpeg(&input).unwrap(), input);
    }

    #[test]
    fn rejects_malformed() {
        assert_eq!(
            strip_and_normalize_jpeg(&[]),
            Err(ImageError::MalformedJpeg)
        );
        assert_eq!(
            strip_and_normalize_jpeg(&[0xFF, SOI]),
            Err(ImageError::MalformedJpeg)
        );
        assert_eq!(
            strip_and_normalize_jpeg(&[0xFF, SOI, 0xFF, EOI]),
            Err(ImageError::MalformedJpeg)
        );
        let input = skeleton(&[]);
        for len in 0..input.len() - 1 {
            assert_eq!(
                strip_and_normalize_jpeg(&input[..len]),
                Err(ImageError::MalformedJpeg),
                "len {len}"
            );
        }
    }

    #[test]
    fn output_never_contains_exif_marker_on_random_metadata() {
        let mut state = 77u64;
        for _ in 0..200 {
            let mut extras = Vec::new();
            for _ in 0..(state % 6) {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let marker = 0xE0 + (state % 16) as u8;
                let len = (state >> 8) as usize % 40;
                extras.push(segment(marker, &vec![(state >> 16) as u8; len]));
            }
            state = state.wrapping_add(1);
            let stripped = strip_and_normalize_jpeg(&skeleton(&extras)).unwrap();
            let has_metadata = segments(&stripped).any(|s| {
                matches!(
                    s,
                    Ok(Segment::Marker {
                        marker: 0xE1..=0xED | 0xEF,
                        ..
                    })
                )
            });
            assert!(!has_metadata);
        }
    }
}
