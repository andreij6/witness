use witness_core::fingerprint::{fingerprint, grid, index_keys, pdq, pdq_dihedral, probe_keys};
use witness_core::format::crypto::sha256;
use witness_core::format::{CaptureId, KeyHistory, Kind, Outcome};
use witness_core::image::{
    Orientation, Rgb8, decode, encode_jpeg, pixel_hash, pixel_hash_of, to_luma,
};
use witness_core::verdict::testkit::{
    CAPTURED, ENROLLED_NS, FixedWatermarker, GENERATOR, MemoryOracle, TestCa, photo,
};
use witness_core::{Capture, ClaimTemplate, Manifest, Mark, Options, SEAL_QUALITY, judge, seal};

const GOLDEN: &str = "3d51aec49a25160ac291f24825a03c722effdcc7a444aa56155be74296e58667";
const IMAGES: u64 = 50;

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn synthetic(rng: &mut SplitMix) -> Rgb8 {
    let width = 16 + rng.below(240) as u32;
    let height = 16 + rng.below(240) as u32;
    let base: [u8; 3] = core::array::from_fn(|_| rng.below(256) as u8);
    let slope: [u32; 3] = core::array::from_fn(|_| rng.below(4) as u32);
    let rects: Vec<[u32; 7]> = (0..rng.below(12))
        .map(|_| {
            [
                rng.below(u64::from(width)) as u32,
                rng.below(u64::from(height)) as u32,
                1 + rng.below(u64::from(width / 2 + 1)) as u32,
                1 + rng.below(u64::from(height / 2 + 1)) as u32,
                rng.below(256) as u32,
                rng.below(256) as u32,
                rng.below(256) as u32,
            ]
        })
        .collect();
    let noise = rng.next();
    Rgb8::from_fn(width, height, |x, y| {
        let mut p: [u8; 3] =
            core::array::from_fn(|c| (u32::from(base[c]) + slope[c] * (x + y)) as u8);
        for &[rx, ry, rw, rh, r, g, b] in &rects {
            if x >= rx && y >= ry && x < rx + rw && y < ry + rh {
                p = [r as u8, g as u8, b as u8];
            }
        }
        let n = (SplitMix(noise ^ (u64::from(x) << 32 | u64::from(y))).next() % 7) as u8;
        p.map(|v| v.saturating_add(n))
    })
    .unwrap()
}

fn absorb_image(out: &mut Vec<u8>, img: &Rgb8, quality: u8) {
    let jpeg = encode_jpeg(img, quality).unwrap();
    out.extend_from_slice(&sha256(&jpeg));
    let decoded = decode(&jpeg).unwrap();
    let decoded_hash = pixel_hash_of(decoded.view());
    assert_eq!(pixel_hash(&jpeg).unwrap(), decoded_hash);
    out.extend_from_slice(&decoded_hash);
    out.extend_from_slice(&pixel_hash_of(img.view()));
    let luma = to_luma(img.view());
    let g = grid(luma.view());
    out.push(u8::from(g.portrait()));
    g.cells()
        .iter()
        .for_each(|c| out.extend_from_slice(&c.to_be_bytes()));
    let Some(p) = pdq(luma.view()) else {
        out.push(0xFF);
        return;
    };
    let dihedral = pdq_dihedral(luma.view()).unwrap();
    assert_eq!(dihedral.pdq(), p);
    out.push(p.quality);
    dihedral
        .hashes
        .iter()
        .for_each(|h| out.extend_from_slice(&h.0));
    index_keys(&p.hash)
        .iter()
        .chain(probe_keys(&p.hash).iter())
        .for_each(|k| out.extend_from_slice(&k.packed().to_be_bytes()));
    out.extend_from_slice(format!("{:?}", fingerprint(img.view())).as_bytes());
}

fn absorb_verdicts(out: &mut Vec<u8>) {
    let ca = TestCa::new(9);
    let dev = ca.device(0x44, 0x0C0D);
    let capture = CaptureId::new(0x60_1D, 0).unwrap();
    let wm = FixedWatermarker::reading(capture);
    let mut oracle = MemoryOracle::new();
    oracle.add_key(dev.key_info(KeyHistory::new(ENROLLED_NS)));
    let manifest = Manifest {
        id: [0x33; 16],
        generator: GENERATOR,
        actions: CAPTURED,
    };
    let mut inputs = Vec::new();
    for counter in 1..=2u64 {
        let camera = encode_jpeg(&photo(192, 144, counter), 95).unwrap();
        let c = dev.claim(capture.to_u64(), Kind::Original, counter);
        let template = ClaimTemplate {
            capture_id: c.capture_id,
            kind: c.kind,
            time: c.time,
            loc: c.loc,
            checks: c.checks,
            key: c.key,
            cert_serial: c.cert_serial,
            coarse: c.coarse,
        };
        let mark = Some(Mark {
            watermarker: &wm,
            capture_id: capture,
        });
        let input = Capture {
            image: &camera,
            orientation: Orientation::Normal,
            quality: SEAL_QUALITY,
            mark,
        };
        let sealed = seal(&input, template, &manifest, &dev, &ca.anchor).unwrap();
        let record = oracle.anchor(sealed.claim.clone(), ENROLLED_NS + 1_000 * counter);
        oracle.set_fingerprints(record.record_ix, sealed.fingerprints.clone());
        let reference = encode_jpeg(&decode(&sealed.jpeg).unwrap(), 85).unwrap();
        oracle.set_reference(record.record_ix, reference.clone());
        let crop = decode(&sealed.jpeg)
            .unwrap()
            .crop(30, 20, 140, 110)
            .unwrap()
            .to_image();
        inputs.push(sealed.jpeg.clone());
        inputs.push(encode_jpeg(&decode(&reference).unwrap(), 70).unwrap());
        inputs.push(encode_jpeg(&crop, 85).unwrap());
    }
    inputs.push(encode_jpeg(&photo(192, 144, 77), 85).unwrap());
    inputs.push(b"not an image".to_vec());
    let blank = FixedWatermarker::blank();
    let mut outcomes = Vec::new();
    for options in [
        Options::android(&ca.anchor).with_watermarker(&wm),
        Options::web(&ca.anchor).with_watermarker(&blank),
    ] {
        for input in &inputs {
            let verdict = judge(input, &oracle, &options);
            outcomes.push(verdict.outcome);
            out.extend_from_slice(format!("{verdict:?}").as_bytes());
        }
    }
    use Outcome::{MatchesWithChanges as Changed, NoRecord, Original};
    assert_eq!(
        outcomes,
        [
            Original, Changed, Changed, Original, Changed, Changed, NoRecord, NoRecord, Original,
            Changed, NoRecord, Original, Changed, NoRecord, NoRecord, NoRecord,
        ]
    );
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn digest() -> String {
    let mut rng = SplitMix(0x5717_7E55);
    let mut out = Vec::new();
    for _ in 0..IMAGES {
        let img = synthetic(&mut rng);
        let quality = 60 + rng.below(36) as u8;
        absorb_image(&mut out, &img, quality);
    }
    absorb_verdicts(&mut out);
    hex(&sha256(&out))
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn core_outputs_match_committed_digest() {
    assert_eq!(digest(), GOLDEN);
}
