use crate::error::ImageError;
use core::ops::Range;

pub const SOI: u8 = 0xD8;
pub const EOI: u8 = 0xD9;
pub const SOS: u8 = 0xDA;
pub const APP0: u8 = 0xE0;
pub const APP1: u8 = 0xE1;
pub const APP14: u8 = 0xEE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    Marker {
        marker: u8,
        bytes: Range<usize>,
        payload: Range<usize>,
    },
    Entropy(Range<usize>),
    End(Range<usize>),
}

pub struct Segments<'a> {
    data: &'a [u8],
    pos: usize,
    in_scan: bool,
    done: bool,
}

pub fn segments(data: &[u8]) -> Segments<'_> {
    Segments {
        data,
        pos: 0,
        in_scan: false,
        done: false,
    }
}

fn is_standalone(marker: u8) -> bool {
    matches!(marker, 0x01 | 0xD0..=0xD7 | SOI)
}

fn entropy_end(data: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 1 < data.len() {
        if data[i] == 0xFF {
            match data[i + 1] {
                0x00 | 0xD0..=0xD7 => i += 2,
                _ => return Some(i),
            }
        } else {
            i += 1;
        }
    }
    None
}

impl Segments<'_> {
    fn fail(&mut self) -> Option<Result<Segment, ImageError>> {
        self.done = true;
        Some(Err(ImageError::MalformedJpeg))
    }
}

impl Iterator for Segments<'_> {
    type Item = Result<Segment, ImageError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let data = self.data;
        if self.pos == 0 {
            if data.len() < 2 || data[0] != 0xFF || data[1] != SOI {
                return self.fail();
            }
            self.pos = 2;
            return Some(Ok(Segment::Marker {
                marker: SOI,
                bytes: 0..2,
                payload: 2..2,
            }));
        }
        if self.in_scan {
            self.in_scan = false;
            let Some(end) = entropy_end(data, self.pos) else {
                return self.fail();
            };
            let range = self.pos..end;
            self.pos = end;
            return Some(Ok(Segment::Entropy(range)));
        }
        let mut start = self.pos;
        while start + 1 < data.len() && data[start] == 0xFF && data[start + 1] == 0xFF {
            start += 1;
        }
        if start + 1 >= data.len() || data[start] != 0xFF {
            return self.fail();
        }
        let marker = data[start + 1];
        let after_marker = start + 2;
        if marker == EOI {
            self.done = true;
            return Some(Ok(Segment::End(start..after_marker)));
        }
        if marker == SOI {
            return self.fail();
        }
        if is_standalone(marker) {
            self.pos = after_marker;
            return Some(Ok(Segment::Marker {
                marker,
                bytes: start..after_marker,
                payload: after_marker..after_marker,
            }));
        }
        let Some(length_bytes) = data.get(after_marker..after_marker + 2) else {
            return self.fail();
        };
        let length = u16::from_be_bytes([length_bytes[0], length_bytes[1]]) as usize;
        let end = after_marker + length;
        if length < 2 || end > data.len() {
            return self.fail();
        }
        self.pos = end;
        self.in_scan = marker == SOS;
        Some(Ok(Segment::Marker {
            marker,
            bytes: start..end,
            payload: after_marker + 2..end,
        }))
    }
}

