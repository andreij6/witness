use witness_c2pa::TrustAnchor;
use witness_image::DEFAULT_MAX_PIXELS;
use witness_watermark::Watermarker;

pub const ANDROID_TOP_K: usize = 8;
pub const WEB_TOP_K: usize = 3;
pub const MAX_TOP_K: usize = 16;

#[derive(Clone, Copy)]
pub struct Options<'a> {
    pub anchor: &'a TrustAnchor,
    pub at_s: Option<u64>,
    pub top_k: usize,
    pub watermarker: Option<&'a dyn Watermarker>,
    pub max_pixels: u64,
}

impl<'a> Options<'a> {
    pub fn new(anchor: &'a TrustAnchor, top_k: usize) -> Self {
        Self {
            anchor,
            at_s: None,
            top_k: top_k.clamp(1, MAX_TOP_K),
            watermarker: None,
            max_pixels: DEFAULT_MAX_PIXELS,
        }
    }

    pub fn android(anchor: &'a TrustAnchor) -> Self {
        Self::new(anchor, ANDROID_TOP_K)
    }

    pub fn web(anchor: &'a TrustAnchor) -> Self {
        Self::new(anchor, WEB_TOP_K)
    }

    pub fn with_watermarker(self, watermarker: &'a dyn Watermarker) -> Self {
        Self {
            watermarker: Some(watermarker),
            ..self
        }
    }

    pub fn at(self, at_s: u64) -> Self {
        Self {
            at_s: Some(at_s),
            ..self
        }
    }

    pub(crate) fn k(&self) -> usize {
        self.top_k.clamp(1, MAX_TOP_K)
    }
}
