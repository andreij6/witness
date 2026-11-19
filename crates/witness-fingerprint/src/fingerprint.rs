use crate::grid::{grid_oriented, is_portrait};
use crate::shape::{ShapeHash, ShapeTag, stored_shapes, working_luma};
use core::fmt;
use witness_format::FingerprintBundle;
use witness_image::RgbView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FingerprintError {
    TooSmall,
    Format(witness_format::Error),
}

impl fmt::Display for FingerprintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooSmall => f.write_str("image too small to fingerprint"),
            Self::Format(error) => write!(f, "fingerprint bundle rejected: {error}"),
        }
    }
}

pub fn fingerprint(rgb: RgbView<'_>) -> Result<FingerprintBundle, FingerprintError> {
    let work = working_luma(rgb);
    let shapes = stored_shapes(work.view());
    let full = shapes
        .first()
        .filter(|s| s.tag == ShapeTag::FULL)
        .ok_or(FingerprintError::TooSmall)?;
    let portrait = is_portrait(rgb.width(), rgb.height());
    let grid = grid_oriented(work.view(), portrait);
    FingerprintBundle::new(
        shapes.iter().map(ShapeHash::to_format).collect(),
        full.pdq.quality,
        *grid.cells(),
        portrait,
    )
    .map_err(FingerprintError::Format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{Grid, grid};
    use crate::shape::stored_shape_tags;
    use crate::testkit::scene;
    use witness_image::Rgb8;

    #[test]
    fn bundle_contains_every_stored_shape_and_grid() {
        let rgb = scene(1600, 1200, 1);
        let bundle = fingerprint(rgb.view()).unwrap();
        let tags: Vec<u8> = bundle.pdq().iter().map(|s| s.tag).collect();
        let expected: Vec<u8> = stored_shape_tags(512, 384)
            .iter()
            .map(|t| t.get())
            .collect();
        assert_eq!(tags, expected);
        assert!(!bundle.portrait());
        assert!(bundle.quality() > 0);
        let work = working_luma(rgb.view());
        assert_eq!(Grid::from_bundle(&bundle), grid(work.view()));
    }

    #[test]
    fn bundle_round_trips_through_canonical_cbor() {
        let bundle = fingerprint(scene(900, 1200, 2).view()).unwrap();
        assert!(bundle.portrait());
        let bytes = witness_format::cbor::to_vec(&bundle);
        let decoded: FingerprintBundle =
            witness_format::cbor::from_slice_canonical(&bytes).unwrap();
        assert_eq!(decoded, bundle);
        assert_eq!(decoded.fp_commit(), bundle.fp_commit());
    }

    #[test]
    fn orientation_follows_original_dimensions_not_rounded_ones() {
        let bundle = fingerprint(scene(2000, 2001, 3).view()).unwrap();
        assert_eq!(
            working_luma(scene(2000, 2001, 3).view()).dimensions(),
            (512, 512)
        );
        assert!(bundle.portrait());
    }

    #[test]
    fn too_small_images_are_rejected() {
        assert_eq!(
            fingerprint(Rgb8::filled(4, 4, [1, 2, 3]).unwrap().view()),
            Err(FingerprintError::TooSmall)
        );
        assert_eq!(
            fingerprint(Rgb8::filled(100, 3, [1, 2, 3]).unwrap().view()),
            Err(FingerprintError::TooSmall)
        );
        assert!(fingerprint(Rgb8::filled(5, 5, [1, 2, 3]).unwrap().view()).is_ok());
        assert!(FingerprintError::TooSmall.to_string().contains("small"));
        assert!(
            FingerprintError::Format(witness_format::Error::Range)
                .to_string()
                .contains("rejected")
        );
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let rgb = scene(777, 555, 4);
        assert_eq!(fingerprint(rgb.view()), fingerprint(rgb.view()));
    }
}
