use super::*;
use crate::testkit::{self as kit, Rng};

#[test]
fn reads_short_form_tlv() {
    let buf = [0x04, 0x03, 1, 2, 3, 0x05, 0x00];
    let mut r = Reader::new(&buf);
    let t = r.read().unwrap();
    assert_eq!(t.tag, Tag::OCTET_STRING);
    assert_eq!(t.value, &[1, 2, 3]);
    assert_eq!(t.raw, &buf[..5]);
    assert_eq!(r.expect(Tag::NULL).unwrap(), &[] as &[u8]);
    assert!(r.is_empty());
    r.finish().unwrap();
}

#[test]
fn reads_long_form_lengths() {
    for len in [0x80usize, 0xff, 0x100, 0x1234, 0x10000] {
        let content = std::vec![0xaa; len];
        let enc = kit::octets(&content);
        assert_eq!(single(&enc, Tag::OCTET_STRING).unwrap().len(), len);
    }
}

#[test]
fn rejects_non_minimal_and_indefinite_lengths() {
    for bad in [
        &[0x04, 0x81, 0x05, 1, 2, 3, 4, 5][..],
        &[0x04, 0x82, 0x00, 0x80],
        &[0x30, 0x80, 0x00, 0x00],
        &[0x04, 0x85, 1, 0, 0, 0, 0],
        &[0x04, 0x81],
        &[0x04],
        &[],
        &[0x04, 0x05, 1, 2],
        &[0x04, 0x84, 0xff, 0xff, 0xff, 0xff, 0],
    ] {
        assert_eq!(
            Reader::new(bad).read().map(|_| ()),
            Err(Error::Der),
            "{bad:x?}"
        );
    }
}

#[test]
fn reads_high_tag_numbers() {
    for n in [31u32, 127, 128, 701, 709, 16_383, 16_384, 2_000_000] {
        let enc = kit::explicit(n, &kit::int(5));
        let t = Reader::new(&enc).read().unwrap();
        assert_eq!(t.tag, Tag::context(n, true), "{n}");
        assert_eq!(uint(single(t.value, Tag::INTEGER).unwrap()), Ok(5));
    }
}

#[test]
fn rejects_bad_high_tags() {
    for bad in [
        &[0xbf, 0x80, 0x05, 0x00][..],
        &[0xbf, 0x1e, 0x00],
        &[0xbf, 0x85],
        &[0xbf],
        &[0xbf, 0xff, 0xff, 0xff, 0x7f, 0x00],
        &[0xbf, 0x81, 0x80, 0x80, 0x00, 0x00],
    ] {
        assert_eq!(
            Reader::new(bad).read().map(|_| ()),
            Err(Error::Der),
            "{bad:x?}"
        );
    }
}

#[test]
fn tag_class_and_constructed_bits() {
    let t = Reader::new(&[0xa3, 0x00]).read().unwrap().tag;
    assert_eq!(t, Tag::context(3, true));
    let t = Reader::new(&[0x81, 0x00]).read().unwrap().tag;
    assert_eq!(t, Tag::context(1, false));
    let t = Reader::new(&[0x43, 0x00]).read().unwrap().tag;
    assert_eq!((t.class, t.constructed, t.number), (1, false, 3));
    assert_eq!(Reader::new(&[0x30, 0x00]).peek_tag(), Some(Tag::SEQUENCE));
    assert_eq!(Reader::new(&[]).peek_tag(), None);
}

#[test]
fn expect_optional_and_finish() {
    let buf = kit::cat(&[&kit::int(1), &kit::boolean(true)]);
    let mut r = Reader::new(&buf);
    assert_eq!(r.optional(Tag::OCTET_STRING), Ok(None));
    assert!(r.optional(Tag::INTEGER).unwrap().is_some());
    assert_eq!(r.finish(), Err(Error::Der));
    let mut r = Reader::new(&buf);
    assert_eq!(r.expect(Tag::BOOLEAN), Err(Error::Der));
    assert_eq!(single(&buf, Tag::INTEGER), Err(Error::Der));
    assert_eq!(single(&kit::int(1), Tag::INTEGER), Ok(&[1u8][..]));
}

