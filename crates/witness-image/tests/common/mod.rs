#![allow(dead_code)]

use witness_image::{Luma8, Rgb8};

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    pub fn byte(&mut self) -> u8 {
        (self.next() >> 24) as u8
    }
}

pub fn gradient(width: u32, height: u32) -> Rgb8 {
    Rgb8::from_fn(width, height, |x, y| {
        [
            (x * 255 / width.max(2).saturating_sub(1).max(1)).min(255) as u8,
            (y * 255 / height.max(2).saturating_sub(1).max(1)).min(255) as u8,
            128,
        ]
    })
    .unwrap()
}

pub fn checkerboard(width: u32, height: u32, cell: u32) -> Rgb8 {
    Rgb8::from_fn(width, height, |x, y| {
        if ((x / cell) + (y / cell)).is_multiple_of(2) {
            [255, 255, 255]
        } else {
            [0, 0, 0]
        }
    })
    .unwrap()
}

pub fn noise(width: u32, height: u32, seed: u64) -> Rgb8 {
    let mut rng = Rng(seed | 1);
    Rgb8::from_fn(width, height, |_, _| [rng.byte(), rng.byte(), rng.byte()]).unwrap()
}

pub fn noise_luma(width: u32, height: u32, seed: u64) -> Luma8 {
    let mut rng = Rng(seed | 1);
    Luma8::from_fn(width, height, |_, _| [rng.byte()]).unwrap()
}

pub const QUADRANT_COLOURS: [[u8; 3]; 4] =
    [[230, 20, 20], [20, 230, 20], [20, 20, 230], [230, 230, 20]];

pub fn quadrants(width: u32, height: u32) -> Rgb8 {
    Rgb8::from_fn(width, height, |x, y| {
        let index = usize::from(x >= width / 2) + 2 * usize::from(y >= height / 2);
        QUADRANT_COLOURS[index]
    })
    .unwrap()
}

pub fn mean_abs_diff(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    let total: u64 = a.iter().zip(b).map(|(&x, &y)| x.abs_diff(y) as u64).sum();
    total as f64 / a.len() as f64
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = if data.is_empty() {
        vec![&[]]
    } else {
        data.chunks(65_535).collect()
    };
    for (index, block) in blocks.iter().enumerate() {
        out.push(u8::from(index + 1 == blocks.len()));
        let len = block.len() as u16;
        out.extend(len.to_le_bytes());
        out.extend((!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend(adler32(data).to_be_bytes());
    out
}

pub fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend(crc.to_be_bytes());
}

pub struct PngSpec<'a> {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub colour_type: u8,
    pub rows: &'a [Vec<u8>],
    pub palette: Option<&'a [u8]>,
    pub transparency: Option<&'a [u8]>,
}

pub fn png(spec: &PngSpec<'_>) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend(spec.width.to_be_bytes());
    header.extend(spec.height.to_be_bytes());
    header.extend([spec.bit_depth, spec.colour_type, 0, 0, 0]);
    png_chunk(&mut out, b"IHDR", &header);
    if let Some(palette) = spec.palette {
        png_chunk(&mut out, b"PLTE", palette);
    }
    if let Some(trns) = spec.transparency {
        png_chunk(&mut out, b"tRNS", trns);
    }
    let mut raw = Vec::new();
    for row in spec.rows {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    png_chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    png_chunk(&mut out, b"IEND", &[]);
    out
}

pub fn png_rgb(image: &Rgb8) -> Vec<u8> {
    let rows: Vec<Vec<u8>> = (0..image.height()).map(|y| image.row(y).to_vec()).collect();
    png(&PngSpec {
        width: image.width(),
        height: image.height(),
        bit_depth: 8,
        colour_type: 2,
        rows: &rows,
        palette: None,
        transparency: None,
    })
}

pub fn exif_orientation_tiff(value: u16) -> Vec<u8> {
    let mut tiff = b"MM\x00\x2a\x00\x00\x00\x08\x00\x01".to_vec();
    tiff.extend(0x0112u16.to_be_bytes());
    tiff.extend(3u16.to_be_bytes());
    tiff.extend(1u32.to_be_bytes());
    tiff.extend(value.to_be_bytes());
    tiff.extend([0, 0, 0, 0, 0, 0]);
    tiff
}

pub fn jpeg_with(
    image: &Rgb8,
    quality: u8,
    orientation: Option<u16>,
    extra_app: &[(u8, Vec<u8>)],
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut out, quality);
    if let Some(value) = orientation {
        encoder
            .add_exif_metadata(&exif_orientation_tiff(value))
            .unwrap();
    }
    for (segment, data) in extra_app {
        encoder.add_app_segment(*segment, data.clone()).unwrap();
    }
    encoder
        .encode(
            image.data(),
            image.width() as u16,
            image.height() as u16,
            jpeg_encoder::ColorType::Rgb,
        )
        .unwrap();
    out
}

pub fn webp_lossless(data: &[u8], width: u32, height: u32, alpha: bool) -> Vec<u8> {
    let mut out = Vec::new();
    let colour = if alpha {
        image_webp::ColorType::Rgba8
    } else {
        image_webp::ColorType::Rgb8
    };
    image_webp::WebPEncoder::new(&mut out)
        .encode(data, width, height, colour)
        .unwrap();
    out
}

pub fn mutate(base: &[u8], rng: &mut Rng, flips: usize) -> Vec<u8> {
    let mut data = base.to_vec();
    for _ in 0..flips {
        let at = rng.below(data.len() as u64) as usize;
        data[at] = rng.byte();
    }
    if rng.below(4) == 0 {
        let keep = rng.below(data.len() as u64 + 1) as usize;
        data.truncate(keep);
    }
    data
}
