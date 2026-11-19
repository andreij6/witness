use super::*;
use crate::testkit::Rng;
use alloc::vec;

fn raw_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    write_box(&mut v, *kind, payload).unwrap();
    v
}

fn desc_bytes(uuid: &Uuid, toggles: u8, tail: &[u8]) -> Vec<u8> {
    let mut v = uuid.to_vec();
    v.push(toggles);
    v.extend_from_slice(tail);
    v
}

#[test]
fn iso_uuids_match_c2pa_constants() {
    assert_eq!(&uuid::STORE[..4], b"c2pa");
    assert_eq!(&uuid::MANIFEST[..4], b"c2ma");
    assert_eq!(&uuid::ASSERTIONS[..4], b"c2as");
    assert_eq!(&uuid::CLAIM[..4], b"c2cl");
    assert_eq!(&uuid::SIGNATURE[..4], b"c2cs");
    assert_eq!(&uuid::CBOR[..4], b"cbor");
    assert_eq!(&uuid::JSON[..4], b"json");
    assert_eq!(&uuid::UPDATE_MANIFEST[..4], b"c2um");
    for u in [uuid::STORE, uuid::CBOR, uuid::CLAIM] {
        assert_eq!(u[4..], ISO_SUFFIX);
    }
    assert_eq!(
        uuid::EMBEDDED_FILE,
        [
            0x40, 0xCB, 0x0C, 0x32, 0xBB, 0x8A, 0x48, 0x9D, 0xA7, 0x0B, 0x2A, 0xD6, 0xF4, 0x7F,
            0x43, 0x69
        ]
    );
}

#[test]
fn split_box_reads_header_and_rest() {
    let mut data = raw_box(b"cbor", &[1, 2, 3]);
    data.extend_from_slice(&[9, 9]);
    let (b, rest) = split_box(&data).unwrap();
    assert_eq!(b.kind, CBOR);
    assert_eq!(b.payload, &[1, 2, 3]);
    assert_eq!(rest, &[9, 9]);
}

#[test]
fn split_box_accepts_empty_payload() {
    let data = raw_box(b"json", &[]);
    assert_eq!(data.len(), 8);
    let (b, rest) = split_box(&data).unwrap();
    assert!(b.payload.is_empty() && rest.is_empty());
}

#[test]
fn split_box_rejects_bad_lengths() {
    for lbox in [0u32, 2, 3, 4, 5, 6, 7] {
        let mut d = lbox.to_be_bytes().to_vec();
        d.extend_from_slice(b"cbor");
        d.extend_from_slice(&[0; 8]);
        assert_eq!(split_box(&d), Err(Error::MalformedJumbf), "lbox {lbox}");
    }
    let mut d = 20u32.to_be_bytes().to_vec();
    d.extend_from_slice(b"cbor");
    d.extend_from_slice(&[0; 11]);
    assert_eq!(split_box(&d), Err(Error::MalformedJumbf));
}

#[test]
fn split_box_rejects_truncated_header() {
    for n in 0..8 {
        assert_eq!(
            split_box(&[0, 0, 0, 8, b'c', b'b', b'o'][..n.min(7)]),
            Err(Error::MalformedJumbf)
        );
    }
}

#[test]
fn split_box_supports_xlbox() {
    let mut d = 1u32.to_be_bytes().to_vec();
    d.extend_from_slice(b"bidb");
    d.extend_from_slice(&19u64.to_be_bytes());
    d.extend_from_slice(&[5, 6, 7]);
    let (b, rest) = split_box(&d).unwrap();
    assert_eq!(b.kind, BIDB);
    assert_eq!(b.payload, &[5, 6, 7]);
    assert!(rest.is_empty());
}

#[test]
fn split_box_rejects_bad_xlbox() {
    for xl in [0u64, 8, 15, 20, u64::MAX] {
        let mut d = 1u32.to_be_bytes().to_vec();
        d.extend_from_slice(b"bidb");
        d.extend_from_slice(&xl.to_be_bytes());
        d.extend_from_slice(&[5, 6, 7]);
        assert_eq!(split_box(&d), Err(Error::MalformedJumbf), "xl {xl}");
    }
    let mut d = 1u32.to_be_bytes().to_vec();
    d.extend_from_slice(b"bidb");
    d.extend_from_slice(&[0, 0, 0]);
    assert_eq!(split_box(&d), Err(Error::MalformedJumbf));
}

