mod common;

use common::*;
use witness_image::*;

#[test]
fn jpeg_round_trip_gradient_within_tolerance() {
    let image = gradient(97, 61);
    for quality in [75u8, 90, 100] {
        let decoded = decode(&encode_jpeg(&image, quality).unwrap()).unwrap();
        assert_eq!(decoded.dimensions(), image.dimensions());
        assert!(
            mean_abs_diff(decoded.data(), image.data()) < 3.0,
            "q{quality}"
        );
    }
}

#[test]
fn jpeg_round_trip_checkerboard_keeps_structure() {
    let image = checkerboard(64, 64, 8);
    let decoded = decode(&encode_jpeg(&image, 95).unwrap()).unwrap();
    for (x, y, expected) in [(4, 4, 255u8), (12, 4, 0), (4, 12, 0), (60, 60, 255)] {
        let value = decoded.pixel(x, y).unwrap()[1];
        assert!(value.abs_diff(expected) < 24, "({x},{y}) {value}");
    }
}

#[test]
fn jpeg_decode_is_deterministic() {
    let jpeg = encode_jpeg(&noise(80, 50, 3), 85).unwrap();
    let first = decode(&jpeg).unwrap();
    for _ in 0..3 {
        assert_eq!(decode(&jpeg).unwrap(), first);
    }
    assert_eq!(pixel_hash(&jpeg).unwrap(), pixel_hash_of(first.view()));
}

#[test]
fn jpeg_odd_sizes_and_subsampling() {
    for (w, h) in [(1, 1), (1, 17), (17, 1), (15, 9), (33, 31)] {
        let image = gradient(w, h);
        for quality in [50u8, 95] {
            let decoded = decode(&encode_jpeg(&image, quality).unwrap()).unwrap();
            assert_eq!(decoded.dimensions(), (w, h));
        }
    }
}

#[test]
fn greyscale_jpeg_expands_to_rgb() {
    let luma = Luma8::from_fn(24, 16, |x, y| [(x * 10 + y) as u8]).unwrap();
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, 95)
        .encode(luma.data(), 24, 16, jpeg_encoder::ColorType::Luma)
        .unwrap();
    let decoded = decode(&jpeg).unwrap();
    assert_eq!(decoded.dimensions(), (24, 16));
    assert!(
        decoded
            .data()
            .chunks_exact(3)
            .all(|p| p[0] == p[1] && p[1] == p[2])
    );
    assert!(mean_abs_diff(to_luma(decoded.view()).data(), luma.data()) < 2.0);
}

#[test]
fn progressive_jpeg_decodes() {
    let image = gradient(40, 30);
    let mut jpeg = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut jpeg, 90);
    encoder.set_progressive(true);
    encoder
        .encode(image.data(), 40, 30, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    let decoded = decode(&jpeg).unwrap();
    assert!(mean_abs_diff(decoded.data(), image.data()) < 3.0);
}

