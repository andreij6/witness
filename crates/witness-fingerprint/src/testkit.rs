#![allow(dead_code)]

use witness_image::{Luma8, Rgb8, decode, encode_jpeg};

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }

    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

struct Blob {
    cx: f64,
    cy: f64,
    radius: f64,
    colour: [f64; 3],
}

struct Wave {
    fx: f64,
    fy: f64,
    phase: f64,
    amplitude: [f64; 3],
}

pub fn scene(width: u32, height: u32, seed: u64) -> Rgb8 {
    let mut rng = Rng::new(seed);
    let base = [
        rng.range(40.0, 200.0),
        rng.range(40.0, 200.0),
        rng.range(40.0, 200.0),
    ];
    let tilt = [rng.range(-80.0, 80.0), rng.range(-80.0, 80.0)];
    let blobs: Vec<Blob> = (0..12)
        .map(|_| Blob {
            cx: rng.unit(),
            cy: rng.unit(),
            radius: rng.range(0.04, 0.25),
            colour: [
                rng.range(-110.0, 110.0),
                rng.range(-110.0, 110.0),
                rng.range(-110.0, 110.0),
            ],
        })
        .collect();
    let waves: Vec<Wave> = (0..5)
        .map(|_| Wave {
            fx: rng.range(-14.0, 14.0),
            fy: rng.range(-14.0, 14.0),
            phase: rng.range(0.0, 6.3),
            amplitude: [
                rng.range(5.0, 30.0),
                rng.range(5.0, 30.0),
                rng.range(5.0, 30.0),
            ],
        })
        .collect();
    let scale = width.max(height) as f64;
    let tau = core::f64::consts::TAU;
    let columns: Vec<Vec<(f64, f64)>> = waves
        .iter()
        .map(|w| {
            (0..width)
                .map(|x| (tau * w.fx * x as f64 / scale).sin_cos())
                .collect()
        })
        .collect();
    let rows: Vec<Vec<(f64, f64)>> = waves
        .iter()
        .map(|w| {
            (0..height)
                .map(|y| (tau * w.fy * y as f64 / scale + w.phase).sin_cos())
                .collect()
        })
        .collect();
    Rgb8::from_fn(width, height, |x, y| {
        let (u, v) = (x as f64 / scale, y as f64 / scale);
        let mut px = base;
        for (c, value) in px.iter_mut().enumerate() {
            *value += tilt[c % 2] * (u - v);
        }
        for blob in &blobs {
            let (dx, dy) = (u - blob.cx, v - blob.cy);
            if dx.abs() < blob.radius
                && dy.abs() < blob.radius
                && dx * dx + dy * dy < blob.radius * blob.radius
            {
                px.iter_mut().zip(blob.colour).for_each(|(p, c)| *p += c);
            }
        }
        for (i, wave) in waves.iter().enumerate() {
            let ((sx, cx), (sy, cy)) = (columns[i][x as usize], rows[i][y as usize]);
            let s = sx * cy + cx * sy;
            px.iter_mut()
                .zip(wave.amplitude)
                .for_each(|(p, a)| *p += s * a);
        }
        px.map(|value| value.round().clamp(0.0, 255.0) as u8)
    })
    .unwrap()
}

pub fn noise_luma(width: u32, height: u32, seed: u64) -> Luma8 {
    let mut rng = Rng::new(seed);
    Luma8::from_fn(width, height, |_, _| [rng.next() as u8]).unwrap()
}

pub fn grey(rgb: &Rgb8) -> Luma8 {
    witness_image::to_luma(rgb.view())
}

pub fn jpeg_round_trip(rgb: &Rgb8, quality: u8) -> Rgb8 {
    decode(&encode_jpeg(rgb, quality).unwrap()).unwrap()
}

pub fn map_pixels(rgb: &Rgb8, f: impl Fn(u8) -> u8) -> Rgb8 {
    Rgb8::new(
        rgb.width(),
        rgb.height(),
        rgb.data().iter().map(|&v| f(v)).collect(),
    )
    .unwrap()
}

pub fn paste(target: &Rgb8, patch: &Rgb8, x0: u32, y0: u32) -> Rgb8 {
    Rgb8::from_fn(target.width(), target.height(), |x, y| {
        if x >= x0 && y >= y0 && x - x0 < patch.width() && y - y0 < patch.height() {
            patch.pixel(x - x0, y - y0).unwrap()
        } else {
            target.pixel(x, y).unwrap()
        }
    })
    .unwrap()
}
