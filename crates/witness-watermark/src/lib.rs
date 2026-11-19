mod bch;
mod error;
mod geometry;
mod payload;
mod residual;
mod resize;
mod tensor;
mod watermarker;

#[cfg(test)]
mod testutil;

pub use error::WatermarkError;
pub use geometry::{
    Hypotheses, MAX_HYPOTHESES, Placement, Rect, SENSOR_ASPECT, hypotheses, mean_gray,
    processing_region,
};
pub use payload::{
    DATA_BITS, DEFAULT_DEAD_ZONE, Decoded, ECC_BITS, ERASURE_BUDGET, HardDecision, MAX_CORRECTABLE,
    PAYLOAD_BITS, VERSION_BITS, decode, decode_logits, encode, payload_tensor, soft_decide,
};
pub use residual::{DEFAULT_STRENGTH, RESIDUAL_LIMIT, Residual};
pub use resize::resize_pil_bilinear;
pub use tensor::{MODEL_SIZE, PLANE_LEN, TENSOR_LEN, model_input, normalize};
pub use watermarker::{NullWatermarker, Watermarker};
