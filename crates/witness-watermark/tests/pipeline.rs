use std::cell::Cell;
use witness_format::CaptureId;
use witness_image::Rgb8;
use witness_watermark::{
    DEFAULT_DEAD_ZONE, MODEL_SIZE, PAYLOAD_BITS, PLANE_LEN, SENSOR_ASPECT, WatermarkError,
    Watermarker, decode_logits, encode,
};

const COLUMNS: usize = 20;
const ROWS: usize = 10;
const MARGIN: usize = 3;

struct BlockModel {
    decodes: Cell<usize>,
}

impl BlockModel {
    fn new() -> Self {
        Self {
            decodes: Cell::new(0),
        }
    }
}

type Block = (core::ops::Range<usize>, core::ops::Range<usize>);

fn blocks(i: usize) -> [Block; 2] {
    let (col, row) = (i % ROWS, i / ROWS);
    let span = |k: usize, n: usize| k * MODEL_SIZE / n + MARGIN..(k + 1) * MODEL_SIZE / n - MARGIN;
    [
        (span(col, COLUMNS), span(row, ROWS)),
        (span(col + ROWS, COLUMNS), span(row, ROWS)),
    ]
}

fn inside_any_block(x: usize, y: usize) -> bool {
    (0..PAYLOAD_BITS)
        .flat_map(blocks)
        .any(|(xs, ys)| xs.contains(&x) && ys.contains(&y))
}

impl Watermarker for BlockModel {
    fn encode(
        &self,
        cover: &[f32],
        payload: &[f32; PAYLOAD_BITS],
        stego: &mut [f32],
    ) -> Result<(), WatermarkError> {
        stego.copy_from_slice(cover);
        for (i, &bit) in payload.iter().enumerate() {
            let delta = if bit > 0.5 { 0.3 } else { -0.3 };
            for (xs, ys) in blocks(i) {
                for c in 0..3 {
                    for y in ys.clone() {
                        for x in xs.clone() {
                            stego[c * PLANE_LEN + y * MODEL_SIZE + x] += delta;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn decode(&self, input: &[f32]) -> Result<[f32; PAYLOAD_BITS], WatermarkError> {
        self.decodes.set(self.decodes.get() + 1);
        let at = |x: usize, y: usize| {
            (0..3)
                .map(|c| input[c * PLANE_LEN + y * MODEL_SIZE + x])
                .sum::<f32>()
                / 3.0
        };
        let (mut sum, mut n) = (0.0f32, 0usize);
        for y in 0..MODEL_SIZE {
            for x in 0..MODEL_SIZE {
                if !inside_any_block(x, y) {
                    sum += at(x, y);
                    n += 1;
                }
            }
        }
        let reference = sum / n as f32;
        Ok(core::array::from_fn(|i| {
            blocks(i)
                .into_iter()
                .map(|(xs, ys)| {
                    let count = (xs.len() * ys.len()) as f32;
                    let mean = ys
                        .flat_map(|y| xs.clone().map(move |x| (x, y)))
                        .map(|(x, y)| at(x, y))
                        .sum::<f32>()
                        / count;
                    (mean - reference) * 20.0
                })
                .sum()
        }))
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u8 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 56) as u8
    }
}

fn textured(width: u32, height: u32, seed: u64) -> Rgb8 {
    let mut rng = Lcg(seed);
    Rgb8::from_fn(width, height, |_, _| {
        let v = 110 + rng.next() % 20;
        [v, v, v]
    })
    .unwrap()
}

fn capture(seed: u64) -> CaptureId {
    CaptureId::from_u64(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) & CaptureId::MAX).unwrap()
}

#[test]
fn embed_then_detect_round_trips() {
    let model = BlockModel::new();
    for (seed, (w, h)) in [(400u32, 300u32), (300, 400), (1024, 768), (257, 199)]
        .into_iter()
        .enumerate()
    {
        let cid = capture(seed as u64);
        let mut img = textured(w, h, seed as u64);
        model.embed(&mut img, &encode(cid)).unwrap();
        let logits = model.detect(img.view()).unwrap();
        let decoded = decode_logits(&logits, DEFAULT_DEAD_ZONE).unwrap();
        assert_eq!(decoded.capture_id, cid, "{w}x{h}");
        assert_eq!(decoded.errors, 0);
    }
}

#[test]
fn embedding_is_deterministic_and_bounded() {
    let model = BlockModel::new();
    let original = textured(640, 480, 9);
    let (mut a, mut b) = (original.clone(), original.clone());
    model.embed(&mut a, &encode(capture(1))).unwrap();
    model.embed(&mut b, &encode(capture(1))).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, original);
    assert!(
        a.data()
            .iter()
            .zip(original.data())
            .all(|(&x, &y)| (x as i32 - y as i32).abs() <= 26)
    );
}

#[test]
fn centered_crop_is_recovered_by_a_canvas_hypothesis() {
    let model = BlockModel::new();
    let cid = capture(77);
    let mut img = textured(400, 300, 3);
    model.embed(&mut img, &encode(cid)).unwrap();
    let crop = img.crop(35, 0, 330, 300).unwrap();
    assert_eq!(
        decode_logits(&model.detect(crop).unwrap(), DEFAULT_DEAD_ZONE),
        None
    );
    model.decodes.set(0);
    let decoded = model
        .read(crop, SENSOR_ASPECT, DEFAULT_DEAD_ZONE)
        .unwrap()
        .unwrap();
    assert_eq!(decoded.capture_id, cid);
    assert_eq!(model.decodes.get(), 2);
}

#[test]
fn left_anchored_crop_is_recovered_by_the_start_hypothesis() {
    let model = BlockModel::new();
    let cid = capture(78);
    let mut img = textured(400, 300, 4);
    model.embed(&mut img, &encode(cid)).unwrap();
    let crop = img.crop(0, 0, 360, 300).unwrap();
    let decoded = model
        .read(crop, SENSOR_ASPECT, DEFAULT_DEAD_ZONE)
        .unwrap()
        .unwrap();
    assert_eq!(decoded.capture_id, cid);
    assert!(model.decodes.get() <= 4);
}

#[test]
fn unmarked_image_reads_nothing_within_budget() {
    let model = BlockModel::new();
    let img = textured(300, 300, 5);
    assert_eq!(
        model.read(img.view(), SENSOR_ASPECT, DEFAULT_DEAD_ZONE),
        Ok(None)
    );
    assert_eq!(model.decodes.get(), 4);
}

#[test]
fn trait_is_object_safe() {
    let model: Box<dyn Watermarker> = Box::new(BlockModel::new());
    let mut img = textured(200, 150, 6);
    model.embed(&mut img, &encode(capture(2))).unwrap();
    assert_eq!(
        decode_logits(&model.detect(img.view()).unwrap(), DEFAULT_DEAD_ZONE)
            .unwrap()
            .capture_id,
        capture(2)
    );
}
