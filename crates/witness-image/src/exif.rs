use crate::jpeg_markers::{APP1, header_segments};
use crate::orient::Orientation;

const EXIF_HEADER: &[u8] = b"Exif\0";
const ORIENTATION_TAG: u16 = 0x0112;
const TYPE_SHORT: u16 = 3;
const TYPE_LONG: u16 = 4;
const ENTRY_LEN: usize = 12;

#[derive(Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
    fn u16(self, bytes: &[u8], at: usize) -> Option<u16> {
        let raw: [u8; 2] = bytes.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(match self {
            Self::Little => u16::from_le_bytes(raw),
            Self::Big => u16::from_be_bytes(raw),
        })
    }

    fn u32(self, bytes: &[u8], at: usize) -> Option<u32> {
        let raw: [u8; 4] = bytes.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(match self {
            Self::Little => u32::from_le_bytes(raw),
            Self::Big => u32::from_be_bytes(raw),
        })
    }
}

pub fn tiff_orientation(tiff: &[u8]) -> Option<Orientation> {
    let order = match tiff.get(..2)? {
        b"II" => ByteOrder::Little,
        b"MM" => ByteOrder::Big,
        _ => return None,
    };
    if order.u16(tiff, 2)? != 42 {
        return None;
    }
    let ifd = order.u32(tiff, 4)? as usize;
    let count = order.u16(tiff, ifd)? as usize;
    let entries = tiff.get(ifd.checked_add(2)?..)?;
    entries
        .chunks_exact(ENTRY_LEN)
        .take(count)
        .find(|entry| order.u16(entry, 0) == Some(ORIENTATION_TAG))
        .and_then(|entry| {
            let kind = order.u16(entry, 2)?;
            let values = order.u32(entry, 4)?;
            let value = match (kind, values) {
                (TYPE_SHORT, 1..=2) => order.u16(entry, 8)? as u32,
                (TYPE_LONG, 1) => order.u32(entry, 8)?,
                _ => return None,
            };
            Orientation::from_exif(value)
        })
}

pub fn exif_payload(jpeg: &[u8]) -> Option<&[u8]> {
    header_segments(jpeg)
        .filter(|(marker, _)| *marker == APP1)
        .find_map(|(_, payload)| {
            payload
                .strip_prefix(EXIF_HEADER)
                .and_then(|rest| rest.get(1..))
        })
}

