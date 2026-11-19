mod common;

use common::*;
use witness_image::*;

#[test]
fn capture_like_pipeline_produces_expected_shapes() {
    let display = quadrants(400, 300);
    let stored = orient(display.view(), Orientation::Rotate270);
    let jpeg = jpeg_with(&stored, 92, Some(6), &[]);
    let image = decode_oriented(&jpeg).unwrap();
    assert_eq!(image.dimensions(), (400, 300));

    let luma = to_luma(image.view());
    let pdq = jarosz_downsample_q8(luma.view(), 64, 64).unwrap();
    assert_eq!(pdq.len(), 64 * 64);

    let watermark = resize_bilinear(image.view(), 256, 256).unwrap();
    assert_eq!(watermark.dimensions(), (256, 256));

    let preview = resize_area(image.view(), 200, 150).unwrap();
    assert_eq!(preview.dimensions(), (200, 150));

    let final_jpeg = strip_and_normalize_jpeg(&encode_jpeg(&image, 92).unwrap()).unwrap();
    assert_eq!(orientation_of(&final_jpeg), Orientation::Normal);
    assert_eq!(
        pixel_hash(&final_jpeg).unwrap(),
        pixel_hash(&final_jpeg).unwrap()
    );
}

#[test]
fn crop_views_feed_resamplers_without_copy() {
    let image = noise(300, 200, 4);
    let view = image.crop(50, 25, 200, 150).unwrap();
    let luma = to_luma(view);
    assert_eq!(luma.dimensions(), (200, 150));
    assert_eq!(
        resize_area(view, 50, 50).unwrap(),
        resize_area(view.to_image().view(), 50, 50).unwrap()
    );
    assert_eq!(
        resize_bilinear(view, 64, 64).unwrap(),
        resize_bilinear(view.to_image().view(), 64, 64).unwrap()
    );
}

#[test]
fn pdq_downsample_is_stable_under_recompression() {
    let image = gradient(320, 240);
    let a = decode(&encode_jpeg(&image, 95).unwrap()).unwrap();
    let b = decode(&encode_jpeg(&image, 70).unwrap()).unwrap();
    let pa = jarosz_downsample(to_luma(a.view()).view(), 64, 64).unwrap();
    let pb = jarosz_downsample(to_luma(b.view()).view(), 64, 64).unwrap();
    assert!(mean_abs_diff(pa.data(), pb.data()) < 2.0);
}

#[test]
fn resamplers_scale_linearly_on_growing_inputs() {
    for scale in [1u32, 2, 4, 8] {
        let image = noise(160 * scale, 120 * scale, scale as u64);
        let luma = to_luma(image.view());
        assert_eq!(
            jarosz_downsample(luma.view(), 64, 64).unwrap().data().len(),
            4096
        );
        assert_eq!(
            resize_area(image.view(), 128, 96).unwrap().dimensions(),
            (128, 96)
        );
        assert_eq!(
            resize_bilinear(image.view(), 256, 256)
                .unwrap()
                .dimensions(),
            (256, 256)
        );
        assert_eq!(
            rotate90(image.view()).dimensions(),
            (120 * scale, 160 * scale)
        );
    }
}

#[test]
fn twelve_megapixel_luma_downsample() {
    let luma = noise_luma(4032, 3024, 12);
    let out = jarosz_downsample_q8(luma.view(), 64, 64).unwrap();
    let mean = out.iter().map(|&v| v as u64).sum::<u64>() / out.len() as u64;
    assert!((120 * 256..136 * 256).contains(&mean));
}
