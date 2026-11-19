mod common;

use common::*;
use witness_image::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn digest(bytes: &[u8]) -> String {
    use sha2::Digest;
    hex(&sha2::Sha256::digest(bytes))
}

fn golden_jpeg() -> Vec<u8> {
    encode_jpeg(&noise(64, 48, 2024), 90).unwrap()
}

#[test]
fn golden_values() {
    let jpeg = golden_jpeg();
    let image = decode(&jpeg).unwrap();
    let luma = to_luma(image.view());
    let q8: Vec<u8> = jarosz_downsample_q8(luma.view(), 16, 16)
        .unwrap()
        .iter()
        .flat_map(|v| v.to_be_bytes())
        .collect();
    let actual = [
        ("encode_jpeg", digest(&jpeg)),
        ("pixel_hash_jpeg", hex(&pixel_hash(&jpeg).unwrap())),
        (
            "pixel_hash_lossy_webp",
            hex(&pixel_hash(include_bytes!("fixtures/lossy.webp")).unwrap()),
        ),
        (
            "pixel_hash_lossy_alpha_webp",
            hex(&pixel_hash(include_bytes!("fixtures/lossy_alpha.webp")).unwrap()),
        ),
        ("jarosz_q8", digest(&q8)),
        (
            "bilinear",
            digest(resize_bilinear(image.view(), 37, 23).unwrap().data()),
        ),
        (
            "area",
            digest(resize_area(image.view(), 37, 23).unwrap().data()),
        ),
    ];
    let expected = [
        (
            "encode_jpeg",
            "006e4e55d276d8cc211941be0d19a5e76f2d3b16b2738bde0cda566b539c4aa8",
        ),
        (
            "pixel_hash_jpeg",
            "714054bdc8a5d5f2427a867c6ebbf09f387e0a922a38b64daa080ce2e7080dbd",
        ),
        (
            "pixel_hash_lossy_webp",
            "f55d108be268d88945b851a1c5f65fd76fa26a58c29fe8bf629b2a41db192243",
        ),
        (
            "pixel_hash_lossy_alpha_webp",
            "f55d108be268d88945b851a1c5f65fd76fa26a58c29fe8bf629b2a41db192243",
        ),
        (
            "jarosz_q8",
            "e26b56b8b9c45f6b427373907fde42d6201eb454bb71077e1d4979a44bd7b43b",
        ),
        (
            "bilinear",
            "cd9bbb1a501f8999709c46dea53d006cd35c321e9be096948c92b14a41bd3015",
        ),
        (
            "area",
            "3dbf4c45f224ae7a64fd6b0471668448bdb41bdfb8a9aec19cbab1fdbe3462a0",
        ),
    ];
    for ((name, value), (expected_name, expected_value)) in actual.iter().zip(expected) {
        assert_eq!(*name, expected_name);
        assert_eq!(value, expected_value, "{name}");
    }
}
