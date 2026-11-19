use crate::error::ImageError;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Image<const C: usize> {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

pub type Rgb8 = Image<3>;
pub type Luma8 = Image<1>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct View<'a, const C: usize> {
    data: &'a [u8],
    width: u32,
    height: u32,
    stride: usize,
}

pub type RgbView<'a> = View<'a, 3>;
pub type LumaView<'a> = View<'a, 1>;

pub(crate) fn packed_len(width: u32, height: u32, channels: usize) -> Result<usize, ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::InvalidDimensions);
    }
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(channels))
        .ok_or(ImageError::TooLarge)
}

impl<const C: usize> Image<C> {
    pub const CHANNELS: usize = C;

    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self, ImageError> {
        if packed_len(width, height, C)? != data.len() {
            return Err(ImageError::BufferSizeMismatch);
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn filled(width: u32, height: u32, pixel: [u8; C]) -> Result<Self, ImageError> {
        let len = packed_len(width, height, C)?;
        let data = if C == 1 {
            vec![pixel[0]; len]
        } else {
            pixel.iter().copied().cycle().take(len).collect()
        };
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn from_fn(
        width: u32,
        height: u32,
        mut pixel: impl FnMut(u32, u32) -> [u8; C],
    ) -> Result<Self, ImageError> {
        let mut data = Vec::with_capacity(packed_len(width, height, C)?);
        for y in 0..height {
            for x in 0..width {
                data.extend_from_slice(&pixel(x, y));
            }
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub(crate) fn from_parts_unchecked(width: u32, height: u32, data: Vec<u8>) -> Self {
        debug_assert_eq!(packed_len(width, height, C), Ok(data.len()));
        Self {
            width,
            height,
            data,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn row_len(&self) -> usize {
        self.width as usize * C
    }

    pub fn row(&self, y: u32) -> &[u8] {
        let len = self.row_len();
        &self.data[y as usize * len..][..len]
    }

    pub fn row_mut(&mut self, y: u32) -> &mut [u8] {
        let len = self.row_len();
        &mut self.data[y as usize * len..][..len]
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; C]> {
        self.view().pixel(x, y)
    }

    pub fn view(&self) -> View<'_, C> {
        View {
            data: &self.data,
            width: self.width,
            height: self.height,
            stride: self.row_len(),
        }
    }

    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Result<View<'_, C>, ImageError> {
        self.view().crop(x, y, width, height)
    }
}

impl<'a, const C: usize> View<'a, C> {
    pub fn new(data: &'a [u8], width: u32, height: u32, stride: usize) -> Result<Self, ImageError> {
        let row_len = packed_len(width, 1, C)?;
        if height == 0 {
            return Err(ImageError::InvalidDimensions);
        }
        if stride < row_len {
            return Err(ImageError::BufferSizeMismatch);
        }
        let required = (height as usize - 1)
            .checked_mul(stride)
            .and_then(|n| n.checked_add(row_len))
            .ok_or(ImageError::TooLarge)?;
        if data.len() < required {
            return Err(ImageError::BufferSizeMismatch);
        }
        Ok(Self {
            data: &data[..required],
            width,
            height,
            stride,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn stride(&self) -> usize {
        self.stride
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn row_len(&self) -> usize {
        self.width as usize * C
    }

    pub fn is_contiguous(&self) -> bool {
        self.stride == self.row_len() || self.height == 1
    }

    pub fn row(&self, y: u32) -> &'a [u8] {
        &self.data[y as usize * self.stride..][..self.row_len()]
    }

    pub fn rows(&self) -> impl ExactSizeIterator<Item = &'a [u8]> + 'a {
        let data = self.data;
        let stride = self.stride;
        let len = self.row_len();
        (0..self.height as usize).map(move |y| &data[y * stride..][..len])
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; C]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let start = y as usize * self.stride + x as usize * C;
        self.data[start..start + C].try_into().ok()
    }

    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Result<View<'a, C>, ImageError> {
        if width == 0 || height == 0 {
            return Err(ImageError::InvalidDimensions);
        }
        let fits = |offset: u32, len: u32, bound: u32| {
            offset.checked_add(len).is_some_and(|end| end <= bound)
        };
        if !fits(x, width, self.width) || !fits(y, height, self.height) {
            return Err(ImageError::OutOfBounds);
        }
        let start = y as usize * self.stride + x as usize * C;
        let len = (height as usize - 1) * self.stride + width as usize * C;
        Ok(View {
            data: &self.data[start..start + len],
            width,
            height,
            stride: self.stride,
        })
    }

    pub fn to_image(&self) -> Image<C> {
        let data = if self.is_contiguous() {
            self.data.to_vec()
        } else {
            let mut data = Vec::with_capacity(self.row_len() * self.height as usize);
            self.rows().for_each(|row| data.extend_from_slice(row));
            data
        };
        Image::from_parts_unchecked(self.width, self.height, data)
    }
}

impl<'a, const C: usize> From<&'a Image<C>> for View<'a, C> {
    fn from(image: &'a Image<C>) -> Self {
        image.view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(width: u32, height: u32) -> Rgb8 {
        Rgb8::from_fn(width, height, |x, y| [x as u8, y as u8, (x ^ y) as u8]).unwrap()
    }

    #[test]
    fn new_validates_length() {
        assert!(Rgb8::new(2, 2, vec![0; 12]).is_ok());
        assert_eq!(
            Rgb8::new(2, 2, vec![0; 11]),
            Err(ImageError::BufferSizeMismatch)
        );
        assert_eq!(
            Rgb8::new(2, 2, vec![0; 13]),
            Err(ImageError::BufferSizeMismatch)
        );
        assert_eq!(Luma8::new(3, 1, vec![0; 3]).unwrap().data().len(), 3);
    }

    #[test]
    fn new_rejects_zero_dimensions() {
        assert_eq!(Rgb8::new(0, 4, vec![]), Err(ImageError::InvalidDimensions));
        assert_eq!(Rgb8::new(4, 0, vec![]), Err(ImageError::InvalidDimensions));
    }

    #[test]
    fn packed_len_detects_overflow() {
        assert_eq!(
            packed_len(u32::MAX, u32::MAX, usize::MAX),
            Err(ImageError::TooLarge)
        );
    }

    #[test]
    fn filled_repeats_pixel() {
        let image = Rgb8::filled(3, 2, [1, 2, 3]).unwrap();
        assert_eq!(image.data().len(), 18);
        assert!(image.data().chunks_exact(3).all(|p| p == [1, 2, 3]));
        let luma = Luma8::filled(4, 4, [9]).unwrap();
        assert!(luma.data().iter().all(|&v| v == 9));
        assert_eq!(Luma8::filled(0, 1, [0]), Err(ImageError::InvalidDimensions));
    }

    #[test]
    fn from_fn_is_row_major() {
        let image = ramp(4, 3);
        assert_eq!(image.pixel(3, 2), Some([3, 2, 1]));
        assert_eq!(&image.row(1)[..3], &[0, 1, 1]);
        assert_eq!(image.dimensions(), (4, 3));
    }

    #[test]
    fn pixel_out_of_range_is_none() {
        let image = ramp(4, 3);
        assert_eq!(image.pixel(4, 0), None);
        assert_eq!(image.pixel(0, 3), None);
        assert_eq!(image.pixel(u32::MAX, u32::MAX), None);
    }

    #[test]
    fn row_mut_writes_through() {
        let mut image = Luma8::filled(3, 2, [0]).unwrap();
        image.row_mut(1).copy_from_slice(&[7, 8, 9]);
        assert_eq!(image.data(), &[0, 0, 0, 7, 8, 9]);
        image.data_mut()[0] = 1;
        assert_eq!(image.into_data(), vec![1, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn crop_is_zero_copy_and_correct() {
        let image = ramp(8, 6);
        let view = image.crop(2, 1, 3, 4).unwrap();
        assert_eq!(view.dimensions(), (3, 4));
        assert_eq!(view.stride(), 24);
        assert!(core::ptr::eq(
            view.data().as_ptr(),
            image.data()[24 + 6..].as_ptr()
        ));
        for y in 0..4 {
            for x in 0..3 {
                assert_eq!(view.pixel(x, y), image.pixel(x + 2, y + 1));
            }
        }
        assert!(!view.is_contiguous());
    }

    #[test]
    fn crop_of_crop_composes() {
        let image = ramp(10, 10);
        let inner = image.crop(1, 2, 8, 7).unwrap().crop(3, 1, 2, 2).unwrap();
        assert_eq!(inner.pixel(0, 0), image.pixel(4, 3));
        assert_eq!(inner.pixel(1, 1), image.pixel(5, 4));
    }

    #[test]
    fn crop_rejects_out_of_bounds_and_overflow() {
        let image = ramp(4, 4);
        assert_eq!(image.crop(0, 0, 5, 1), Err(ImageError::OutOfBounds));
        assert_eq!(image.crop(4, 0, 1, 1), Err(ImageError::OutOfBounds));
        assert_eq!(image.crop(u32::MAX, 0, 2, 1), Err(ImageError::OutOfBounds));
        assert_eq!(image.crop(0, u32::MAX, 1, 2), Err(ImageError::OutOfBounds));
        assert_eq!(image.crop(0, 0, 0, 1), Err(ImageError::InvalidDimensions));
        assert!(image.crop(0, 0, 4, 4).is_ok());
        assert!(image.crop(3, 3, 1, 1).is_ok());
    }

    #[test]
    fn view_to_image_round_trips() {
        let image = ramp(5, 5);
        assert_eq!(image.view().to_image(), image);
        let cropped = image.crop(1, 1, 3, 2).unwrap().to_image();
        assert_eq!(cropped.dimensions(), (3, 2));
        assert_eq!(cropped.pixel(0, 0), image.pixel(1, 1));
        assert_eq!(cropped.pixel(2, 1), image.pixel(3, 2));
    }

    #[test]
    fn view_new_validates_stride_and_length() {
        let data = [0u8; 20];
        assert!(View::<1>::new(&data, 4, 4, 5).is_ok());
        assert_eq!(
            View::<1>::new(&data, 4, 5, 5),
            Err(ImageError::BufferSizeMismatch)
        );
        assert_eq!(
            View::<1>::new(&data, 4, 2, 3),
            Err(ImageError::BufferSizeMismatch)
        );
        assert_eq!(
            View::<1>::new(&data, 0, 2, 3),
            Err(ImageError::InvalidDimensions)
        );
        assert_eq!(
            View::<1>::new(&data, 2, 0, 3),
            Err(ImageError::InvalidDimensions)
        );
        assert_eq!(
            View::<3>::new(&data, 1, 2, usize::MAX),
            Err(ImageError::TooLarge)
        );
    }

    #[test]
    fn view_new_trims_trailing_bytes() {
        let data = [1u8, 2, 3, 0, 4, 5, 6, 0, 99];
        let view = View::<1>::new(&data, 3, 2, 4).unwrap();
        assert_eq!(view.data().len(), 7);
        assert_eq!(view.to_image().data(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn rows_iterator_matches_row() {
        let image = ramp(6, 4);
        let view = image.crop(1, 0, 4, 4).unwrap();
        let rows: Vec<&[u8]> = view.rows().collect();
        assert_eq!(rows.len(), 4);
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(*row, view.row(y as u32));
            assert_eq!(row.len(), 12);
        }
    }

    #[test]
    fn single_row_view_is_contiguous() {
        let image = ramp(6, 4);
        assert!(image.crop(1, 1, 2, 1).unwrap().is_contiguous());
        assert!(image.view().is_contiguous());
    }

    #[test]
    fn from_reference_conversion() {
        let image = ramp(2, 2);
        let view: RgbView<'_> = (&image).into();
        assert_eq!(view, image.view());
        assert_eq!(Rgb8::CHANNELS, 3);
        assert_eq!(Luma8::CHANNELS, 1);
    }
}