#[test]
fn restart_interval_jpeg_decodes_identically() {
    let image = noise(48, 48, 9);
    let mut plain = Vec::new();
    jpeg_encoder::Encoder::new(&mut plain, 90)
        .encode(image.data(), 48, 48, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    let mut restart = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut restart, 90);
    encoder.set_restart_interval(2);
    encoder
        .encode(image.data(), 48, 48, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    assert_ne!(plain, restart);
    assert_eq!(decode(&plain).unwrap(), decode(&restart).unwrap());
}

#[test]
fn png_rgb8_is_exact() {
    let image = noise(37, 29, 5);
    assert_eq!(decode(&png_rgb(&image)).unwrap(), image);
}

#[test]
fn png_rgba_drops_alpha() {
    let rows: Vec<Vec<u8>> = vec![
        vec![1, 2, 3, 0, 4, 5, 6, 255],
        vec![7, 8, 9, 128, 10, 11, 12, 1],
    ];
    let bytes = png(&PngSpec {
        width: 2,
        height: 2,
        bit_depth: 8,
        colour_type: 6,
        rows: &rows,
        palette: None,
        transparency: None,
    });
    assert_eq!(
        decode(&bytes).unwrap().data(),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    );
}

#[test]
fn png_grey_and_grey_alpha_expand() {
    let rows = vec![vec![0, 100, 255]];
    let bytes = png(&PngSpec {
        width: 3,
        height: 1,
        bit_depth: 8,
        colour_type: 0,
        rows: &rows,
        palette: None,
        transparency: None,
    });
    assert_eq!(
        decode(&bytes).unwrap().data(),
        &[0, 0, 0, 100, 100, 100, 255, 255, 255]
    );
    let rows = vec![vec![50, 0, 60, 255]];
    let bytes = png(&PngSpec {
        width: 2,
        height: 1,
        bit_depth: 8,
        colour_type: 4,
        rows: &rows,
        palette: None,
        transparency: None,
    });
    assert_eq!(decode(&bytes).unwrap().data(), &[50, 50, 50, 60, 60, 60]);
}

#[test]
fn png_sixteen_bit_takes_high_byte() {
    let rows = vec![vec![0x12, 0x34, 0xAB, 0xCD, 0xFF, 0xFF]];
    let bytes = png(&PngSpec {
        width: 1,
        height: 1,
        bit_depth: 16,
        colour_type: 2,
        rows: &rows,
        palette: None,
        transparency: None,
    });
    assert_eq!(decode(&bytes).unwrap().data(), &[0x12, 0xAB, 0xFF]);
}

#[test]
fn png_palette_expands() {
    let palette = [255, 0, 0, 0, 255, 0, 0, 0, 255];
    let rows = vec![vec![0, 1, 2, 1]];
    let bytes = png(&PngSpec {
        width: 4,
        height: 1,
        bit_depth: 8,
        colour_type: 3,
        rows: &rows,
        palette: Some(&palette),
        transparency: None,
    });
    assert_eq!(
        decode(&bytes).unwrap().data(),
        &[255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 255, 0]
    );
    let bytes = png(&PngSpec {
        width: 4,
        height: 1,
        bit_depth: 8,
        colour_type: 3,
        rows: &rows,
        palette: Some(&palette),
        transparency: Some(&[0, 128, 255]),
    });
    assert_eq!(
        decode(&bytes).unwrap().data(),
        &[255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 255, 0]
    );
}

#[test]
fn png_low_bit_depth_grey_scales_to_full_range() {
    let rows = vec![vec![0b1010_0000]];
    let bytes = png(&PngSpec {
        width: 4,
        height: 1,
        bit_depth: 1,
        colour_type: 0,
        rows: &rows,
        palette: None,
        transparency: None,
    });
    let decoded = decode(&bytes).unwrap();
    let greys: Vec<u8> = decoded.data().chunks_exact(3).map(|p| p[0]).collect();
    assert_eq!(greys, vec![255, 0, 255, 0]);
    let rows = vec![vec![0b0001_1011]];
    let bytes = png(&PngSpec {
        width: 4,
        height: 1,
        bit_depth: 2,
        colour_type: 0,
        rows: &rows,
        palette: None,
        transparency: None,
    });
    let greys: Vec<u8> = decode(&bytes)
        .unwrap()
        .data()
        .chunks_exact(3)
        .map(|p| p[0])
        .collect();
    assert_eq!(greys, vec![0, 85, 170, 255]);
}

#[test]
fn png_large_multi_block_round_trip() {
    let image = gradient(300, 260);
    assert_eq!(decode(&png_rgb(&image)).unwrap(), image);
}

#[test]
fn webp_lossless_rgb_is_exact() {
    let image = noise(41, 23, 77);
    let bytes = webp_lossless(image.data(), 41, 23, false);
    assert_eq!(sniff(&bytes), Some(Format::WebP));
    assert_eq!(decode(&bytes).unwrap(), image);
}

#[test]
fn webp_lossless_rgba_drops_alpha() {
    let rgba: Vec<u8> = (0..16u8)
        .flat_map(|i| [i, i * 2, i * 3, 255 - i * 10])
        .collect();
    let bytes = webp_lossless(&rgba, 4, 4, true);
    let expected: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    assert_eq!(decode(&bytes).unwrap().data(), expected.as_slice());
}

#[test]
fn webp_lossy_fixture_decodes_close_to_source() {
    let bytes = include_bytes!("fixtures/lossy.webp");
    let decoded = decode(bytes).unwrap();
    assert_eq!(decoded.dimensions(), (48, 32));
    let source = Rgb8::from_fn(48, 32, |x, y| [(x * 5) as u8, (y * 7) as u8, 128]).unwrap();
    assert!(mean_abs_diff(decoded.data(), source.data()) < 6.0);
    assert_eq!(decode(bytes).unwrap(), decoded);
}

#[test]
fn webp_lossy_alpha_fixture_decodes() {
    let decoded = decode(include_bytes!("fixtures/lossy_alpha.webp")).unwrap();
    assert_eq!(decoded.dimensions(), (48, 32));
    let source = Rgb8::from_fn(48, 32, |x, y| [(x * 5) as u8, (y * 7) as u8, 128]).unwrap();
    assert!(mean_abs_diff(decoded.data(), source.data()) < 6.0);
}

#[test]
fn limits_apply_to_every_format() {
    let image = gradient(20, 10);
    let inputs = [
        encode_jpeg(&image, 90).unwrap(),
        png_rgb(&image),
        webp_lossless(image.data(), 20, 10, false),
    ];
    for bytes in &inputs {
        assert_eq!(decode_with_limit(bytes, 199), Err(ImageError::TooLarge));
        assert!(decode_with_limit(bytes, 200).is_ok());
        assert_eq!(
            decode_oriented_with_limit(bytes, 10),
            Err(ImageError::TooLarge)
        );
    }
}

#[test]
fn png_dimension_bomb_rejected_before_allocation() {
    let rows: Vec<Vec<u8>> = vec![];
    let bytes = png(&PngSpec {
        width: 1 << 20,
        height: 1 << 20,
        bit_depth: 8,
        colour_type: 2,
        rows: &rows,
        palette: None,
        transparency: None,
    });
    assert!(matches!(
        decode(&bytes),
        Err(ImageError::TooLarge) | Err(ImageError::Decode(Format::Png))
    ));
}

#[test]
fn pixel_hash_is_format_independent_for_lossless_inputs() {
    let image = noise(19, 13, 21);
    let from_png = pixel_hash(&png_rgb(&image)).unwrap();
    let from_webp = pixel_hash(&webp_lossless(image.data(), 19, 13, false)).unwrap();
    assert_eq!(from_png, from_webp);
    assert_eq!(from_png, pixel_hash_of(image.view()));
}

#[test]
fn pixel_hash_survives_reencode_cycle_only_when_bytes_identical() {
    let jpeg = encode_jpeg(&gradient(32, 32), 92).unwrap();
    let decoded = decode(&jpeg).unwrap();
    let again = encode_jpeg(&decoded, 92).unwrap();
    assert_eq!(pixel_hash(&jpeg).unwrap(), pixel_hash(&jpeg).unwrap());
    assert_eq!(
        pixel_hash(&again).unwrap(),
        pixel_hash_of(decode(&again).unwrap().view())
    );
}
