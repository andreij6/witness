use super::*;
use crate::cbor::{from_slice, from_slice_canonical, to_vec};
use crate::testutil::Rng;
use alloc::vec;
use alloc::vec::Vec;

fn rect(x: u32, y: u32, w: u32, h: u32) -> Rect {
    Rect { x, y, w, h }
}

fn samples() -> Vec<Edit> {
    vec![
        Edit::Resize {
            w: 1080,
            h: 1350,
            filter: 1,
        },
        Edit::Recompress {
            format: ImageFormat::Jpeg,
            quality: 85,
        },
        Edit::Orient { orientation: 6 },
        Edit::Crop(rect(10, 20, 300, 400)),
        Edit::Rotate { deg_e2: -350 },
        Edit::Tone {
            exposure_e2: 30,
            contrast_e2: -15,
            saturation_e2: 0,
        },
        Edit::Filter {
            id: 4,
            strength_e2: 75,
        },
        Edit::Blur {
            rect: rect(0, 0, 64, 32),
            class: BlurClass::Plate,
            sigma_e2: 800,
        },
    ]
}

#[test]
fn class_table_matches_architecture() {
    let want = [
        (EditKind::Resize, EditClass::Presentation),
        (EditKind::Recompress, EditClass::Presentation),
        (EditKind::Orient, EditClass::Presentation),
        (EditKind::Crop, EditClass::Global),
        (EditKind::Rotate, EditClass::Global),
        (EditKind::Tone, EditClass::Global),
        (EditKind::Filter, EditClass::Global),
        (EditKind::Blur, EditClass::LocalDeclared),
    ];
    for (k, c) in want {
        assert_eq!(k.class(), c, "{k:?}");
    }
    assert_eq!(EditKind::ALL.len(), want.len());
    for (e, (k, _)) in samples().iter().zip(want) {
        assert_eq!(e.kind(), k);
        assert_eq!(e.class(), k.class());
    }
}

#[test]
fn presentation_only_rules() {
    let s = samples();
    assert!(presentation_only(&[]));
    assert!(presentation_only(&s[..3]));
    assert!(!presentation_only(&s));
    for e in &s[3..] {
        assert!(!presentation_only(&[s[0], *e]));
    }
    assert!(has_local(&s));
    assert!(!has_local(&s[..7]));
    assert!(!has_local(&[]));
}

#[test]
fn golden_encodings() {
    assert_eq!(
        to_vec(&Edit::Orient { orientation: 6 }),
        [0xa2, 0x00, 0x02, 0x01, 0xa1, 0x00, 0x06]
    );
    assert_eq!(
        to_vec(&Edit::Crop(rect(1, 2, 3, 4))),
        [
            0xa2, 0x00, 0x03, 0x01, 0xa1, 0x00, 0xa4, 0x00, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03,
            0x04
        ]
    );
    assert_eq!(
        to_vec(&Edit::Rotate { deg_e2: -350 }),
        [0xa2, 0x00, 0x04, 0x01, 0xa1, 0x00, 0x39, 0x01, 0x5d]
    );
    assert_eq!(
        to_vec(&Edit::Recompress {
            format: ImageFormat::Webp,
            quality: 80
        }),
        [0xa2, 0x00, 0x01, 0x01, 0xa2, 0x00, 0x02, 0x01, 0x18, 0x50]
    );
    assert_eq!(
        to_vec(&Edit::Blur {
            rect: rect(0, 0, 1, 1),
            class: BlurClass::Face,
            sigma_e2: 24
        }),
        [
            0xa2, 0x00, 0x07, 0x01, 0xa3, 0x00, 0xa4, 0x00, 0x00, 0x01, 0x00, 0x02, 0x01, 0x03,
            0x01, 0x01, 0x00, 0x02, 0x18, 0x18
        ]
    );
}

#[test]
fn every_sample_roundtrips_canonically() {
    for e in samples() {
        let b = to_vec(&e);
        assert_eq!(from_slice_canonical::<Edit>(&b), Ok(e), "{e:?}");
    }
    let list = samples();
    assert_eq!(from_slice::<Vec<Edit>>(&to_vec(&list)), Ok(list));
}

#[test]
fn validation_bounds() {
    assert!(Edit::Rotate { deg_e2: 1000 }.is_valid());
    assert!(Edit::Rotate { deg_e2: -1000 }.is_valid());
    assert!(!Edit::Rotate { deg_e2: 1001 }.is_valid());
    assert!(!Edit::Rotate { deg_e2: i16::MIN }.is_valid());
    assert!(Edit::Orient { orientation: 7 }.is_valid());
    assert!(!Edit::Orient { orientation: 8 }.is_valid());
    assert!(
        !Edit::Resize {
            w: 0,
            h: 1,
            filter: 0
        }
        .is_valid()
    );
    assert!(
        !Edit::Resize {
            w: 1,
            h: 0,
            filter: 0
        }
        .is_valid()
    );
    assert!(
        !Edit::Recompress {
            format: ImageFormat::Png,
            quality: 101
        }
        .is_valid()
    );
    assert!(!Edit::Crop(rect(0, 0, 0, 5)).is_valid());
    assert!(!Edit::Crop(rect(u32::MAX, 0, 1, 1)).is_valid());
    assert!(!Edit::Crop(rect(0, u32::MAX, 1, 1)).is_valid());
    assert!(Edit::Crop(rect(u32::MAX - 1, 0, 1, 1)).is_valid());
    assert!(
        !Edit::Blur {
            rect: rect(0, 0, 1, 0),
            class: BlurClass::Other,
            sigma_e2: 1
        }
        .is_valid()
    );
    assert_eq!(
        rect(0, 0, u32::MAX, u32::MAX).area(),
        u64::from(u32::MAX) * u64::from(u32::MAX)
    );
}

