use crate::error::WitnessError;
use std::sync::Arc;
use witness_core::watermark::Watermarker;

#[derive(uniffi::Object)]
pub struct Trustmark {
    inner: Box<dyn Watermarker + Send + Sync>,
}

impl Trustmark {
    #[cfg(any(test, feature = "trustmark"))]
    pub(crate) fn with(inner: Box<dyn Watermarker + Send + Sync>) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    pub(crate) fn watermarker(&self) -> &dyn Watermarker {
        self.inner.as_ref()
    }
}

#[uniffi::export]
impl Trustmark {
    #[uniffi::constructor]
    pub fn new(encoder: Vec<u8>, decoder: Option<Vec<u8>>) -> Result<Arc<Self>, WitnessError> {
        if encoder.is_empty() {
            return Err(WitnessError::invalid("encoder model is empty"));
        }
        build(&encoder, decoder.as_deref())
    }
}

#[cfg(not(feature = "trustmark"))]
fn build(_: &[u8], _: Option<&[u8]>) -> Result<Arc<Trustmark>, WitnessError> {
    Err(WitnessError::WatermarkUnavailable {
        reason: "built without the trustmark feature".into(),
    })
}

#[cfg(feature = "trustmark")]
fn build(encoder: &[u8], decoder: Option<&[u8]>) -> Result<Arc<Trustmark>, WitnessError> {
    onnx::Onnx::from_memory(encoder, decoder).map(|o| Trustmark::with(Box::new(o)))
}

#[cfg(feature = "trustmark")]
mod onnx {
    use crate::error::WitnessError;
    use ort::session::Session;
    use ort::value::TensorRef;
    use std::sync::{Mutex, PoisonError};
    use witness_core::watermark::{
        MODEL_SIZE, PAYLOAD_BITS, TENSOR_LEN, WatermarkError, Watermarker,
    };

    const INTRA_THREADS: usize = 2;
    const IMAGE_SHAPE: [i64; 4] = [1, 3, MODEL_SIZE as i64, MODEL_SIZE as i64];
    const PAYLOAD_SHAPE: [i64; 2] = [1, PAYLOAD_BITS as i64];

    pub(super) struct Onnx {
        encoder: Mutex<Session>,
        decoder: Option<Mutex<Session>>,
    }

    fn unavailable(e: impl ToString) -> WitnessError {
        WitnessError::WatermarkUnavailable {
            reason: e.to_string(),
        }
    }

    fn session(env: &ort::environment::Environment, model: &[u8]) -> Result<Session, WitnessError> {
        Session::builder(env)
            .map_err(unavailable)?
            .with_intra_threads(INTRA_THREADS)
            .map_err(unavailable)?
            .commit_from_memory(model)
            .map_err(unavailable)
    }

    impl Onnx {
        pub(super) fn from_memory(
            encoder: &[u8],
            decoder: Option<&[u8]>,
        ) -> Result<Self, WitnessError> {
            let env = ort::environment::init()
                .with_name("witness")
                .build()
                .map_err(unavailable)?;
            Ok(Self {
                encoder: Mutex::new(session(&env, encoder)?),
                decoder: decoder
                    .map(|d| session(&env, d).map(Mutex::new))
                    .transpose()?,
            })
        }
    }

    fn inference(_: impl core::fmt::Display) -> WatermarkError {
        WatermarkError::Inference
    }

    impl Watermarker for Onnx {
        fn encode(
            &self,
            cover: &[f32],
            payload: &[f32; PAYLOAD_BITS],
            stego: &mut [f32],
        ) -> Result<(), WatermarkError> {
            if cover.len() != TENSOR_LEN || stego.len() != TENSOR_LEN {
                return Err(WatermarkError::Inference);
            }
            let mut session = self.encoder.lock().unwrap_or_else(PoisonError::into_inner);
            let image = TensorRef::from_array_view((IMAGE_SHAPE, cover)).map_err(inference)?;
            let secret =
                TensorRef::from_array_view((PAYLOAD_SHAPE, &payload[..])).map_err(inference)?;
            let outputs = session
                .run(ort::inputs![image, secret])
                .map_err(inference)?;
            let (_, out) = outputs[0].try_extract_tensor::<f32>().map_err(inference)?;
            if out.len() != TENSOR_LEN {
                return Err(WatermarkError::Inference);
            }
            stego.copy_from_slice(out);
            Ok(())
        }

        fn decode(&self, input: &[f32]) -> Result<[f32; PAYLOAD_BITS], WatermarkError> {
            let decoder = self.decoder.as_ref().ok_or(WatermarkError::Unavailable)?;
            if input.len() != TENSOR_LEN {
                return Err(WatermarkError::Inference);
            }
            let mut session = decoder.lock().unwrap_or_else(PoisonError::into_inner);
            let image = TensorRef::from_array_view((IMAGE_SHAPE, input)).map_err(inference)?;
            let outputs = session.run(ort::inputs![image]).map_err(inference)?;
            let (_, out) = outputs[0].try_extract_tensor::<f32>().map_err(inference)?;
            <[f32; PAYLOAD_BITS]>::try_from(out).map_err(|_| WatermarkError::Inference)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use witness_core::format::CaptureId;
    use witness_core::verdict::testkit::{FixedWatermarker, photo};
    use witness_core::watermark::{NullWatermarker, WatermarkError, encode};

    #[test]
    fn empty_encoder_is_invalid() {
        assert!(matches!(
            Trustmark::new(Vec::new(), None),
            Err(WitnessError::InvalidInput { .. })
        ));
    }

    #[cfg(not(feature = "trustmark"))]
    #[test]
    fn without_feature_models_are_unavailable() {
        assert!(matches!(
            Trustmark::new(vec![1, 2, 3], Some(vec![4])),
            Err(WitnessError::WatermarkUnavailable { .. })
        ));
    }

    #[test]
    fn wrapped_watermarker_is_used() {
        let id = CaptureId::new(77, 0).unwrap();
        let t = Trustmark::with(Box::new(FixedWatermarker::reading(id)));
        let mut img = photo(64, 48, 1);
        assert!(t.watermarker().embed(&mut img, &encode(id)).is_ok());
        let null = Trustmark::with(Box::new(NullWatermarker));
        assert_eq!(
            null.watermarker().embed(&mut img, &encode(id)),
            Err(WatermarkError::Unavailable)
        );
    }
}
