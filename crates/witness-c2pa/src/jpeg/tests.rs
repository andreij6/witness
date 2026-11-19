use super::*;
use crate::testkit::{Rng, jpeg, jpeg_with};
use alloc::vec;
use alloc::vec::Vec;

fn markers(buf: &[u8]) -> Result<Vec<u8>, Error> {
    segments(buf)?.map(|s| s.map(|s| s.marker)).collect()
}

fn seg(m: u8, contents: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    write_segment(&mut v, m, &[contents]).unwrap();
    v
}

fn file(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = vec![0xff, marker::SOI];
    parts.iter().for_each(|p| v.extend_from_slice(p));
    v
}

#[test]
fn rejects_non_jpeg() {
    for b in [
        &[][..],
        &[0xff][..],
        &[0xff, 0xd9][..],
        &[0x89, b'P', b'N', b'G'][..],
    ] {
        assert_eq!(segments(b).err(), Some(Error::NotJpeg));
    }
}

#[test]
fn lists_header_segments_and_skips_entropy() {
    let j = jpeg(1);
    assert_eq!(
        markers(&j).unwrap(),
        vec![0xe0, 0xdb, 0xc0, 0xc4, marker::SOS]
    );
}

#[test]
fn segment_ranges_tile_the_header() {
    let j = jpeg(2);
    let segs: Vec<_> = segments(&j).unwrap().map(Result::unwrap).collect();
    assert_eq!(segs[0].offset, 2);
    for w in segs.windows(2) {
        assert_eq!(w[0].end(), w[1].offset);
    }
    for s in &segs {
        assert_eq!(j[s.offset], 0xff);
        assert_eq!(j[s.offset + 1], s.marker);
        assert_eq!(&j[s.range()][4..], s.contents);
    }
}

#[test]
fn empty_soi_only_file_has_no_segments() {
    assert_eq!(markers(&[0xff, 0xd8]).unwrap(), Vec::<u8>::new());
}

#[test]
fn eoi_ends_iteration_even_with_trailing_data() {
    let f = file(&[
        &seg(0xe0, b"a"),
        &[0xff, marker::EOI],
        &seg(0xeb, b"JPxxxxxx"),
    ]);
    assert_eq!(markers(&f).unwrap(), vec![0xe0]);
}

#[test]
fn fill_bytes_before_marker_are_skipped() {
    let mut f = file(&[&[0xff, 0xff, 0xff]]);
    f.extend(seg(0xe1, b"exif"));
    let segs: Vec<_> = segments(&f).unwrap().map(Result::unwrap).collect();
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0].offset, 5);
    assert_eq!(segs[0].contents, b"exif");
}

#[test]
fn standalone_markers_carry_no_length() {
    let f = file(&[&[0xff, marker::TEM, 0xff, 0xd3], &seg(0xfe, b"c")]);
    assert_eq!(markers(&f).unwrap(), vec![0xfe]);
}

#[test]
fn malformed_structures_are_rejected() {
    let cases: Vec<Vec<u8>> = vec![
        file(&[&[0x00]]),
        file(&[&[0xff, 0xe0, 0x00, 0x01]]),
        file(&[&[0xff, 0xe0, 0x00]]),
        file(&[&[0xff, 0xe0, 0x00, 0x10, 1, 2]]),
        file(&[&[0xff, 0xff, 0xff]]),
        file(&[&[0xff]]),
        file(&[&[0xff, marker::SOI]]),
        file(&[&[0xff, 0x00]]),
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_eq!(markers(c), Err(Error::MalformedJpeg), "case {i}");
    }
}

#[test]
fn zero_length_contents_is_valid() {
    let f = file(&[&[0xff, 0xe2, 0x00, 0x02]]);
    let s: Vec<_> = segments(&f).unwrap().map(Result::unwrap).collect();
    assert_eq!(s[0].contents, b"");
    assert_eq!(s[0].end(), 6);
}

#[test]
fn iterator_is_fused_after_error() {
    let f = file(&[&seg(0xe0, b"a"), &[0x12]]);
    let mut it = segments(&f).unwrap();
    assert!(it.next().unwrap().is_ok());
    assert_eq!(it.next(), Some(Err(Error::MalformedJpeg)));
    assert_eq!(it.next(), None);
}

#[test]
fn entropy_without_eoi_ends_cleanly() {
    let f = file(&[
        &seg(marker::SOS, &[1, 1, 0, 0, 63, 0]),
        &[1, 2, 0xff, 0x00, 3, 0xff],
    ]);
    assert_eq!(markers(&f).unwrap(), vec![marker::SOS]);
}

#[test]
fn segments_after_first_scan_are_visible() {
    let f = file(&[
        &seg(marker::SOS, &[1]),
        &[1, 2, 0xff, 0x00, 0xff, 0xd1, 3],
        &seg(0xc4, &[7]),
        &seg(marker::SOS, &[2]),
        &[4, 5],
        &[0xff, 0xff, marker::EOI],
    ]);
    assert_eq!(markers(&f).unwrap(), vec![marker::SOS, 0xc4, marker::SOS]);
}

#[test]
fn write_segment_encodes_length_and_rejects_oversize() {
    let mut out = Vec::new();
    write_segment(&mut out, 0xeb, &[b"ab", b"c"]).unwrap();
    assert_eq!(out, vec![0xff, 0xeb, 0x00, 0x05, b'a', b'b', b'c']);
    let big = vec![0u8; MAX_SEGMENT_CONTENTS];
    let mut out = Vec::new();
    write_segment(&mut out, 0xeb, &[&big]).unwrap();
    assert_eq!(&out[2..4], &[0xff, 0xff]);
    assert_eq!(
        write_segment(&mut Vec::new(), 0xeb, &[&big, &[1]]),
        Err(Error::TooLarge)
    );
}

#[test]
fn extra_app_segments_are_listed_in_order() {
    let mut rng = Rng::new(3);
    let j = jpeg_with(
        &mut rng,
        &[(0xe1, b"Exif\0\0".to_vec()), (0xe2, vec![1; 300])],
        64,
    );
    assert_eq!(
        markers(&j).unwrap(),
        vec![0xe0, 0xe1, 0xe2, 0xdb, 0xc0, 0xc4, marker::SOS]
    );
}

#[test]
fn random_mutations_never_panic() {
    let base = jpeg(4);
    let mut rng = Rng::new(5);
    for _ in 0..3000 {
        let mut d = base.clone();
        for _ in 0..1 + rng.below(4) {
            let i = rng.below(d.len());
            d[i] = rng.next() as u8;
        }
        if let Ok(it) = segments(&d) {
            for s in it.flatten() {
                assert!(s.end() <= d.len());
            }
        }
    }
}

#[test]
fn random_truncations_never_panic() {
    let base = jpeg(6);
    for n in 0..base.len() {
        if let Ok(it) = segments(&base[..n]) {
            let _ = it.count();
        }
    }
}
