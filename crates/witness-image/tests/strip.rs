mod common;

use common::*;
use witness_image::*;

fn metadata_heavy_jpeg(image: &Rgb8) -> Vec<u8> {
    let xmp = b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta hdrgm:Version=\"1.0\"/>".to_vec();
    let icc = b"ICC_PROFILE\0\x01\x01fake-profile".to_vec();
    let mpf = b"MPF\0MM\0\x2a\0\0\0\x08".to_vec();
    let mut jpeg = jpeg_with(
        image,
        92,
        Some(6),
        &[
            (1, xmp),
            (2, icc),
            (2, mpf),
            (13, b"Photoshop 3.0\0".to_vec()),
        ],
    );
    jpeg.extend(encode_jpeg(&gradient(16, 16), 80).unwrap());
    jpeg
}

#[test]
fn stripped_jpeg_has_identical_pixels() {
    let image = noise(48, 40, 99);
    let original = metadata_heavy_jpeg(&image);
    let stripped = strip_and_normalize_jpeg(&original).unwrap();
    assert!(stripped.len() < original.len());
    assert_eq!(decode(&stripped).unwrap(), decode(&original).unwrap());
    assert_eq!(
        pixel_hash(&stripped).unwrap(),
        pixel_hash(&original).unwrap()
    );
}

#[test]
fn stripped_jpeg_implies_no_rotation() {
    let original = metadata_heavy_jpeg(&quadrants(30, 20));
    assert_eq!(orientation_of(&original), Orientation::Rotate90);
    let stripped = strip_and_normalize_jpeg(&original).unwrap();
    assert_eq!(jpeg_orientation(&stripped), None);
    assert_eq!(exif_payload(&stripped), None);
    assert_eq!(
        decode_oriented(&stripped).unwrap(),
        decode(&stripped).unwrap()
    );
}

#[test]
fn stripped_jpeg_has_no_app1_or_app2_bytes_pattern() {
    let stripped = strip_and_normalize_jpeg(&metadata_heavy_jpeg(&gradient(20, 20))).unwrap();
    let text = String::from_utf8_lossy(&stripped);
    for needle in ["Exif", "ICC_PROFILE", "MPF", "xmpmeta", "Photoshop"] {
        assert!(!text.contains(needle), "{needle}");
    }
    assert_eq!(
        stripped.windows(2).filter(|w| *w == [0xFF, 0xD8]).count(),
        1
    );
}

#[test]
fn encoder_output_is_already_normal() {
    let jpeg = encode_jpeg(&gradient(24, 24), 85).unwrap();
    assert_eq!(strip_and_normalize_jpeg(&jpeg).unwrap(), jpeg);
}

#[test]
fn rejects_non_jpeg() {
    assert_eq!(
        strip_and_normalize_jpeg(&png_rgb(&gradient(4, 4))),
        Err(ImageError::MalformedJpeg)
    );
}
