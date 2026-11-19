use ic_stable_structures::Storable;
use ic_stable_structures::storable::Bound;
use std::borrow::Cow;
use witness_fingerprint::{HASH_BYTES, MAX_STORED_SHAPES, MIN_INDEX_QUALITY, PdqHash};
use witness_format::FingerprintBundle;

pub const ROW_SHAPES: usize = MAX_STORED_SHAPES;
const HEADER: usize = 3;
const TAGS_AT: usize = HEADER;
const HASHES_AT: usize = TAGS_AT + ROW_SHAPES;
pub const ROW_BYTES: usize = HASHES_AT + ROW_SHAPES * HASH_BYTES;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Row([u8; ROW_BYTES]);

impl Row {
    pub const EMPTY: Self = Self([0; ROW_BYTES]);

    pub fn from_bundle(bundle: &FingerprintBundle) -> Option<Self> {
        let shapes = bundle.pdq();
        if shapes.len() > ROW_SHAPES {
            return None;
        }
        let mut bytes = [0u8; ROW_BYTES];
        bytes[0] = shapes.len() as u8;
        bytes[1] = bundle.quality();
        bytes[2] = bundle.portrait() as u8;
        for (i, shape) in shapes.iter().enumerate() {
            bytes[TAGS_AT + i] = shape.tag;
            bytes[HASHES_AT + i * HASH_BYTES..][..HASH_BYTES].copy_from_slice(&shape.hash);
        }
        Some(Self(bytes))
    }

    pub fn len(&self) -> usize {
        (self.0[0] as usize).min(ROW_SHAPES)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn quality(&self) -> u8 {
        self.0[1]
    }

    pub fn portrait(&self) -> bool {
        self.0[2] == 1
    }

    pub fn indexed(&self) -> bool {
        !self.is_empty() && self.quality() >= MIN_INDEX_QUALITY
    }

    pub fn tag(&self, i: usize) -> u8 {
        self.0[TAGS_AT + i]
    }

    pub fn hash(&self, i: usize) -> PdqHash {
        let mut out = [0u8; HASH_BYTES];
        out.copy_from_slice(&self.0[HASHES_AT + i * HASH_BYTES..][..HASH_BYTES]);
        PdqHash(out)
    }

    pub fn shapes(&self) -> impl Iterator<Item = (u8, PdqHash)> + '_ {
        (0..self.len()).map(|i| (self.tag(i), self.hash(i)))
    }

    pub fn min_distance(&self, query: &PdqHash) -> Option<u32> {
        (0..self.len()).map(|i| self.hash(i).distance(query)).min()
    }
}

impl Storable for Row {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(&self.0)
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let mut out = [0u8; ROW_BYTES];
        out.copy_from_slice(&bytes);
        Self(out)
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: ROW_BYTES as u32,
        is_fixed_size: true,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{bundle, hash};
    use witness_format::fingerprint::GRID_CELLS;

    #[test]
    fn layout_is_fixed_and_compact() {
        assert_eq!(ROW_BYTES, 3 + 19 + 19 * 32);
        assert!(Row::BOUND.is_fixed_size());
        assert_eq!(Row::EMPTY.to_bytes().len(), ROW_BYTES);
    }

    #[test]
    fn from_bundle_keeps_every_shape_in_tag_order() {
        let b = bundle(7, 19, 80, true);
        let row = Row::from_bundle(&b).unwrap();
        assert_eq!(row.len(), 19);
        assert_eq!(row.quality(), 80);
        assert!(row.portrait());
        let shapes: Vec<_> = row.shapes().collect();
        for (stored, original) in shapes.iter().zip(b.pdq()) {
            assert_eq!(stored.0, original.tag);
            assert_eq!(stored.1.0, original.hash);
        }
    }

    #[test]
    fn empty_bundle_gives_empty_unindexed_row() {
        let b = bundle(1, 0, 90, false);
        let row = Row::from_bundle(&b).unwrap();
        assert!(row.is_empty());
        assert!(!row.indexed());
        assert!(!row.portrait());
        assert_eq!(row.min_distance(&hash(1)), None);
        assert_eq!(row.shapes().count(), 0);
    }

    #[test]
    fn too_many_shapes_rejected() {
        assert!(Row::from_bundle(&bundle(2, ROW_SHAPES, 90, false)).is_some());
        assert!(Row::from_bundle(&bundle(2, ROW_SHAPES + 1, 90, false)).is_none());
        assert!(Row::from_bundle(&bundle(2, 32, 90, false)).is_none());
    }

    #[test]
    fn indexed_respects_quality_floor() {
        assert!(
            !Row::from_bundle(&bundle(3, 4, MIN_INDEX_QUALITY - 1, false))
                .unwrap()
                .indexed()
        );
        assert!(
            Row::from_bundle(&bundle(3, 4, MIN_INDEX_QUALITY, false))
                .unwrap()
                .indexed()
        );
        assert!(
            Row::from_bundle(&bundle(3, 4, 100, false))
                .unwrap()
                .indexed()
        );
        assert!(!Row::EMPTY.indexed());
    }

    #[test]
    fn min_distance_is_minimum_over_shapes() {
        let b = bundle(4, 5, 90, false);
        let row = Row::from_bundle(&b).unwrap();
        for shape in b.pdq() {
            assert_eq!(row.min_distance(&PdqHash(shape.hash)), Some(0));
        }
        let q = hash(999);
        let naive = b.pdq().iter().map(|s| PdqHash(s.hash).distance(&q)).min();
        assert_eq!(row.min_distance(&q), naive);
    }

    #[test]
    fn storable_round_trip() {
        let row = Row::from_bundle(&bundle(5, 11, 66, true)).unwrap();
        let back = Row::from_bytes(row.to_bytes());
        assert_eq!(back, row);
        assert_eq!(Row::from_bytes(Cow::Owned(row.clone().into_bytes())), row);
    }

    #[test]
    #[should_panic]
    fn storable_rejects_wrong_length() {
        Row::from_bytes(Cow::Owned(vec![0; ROW_BYTES - 1]));
    }

    #[test]
    fn corrupt_count_is_clamped() {
        let mut bytes = [0u8; ROW_BYTES];
        bytes[0] = 0xFF;
        let row = Row::from_bytes(Cow::Owned(bytes.to_vec()));
        assert_eq!(row.len(), ROW_SHAPES);
        assert_eq!(row.shapes().count(), ROW_SHAPES);
    }

    #[test]
    fn deterministic_encoding() {
        let a = Row::from_bundle(&bundle(6, 9, 70, false)).unwrap();
        let b = Row::from_bundle(&bundle(6, 9, 70, false)).unwrap();
        assert_eq!(a.to_bytes(), b.to_bytes());
        assert_eq!(GRID_CELLS, 192);
    }
}
