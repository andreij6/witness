mod common;

use common::*;
use witness_image::*;

fn block_colour(image: &Rgb8, qx: u32, qy: u32) -> [u8; 3] {
    let x = image.width() / 4 + qx * image.width() / 2;
    let y = image.height() / 4 + qy * image.height() / 2;
    image.pixel(x, y).unwrap()
}

fn close(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(&x, y)| x.abs_diff(y) < 20)
}

#[test]
fn decode_oriented_restores_display_for_every_exif_value() {
    let display = quadrants(64, 32);
    for orientation in Orientation::ALL {
        let stored = orient(display.view(), orientation.inverse());
        let jpeg = jpeg_with(&stored, 98, Some(orientation.exif_value() as u16), &[]);
        assert_eq!(jpeg_orientation(&jpeg), Some(orientation));
        assert_eq!(orientation_of(&jpeg), orientation);
        let oriented = decode_oriented(&jpeg).unwrap();
        assert_eq!(oriented.dimensions(), (64, 32), "{orientation:?}");
        for qy in 0..2 {
            for qx in 0..2 {
                let expected = QUADRANT_COLOURS[(qx + 2 * qy) as usize];
                assert!(
                    close(block_colour(&oriented, qx, qy), expected),
                    "{orientation:?} quadrant {qx},{qy}"
                );
            }
        }
        let raw = decode(&jpeg).unwrap();
        assert_eq!(raw.dimensions(), stored.dimensions());
    }
}

#[test]
fn invalid_or_missing_orientation_is_identity() {
    let image = quadrants(32, 16);
    for value in [None, Some(0u16), Some(9), Some(0xFFFF)] {
        let jpeg = jpeg_with(&image, 95, value, &[]);
        assert_eq!(decode_oriented(&jpeg).unwrap(), decode(&jpeg).unwrap());
        assert_eq!(orientation_of(&jpeg), Orientation::Normal);
    }
}

#[test]
fn orientation_ignored_for_lossless_formats() {
    let image = quadrants(10, 6);
    assert_eq!(decode_oriented(&png_rgb(&image)).unwrap(), image);
    let webp = webp_lossless(image.data(), 10, 6, false);
    assert_eq!(decode_oriented(&webp).unwrap(), image);
}

#[test]
fn oriented_decode_equals_orient_of_raw_decode() {
    let stored = noise(40, 24, 13);
    for orientation in Orientation::ALL {
        let jpeg = jpeg_with(&stored, 90, Some(orientation.exif_value() as u16), &[]);
        let raw = decode(&jpeg).unwrap();
        assert_eq!(
            decode_oriented(&jpeg).unwrap(),
            orient(raw.view(), orientation)
        );
    }
}

#[test]
fn exif_payload_exposes_tiff() {
    let jpeg = jpeg_with(&gradient(8, 8), 90, Some(6), &[]);
    let tiff = exif_payload(&jpeg).unwrap();
    assert!(tiff.starts_with(b"MM"));
    assert_eq!(tiff_orientation(tiff), Some(Orientation::Rotate90));
}
