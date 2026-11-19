#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Jpeg,
    Png,
    WebP,
}

const JPEG_MAGIC: [u8; 3] = [0xFF, 0xD8, 0xFF];
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

pub fn sniff(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(&JPEG_MAGIC) {
        Some(Format::Jpeg)
    } else if bytes.starts_with(&PNG_MAGIC) {
        Some(Format::Png)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(Format::WebP)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_jpeg() {
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some(Format::Jpeg));
    }

    #[test]
    fn detects_png() {
        assert_eq!(sniff(&PNG_MAGIC), Some(Format::Png));
    }

    #[test]
    fn detects_webp() {
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some(Format::WebP));
    }

    #[test]
    fn rejects_empty_and_short() {
        assert_eq!(sniff(&[]), None);
        assert_eq!(sniff(&[0xFF, 0xD8]), None);
        assert_eq!(sniff(b"RIFF\0\0\0\0WEB"), None);
        assert_eq!(sniff(&PNG_MAGIC[..7]), None);
    }

    #[test]
    fn rejects_other_formats() {
        assert_eq!(sniff(b"GIF89a......"), None);
        assert_eq!(sniff(b"BM\0\0\0\0\0\0\0\0\0\0"), None);
        assert_eq!(sniff(b"\0\0\0\x18ftypheic"), None);
        assert_eq!(sniff(b"RIFF\0\0\0\0WAVEfmt "), None);
    }

    #[test]
    fn requires_prefix_not_substring() {
        assert_eq!(sniff(&[0x00, 0xFF, 0xD8, 0xFF]), None);
    }
}
