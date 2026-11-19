mod common;

use common::*;
use witness_image::*;

fn exercise(bytes: &[u8]) {
    let decoded = decode_with_limit(bytes, 1 << 20);
    if let Ok(image) = &decoded {
        assert_eq!(
            image.data().len(),
            image.width() as usize * image.height() as usize * 3
        );
    }
    let _ = decode_oriented_with_limit(bytes, 1 << 20);
    let _ = jpeg_orientation(bytes);
    let _ = strip_and_normalize_jpeg(bytes);
    let _ = sniff(bytes);
}

fn fuzz(base: &[u8], seed: u64, rounds: usize) {
    let mut rng = Rng(seed);
    for _ in 0..rounds {
        let flips = 1 + rng.below(8) as usize;
        exercise(&mutate(base, &mut rng, flips));
    }
    for len in (0..base.len()).step_by((base.len() / 64).max(1)) {
        exercise(&base[..len]);
    }
}

#[test]
fn mutated_jpegs_never_panic() {
    let image = noise(32, 24, 1);
    fuzz(
        &jpeg_with(&image, 85, Some(6), &[(2, b"MPF\0".to_vec())]),
        0xA1,
        1500,
    );
    let mut progressive = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut progressive, 80);
    encoder.set_progressive(true);
    encoder
        .encode(image.data(), 32, 24, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    fuzz(&progressive, 0xA2, 800);
}

#[test]
fn mutated_pngs_never_panic() {
    fuzz(&png_rgb(&noise(16, 12, 2)), 0xB1, 1500);
}

#[test]
fn mutated_webps_never_panic() {
    let image = noise(16, 12, 3);
    fuzz(&webp_lossless(image.data(), 16, 12, false), 0xC1, 1500);
    fuzz(include_bytes!("fixtures/lossy.webp"), 0xC2, 1500);
    fuzz(include_bytes!("fixtures/lossy_alpha.webp"), 0xC3, 800);
}

#[test]
fn random_bytes_with_valid_magic_never_panic() {
    let mut rng = Rng(0xD1);
    let magics: [&[u8]; 3] = [
        &[0xFF, 0xD8, 0xFF],
        b"\x89PNG\r\n\x1a\n",
        b"RIFF\x40\0\0\0WEBPVP8 ",
    ];
    for _ in 0..3000 {
        let magic = magics[rng.below(3) as usize];
        let mut data = magic.to_vec();
        let len = rng.below(200) as usize;
        data.extend((0..len).map(|_| rng.byte()));
        exercise(&data);
    }
}

#[test]
fn hostile_jpeg_headers_are_rejected() {
    let mut huge = vec![
        0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0x03,
    ];
    huge.extend([1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    huge.extend([0xFF, 0xD9]);
    assert!(decode(&huge).is_err());
    assert!(decode_with_limit(&huge, 100).is_err());
}