pub fn jpeg_orientation(jpeg: &[u8]) -> Option<Orientation> {
    tiff_orientation(exif_payload(jpeg)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiff(big_endian: bool, entries: &[(u16, u16, u32, [u8; 4])]) -> Vec<u8> {
        let u16b = |v: u16| {
            if big_endian {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let u32b = |v: u32| {
            if big_endian {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let mut out = Vec::new();
        out.extend_from_slice(if big_endian { b"MM" } else { b"II" });
        out.extend(u16b(42));
        out.extend(u32b(8));
        out.extend(u16b(entries.len() as u16));
        for &(tag, kind, count, value) in entries {
            out.extend(u16b(tag));
            out.extend(u16b(kind));
            out.extend(u32b(count));
            out.extend(value);
        }
        out.extend(u32b(0));
        out
    }

    fn short_value(big_endian: bool, v: u16) -> [u8; 4] {
        let b = if big_endian {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        };
        [b[0], b[1], 0, 0]
    }

    fn jpeg_with_app1(payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, 0xD8, 0xFF, APP1];
        out.extend(((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out.extend([0xFF, 0xD9]);
        out
    }

    fn exif_block(tiff: &[u8]) -> Vec<u8> {
        let mut out = b"Exif\0\0".to_vec();
        out.extend_from_slice(tiff);
        out
    }

    #[test]
    fn reads_every_valid_value_both_endians() {
        for big_endian in [false, true] {
            for value in 1..=8u16 {
                let data = tiff(
                    big_endian,
                    &[(
                        ORIENTATION_TAG,
                        TYPE_SHORT,
                        1,
                        short_value(big_endian, value),
                    )],
                );
                assert_eq!(
                    tiff_orientation(&data),
                    Orientation::from_exif(value as u32)
                );
            }
        }
    }

    #[test]
    fn finds_tag_among_other_entries() {
        let data = tiff(
            false,
            &[
                (0x010F, 2, 4, *b"ACME"),
                (0x0110, 2, 4, *b"CAM\0"),
                (ORIENTATION_TAG, TYPE_SHORT, 1, short_value(false, 6)),
                (0x011A, 5, 1, [0, 0, 0, 0]),
            ],
        );
        assert_eq!(tiff_orientation(&data), Some(Orientation::Rotate90));
    }

    #[test]
    fn first_orientation_entry_wins() {
        let data = tiff(
            true,
            &[
                (ORIENTATION_TAG, TYPE_SHORT, 1, short_value(true, 3)),
                (ORIENTATION_TAG, TYPE_SHORT, 1, short_value(true, 8)),
            ],
        );
        assert_eq!(tiff_orientation(&data), Some(Orientation::Rotate180));
    }

    #[test]
    fn accepts_long_type() {
        let data = tiff(
            false,
            &[(ORIENTATION_TAG, TYPE_LONG, 1, 8u32.to_le_bytes())],
        );
        assert_eq!(tiff_orientation(&data), Some(Orientation::Rotate270));
    }

    #[test]
    fn rejection_table() {
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("empty", vec![]),
            ("one byte", vec![b'I']),
            ("bad order", b"IM\x2a\x00\x08\x00\x00\x00".to_vec()),
            ("bad magic", b"II\x2b\x00\x08\x00\x00\x00".to_vec()),
            ("header only", b"II\x2a\x00".to_vec()),
            ("ifd past end", b"II\x2a\x00\xff\xff\x00\x00".to_vec()),
            (
                "ifd offset overflow",
                b"II\x2a\x00\xff\xff\xff\xff".to_vec(),
            ),
            ("missing count", b"II\x2a\x00\x08\x00\x00\x00\x01".to_vec()),
            ("zero entries", tiff(false, &[])),
            (
                "value zero",
                tiff(
                    false,
                    &[(ORIENTATION_TAG, TYPE_SHORT, 1, short_value(false, 0))],
                ),
            ),
            (
                "value nine",
                tiff(
                    false,
                    &[(ORIENTATION_TAG, TYPE_SHORT, 1, short_value(false, 9))],
                ),
            ),
            (
                "value max",
                tiff(
                    false,
                    &[(ORIENTATION_TAG, TYPE_SHORT, 1, [0xFF, 0xFF, 0, 0])],
                ),
            ),
            (
                "wrong type",
                tiff(false, &[(ORIENTATION_TAG, 2, 1, short_value(false, 6))]),
            ),
            (
                "zero count",
                tiff(
                    false,
                    &[(ORIENTATION_TAG, TYPE_SHORT, 0, short_value(false, 6))],
                ),
            ),
            (
                "huge count",
                tiff(
                    false,
                    &[(ORIENTATION_TAG, TYPE_SHORT, 9, short_value(false, 6))],
                ),
            ),
            (
                "long count two",
                tiff(
                    false,
                    &[(ORIENTATION_TAG, TYPE_LONG, 2, 6u32.to_le_bytes())],
                ),
            ),
            (
                "long huge value",
                tiff(
                    false,
                    &[(ORIENTATION_TAG, TYPE_LONG, 1, u32::MAX.to_le_bytes())],
                ),
            ),
            (
                "other tag only",
                tiff(false, &[(0x0113, TYPE_SHORT, 1, short_value(false, 6))]),
            ),
        ];
        for (name, data) in cases {
            assert_eq!(tiff_orientation(&data), None, "{name}");
        }
    }

    #[test]
    fn count_larger_than_data_is_bounded() {
        let mut data = tiff(
            false,
            &[(ORIENTATION_TAG, TYPE_SHORT, 1, short_value(false, 5))],
        );
        data[8] = 0xFF;
        data[9] = 0xFF;
        assert_eq!(tiff_orientation(&data), Some(Orientation::Transpose));
        data.truncate(10 + 11);
        assert_eq!(tiff_orientation(&data), None);
    }

    #[test]
    fn count_smaller_than_entries_hides_later_tags() {
        let mut data = tiff(
            false,
            &[
                (0x010F, 2, 4, *b"ACME"),
                (ORIENTATION_TAG, TYPE_SHORT, 1, short_value(false, 6)),
            ],
        );
        data[8] = 1;
        assert_eq!(tiff_orientation(&data), None);
    }

    #[test]
    fn ifd_offset_may_point_anywhere_in_bounds() {
        let mut data = b"MM\x00\x2a\x00\x00\x00\x10".to_vec();
        data.extend([0xAA; 8]);
        data.extend(1u16.to_be_bytes());
        data.extend(ORIENTATION_TAG.to_be_bytes());
        data.extend(TYPE_SHORT.to_be_bytes());
        data.extend(1u32.to_be_bytes());
        data.extend(short_value(true, 7));
        assert_eq!(tiff_orientation(&data), Some(Orientation::Transverse));
    }

    #[test]
    fn jpeg_orientation_from_app1() {
        let tiff = tiff(
            true,
            &[(ORIENTATION_TAG, TYPE_SHORT, 1, short_value(true, 6))],
        );
        assert_eq!(
            jpeg_orientation(&jpeg_with_app1(&exif_block(&tiff))),
            Some(Orientation::Rotate90)
        );
    }

    #[test]
    fn jpeg_orientation_skips_xmp_app1() {
        let tiff = tiff(
            false,
            &[(ORIENTATION_TAG, TYPE_SHORT, 1, short_value(false, 3))],
        );
        let mut data = vec![0xFF, 0xD8];
        let xmp = b"http://ns.adobe.com/xap/1.0/\0<x/>";
        data.extend([0xFF, APP1]);
        data.extend(((xmp.len() + 2) as u16).to_be_bytes());
        data.extend_from_slice(xmp);
        data.extend(&jpeg_with_app1(&exif_block(&tiff))[2..]);
        assert_eq!(jpeg_orientation(&data), Some(Orientation::Rotate180));
    }

    #[test]
    fn jpeg_without_exif_has_none() {
        assert_eq!(jpeg_orientation(&[0xFF, 0xD8, 0xFF, 0xD9]), None);
        assert_eq!(jpeg_orientation(&[]), None);
        assert_eq!(jpeg_orientation(&jpeg_with_app1(b"Exif")), None);
        assert_eq!(jpeg_orientation(&jpeg_with_app1(b"Exif\0")), None);
    }

    #[test]
    fn jpeg_exif_after_scan_is_ignored() {
        let tiff = tiff(
            false,
            &[(ORIENTATION_TAG, TYPE_SHORT, 1, short_value(false, 6))],
        );
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0x00, 0xFF, 0xD9];
        data.extend(jpeg_with_app1(&exif_block(&tiff)));
        assert_eq!(jpeg_orientation(&data), None);
    }

    #[test]
    fn random_mutations_never_panic() {
        let base = exif_block(&tiff(
            false,
            &[
                (0x010F, 2, 4, *b"ACME"),
                (ORIENTATION_TAG, TYPE_SHORT, 1, short_value(false, 6)),
            ],
        ));
        let mut state = 0xC0FF_EE00_1234_5678u64;
        for _ in 0..5000 {
            let mut mutated = base.clone();
            for _ in 0..3 {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let at = (state as usize) % mutated.len();
                mutated[at] = (state >> 32) as u8;
            }
            state ^= state << 13;
            let keep = (state as usize) % (mutated.len() + 1);
            mutated.truncate(keep);
            let result = jpeg_orientation(&jpeg_with_app1(&mutated));
            assert!(result.is_none() || (1..=8).contains(&result.unwrap().exif_value()));
        }
    }
}