#[test]
fn decoder_rejects_invalid_params() {
    let rot = to_vec(&Edit::Rotate { deg_e2: 1001 });
    assert_eq!(from_slice::<Edit>(&rot), Err(Error::Invalid));
    let orient = [0xa2, 0x00, 0x02, 0x01, 0xa1, 0x00, 0x08];
    assert_eq!(from_slice::<Edit>(&orient), Err(Error::Invalid));
    let zero_crop = [
        0xa2, 0x00, 0x03, 0x01, 0xa1, 0x00, 0xa4, 0x00, 0x01, 0x01, 0x02, 0x02, 0x00, 0x03, 0x04,
    ];
    assert_eq!(from_slice::<Edit>(&zero_crop), Err(Error::Invalid));
    let unknown_kind = [0xa2, 0x00, 0x08, 0x01, 0xa0];
    assert_eq!(from_slice::<Edit>(&unknown_kind), Err(Error::Range));
    let unknown_format = [0xa2, 0x00, 0x01, 0x01, 0xa2, 0x00, 0x05, 0x01, 0x01];
    assert_eq!(from_slice::<Edit>(&unknown_format), Err(Error::Range));
    let unknown_blur_class = [
        0xa2, 0x00, 0x07, 0x01, 0xa3, 0x00, 0xa4, 0x00, 0x00, 0x01, 0x00, 0x02, 0x01, 0x03, 0x01,
        0x01, 0x04, 0x02, 0x01,
    ];
    assert_eq!(from_slice::<Edit>(&unknown_blur_class), Err(Error::Range));
}

#[test]
fn decoder_rejects_structural_errors() {
    assert_eq!(
        from_slice::<Edit>(&[0xa1, 0x00, 0x02]),
        Err(Error::MissingKey(1))
    );
    assert_eq!(
        from_slice::<Edit>(&[0xa1, 0x01, 0xa0]),
        Err(Error::MissingKey(0))
    );
    assert_eq!(
        from_slice::<Edit>(&[0xa3, 0x00, 0x02, 0x01, 0xa1, 0x00, 0x01, 0x02, 0x00]),
        Err(Error::UnknownKey(2))
    );
    assert_eq!(
        from_slice::<Edit>(&[0xa2, 0x00, 0x02, 0x01, 0xa2, 0x00, 0x01, 0x01, 0x00]),
        Err(Error::UnknownKey(1))
    );
    assert_eq!(
        from_slice::<Edit>(&[0xa2, 0x00, 0x02, 0x01, 0xa0]),
        Err(Error::MissingKey(0))
    );
    assert_eq!(
        from_slice::<Edit>(&[0xa2, 0x00, 0x02, 0x01, 0x80]),
        Err(Error::Type)
    );
    assert_eq!(
        from_slice::<Edit>(&[0xa2, 0x00, 0x02, 0x02, 0xa1, 0x00, 0x01]),
        Err(Error::MissingKey(1))
    );
    assert_eq!(from_slice::<Edit>(&[0x82, 0x00, 0x00]), Err(Error::Type));
    let rotate_overflow = [0xa2, 0x00, 0x04, 0x01, 0xa1, 0x00, 0x19, 0x80, 0x00];
    assert_eq!(from_slice::<Edit>(&rotate_overflow), Err(Error::Range));
}

fn random_edit(rng: &mut Rng) -> Edit {
    let r = |rng: &mut Rng| Rect {
        x: rng.below(5000) as u32,
        y: rng.below(5000) as u32,
        w: 1 + rng.below(5000) as u32,
        h: 1 + rng.below(5000) as u32,
    };
    match rng.below(8) {
        0 => Edit::Resize {
            w: 1 + rng.below(9000) as u32,
            h: 1 + rng.below(9000) as u32,
            filter: rng.next() as u8,
        },
        1 => Edit::Recompress {
            format: ImageFormat::ALL[rng.below(5) as usize],
            quality: rng.below(101) as u8,
        },
        2 => Edit::Orient {
            orientation: rng.below(8) as u8,
        },
        3 => Edit::Crop(r(rng)),
        4 => Edit::Rotate {
            deg_e2: rng.below(2001) as i16 - 1000,
        },
        5 => Edit::Tone {
            exposure_e2: rng.next() as i16,
            contrast_e2: rng.next() as i16,
            saturation_e2: rng.next() as i16,
        },
        6 => Edit::Filter {
            id: rng.next() as u16,
            strength_e2: rng.next() as u16,
        },
        _ => Edit::Blur {
            rect: r(rng),
            class: BlurClass::ALL[rng.below(4) as usize],
            sigma_e2: rng.next() as u16,
        },
    }
}

#[test]
fn property_random_edits_roundtrip() {
    let mut rng = Rng::new(30);
    for _ in 0..3000 {
        let e = random_edit(&mut rng);
        assert!(e.is_valid());
        let b = to_vec(&e);
        assert_eq!(from_slice_canonical::<Edit>(&b), Ok(e));
    }
}

#[test]
fn property_mutated_edits_never_panic_and_accepted_are_canonical() {
    let mut rng = Rng::new(31);
    for _ in 0..5000 {
        let mut b = to_vec(&random_edit(&mut rng));
        let at = rng.below(b.len() as u64) as usize;
        b[at] = rng.next() as u8;
        if let Ok(e) = from_slice::<Edit>(&b) {
            assert_eq!(to_vec(&e), b);
            assert!(e.is_valid());
        }
    }
}
