use crate::buffer::{Image, View};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Orientation {
    #[default]
    Normal = 1,
    FlipHorizontal = 2,
    Rotate180 = 3,
    FlipVertical = 4,
    Transpose = 5,
    Rotate90 = 6,
    Transverse = 7,
    Rotate270 = 8,
}

impl Orientation {
    pub const ALL: [Orientation; 8] = [
        Self::Normal,
        Self::FlipHorizontal,
        Self::Rotate180,
        Self::FlipVertical,
        Self::Transpose,
        Self::Rotate90,
        Self::Transverse,
        Self::Rotate270,
    ];

    pub fn from_exif(value: u32) -> Option<Self> {
        match value {
            1..=8 => Some(Self::ALL[value as usize - 1]),
            _ => None,
        }
    }

    pub fn exif_value(self) -> u8 {
        self as u8
    }

    pub fn swaps_dimensions(self) -> bool {
        matches!(
            self,
            Self::Transpose | Self::Rotate90 | Self::Transverse | Self::Rotate270
        )
    }

    pub fn inverse(self) -> Self {
        match self {
            Self::Rotate90 => Self::Rotate270,
            Self::Rotate270 => Self::Rotate90,
            other => other,
        }
    }
}

struct Walk {
    width: usize,
    height: usize,
    base: isize,
    dx: isize,
    dy: isize,
}

fn walk(
    orientation: Orientation,
    width: usize,
    height: usize,
    stride: usize,
    channels: usize,
) -> Walk {
    let (w, h, s, c) = (
        width as isize,
        height as isize,
        stride as isize,
        channels as isize,
    );
    let last_row = (h - 1) * s;
    let last_col = (w - 1) * c;
    let (base, dx, dy) = match orientation {
        Orientation::Normal => (0, c, s),
        Orientation::FlipHorizontal => (last_col, -c, s),
        Orientation::Rotate180 => (last_row + last_col, -c, -s),
        Orientation::FlipVertical => (last_row, c, -s),
        Orientation::Transpose => (0, s, c),
        Orientation::Rotate90 => (last_row, -s, c),
        Orientation::Transverse => (last_row + last_col, -s, -c),
        Orientation::Rotate270 => (last_col, s, -c),
    };
    let (width, height) = if orientation.swaps_dimensions() {
        (height, width)
    } else {
        (width, height)
    };
    Walk {
        width,
        height,
        base,
        dx,
        dy,
    }
}

const TILE: usize = 64;

pub fn orient<const C: usize>(src: View<'_, C>, orientation: Orientation) -> Image<C> {
    let walk = walk(
        orientation,
        src.width() as usize,
        src.height() as usize,
        src.stride(),
        C,
    );
    let data = src.data();
    let out_row = walk.width * C;
    let mut out = vec![0u8; out_row * walk.height];
    let at =
        |x: usize, y: usize| (walk.base + y as isize * walk.dy + x as isize * walk.dx) as usize;
    if walk.dx == C as isize {
        for (y, row) in out.chunks_exact_mut(out_row).enumerate() {
            let start = at(0, y);
            row.copy_from_slice(&data[start..start + out_row]);
        }
    } else if walk.dx == -(C as isize) {
        for (y, row) in out.chunks_exact_mut(out_row).enumerate() {
            for (x, pixel) in row.chunks_exact_mut(C).enumerate() {
                let start = at(x, y);
                pixel.copy_from_slice(&data[start..start + C]);
            }
        }
    } else {
        for tile_y in (0..walk.height).step_by(TILE) {
            let y_end = (tile_y + TILE).min(walk.height);
            for tile_x in (0..walk.width).step_by(TILE) {
                let x_end = (tile_x + TILE).min(walk.width);
                for y in tile_y..y_end {
                    let row = &mut out[y * out_row..][..out_row];
                    for x in tile_x..x_end {
                        let start = at(x, y);
                        row[x * C..][..C].copy_from_slice(&data[start..start + C]);
                    }
                }
            }
        }
    }
    Image::from_parts_unchecked(walk.width as u32, walk.height as u32, out)
}

pub fn rotate90<const C: usize>(src: View<'_, C>) -> Image<C> {
    orient(src, Orientation::Rotate90)
}

pub fn rotate180<const C: usize>(src: View<'_, C>) -> Image<C> {
    orient(src, Orientation::Rotate180)
}

pub fn rotate270<const C: usize>(src: View<'_, C>) -> Image<C> {
    orient(src, Orientation::Rotate270)
}

pub fn flip_horizontal<const C: usize>(src: View<'_, C>) -> Image<C> {
    orient(src, Orientation::FlipHorizontal)
}

pub fn flip_vertical<const C: usize>(src: View<'_, C>) -> Image<C> {
    orient(src, Orientation::FlipVertical)
}

