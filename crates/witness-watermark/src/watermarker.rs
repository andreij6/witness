use crate::error::WatermarkError;
use crate::geometry::{Placement, hypotheses, mean_gray};
use crate::payload::{Decoded, PAYLOAD_BITS, decode_logits, payload_tensor};
use crate::residual::{DEFAULT_STRENGTH, Residual};
use crate::tensor::{TENSOR_LEN, model_input};
use witness_image::{Rgb8, RgbView};

pub trait Watermarker {
    fn encode(
        &self,
        cover: &[f32],
        payload: &[f32; PAYLOAD_BITS],
        stego: &mut [f32],
    ) -> Result<(), WatermarkError>;

    fn decode(&self, input: &[f32]) -> Result<[f32; PAYLOAD_BITS], WatermarkError>;

    fn strength(&self) -> f32 {
        DEFAULT_STRENGTH
    }

    fn embed(
        &self,
        image: &mut Rgb8,
        payload: &[bool; PAYLOAD_BITS],
    ) -> Result<(), WatermarkError> {
        let (width, height) = image.dimensions();
        let mut cover = vec![0.0f32; TENSOR_LEN];
        model_input(
            image.view(),
            Placement::identity(width, height),
            0,
            &mut cover,
        )?;
        let mut stego = vec![0.0f32; TENSOR_LEN];
        self.encode(&cover, &payload_tensor(payload), &mut stego)?;
        Residual::from_model(stego, &cover, self.strength())?.apply(image);
        Ok(())
    }

    fn detect(&self, image: RgbView<'_>) -> Result<[f32; PAYLOAD_BITS], WatermarkError> {
        let mut input = vec![0.0f32; TENSOR_LEN];
        model_input(
            image,
            Placement::identity(image.width(), image.height()),
            0,
            &mut input,
        )?;
        self.decode(&input)
    }

