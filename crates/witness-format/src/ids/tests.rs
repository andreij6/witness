use super::*;
use crate::cbor::{from_slice, to_vec};
use crate::testutil::Rng;
use alloc::string::ToString;

#[test]
fn capture_id_layout() {
    let id = CaptureId::new(0x00ab_cdef_0123_4567, 0x1f).unwrap();
    assert_eq!(id.to_u64(), 0x1fab_cdef_0123_4567);
    assert_eq!(id.schema(), 0x1f);
    assert_eq!(id.random(), 0x00ab_cdef_0123_4567);
    assert_eq!(CaptureId::new(0, 0).unwrap().to_u64(), 0);
    assert_eq!(
        CaptureId::new(CaptureId::RANDOM_MASK, 31).unwrap().to_u64(),
        CaptureId::MAX
    );
    assert_eq!(CaptureId::MAX, (1 << 61) - 1);
}

#[test]
fn capture_id_rejects_out_of_range() {
    assert_eq!(CaptureId::new(1 << 56, 0), None);
    assert_eq!(CaptureId::new(0, 32), None);
    assert_eq!(CaptureId::new(u64::MAX, 0), None);
    assert_eq!(CaptureId::from_u64(1 << 61), None);
    assert_eq!(CaptureId::from_u64(u64::MAX), None);
    assert_eq!(
        CaptureId::from_u64(CaptureId::MAX).map(CaptureId::to_u64),
        Some(CaptureId::MAX)
    );
}

#[test]
fn capture_id_from_random_bytes() {
    let id = CaptureId::from_random_bytes([1, 2, 3, 4, 5, 6, 7], 3).unwrap();
    assert_eq!(id.random(), 0x0001_0203_0405_0607);
    assert_eq!(id.schema(), 3);
    assert_eq!(CaptureId::from_random_bytes([0xff; 7], 32), None);
}

#[test]
fn capture_id_cbor() {
    let id = CaptureId::new(5, 1).unwrap();
    assert_eq!(to_vec(&id), [0x1b, 0x01, 0, 0, 0, 0, 0, 0, 0x05]);
    assert_eq!(from_slice::<CaptureId>(&to_vec(&id)), Ok(id));
    assert_eq!(to_vec(&CaptureId::new(7, 0).unwrap()), [0x07]);
    let too_big = to_vec(&(1u64 << 61));
    assert_eq!(from_slice::<CaptureId>(&too_big), Err(Error::Range));
    assert_eq!(from_slice::<CaptureId>(&[0x20]), Err(Error::Type));
}

#[test]
fn capture_id_property_roundtrip() {
    let mut rng = Rng::new(20);
    for _ in 0..2000 {
        let r = rng.next() & CaptureId::RANDOM_MASK;
        let s = rng.below(32) as u8;
        let id = CaptureId::new(r, s).unwrap();
        assert_eq!((id.random(), id.schema()), (r, s));
        assert_eq!(CaptureId::from_u64(id.to_u64()), Some(id));
        assert_eq!(from_slice::<CaptureId>(&to_vec(&id)), Ok(id));
    }
}

#[test]
fn record_id_golden() {
    let id = RecordId::from_claim_cbor(b"");
    assert_eq!(
        id.as_bytes(),
        &[0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb]
    );
    assert_eq!(id.to_string(), "WERC8GMRZGE196QV");
    assert_eq!(&id.to_grouped(), b"WERC-8GMR-ZGE1-96QV");
    assert_eq!(id.grouped(), "WERC-8GMR-ZGE1-96QV");
    assert_eq!(
        RecordId::from_bytes([0; 10]).to_string(),
        "0000000000000000"
    );
    assert_eq!(
        RecordId::from_bytes([0xff; 10]).to_string(),
        "ZZZZZZZZZZZZZZZZ"
    );
}

#[test]
fn record_id_parse_variants() {
    let id = RecordId::from_claim_cbor(b"");
    for text in [
        "WERC8GMRZGE196QV",
        "werc-8gmr-zge1-96qv",
        "WERC-8GMR-ZGEI-96QV",
        "WERC-8GMR-ZGEL-96QV",
        "wErC8gMrZgEl96Qv",
        "-WERC--8GMR-ZGE1-96QV-",
    ] {
        assert_eq!(text.parse::<RecordId>(), Ok(id), "{text}");
    }
    assert_eq!(
        RecordId::parse("0OOO-oooo-0000-0000"),
        Ok(RecordId::from_bytes([0; 10]))
    );
}