pub fn transpose<const C: usize>(src: View<'_, C>) -> Image<C> {
    orient(src, Orientation::Transpose)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Luma8, Rgb8};

    fn labelled(width: u32, height: u32) -> Luma8 {
        Luma8::from_fn(width, height, |x, y| [(y * width + x) as u8]).unwrap()
    }

    fn reference(src: &Luma8, orientation: Orientation) -> Luma8 {
        let (w, h) = src.dimensions();
        let (ow, oh) = if orientation.swaps_dimensions() {
            (h, w)
        } else {
            (w, h)
        };
        Luma8::from_fn(ow, oh, |x, y| {
            let (sx, sy) = match orientation {
                Orientation::Normal => (x, y),
                Orientation::FlipHorizontal => (w - 1 - x, y),
                Orientation::Rotate180 => (w - 1 - x, h - 1 - y),
                Orientation::FlipVertical => (x, h - 1 - y),
                Orientation::Transpose => (y, x),
                Orientation::Rotate90 => (y, h - 1 - x),
                Orientation::Transverse => (w - 1 - y, h - 1 - x),
                Orientation::Rotate270 => (w - 1 - y, x),
            };
            src.pixel(sx, sy).unwrap()
        })
        .unwrap()
    }

    #[test]
    fn exif_values_round_trip() {
        for (index, orientation) in Orientation::ALL.into_iter().enumerate() {
            assert_eq!(orientation.exif_value() as usize, index + 1);
            assert_eq!(Orientation::from_exif(index as u32 + 1), Some(orientation));
        }
        assert_eq!(Orientation::from_exif(0), None);
        assert_eq!(Orientation::from_exif(9), None);
        assert_eq!(Orientation::from_exif(u32::MAX), None);
        assert_eq!(Orientation::default(), Orientation::Normal);
    }

    #[test]
    fn rotate90_is_clockwise() {
        let src = Luma8::new(3, 2, vec![1, 2, 3, 4, 5, 6]).unwrap();
        let out = rotate90(src.view());
        assert_eq!(out.dimensions(), (2, 3));
        assert_eq!(out.data(), &[4, 1, 5, 2, 6, 3]);
    }

    #[test]
    fn rotate270_is_counter_clockwise() {
        let src = Luma8::new(3, 2, vec![1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(rotate270(src.view()).data(), &[3, 6, 2, 5, 1, 4]);
    }

    #[test]
    fn rotate180_reverses() {
        let src = Luma8::new(3, 2, vec![1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(rotate180(src.view()).data(), &[6, 5, 4, 3, 2, 1]);
    }

    #[test]
    fn flips_mirror_axes() {
        let src = Luma8::new(3, 2, vec![1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(flip_horizontal(src.view()).data(), &[3, 2, 1, 6, 5, 4]);
        assert_eq!(flip_vertical(src.view()).data(), &[4, 5, 6, 1, 2, 3]);
        assert_eq!(transpose(src.view()).data(), &[1, 4, 2, 5, 3, 6]);
        assert_eq!(
            orient(src.view(), Orientation::Transverse).data(),
            &[6, 3, 5, 2, 4, 1]
        );
    }

    #[test]
    fn all_orientations_match_reference_on_asymmetric_shapes() {
        for (w, h) in [(1, 1), (1, 5), (5, 1), (3, 7), (7, 3), (64, 65), (130, 67)] {
            let src = labelled(w, h);
            for orientation in Orientation::ALL {
                assert_eq!(
                    orient(src.view(), orientation),
                    reference(&src, orientation),
                    "{w}x{h} {orientation:?}"
                );
            }
        }
    }

    #[test]
    fn inverse_restores_original() {
        let src = Rgb8::from_fn(37, 23, |x, y| [x as u8, y as u8, (x * y) as u8]).unwrap();
        for orientation in Orientation::ALL {
            let once = orient(src.view(), orientation);
            assert_eq!(
                orient(once.view(), orientation.inverse()),
                src,
                "{orientation:?}"
            );
        }
    }

    #[test]
    fn four_quarter_turns_are_identity() {
        let src = labelled(9, 4);
        let mut image = src.clone();
        for _ in 0..4 {
            image = rotate90(image.view());
        }
        assert_eq!(image, src);
        assert_eq!(rotate90(rotate90(src.view()).view()), rotate180(src.view()));
    }

    #[test]
    fn works_on_strided_crop_views() {
        let full = Rgb8::from_fn(20, 16, |x, y| [x as u8, y as u8, 7]).unwrap();
        let view = full.crop(3, 2, 9, 5).unwrap();
        let owned = view.to_image();
        for orientation in Orientation::ALL {
            assert_eq!(orient(view, orientation), orient(owned.view(), orientation));
        }
    }

    #[test]
    fn swaps_dimensions_matches_output() {
        let src = labelled(5, 3);
        for orientation in Orientation::ALL {
            let out = orient(src.view(), orientation);
            assert_eq!(out.width() == 3, orientation.swaps_dimensions());
        }
    }

    #[test]
    fn rgb_channels_stay_together() {
        let src = Rgb8::new(2, 1, vec![1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(rotate90(src.view()).data(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(flip_horizontal(src.view()).data(), &[4, 5, 6, 1, 2, 3]);
    }

    #[test]
    fn large_image_rotation_preserves_histogram() {
        let src = Luma8::from_fn(1031, 517, |x, y| {
            [(x.wrapping_mul(31) ^ y.wrapping_mul(17)) as u8]
        })
        .unwrap();
        let out = rotate90(src.view());
        let histogram = |data: &[u8]| {
            let mut counts = [0usize; 256];
            data.iter().for_each(|&v| counts[v as usize] += 1);
            counts
        };
        assert_eq!(histogram(src.data()), histogram(out.data()));
        assert_eq!(out.pixel(0, 0), src.pixel(0, 516));
        assert_eq!(out.pixel(516, 1030), src.pixel(1030, 0));
    }
}