pub fn header_segments(data: &[u8]) -> impl Iterator<Item = (u8, &[u8])> {
    segments(data)
        .map_while(Result::ok)
        .map_while(move |segment| match segment {
            Segment::Marker {
                marker, payload, ..
            } if marker != SOS => Some((marker, &data[payload])),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, marker];
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn minimal() -> Vec<u8> {
        let mut out = vec![0xFF, SOI];
        out.extend(segment(APP0, b"JFIF\0"));
        out.extend(segment(0xDB, &[0; 65]));
        out.extend(segment(SOS, &[1, 1, 0, 0, 63, 0]));
        out.extend([0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD3, 0x56]);
        out.extend([0xFF, EOI]);
        out
    }

    #[test]
    fn walks_minimal_stream() {
        let data = minimal();
        let items: Vec<Segment> = segments(&data).map(Result::unwrap).collect();
        assert_eq!(items.len(), 6);
        assert!(matches!(items[0], Segment::Marker { marker: SOI, .. }));
        assert!(matches!(items[1], Segment::Marker { marker: APP0, .. }));
        assert!(matches!(items[3], Segment::Marker { marker: SOS, .. }));
        let Segment::Entropy(range) = &items[4] else {
            panic!()
        };
        assert_eq!(
            &data[range.clone()],
            &[0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD3, 0x56]
        );
        assert_eq!(items[5], Segment::End(data.len() - 2..data.len()));
    }

    #[test]
    fn payload_excludes_marker_and_length() {
        let data = minimal();
        let first = segments(&data).nth(1).unwrap().unwrap();
        let Segment::Marker { payload, bytes, .. } = first else {
            panic!()
        };
        assert_eq!(&data[payload], b"JFIF\0");
        assert_eq!(bytes.len(), 2 + 2 + 5);
    }

    #[test]
    fn header_segments_stop_at_scan() {
        let data = minimal();
        let markers: Vec<u8> = header_segments(&data).map(|(m, _)| m).collect();
        assert_eq!(markers, vec![SOI, APP0, 0xDB]);
    }

    #[test]
    fn skips_fill_bytes_between_markers() {
        let mut data = vec![0xFF, SOI, 0xFF, 0xFF, 0xFF];
        data.extend(&segment(APP1, b"x")[1..]);
        data.extend([0xFF, EOI]);
        let items: Vec<_> = segments(&data).map(Result::unwrap).collect();
        assert!(matches!(items[1], Segment::Marker { marker: APP1, .. }));
        assert!(matches!(items[2], Segment::End(_)));
    }

    #[test]
    fn stops_after_end_marker() {
        let mut data = minimal();
        data.extend([0xFF, SOI, 1, 2, 3]);
        let items: Vec<_> = segments(&data).collect();
        assert!(matches!(items.last(), Some(Ok(Segment::End(_)))));
    }

    #[test]
    fn rejects_missing_soi() {
        let items: Vec<_> = segments(&[0x00, 0x01]).collect();
        assert_eq!(items, vec![Err(ImageError::MalformedJpeg)]);
        assert_eq!(segments(&[]).count(), 1);
    }

    #[test]
    fn rejects_truncated_inputs_without_panicking() {
        let data = minimal();
        for len in 0..data.len() - 1 {
            let results: Vec<_> = segments(&data[..len]).collect();
            assert!(results.iter().any(Result::is_err), "len {len}");
            assert!(results.len() <= 7);
        }
    }

    #[test]
    fn rejects_short_length_field() {
        let data = [0xFF, SOI, 0xFF, APP1, 0x00, 0x01];
        assert!(segments(&data).any(|r| r.is_err()));
    }

    #[test]
    fn rejects_nested_soi() {
        let data = [0xFF, SOI, 0xFF, SOI];
        assert!(segments(&data).any(|r| r.is_err()));
    }

    #[test]
    fn rejects_garbage_between_segments() {
        let data = [0xFF, SOI, 0x12, 0x34, 0xFF, EOI];
        assert!(segments(&data).any(|r| r.is_err()));
    }

    #[test]
    fn standalone_markers_have_no_length() {
        let data = [0xFF, SOI, 0xFF, 0x01, 0xFF, EOI];
        let items: Vec<_> = segments(&data).map(Result::unwrap).collect();
        assert_eq!(
            items[1],
            Segment::Marker {
                marker: 0x01,
                bytes: 2..4,
                payload: 4..4
            }
        );
    }

    #[test]
    fn random_bytes_never_panic_and_terminate() {
        let mut state = 0x1234_5678_9ABC_DEF0u64;
        for _ in 0..2000 {
            let len = (state % 64) as usize;
            let mut data = vec![0xFF, SOI];
            for _ in 0..len {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                data.push(if state & 3 == 0 {
                    0xFF
                } else {
                    (state >> 8) as u8
                });
            }
            assert!(segments(&data).count() <= data.len());
        }
    }
}
