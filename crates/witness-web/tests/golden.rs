use witness_core::format::crypto::sha256;
use witness_core::format::{BlurClass, Edit, Kind, Outcome, Rect, VerdictFlags};
use witness_core::image::{Rgb8, decode, encode_jpeg, resize_area, strip_and_normalize_jpeg};
use witness_core::verdict::{Reason, Verdict, judge};
use witness_verdict::testkit::photo;
use witness_web::render;
use witness_web::testkit::{H, MAX_ROUNDS, W, World, jpeg_of, recompress, replay};

const GOLDEN: &str = "82424ad6e1ca630b094bd94ae55f161b515b1b2fe1fdc867f520e9ea7b25d67c";

struct Case {
    name: &'static str,
    input: Vec<u8>,
    expect: Outcome,
}

fn paste(img: &Rgb8, r: Rect, from: &Rgb8) -> Rgb8 {
    Rgb8::from_fn(img.width(), img.height(), |x, y| {
        let inside = x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h;
        img.pixel(x, y)
            .filter(|_| !inside)
            .or_else(|| from.pixel(x, y))
            .unwrap()
    })
    .unwrap()
}

fn jpeg(img: &Rgb8, q: u8) -> Vec<u8> {
    encode_jpeg(img, q).unwrap()
}

fn cases(w: &mut World) -> Vec<Case> {
    let a = w.original(1, true);
    let b = w.original(2, false);
    let c = w.original(3, true);
    let unanchored = w.seal(&jpeg_of(4, 95), Kind::Original);
    let decoded = decode(&c.jpeg).unwrap();
    let rect = Rect {
        x: 40,
        y: 40,
        w: 80,
        h: 60,
    };
    let blurred = Rgb8::from_fn(W, H, |x, y| {
        let inside = x >= rect.x && y >= rect.y && x < rect.x + rect.w && y < rect.y + rect.h;
        if inside {
            [120, 118, 116]
        } else {
            decoded.pixel(x, y).unwrap()
        }
    })
    .unwrap();
    let edits = vec![Edit::Blur {
        rect,
        class: BlurClass::Face,
        sigma_e2: 900,
    }];
    let derivative = w.seal(
        &jpeg(&blurred, 95),
        Kind::Derivative {
            parent: c.record_id,
            root: c.claim.capture_id,
            edits,
        },
    );
    w.anchor(&derivative, true);
    let small = resize_area(decode(&a.jpeg).unwrap().view(), W * 2 / 3, H * 2 / 3).unwrap();
    let crop = decode(&a.jpeg)
        .unwrap()
        .crop(120, 60, 320, 260)
        .unwrap()
        .to_image();
    let edited = paste(
        &decode(&a.jpeg).unwrap(),
        Rect {
            x: 210,
            y: 150,
            w: 60,
            h: 50,
        },
        &photo(W, H, 777),
    );
    let mut tampered = a.jpeg.clone();
    let at = tampered.len() - 64;
    tampered[at] = tampered[at].wrapping_add(1);
    vec![
        Case {
            name: "sealed",
            input: a.jpeg.clone(),
            expect: Outcome::Original,
        },
        Case {
            name: "stripped",
            input: strip_and_normalize_jpeg(&a.jpeg).unwrap(),
            expect: Outcome::Original,
        },
        Case {
            name: "recompressed",
            input: recompress(&a.jpeg, 70),
            expect: Outcome::MatchesWithChanges,
        },
        Case {
            name: "downscaled",
            input: jpeg(&small, 80),
            expect: Outcome::MatchesWithChanges,
        },
        Case {
            name: "cropped",
            input: jpeg(&crop, 85),
            expect: Outcome::NoRecord,
        },
        Case {
            name: "local_edit",
            input: jpeg(&edited, 85),
            expect: Outcome::Altered,
        },
        Case {
            name: "no_reference",
            input: recompress(&b.jpeg, 75),
            expect: Outcome::MatchesWithChanges,
        },
        Case {
            name: "tampered",
            input: tampered,
            expect: Outcome::Altered,
        },
        Case {
            name: "unanchored",
            input: unanchored.jpeg.clone(),
            expect: Outcome::SealedUnanchored,
        },
        Case {
            name: "derivative",
            input: derivative.jpeg.clone(),
            expect: Outcome::MatchesWithChanges,
        },
        Case {
            name: "derivative_copy",
            input: recompress(&derivative.jpeg, 75),
            expect: Outcome::MatchesWithChanges,
        },
        Case {
            name: "unrelated",
            input: jpeg_of(14, 85),
            expect: Outcome::NoRecord,
        },
        Case {
            name: "garbage",
            input: vec![0x5a; 97],
            expect: Outcome::NoRecord,
        },
    ]
}

fn check_case(w: &World, case: &Case) -> (Verdict, usize) {
    let options = w.options();
    let anchor = options.anchor().unwrap();
    let native = judge(&case.input, &w.oracle, &options.options(&anchor));
    let (step, rounds, _) = replay(&case.input, &options, &w.oracle);
    assert_eq!(step.verdict, native, "{}", case.name);
    assert_eq!(native.outcome, case.expect, "{}: {native:?}", case.name);
    assert!(rounds < MAX_ROUNDS, "{}", case.name);
    (native, rounds)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn replayed_verdicts_equal_native_judge_and_match_golden() {
    let mut w = World::new();
    let cases = cases(&mut w);
    let mut digest_input = Vec::new();
    for case in &cases {
        let (v, rounds) = check_case(&w, case);
        digest_input.extend_from_slice(case.name.as_bytes());
        digest_input.extend_from_slice(render::verdict(&v).to_string().as_bytes());
        digest_input.push(rounds as u8);
        match case.name {
            "stripped" => assert!(
                v.reasons.contains(Reason::ExactAssetHash)
                    || v.reasons.contains(Reason::ExactPixelHash)
            ),
            "no_reference" => assert!(v.flags.contains(VerdictFlags::NO_PUBLIC_REFERENCE)),
            "tampered" => assert!(v.reasons.contains(Reason::SealBroken)),
            "local_edit" => assert_eq!(v.regions.len(), 1),
            "garbage" => assert!(v.reasons.contains(Reason::Undecodable)),
            _ => {}
        }
    }
    assert_eq!(hex::encode(sha256(&digest_input)), GOLDEN);
}