#[test]
fn record_id_parse_rejects() {
    assert_eq!(
        RecordId::parse("WERC8GMRZGE196Q"),
        Err(IdParseError::Base32(Base32Error::Length))
    );
    assert_eq!(
        RecordId::parse("WERC8GMRZGE196QVV"),
        Err(IdParseError::Base32(Base32Error::Length))
    );
    assert_eq!(
        RecordId::parse("WERC8GMRZGE196QU"),
        Err(IdParseError::Base32(Base32Error::Symbol(b'U')))
    );
    assert_eq!(
        RecordId::parse("WERC 8GMR ZGE1 96QV"),
        Err(IdParseError::Base32(Base32Error::Symbol(b' ')))
    );
    assert_eq!(
        RecordId::parse(""),
        Err(IdParseError::Base32(Base32Error::Length))
    );
    assert!(RecordId::parse("é234567890123456").is_err());
    assert!(
        !IdParseError::Base32(Base32Error::Length)
            .to_string()
            .is_empty()
    );
}

#[test]
fn record_id_property_roundtrip() {
    let mut rng = Rng::new(21);
    for _ in 0..2000 {
        let id = RecordId::from_bytes(rng.bytes());
        let text = id.to_string();
        assert_eq!(text.len(), RecordId::TEXT_LEN);
        assert_eq!(text.parse::<RecordId>(), Ok(id));
        assert_eq!(id.grouped().parse::<RecordId>(), Ok(id));
        assert_eq!(text.to_ascii_lowercase().parse::<RecordId>(), Ok(id));
        assert_eq!(from_slice::<RecordId>(&to_vec(&id)), Ok(id));
    }
}

#[test]
fn record_id_text_order_matches_byte_order() {
    let mut rng = Rng::new(22);
    for _ in 0..1000 {
        let a = RecordId::from_bytes(rng.bytes());
        let b = RecordId::from_bytes(rng.bytes());
        assert_eq!(a.cmp(&b), a.to_base32().cmp(&b.to_base32()));
    }
}

#[test]
fn byte_ids_cbor_and_conversions() {
    let id = RecordId::from_bytes([7; 10]);
    let enc = to_vec(&id);
    assert_eq!(enc[0], 0x4a);
    assert_eq!(&enc[1..], &[7; 10]);
    assert_eq!(
        from_slice::<RecordId>(&[0x49, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        Err(Error::Length)
    );
    assert_eq!(
        RecordId::try_from(&[1u8; 10][..]),
        Ok(RecordId::from_bytes([1; 10]))
    );
    assert_eq!(RecordId::try_from(&[1u8; 11][..]), Err(Error::Length));
    assert_eq!(id.to_bytes(), [7; 10]);
    let k = KeyId::from_bytes([9; 16]);
    assert_eq!(from_slice::<KeyId>(&to_vec(&k)), Ok(k));
    assert_eq!(to_vec(&k)[0], 0x50);
    assert_eq!(KeyId::try_from(&[0u8; 15][..]), Err(Error::Length));
}

#[test]
fn key_id_from_spki() {
    let id = KeyId::from_spki(b"abc");
    assert_eq!(
        id.as_bytes(),
        &[
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23
        ]
    );
    assert_ne!(KeyId::from_spki(b"abd"), id);
}

#[test]
fn case_id_layout_and_text() {
    let id = CaseId::new(3, [0xaa; 15]);
    assert_eq!(id.shard(), 3);
    assert_eq!(id.random(), &[0xaa; 15]);
    assert_eq!(id.as_bytes()[0], 3);
    let text = id.to_string();
    assert_eq!(text.len(), CaseId::TEXT_LEN);
    assert_eq!(text.parse::<CaseId>(), Ok(id));
    assert_eq!(
        CaseId::new(0, [0; 15]).to_string(),
        "00000000000000000000000000"
    );
    assert_eq!(
        CaseId::new(255, [0xff; 15]).to_string(),
        "ZZZZZZZZZZZZZZZZZZZZZZZZZW"
    );
    assert_eq!(
        CaseId::parse("ZZZZZZZZZZZZZZZZZZZZZZZZZZ"),
        Err(IdParseError::Base32(Base32Error::Padding))
    );
    assert_eq!(from_slice::<CaseId>(&to_vec(&id)), Ok(id));
}

#[test]
fn case_id_property() {
    let mut rng = Rng::new(23);
    for _ in 0..1000 {
        let shard = rng.next() as u8;
        let id = CaseId::new(shard, rng.bytes());
        assert_eq!(id.shard(), shard);
        assert_eq!(id.to_string().parse::<CaseId>(), Ok(id));
    }
}
