use crate::buffer::RgbView;
use crate::decode::decode;
use crate::error::ImageError;
use sha2::{Digest, Sha256};

pub type PixelHash = [u8; 32];

pub fn pixel_hash(bytes: &[u8]) -> Result<PixelHash, ImageError> {
    Ok(pixel_hash_of(decode(bytes)?.view()))
}

pub fn pixel_hash_of(image: RgbView<'_>) -> PixelHash {
    let mut hasher = Sha256::new();
    hasher.update(image.width().to_be_bytes());
    hasher.update(image.height().to_be_bytes());
    if image.is_contiguous() {
        hasher.update(image.data());
    } else {
        image.rows().for_each(|row| hasher.update(row));
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Rgb8;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn known_vector_for_single_black_pixel() {
        let image = Rgb8::filled(1, 1, [0, 0, 0]).unwrap();
        let expected: [u8; 32] = Sha256::digest([0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]).into();
        assert_eq!(pixel_hash_of(image.view()), expected);
        assert_eq!(hex(&expected).len(), 64);
    }

    #[test]
    fn dimensions_are_bound() {
        let a = Rgb8::new(2, 3, vec![5; 18]).unwrap();
        let b = Rgb8::new(3, 2, vec![5; 18]).unwrap();
        let c = Rgb8::new(6, 1, vec![5; 18]).unwrap();
        assert_ne!(pixel_hash_of(a.view()), pixel_hash_of(b.view()));
        assert_ne!(pixel_hash_of(b.view()), pixel_hash_of(c.view()));
    }

    #[test]
    fn single_pixel_change_changes_hash() {
        let a = Rgb8::filled(10, 10, [1, 2, 3]).unwrap();
        let mut b = a.clone();
        b.data_mut()[150] ^= 1;
        assert_ne!(pixel_hash_of(a.view()), pixel_hash_of(b.view()));
    }

    #[test]
    fn strided_view_hashes_like_owned_copy() {
        let full = Rgb8::from_fn(20, 10, |x, y| [x as u8, y as u8, 0]).unwrap();
        let view = full.crop(3, 2, 7, 5).unwrap();
        assert_eq!(pixel_hash_of(view), pixel_hash_of(view.to_image().view()));
    }

    #[test]
    fn rejects_undecodable_bytes() {
        assert_eq!(
            pixel_hash(b"not an image"),
            Err(ImageError::UnsupportedFormat)
        );
    }
}