#[test]
fn single_box_rejects_trailing_bytes() {
    let mut d = raw_box(b"cbor", &[1]);
    assert!(single_box(&d).is_ok());
    d.push(0);
    assert_eq!(single_box(&d), Err(Error::MalformedJumbf));
}

#[test]
fn boxes_iterates_and_stops_after_error() {
    let mut d = raw_box(b"cbor", &[1]);
    d.extend(raw_box(b"json", &[2, 3]));
    let kinds: Vec<_> = Boxes::new(&d).map(|b| b.unwrap().kind).collect();
    assert_eq!(kinds, vec![CBOR, JSON]);
    d.extend_from_slice(&[0, 0, 0, 99, b'x']);
    let items: Vec<_> = Boxes::new(&d).collect();
    assert_eq!(items.len(), 3);
    assert_eq!(items[2], Err(Error::MalformedJumbf));
    assert_eq!(Boxes::new(&[]).count(), 0);
}

#[test]
fn description_round_trips_label() {
    let mut out = Vec::new();
    write_description(&mut out, &uuid::CLAIM, "c2pa.claim.v2").unwrap();
    let b = single_box(&out).unwrap();
    assert_eq!(b.kind, JUMD);
    let d = Description::parse(b.payload).unwrap();
    assert_eq!(d.uuid, uuid::CLAIM);
    assert_eq!(d.toggles, 0x03);
    assert_eq!(d.label, Some("c2pa.claim.v2"));
    assert_eq!((d.id, d.hash, d.private), (None, None, None));
}

#[test]
fn description_parses_every_optional_field() {
    let mut tail = b"lbl\0".to_vec();
    tail.extend_from_slice(&7u32.to_be_bytes());
    tail.extend_from_slice(&[0xaa; 32]);
    tail.extend(raw_box(b"c2sh", &[1, 2, 3]));
    let bytes = desc_bytes(&uuid::CBOR, 0x1f, &tail);
    let d = Description::parse(&bytes).unwrap();
    assert_eq!(d.label, Some("lbl"));
    assert_eq!(d.id, Some(7));
    assert_eq!(d.hash, Some(&[0xaa; 32]));
    assert_eq!(d.private.unwrap().kind, *b"c2sh");
    assert_eq!(d.private.unwrap().payload, &[1, 2, 3]);
}

#[test]
fn description_without_label() {
    let bytes = desc_bytes(&uuid::JSON, 0x01, &[]);
    let d = Description::parse(&bytes).unwrap();
    assert_eq!(d.label, None);
}

#[test]
fn description_rejects_malformed() {
    let cases: Vec<Vec<u8>> = vec![
        uuid::CBOR[..15].to_vec(),
        uuid::CBOR.to_vec(),
        desc_bytes(&uuid::CBOR, 0x20, b"x\0"),
        desc_bytes(&uuid::CBOR, 0x03, b"no-terminator"),
        desc_bytes(&uuid::CBOR, 0x03, &[0xff, 0xfe, 0]),
        desc_bytes(&uuid::CBOR, 0x07, b"l\0\0\0"),
        desc_bytes(&uuid::CBOR, 0x0b, &[b'l', 0, 1, 2]),
        desc_bytes(&uuid::CBOR, 0x13, b"l\0\0\0\0\x09ab"),
        desc_bytes(&uuid::CBOR, 0x03, b"l\0trailing"),
        desc_bytes(&uuid::CBOR, 0x01, b"x"),
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_eq!(
            Description::parse(c),
            Err(Error::MalformedJumbf),
            "case {i}"
        );
    }
}

#[test]
fn write_description_rejects_nul_in_label() {
    assert_eq!(
        write_description(&mut Vec::new(), &uuid::CBOR, "a\0b"),
        Err(Error::MalformedJumbf)
    );
}

fn nested() -> Vec<u8> {
    let mut out = Vec::new();
    write_superbox(&mut out, &uuid::STORE, "c2pa", |o| {
        write_superbox(o, &uuid::CBOR, "inner.a", |o| write_box(o, CBOR, &[0xa0]))?;
        write_superbox(o, &uuid::JSON, "inner.b", |o| write_box(o, JSON, b"{}"))?;
        Ok(())
    })
    .unwrap();
    out
}