#[test]
fn integer_rules() {
    assert_eq!(uint(&[0]), Ok(0));
    assert_eq!(uint(&[0x7f]), Ok(127));
    assert_eq!(uint(&[0x00, 0x80]), Ok(128));
    assert_eq!(
        uint(&[0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
        Ok(u64::MAX)
    );
    assert_eq!(uint(&[]), Err(Error::Der));
    assert_eq!(uint(&[0x80]), Err(Error::Der));
    assert_eq!(uint(&[0x00, 0x01]), Err(Error::Der));
    assert_eq!(uint(&[0xff, 0x80]), Err(Error::Der));
    assert_eq!(uint(&[0x01, 0, 0, 0, 0, 0, 0, 0, 0]), Err(Error::Der));
    assert_eq!(integer_bytes(&[0xff, 0x7f]), Ok(&[0xff, 0x7f][..]));
    assert_eq!(
        unsigned_magnitude(&[0x00, 0x80, 0x01]),
        Ok(&[0x80, 0x01][..])
    );
    assert_eq!(unsigned_magnitude(&[0x81]), Err(Error::Der));
}

#[test]
fn uint_round_trips_random() {
    let mut rng = Rng::new(1);
    for _ in 0..5000 {
        let v = rng.next() >> rng.below(64);
        let enc = kit::int(v);
        assert_eq!(uint(single(&enc, Tag::INTEGER).unwrap()), Ok(v));
    }
}

#[test]
fn boolean_rules() {
    assert_eq!(boolean(&[0x00]), Ok(false));
    assert_eq!(boolean(&[0xff]), Ok(true));
    assert_eq!(boolean(&[0x01]), Err(Error::Der));
    assert_eq!(boolean(&[]), Err(Error::Der));
    assert_eq!(boolean(&[0xff, 0xff]), Err(Error::Der));
}

#[test]
fn bit_string_rules() {
    assert_eq!(bit_string(&[0x00]), Ok((0, &[][..])));
    assert_eq!(bit_string(&[0x00, 0xab]), Ok((0, &[0xab][..])));
    assert_eq!(bit_string(&[0x07, 0x80]), Ok((7, &[0x80][..])));
    assert_eq!(bit_string(&[0x01, 0x86]), Ok((1, &[0x86][..])));
    assert_eq!(bit_string(&[0x01, 0x87]), Err(Error::Der));
    assert_eq!(bit_string(&[0x08, 0x00]), Err(Error::Der));
    assert_eq!(bit_string(&[0x01]), Err(Error::Der));
    assert_eq!(bit_string(&[]), Err(Error::Der));
    assert_eq!(octet_aligned_bits(&[0x00, 1, 2]), Ok(&[1u8, 2][..]));
    assert_eq!(octet_aligned_bits(&[0x01, 0x02]), Err(Error::Der));
}

#[test]
fn random_bytes_never_panic_and_never_overrun() {
    let mut rng = Rng::new(99);
    for _ in 0..20_000 {
        let len = rng.below(48) as usize;
        let mut buf = std::vec![0u8; len];
        rng.fill(&mut buf);
        let mut r = Reader::new(&buf);
        let mut consumed = 0;
        while let Ok(t) = r.read() {
            assert!(t.raw.len() >= 2 && t.raw.ends_with(t.value));
            consumed += t.raw.len();
            assert!(consumed <= len);
        }
    }
}

#[test]
fn nested_round_trip() {
    let inner = kit::seq(&[
        &kit::int(7),
        &kit::octets(b"abc"),
        &kit::explicit(709, &kit::octets(b"x")),
    ]);
    let outer = kit::seq(&[&inner, &kit::null()]);
    let mut r = Reader::new(single(&outer, Tag::SEQUENCE).unwrap());
    let mut i = Reader::new(r.expect(Tag::SEQUENCE).unwrap());
    assert_eq!(uint(i.expect(Tag::INTEGER).unwrap()), Ok(7));
    assert_eq!(i.expect(Tag::OCTET_STRING), Ok(&b"abc"[..]));
    assert_eq!(
        single(
            i.expect(Tag::context(709, true)).unwrap(),
            Tag::OCTET_STRING
        ),
        Ok(&b"x"[..])
    );
    i.finish().unwrap();
    r.expect(Tag::NULL).unwrap();
    r.finish().unwrap();
}
