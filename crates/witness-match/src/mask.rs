#[derive(Clone, Debug, PartialEq, Default)]
pub struct OverlayMask {
    origin_x: f64,
    origin_y: f64,
    block_width: f64,
    block_height: f64,
    cols: u32,
    rows: u32,
    blocks: Vec<bool>,
}

impl OverlayMask {
    pub fn new(
        origin: (f64, f64),
        block_size: (f64, f64),
        cols: u32,
        rows: u32,
        blocks: Vec<bool>,
    ) -> Option<Self> {
        let sized = blocks.len() as u64 == cols as u64 * rows as u64;
        let positive = block_size.0 > 0.0
            && block_size.1 > 0.0
            && block_size.0.is_finite()
            && block_size.1.is_finite();
        let finite = origin.0.is_finite() && origin.1.is_finite();
        (sized && positive && finite).then_some(Self {
            origin_x: origin.0,
            origin_y: origin.1,
            block_width: block_size.0,
            block_height: block_size.1,
            cols,
            rows,
            blocks,
        })
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.cols, self.rows)
    }

    pub fn block(&self, col: u32, row: u32) -> bool {
        col < self.cols
            && row < self.rows
            && self.blocks[row as usize * self.cols as usize + col as usize]
    }

    pub fn masked_blocks(&self) -> usize {
        self.blocks.iter().filter(|&&b| b).count()
    }

    pub fn is_empty(&self) -> bool {
        self.masked_blocks() == 0
    }

    pub fn relative_to(&self, x: f64, y: f64) -> OverlayMask {
        OverlayMask {
            origin_x: self.origin_x - x,
            origin_y: self.origin_y - y,
            ..self.clone()
        }
    }

    pub fn covers(&self, x: f64, y: f64) -> bool {
        if self.blocks.is_empty() {
            return false;
        }
        let (fx, fy) = (
            (x - self.origin_x) / self.block_width,
            (y - self.origin_y) / self.block_height,
        );
        if !(fx >= 0.0 && fy >= 0.0) {
            return false;
        }
        let (col, row) = (fx.floor(), fy.floor());
        col < self.cols as f64
            && row < self.rows as f64
            && self.blocks[row as usize * self.cols as usize + col as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_validates_inputs() {
        assert!(OverlayMask::new((0.0, 0.0), (8.0, 8.0), 2, 2, vec![false; 4]).is_some());
        assert!(OverlayMask::new((0.0, 0.0), (8.0, 8.0), 2, 2, vec![false; 3]).is_none());
        assert!(OverlayMask::new((0.0, 0.0), (0.0, 8.0), 2, 2, vec![false; 4]).is_none());
        assert!(OverlayMask::new((0.0, 0.0), (f64::NAN, 8.0), 2, 2, vec![false; 4]).is_none());
        assert!(OverlayMask::new((f64::INFINITY, 0.0), (8.0, 8.0), 2, 2, vec![false; 4]).is_none());
    }

    #[test]
    fn covers_maps_coordinates_to_blocks() {
        let mask = OverlayMask::new(
            (100.0, 50.0),
            (10.0, 5.0),
            3,
            2,
            vec![false, true, false, false, false, true],
        )
        .unwrap();
        assert!(mask.covers(115.0, 52.0));
        assert!(!mask.covers(105.0, 52.0));
        assert!(mask.covers(129.9, 59.9));
        assert!(!mask.covers(130.0, 59.9));
        assert!(!mask.covers(99.0, 52.0));
        assert!(!mask.covers(f64::NAN, 52.0));
        assert!(!mask.covers(115.0, 60.0));
        assert_eq!(mask.masked_blocks(), 2);
        assert!(mask.block(1, 0) && !mask.block(0, 0) && !mask.block(9, 9));
        assert_eq!(mask.dimensions(), (3, 2));
    }

    #[test]
    fn relative_to_shifts_origin() {
        let mask = OverlayMask::new((100.0, 50.0), (10.0, 5.0), 1, 1, vec![true]).unwrap();
        let shifted = mask.relative_to(100.0, 40.0);
        assert!(shifted.covers(5.0, 12.0));
        assert!(!shifted.covers(105.0, 52.0));
        assert_eq!(shifted.masked_blocks(), 1);
    }

    #[test]
    fn empty_mask_covers_nothing() {
        let mask = OverlayMask::empty();
        assert!(mask.is_empty());
        assert!(!mask.covers(0.0, 0.0));
    }
}