    fn read(
        &self,
        image: RgbView<'_>,
        sensor: (u32, u32),
        dead_zone: f32,
    ) -> Result<Option<Decoded>, WatermarkError> {
        let (width, height) = image.dimensions();
        let mut input = vec![0.0f32; TENSOR_LEN];
        let mut gray = None;
        for placement in hypotheses(width, height, sensor).iter() {
            let fill = if placement.is_identity_for(width, height) {
                0
            } else {
                *gray.get_or_insert_with(|| mean_gray(image))
            };
            model_input(image, *placement, fill, &mut input)?;
            if let Some(decoded) = decode_logits(&self.decode(&input)?, dead_zone) {
                return Ok(Some(decoded));
            }
        }
        Ok(None)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NullWatermarker;

impl Watermarker for NullWatermarker {
    fn encode(
        &self,
        _: &[f32],
        _: &[f32; PAYLOAD_BITS],
        _: &mut [f32],
    ) -> Result<(), WatermarkError> {
        Err(WatermarkError::Unavailable)
    }

    fn decode(&self, _: &[f32]) -> Result<[f32; PAYLOAD_BITS], WatermarkError> {
        Err(WatermarkError::Unavailable)
    }

    fn embed(&self, _: &mut Rgb8, _: &[bool; PAYLOAD_BITS]) -> Result<(), WatermarkError> {
        Err(WatermarkError::Unavailable)
    }

    fn detect(&self, _: RgbView<'_>) -> Result<[f32; PAYLOAD_BITS], WatermarkError> {
        Err(WatermarkError::Unavailable)
    }

    fn read(
        &self,
        _: RgbView<'_>,
        _: (u32, u32),
        _: f32,
    ) -> Result<Option<Decoded>, WatermarkError> {
        Err(WatermarkError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::SENSOR_ASPECT;
    use crate::payload::{DEFAULT_DEAD_ZONE, encode};
    use core::cell::{Cell, RefCell};
    use witness_format::CaptureId;

    struct Recording {
        calls: Cell<usize>,
        inputs: RefCell<Vec<Vec<f32>>>,
        logits: [f32; PAYLOAD_BITS],
        stego_offset: f32,
    }

    impl Recording {
        fn new(logits: [f32; PAYLOAD_BITS], stego_offset: f32) -> Self {
            Self {
                calls: Cell::new(0),
                inputs: RefCell::new(Vec::new()),
                logits,
                stego_offset,
            }
        }
    }

    impl Watermarker for Recording {
        fn encode(
            &self,
            cover: &[f32],
            _: &[f32; PAYLOAD_BITS],
            stego: &mut [f32],
        ) -> Result<(), WatermarkError> {
            for (s, c) in stego.iter_mut().zip(cover) {
                *s = c + self.stego_offset;
            }
            Ok(())
        }

        fn decode(&self, input: &[f32]) -> Result<[f32; PAYLOAD_BITS], WatermarkError> {
            self.calls.set(self.calls.get() + 1);
            self.inputs.borrow_mut().push(input.to_vec());
            Ok(self.logits)
        }
    }

    struct Failing;

    impl Watermarker for Failing {
        fn encode(
            &self,
            _: &[f32],
            _: &[f32; PAYLOAD_BITS],
            _: &mut [f32],
        ) -> Result<(), WatermarkError> {
            Err(WatermarkError::Inference)
        }

        fn decode(&self, _: &[f32]) -> Result<[f32; PAYLOAD_BITS], WatermarkError> {
            Err(WatermarkError::Inference)
        }
    }

    #[test]
    fn null_watermarker_is_unavailable_everywhere() {
        let mut img = Rgb8::filled(10, 10, [1; 3]).unwrap();
        let w: &dyn Watermarker = &NullWatermarker;
        let unavailable = Err(WatermarkError::Unavailable);
        assert_eq!(w.embed(&mut img, &[false; PAYLOAD_BITS]), unavailable);
        assert_eq!(w.detect(img.view()), Err(WatermarkError::Unavailable));
        assert_eq!(
            w.read(img.view(), SENSOR_ASPECT, 0.25),
            Err(WatermarkError::Unavailable)
        );
        assert_eq!(w.decode(&[]), Err(WatermarkError::Unavailable));
        assert_eq!(w.encode(&[], &[0.0; PAYLOAD_BITS], &mut []), unavailable);
        assert_eq!(img, Rgb8::filled(10, 10, [1; 3]).unwrap());
    }

    #[test]
    fn identity_model_leaves_image_unchanged() {
        let original = Rgb8::from_fn(320, 240, |x, y| [x as u8, y as u8, (x ^ y) as u8]).unwrap();
        let mut img = original.clone();
        Recording::new([0.0; PAYLOAD_BITS], 0.0)
            .embed(&mut img, &[true; PAYLOAD_BITS])
            .unwrap();
        assert_eq!(img, original);
    }

    #[test]
    fn offset_model_brightens_by_strength() {
        let mut img = Rgb8::filled(300, 200, [100; 3]).unwrap();
        Recording::new([0.0; PAYLOAD_BITS], 0.1)
            .embed(&mut img, &[true; PAYLOAD_BITS])
            .unwrap();
        assert!(img.data().iter().all(|&v| v == 113));
    }

    #[test]
    fn failing_model_propagates_and_preserves_image() {
        let original = Rgb8::filled(30, 20, [9; 3]).unwrap();
        let mut img = original.clone();
        assert_eq!(
            Failing.embed(&mut img, &[false; PAYLOAD_BITS]),
            Err(WatermarkError::Inference)
        );
        assert_eq!(img, original);
        assert_eq!(
            Failing.read(img.view(), SENSOR_ASPECT, 0.25),
            Err(WatermarkError::Inference)
        );
    }

    #[test]
    fn detect_feeds_identity_model_input() {
        let img = Rgb8::from_fn(400, 300, |x, y| [x as u8, y as u8, 7]).unwrap();
        let rec = Recording::new([1.0; PAYLOAD_BITS], 0.0);
        assert_eq!(rec.detect(img.view()).unwrap(), [1.0; PAYLOAD_BITS]);
        let mut expected = vec![0.0; TENSOR_LEN];
        model_input(img.view(), Placement::identity(400, 300), 0, &mut expected).unwrap();
        assert_eq!(rec.inputs.borrow()[0], expected);
    }

    #[test]
    fn read_stops_at_first_decodable_hypothesis() {
        let cid = CaptureId::from_u64(0x0123_4567_89ab_cdef).unwrap();
        let logits = encode(cid).map(|b| if b { 4.0 } else { -4.0 });
        let rec = Recording::new(logits, 0.0);
        let img = Rgb8::filled(300, 300, [50; 3]).unwrap();
        let got = rec
            .read(img.view(), SENSOR_ASPECT, DEFAULT_DEAD_ZONE)
            .unwrap()
            .unwrap();
        assert_eq!(got.capture_id, cid);
        assert_eq!(rec.calls.get(), 1);
    }

    #[test]
    fn read_tries_at_most_four_hypotheses() {
        let rec = Recording::new([0.0; PAYLOAD_BITS], 0.0);
        let img = Rgb8::filled(300, 300, [50; 3]).unwrap();
        assert_eq!(
            rec.read(img.view(), SENSOR_ASPECT, DEFAULT_DEAD_ZONE),
            Ok(None)
        );
        assert_eq!(rec.calls.get(), 4);
        let same_aspect = Recording::new([0.0; PAYLOAD_BITS], 0.0);
        let img = Rgb8::filled(400, 300, [50; 3]).unwrap();
        assert_eq!(
            same_aspect.read(img.view(), SENSOR_ASPECT, DEFAULT_DEAD_ZONE),
            Ok(None)
        );
        assert_eq!(same_aspect.calls.get(), 1);
    }

    #[test]
    fn read_pads_with_mean_gray() {
        let rec = Recording::new([0.0; PAYLOAD_BITS], 0.0);
        let img = Rgb8::filled(300, 300, [80; 3]).unwrap();
        rec.read(img.view(), SENSOR_ASPECT, DEFAULT_DEAD_ZONE)
            .unwrap();
        let level = crate::tensor::normalize(80);
        assert!(
            rec.inputs
                .borrow()
                .iter()
                .all(|t| t.iter().all(|&v| v == level))
        );
    }

    #[test]
    fn default_strength_is_trustmark_default() {
        assert_eq!(NullWatermarker.strength(), DEFAULT_STRENGTH);
    }
}
