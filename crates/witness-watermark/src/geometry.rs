use crate::resize::RowSource;
use core::ops::Deref;
use witness_image::{ImageError, RgbView};

pub const SENSOR_ASPECT: (u32, u32) = (4, 3);
pub const MAX_HYPOTHESES: usize = 4;
const CHANNELS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn processing_region(width: u32, height: u32) -> Rect {
    let (w, h) = (width as u64, height as u64);
    if h > 2 * w || w > 2 * h {
        let side = width.min(height);
        Rect {
            x: (width - side) / 2,
            y: (height - side) / 2,
            width: side,
            height: side,
        }
    } else {
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub x: u32,
    pub y: u32,
}

impl Placement {
    pub fn identity(width: u32, height: u32) -> Self {
        Self {
            canvas_width: width,
            canvas_height: height,
            x: 0,
            y: 0,
        }
    }

    pub fn is_identity_for(&self, width: u32, height: u32) -> bool {
        *self == Self::identity(width, height)
    }

    fn validate(&self, width: u32, height: u32) -> Result<(), ImageError> {
        if self.canvas_width == 0 || self.canvas_height == 0 || width == 0 || height == 0 {
            return Err(ImageError::InvalidDimensions);
        }
        let fits = |offset: u32, len: u32, bound: u32| {
            offset.checked_add(len).is_some_and(|end| end <= bound)
        };
        if fits(self.x, width, self.canvas_width) && fits(self.y, height, self.canvas_height) {
            Ok(())
        } else {
            Err(ImageError::OutOfBounds)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hypotheses {
    items: [Placement; MAX_HYPOTHESES],
    len: usize,
}

impl Hypotheses {
    fn push(&mut self, placement: Placement) {
        if !self.items[..self.len].contains(&placement) {
            self.items[self.len] = placement;
            self.len += 1;
        }
    }
}

impl Deref for Hypotheses {
    type Target = [Placement];

    fn deref(&self) -> &[Placement] {
        &self.items[..self.len]
    }
}

pub fn hypotheses(width: u32, height: u32, sensor: (u32, u32)) -> Hypotheses {
    let identity = Placement::identity(width, height);
    let mut out = Hypotheses {
        items: [identity; MAX_HYPOTHESES],
        len: 1,
    };
    let (long, short) = (sensor.0.max(sensor.1) as u64, sensor.0.min(sensor.1) as u64);
    if short == 0 || width == 0 || height == 0 {
        return out;
    }
    let (aw, ah) = if width >= height {
        (long, short)
    } else {
        (short, long)
    };
    let (w, h) = (width as u64, height as u64);
    let (canvas_w, canvas_h) = match (w * ah).cmp(&(h * aw)) {
        core::cmp::Ordering::Equal => return out,
        core::cmp::Ordering::Less => ((h * aw + ah / 2) / ah, h),
        core::cmp::Ordering::Greater => (w, (w * ah + aw / 2) / aw),
    };
    let (Ok(cw), Ok(ch)) = (u32::try_from(canvas_w), u32::try_from(canvas_h)) else {
        return out;
    };
    let (slack_x, slack_y) = (cw - width, ch - height);
    for (x, y) in [(slack_x / 2, slack_y / 2), (0, 0), (slack_x, slack_y)] {
        out.push(Placement {
            canvas_width: cw,
            canvas_height: ch,
            x,
            y,
        });
    }
    out
}

pub fn mean_gray(image: RgbView<'_>) -> u8 {
    let n = image.width() as u64 * image.height() as u64 * CHANNELS as u64;
    let sum: u64 = image
        .rows()
        .map(|row| row.iter().map(|&v| v as u64).sum::<u64>())
        .sum();
    ((sum + n / 2) / n) as u8
}

pub(crate) struct CanvasRows<'a> {
    copy: RgbView<'a>,
    window: Rect,
    copy_y: u32,
    columns: Option<(usize, usize, usize)>,
    full_width: bool,
    gray_row: Vec<u8>,
    mixed_row: Vec<u8>,
}

impl<'a> CanvasRows<'a> {
    pub(crate) fn new(
        copy: RgbView<'a>,
        placement: Placement,
        gray: u8,
    ) -> Result<Self, ImageError> {
        placement.validate(copy.width(), copy.height())?;
        let window = processing_region(placement.canvas_width, placement.canvas_height);
        let start = placement.x.max(window.x);
        let end = (placement.x + copy.width()).min(window.x + window.width);
        let columns = (start < end).then(|| {
            (
                (start - window.x) as usize,
                (start - placement.x) as usize,
                (end - start) as usize,
            )
        });
        let full_width = columns.is_some_and(|(_, _, n)| n == window.width as usize);
        let covers_rows =
            placement.y <= window.y && placement.y + copy.height() >= window.y + window.height;
        let row_len = window.width as usize * CHANNELS;
        let (gray_row, mixed_row) = if full_width && covers_rows {
            (Vec::new(), Vec::new())
        } else {
            (vec![gray; row_len], vec![gray; row_len])
        };
        Ok(Self {
            copy,
            window,
            copy_y: placement.y,
            columns,
            full_width,
            gray_row,
            mixed_row,
        })
    }
}

impl RowSource for CanvasRows<'_> {
    fn width(&self) -> u32 {
        self.window.width
    }

    fn height(&self) -> u32 {
        self.window.height
    }

    fn row(&mut self, y: u32) -> &[u8] {
        let canvas_y = self.window.y + y;
        let Some((dst, src, n)) = self.columns else {
            return &self.gray_row;
        };
        if canvas_y < self.copy_y || canvas_y >= self.copy_y + self.copy.height() {
            return &self.gray_row;
        }
        let segment = &self.copy.row(canvas_y - self.copy_y)[src * CHANNELS..][..n * CHANNELS];
        if self.full_width {
            return segment;
        }
        self.mixed_row[dst * CHANNELS..][..n * CHANNELS].copy_from_slice(segment);
        &self.mixed_row
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::Rng;
    use witness_image::Rgb8;

    fn materialize(copy: &Rgb8, placement: Placement, gray: u8) -> Rgb8 {
        let window = processing_region(placement.canvas_width, placement.canvas_height);
        Rgb8::from_fn(window.width, window.height, |x, y| {
            let (cx, cy) = (x + window.x, y + window.y);
            if cx >= placement.x && cy >= placement.y {
                copy.pixel(cx - placement.x, cy - placement.y)
                    .unwrap_or([gray; 3])
            } else {
                [gray; 3]
            }
        })
        .unwrap()
    }

    fn collect(rows: &mut CanvasRows<'_>) -> Vec<u8> {
        (0..rows.height())
            .flat_map(|y| rows.row(y).to_vec())
            .collect()
    }

    #[test]
    fn region_matches_trustmark_reference_cases() {
        assert_eq!(
            processing_region(100, 110),
            Rect {
                x: 0,
                y: 0,
                width: 100,
                height: 110
            }
        );
        assert_eq!(
            processing_region(10, 100),
            Rect {
                x: 0,
                y: 45,
                width: 10,
                height: 10
            }
        );
        assert_eq!(
            processing_region(101, 10),
            Rect {
                x: 45,
                y: 0,
                width: 10,
                height: 10
            }
        );
    }

    #[test]
    fn region_boundary_is_strictly_more_than_double() {
        assert_eq!(
            processing_region(200, 100),
            Rect {
                x: 0,
                y: 0,
                width: 200,
                height: 100
            }
        );
        assert_eq!(
            processing_region(201, 100),
            Rect {
                x: 50,
                y: 0,
                width: 100,
                height: 100
            }
        );
        assert_eq!(
            processing_region(100, 200),
            Rect {
                x: 0,
                y: 0,
                width: 100,
                height: 200
            }
        );
        assert_eq!(
            processing_region(100, 201),
            Rect {
                x: 0,
                y: 50,
                width: 100,
                height: 100
            }
        );
    }

    #[test]
    fn region_survives_huge_dimensions() {
        assert_eq!(
            processing_region(u32::MAX, 1),
            Rect {
                x: (u32::MAX - 1) / 2,
                y: 0,
                width: 1,
                height: 1
            }
        );
        assert_eq!(processing_region(u32::MAX, u32::MAX).width, u32::MAX);
    }

    #[test]
    fn region_always_inside_image() {
        let mut rng = Rng::new(40);
        for _ in 0..2000 {
            let (w, h) = (1 + rng.below(10_000) as u32, 1 + rng.below(10_000) as u32);
            let r = processing_region(w, h);
            assert!(r.x + r.width <= w && r.y + r.height <= h);
            assert!(r.width as u64 <= 2 * r.height as u64 && r.height as u64 <= 2 * r.width as u64);
        }
    }

    #[test]
    fn same_aspect_yields_identity_only() {
        assert_eq!(
            &*hypotheses(400, 300, SENSOR_ASPECT),
            &[Placement::identity(400, 300)]
        );
        assert_eq!(
            &*hypotheses(300, 400, SENSOR_ASPECT),
            &[Placement::identity(300, 400)]
        );
    }

    #[test]
    fn square_crop_of_landscape_places_horizontally() {
        let h = hypotheses(300, 300, SENSOR_ASPECT);
        assert_eq!(h.len(), 4);
        assert_eq!(h[0], Placement::identity(300, 300));
        let canvas = |x| Placement {
            canvas_width: 400,
            canvas_height: 300,
            x,
            y: 0,
        };
        assert_eq!(&h[1..], &[canvas(50), canvas(0), canvas(100)]);
    }

    #[test]
    fn widescreen_crop_places_vertically() {
        let h = hypotheses(1920, 1080, SENSOR_ASPECT);
        let canvas = |y| Placement {
            canvas_width: 1920,
            canvas_height: 1440,
            x: 0,
            y,
        };
        assert_eq!(&h[1..], &[canvas(180), canvas(0), canvas(360)]);
    }

    #[test]
    fn portrait_story_crop_uses_portrait_sensor() {
        let h = hypotheses(1080, 1920, SENSOR_ASPECT);
        let canvas = |x| Placement {
            canvas_width: 1440,
            canvas_height: 1920,
            x,
            y: 0,
        };
        assert_eq!(&h[1..], &[canvas(180), canvas(0), canvas(360)]);
    }

    #[test]
    fn tiny_slack_dedupes() {
        let h = hypotheses(399, 300, SENSOR_ASPECT);
        let canvas = |x| Placement {
            canvas_width: 400,
            canvas_height: 300,
            x,
            y: 0,
        };
        assert_eq!(&h[1..], &[canvas(0), canvas(1)]);
        assert_eq!(hypotheses(4000, 2999, SENSOR_ASPECT).len(), 3);
        assert_eq!(hypotheses(4000, 3000, SENSOR_ASPECT).len(), 1);
    }

    #[test]
    fn degenerate_sensor_gives_identity() {
        assert_eq!(hypotheses(10, 20, (0, 3)).len(), 1);
        assert_eq!(hypotheses(10, 20, (0, 0)).len(), 1);
        assert_eq!(hypotheses(u32::MAX, 1, (u32::MAX, 1)).len(), 1);
        assert_eq!(hypotheses(1, u32::MAX, (u32::MAX, 1)).len(), 1);
        assert_eq!(hypotheses(u32::MAX, u32::MAX / 2, (1000, 1)).len(), 1);
    }

    #[test]
    fn hypotheses_are_bounded_and_contain_the_copy() {
        let mut rng = Rng::new(41);
        for _ in 0..3000 {
            let (w, h) = (1 + rng.below(8000) as u32, 1 + rng.below(8000) as u32);
            let sensor = (1 + rng.below(20) as u32, 1 + rng.below(20) as u32);
            let hs = hypotheses(w, h, sensor);
            assert!(!hs.is_empty() && hs.len() <= MAX_HYPOTHESES);
            assert_eq!(hs[0], Placement::identity(w, h));
            for p in hs.iter() {
                assert_eq!(p.validate(w, h), Ok(()));
                assert!(p.canvas_width == w || p.canvas_height == h);
            }
            for (i, a) in hs.iter().enumerate() {
                assert!(hs[i + 1..].iter().all(|b| b != a));
            }
        }
    }

    #[test]
    fn mean_gray_rounds() {
        let img = Rgb8::new(2, 1, vec![0, 0, 0, 1, 1, 2]).unwrap();
        assert_eq!(mean_gray(img.view()), 1);
        assert_eq!(mean_gray(Rgb8::filled(3, 3, [255; 3]).unwrap().view()), 255);
        assert_eq!(mean_gray(Rgb8::filled(3, 3, [0, 0, 1]).unwrap().view()), 0);
        let big = Rgb8::from_fn(64, 64, |x, _| [x as u8 * 4; 3]).unwrap();
        assert_eq!(mean_gray(big.crop(0, 0, 32, 64).unwrap()), 62);
    }

    #[test]
    fn identity_rows_are_zero_copy() {
        let mut rng = Rng::new(42);
        let img = rng.image(50, 40);
        let mut rows = CanvasRows::new(img.view(), Placement::identity(50, 40), 7).unwrap();
        assert!(rows.gray_row.is_empty() && rows.mixed_row.is_empty());
        for y in 0..40 {
            assert!(core::ptr::eq(rows.row(y).as_ptr(), img.row(y).as_ptr()));
        }
    }

    #[test]
    fn identity_of_extreme_aspect_crops_center_square() {
        let mut rng = Rng::new(43);
        let img = rng.image(90, 20);
        let mut rows = CanvasRows::new(img.view(), Placement::identity(90, 20), 0).unwrap();
        assert_eq!((rows.width(), rows.height()), (20, 20));
        assert_eq!(
            collect(&mut rows),
            img.crop(35, 0, 20, 20).unwrap().to_image().into_data()
        );
    }

    #[test]
    fn canvas_rows_match_materialized_canvas() {
        let mut rng = Rng::new(44);
        for _ in 0..300 {
            let (w, h) = (1 + rng.below(40) as u32, 1 + rng.below(40) as u32);
            let img = rng.image(w, h);
            let (cw, ch) = (w + rng.below(60) as u32, h + rng.below(60) as u32);
            let p = Placement {
                canvas_width: cw,
                canvas_height: ch,
                x: rng.below((cw - w + 1) as u64) as u32,
                y: rng.below((ch - h + 1) as u64) as u32,
            };
            let gray = rng.next() as u8;
            let mut rows = CanvasRows::new(img.view(), p, gray).unwrap();
            assert_eq!(
                collect(&mut rows),
                materialize(&img, p, gray).into_data(),
                "{w}x{h} in {p:?}"
            );
        }
    }

    #[test]
    fn canvas_rows_validate_placement() {
        let img = Rgb8::filled(10, 10, [1; 3]).unwrap();
        let p = |cw, ch, x, y| Placement {
            canvas_width: cw,
            canvas_height: ch,
            x,
            y,
        };
        assert_eq!(
            CanvasRows::new(img.view(), p(9, 10, 0, 0), 0).err(),
            Some(ImageError::OutOfBounds)
        );
        assert_eq!(
            CanvasRows::new(img.view(), p(20, 10, 11, 0), 0).err(),
            Some(ImageError::OutOfBounds)
        );
        assert_eq!(
            CanvasRows::new(img.view(), p(20, 10, u32::MAX, 0), 0).err(),
            Some(ImageError::OutOfBounds)
        );
        assert_eq!(
            CanvasRows::new(img.view(), p(0, 10, 0, 0), 0).err(),
            Some(ImageError::InvalidDimensions)
        );
        assert!(CanvasRows::new(img.view(), p(20, 10, 10, 0), 0).is_ok());
    }

    #[test]
    fn copy_outside_processing_window_yields_gray() {
        let img = Rgb8::filled(10, 10, [200; 3]).unwrap();
        let p = Placement {
            canvas_width: 100,
            canvas_height: 10,
            x: 0,
            y: 0,
        };
        let mut rows = CanvasRows::new(img.view(), p, 33).unwrap();
        assert!(collect(&mut rows).iter().all(|&v| v == 33));
    }

    #[test]
    fn identity_detection() {
        assert!(Placement::identity(3, 4).is_identity_for(3, 4));
        assert!(!Placement::identity(3, 4).is_identity_for(4, 3));
    }
}