#[test]
fn superbox_round_trip() {
    let data = nested();
    let sb = SuperBox::from_bytes(&data).unwrap();
    assert_eq!(sb.desc.uuid, uuid::STORE);
    assert_eq!(sb.label(), Some("c2pa"));
    assert_eq!(sb.payload, &data[8..]);
    let kids: Vec<_> = sb.superboxes().map(|s| s.unwrap()).collect();
    assert_eq!(kids.len(), 2);
    assert_eq!(kids[0].label(), Some("inner.a"));
    assert_eq!(kids[0].only_child(CBOR), Ok(&[0xa0][..]));
    assert_eq!(kids[1].only_child(JSON), Ok(&b"{}"[..]));
    assert_eq!(kids[1].only_child(CBOR), Err(Error::MalformedJumbf));
    assert_eq!(sb.children().count(), 2);
}

#[test]
fn superbox_payload_points_after_header() {
    let mut out = vec![1, 2, 3];
    let at = write_superbox(&mut out, &uuid::CBOR, "x", |o| write_box(o, CBOR, &[1])).unwrap();
    assert_eq!(at, 3);
    assert_eq!(superbox_payload(&out, at), &out[11..]);
    assert_eq!(
        SuperBox::from_bytes(&out[3..]).unwrap().payload,
        superbox_payload(&out, at)
    );
}

#[test]
fn superbox_rejects_wrong_kinds() {
    assert_eq!(
        SuperBox::from_bytes(&raw_box(b"cbor", &[])),
        Err(Error::MalformedJumbf)
    );
    let inner = raw_box(b"cbor", &[1]);
    assert_eq!(
        SuperBox::from_bytes(&raw_box(b"jumb", &inner)),
        Err(Error::MalformedJumbf)
    );
    assert_eq!(
        SuperBox::from_bytes(&raw_box(b"jumb", &[])),
        Err(Error::MalformedJumbf)
    );
}

#[test]
fn embedded_file_description_round_trips() {
    for (name, external) in [
        (None, false),
        (Some("thumb.jpg"), false),
        (Some("x"), true),
        (None, true),
    ] {
        let d = EmbeddedFileDescription {
            media_type: "image/jpeg",
            file_name: name,
            external,
        };
        let mut out = Vec::new();
        d.write(&mut out).unwrap();
        let b = single_box(&out).unwrap();
        assert_eq!(b.kind, BFDB);
        assert_eq!(EmbeddedFileDescription::parse(b.payload), Ok(d));
    }
}

#[test]
fn embedded_file_description_rejects_malformed() {
    for c in [
        &[][..],
        &[0x04, b'a', 0][..],
        &[0x00, b'a'][..],
        &[0x01, b'a', 0, b'n'][..],
        &[0x00, b'a', 0, b'x'][..],
        &[0x00, 0xff, 0][..],
    ] {
        assert_eq!(
            EmbeddedFileDescription::parse(c),
            Err(Error::MalformedJumbf)
        );
    }
    let bad = EmbeddedFileDescription {
        media_type: "a\0b",
        file_name: None,
        external: false,
    };
    assert_eq!(bad.write(&mut Vec::new()), Err(Error::MalformedJumbf));
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = Rng::new(1);
    for _ in 0..3000 {
        let len = rng.below(64);
        let mut d = rng.bytes(len);
        if len >= 8 && rng.below(2) == 0 {
            d[..4].copy_from_slice(&(len as u32).to_be_bytes());
            d[4..8].copy_from_slice(b"jumb");
        }
        let _ = SuperBox::from_bytes(&d);
        let _ = Description::parse(&d);
        let _ = EmbeddedFileDescription::parse(&d);
        let _ = Boxes::new(&d).count();
    }
}

#[test]
fn every_truncation_of_valid_tree_is_rejected() {
    let data = nested();
    for n in 0..data.len() {
        assert!(SuperBox::from_bytes(&data[..n]).is_err(), "len {n}");
    }
}

#[test]
fn random_mutations_parse_or_error_without_panic() {
    let data = nested();
    let mut rng = Rng::new(2);
    for _ in 0..3000 {
        let mut d = data.clone();
        let i = rng.below(d.len());
        d[i] ^= 1 << rng.below(8);
        if let Ok(sb) = SuperBox::from_bytes(&d) {
            for k in sb.superboxes().flatten() {
                let _ = k.only_child(CBOR);
            }
        }
    }
}

#[test]
fn write_is_deterministic() {
    assert_eq!(nested(), nested());
}
