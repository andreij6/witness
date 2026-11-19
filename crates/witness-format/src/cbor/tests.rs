use super::*;
use crate::testutil::Rng;
use alloc::vec;
use alloc::vec::Vec;

fn hex(s: &str) -> Vec<u8> {
    let s: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    s.chunks(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

fn enc_value(v: &Value<'_>) -> Vec<u8> {
    v.to_vec().unwrap()
}

#[test]
fn rfc8949_appendix_a_unsigned() {
    let cases: &[(u64, &str)] = &[
        (0, "00"),
        (1, "01"),
        (10, "0a"),
        (23, "17"),
        (24, "1818"),
        (25, "1819"),
        (100, "1864"),
        (1000, "1903e8"),
        (1_000_000, "1a000f4240"),
        (1_000_000_000_000, "1b000000e8d4a51000"),
        (u64::MAX, "1bffffffffffffffff"),
    ];
    for &(v, h) in cases {
        assert_eq!(to_vec(&v), hex(h), "{v}");
        assert_eq!(from_slice::<u64>(&hex(h)), Ok(v));
    }
}

#[test]
fn rfc8949_appendix_a_negative() {
    let cases: &[(i64, &str)] = &[
        (-1, "20"),
        (-10, "29"),
        (-100, "3863"),
        (-1000, "3903e7"),
        (i64::MIN, "3b7fffffffffffffff"),
    ];
    for &(v, h) in cases {
        assert_eq!(to_vec(&v), hex(h), "{v}");
        assert_eq!(from_slice::<i64>(&hex(h)), Ok(v));
    }
    assert_eq!(enc_value(&Value::Nint(u64::MAX)), hex("3bffffffffffffffff"));
    assert_eq!(
        from_slice::<i64>(&hex("3bffffffffffffffff")),
        Err(Error::Range)
    );
    assert_eq!(
        from_slice::<i64>(&hex("3b8000000000000000")),
        Err(Error::Range)
    );
    assert_eq!(
        from_slice::<i64>(&hex("1b8000000000000000")),
        Err(Error::Range)
    );
    assert_eq!(from_slice::<i64>(&hex("1b7fffffffffffffff")), Ok(i64::MAX));
}

#[test]
fn rfc8949_appendix_a_simple_strings_containers() {
    let cases: &[(Value<'_>, &str)] = &[
        (Value::Bool(false), "f4"),
        (Value::Bool(true), "f5"),
        (Value::Null, "f6"),
        (Value::Bytes(&[]), "40"),
        (Value::Bytes(&[1, 2, 3, 4]), "4401020304"),
        (Value::Text(""), "60"),
        (Value::Text("a"), "6161"),
        (Value::Text("IETF"), "6449455446"),
        (Value::Text("\"\\"), "62225c"),
        (Value::Text("\u{00fc}"), "62c3bc"),
        (Value::Text("\u{6c34}"), "63e6b0b4"),
        (Value::Array(vec![]), "80"),
        (
            Value::Array(vec![Value::Uint(1), Value::Uint(2), Value::Uint(3)]),
            "83010203",
        ),
        (
            Value::Array(vec![
                Value::Uint(1),
                Value::Array(vec![Value::Uint(2), Value::Uint(3)]),
                Value::Array(vec![Value::Uint(4), Value::Uint(5)]),
            ]),
            "8301820203820405",
        ),
        (Value::Map(vec![]), "a0"),
        (
            Value::Map(vec![
                (Value::Uint(1), Value::Uint(2)),
                (Value::Uint(3), Value::Uint(4)),
            ]),
            "a201020304",
        ),
        (
            Value::Map(vec![
                (Value::Text("a"), Value::Uint(1)),
                (
                    Value::Text("b"),
                    Value::Array(vec![Value::Uint(2), Value::Uint(3)]),
                ),
            ]),
            "a26161016162820203",
        ),
        (
            Value::Array(vec![
                Value::Text("a"),
                Value::Map(vec![(Value::Text("b"), Value::Text("c"))]),
            ]),
            "826161a161626163",
        ),
    ];
    for (v, h) in cases {
        let bytes = hex(h);
        assert_eq!(&enc_value(v), &bytes, "{h}");
        assert_eq!(from_slice::<Value<'_>>(&bytes).as_ref(), Ok(v), "{h}");
        assert_eq!(validate(&bytes), Ok(()));
    }
}

#[test]
fn rfc8949_appendix_a_25_element_array() {
    let items: Vec<Value<'_>> = (1..=25).map(Value::Uint).collect();
    let bytes = enc_value(&Value::Array(items.clone()));
    assert_eq!(
        bytes,
        hex("98190102030405060708090a0b0c0d0e0f101112131415161718181819")
    );
    assert_eq!(from_slice::<Value<'_>>(&bytes), Ok(Value::Array(items)));
}

#[test]
fn length_boundaries_use_shortest_head() {
    let cases: &[(u64, usize)] = &[
        (0, 1),
        (23, 1),
        (24, 2),
        (255, 2),
        (256, 3),
        (65_535, 3),
        (65_536, 5),
        (u32::MAX as u64, 5),
        (u32::MAX as u64 + 1, 9),
        (u64::MAX, 9),
    ];
    for &(v, len) in cases {
        let b = to_vec(&v);
        assert_eq!(b.len(), len, "{v}");
        assert_eq!(from_slice::<u64>(&b), Ok(v));
        let n = enc_value(&Value::Nint(v));
        assert_eq!(n.len(), len);
        assert_eq!(n[0] >> 5, 1);
    }
}

#[test]
fn byte_and_text_lengths_at_boundaries() {
    for len in [0usize, 1, 23, 24, 255, 256, 65_535, 65_536] {
        let data = vec![0x5au8; len];
        let b = to_vec(data.as_slice());
        assert_eq!(from_slice::<&[u8]>(&b), Ok(data.as_slice()));
        let text = "z".repeat(len);
        let t = to_vec(text.as_str());
        assert_eq!(from_slice::<&str>(&t), Ok(text.as_str()));
        assert_eq!(b.len(), t.len());
    }
}

#[test]
fn malformed_input_table() {
    let cases: &[(&str, Error)] = &[
        ("", Error::Eof),
        ("18", Error::Eof),
        ("19 01", Error::Eof),
        ("1a 01 02 03", Error::Eof),
        ("1b 01 02 03 04 05 06 07", Error::Eof),
        ("18 17", Error::NonMinimal),
        ("18 00", Error::NonMinimal),
        ("19 00 ff", Error::NonMinimal),
        ("19 00 00", Error::NonMinimal),
        ("1a 00 00 ff ff", Error::NonMinimal),
        ("1b 00 00 00 00 ff ff ff ff", Error::NonMinimal),
        ("38 10", Error::NonMinimal),
        ("39 00 10", Error::NonMinimal),
        ("58 01 aa", Error::NonMinimal),
        ("78 00", Error::NonMinimal),
        ("98 00", Error::NonMinimal),
        ("b8 00", Error::NonMinimal),
        ("59 00 01 aa", Error::NonMinimal),
        ("1c", Error::Reserved),
        ("1d", Error::Reserved),
        ("1e", Error::Reserved),
        ("3c", Error::Reserved),
        ("5d", Error::Reserved),
        ("1f", Error::Indefinite),
        ("3f", Error::Indefinite),
        ("5f 41 00 ff", Error::Indefinite),
        ("7f 61 61 ff", Error::Indefinite),
        ("9f 01 ff", Error::Indefinite),
        ("bf 01 02 ff", Error::Indefinite),
        ("ff", Error::Indefinite),
        ("c0 00", Error::Tag),
        ("c1 1a 00 01 00 00", Error::Tag),
        ("c2 41 01", Error::Tag),
        ("d8 20 60", Error::Tag),
        ("d9 d9 f7 00", Error::Tag),
        ("f9 00 00", Error::Float),
        ("f9 3c 00", Error::Float),
        ("fa 47 c3 50 00", Error::Float),
        ("fb 3f f1 99 99 99 99 99 9a", Error::Float),
        ("f7", Error::Simple),
        ("f0", Error::Simple),
        ("e0", Error::Simple),
        ("f8 20", Error::Simple),
        ("f8 ff", Error::Simple),
        ("fc", Error::Reserved),
        ("fd", Error::Reserved),
        ("fe", Error::Reserved),
        ("00 00", Error::Trailing),
        ("80 00", Error::Trailing),
        ("f6 f6", Error::Trailing),
        ("42 01", Error::Eof),
        ("62 61", Error::Eof),
        ("62 c3 28", Error::Utf8),
        ("61 ff", Error::Utf8),
        ("61 80", Error::Utf8),
        ("63 ed a0 80", Error::Utf8),
        ("62 c0 80", Error::Utf8),
        ("64 f4 90 80 80", Error::Utf8),
        ("81", Error::Eof),
        ("82 01", Error::Eof),
        ("a1 01", Error::Eof),
        ("a1", Error::Eof),
        ("a2 01 02 01 03", Error::KeyOrder),
        ("a2 03 04 01 02", Error::KeyOrder),
        ("a2 61 62 01 61 61 02", Error::KeyOrder),
        ("a2 18 18 01 17 02", Error::KeyOrder),
        ("a2 20 01 00 02", Error::KeyOrder),
        ("a2 61 61 01 01 02", Error::KeyOrder),
        ("a2 41 01 01 41 01 02", Error::KeyOrder),
        ("a2 62 61 61 00 61 62 00", Error::KeyOrder),
        ("a2 80 00 80 00", Error::KeyOrder),
        ("a3 01 00 02 00 02 00", Error::KeyOrder),
        ("81 a2 02 00 01 00", Error::KeyOrder),
        ("5b ff ff ff ff ff ff ff ff", Error::Eof),
        ("7b 00 00 00 01 00 00 00 00", Error::Eof),
        ("9b ff ff ff ff ff ff ff ff", Error::Eof),
        ("bb ff ff ff ff ff ff ff ff", Error::Eof),
        ("9a ff ff ff ff 00", Error::Eof),
        ("ba 00 01 00 00 00 00", Error::Eof),
        ("83 01 02", Error::Eof),
        ("81 c0 00", Error::Tag),
        ("a1 00 f9 00 00", Error::Float),
        ("82 01 9f ff", Error::Indefinite),
        ("a1 c0 00 00", Error::Tag),
        ("a1 f7 00", Error::Simple),
    ];
    for (h, want) in cases {
        let bytes = hex(h);
        assert_eq!(validate(&bytes), Err(*want), "validate {h}");
        assert_eq!(
            from_slice::<Value<'_>>(&bytes).err(),
            Some(*want),
            "value {h}"
        );
    }
}

#[test]
fn nesting_depth_is_bounded() {
    let ok_depth = MAX_DEPTH as usize;
    let mut ok = vec![0x81u8; ok_depth];
    ok.push(0x00);
    assert_eq!(validate(&ok), Ok(()));
    assert!(from_slice::<Value<'_>>(&ok).is_ok());
    let mut deep = vec![0x81u8; ok_depth + 1];
    deep.push(0x00);
    assert_eq!(validate(&deep), Err(Error::Depth));
    assert_eq!(from_slice::<Value<'_>>(&deep).err(), Some(Error::Depth));
    let mut maps = Vec::new();
    for _ in 0..=ok_depth {
        maps.extend_from_slice(&[0xa1, 0x00]);
    }
    maps.push(0x00);
    assert_eq!(validate(&maps), Err(Error::Depth));
    let huge = vec![0x81u8; 100_000];
    assert_eq!(validate(&huge), Err(Error::Depth));
}

#[test]
fn typed_ranges_are_enforced() {
    assert_eq!(from_slice::<u8>(&hex("18ff")), Ok(255));
    assert_eq!(from_slice::<u8>(&hex("190100")), Err(Error::Range));
    assert_eq!(from_slice::<u16>(&hex("1a00010000")), Err(Error::Range));
    assert_eq!(
        from_slice::<u32>(&hex("1b0000000100000000")),
        Err(Error::Range)
    );
    assert_eq!(from_slice::<i8>(&hex("387f")), Ok(-128));
    assert_eq!(from_slice::<i8>(&hex("3880")), Err(Error::Range));
    assert_eq!(from_slice::<i8>(&hex("1880")), Err(Error::Range));
    assert_eq!(from_slice::<i16>(&hex("397fff")), Ok(i16::MIN));
    assert_eq!(from_slice::<i32>(&hex("3a7fffffff")), Ok(i32::MIN));
    assert_eq!(from_slice::<u8>(&hex("20")), Err(Error::Type));
    assert_eq!(from_slice::<bool>(&hex("00")), Err(Error::Type));
    assert_eq!(from_slice::<bool>(&hex("f6")), Err(Error::Type));
    assert_eq!(from_slice::<[u8; 2]>(&hex("43010203")), Err(Error::Length));
    assert_eq!(from_slice::<[u8; 2]>(&hex("420102")), Ok([1, 2]));
    assert_eq!(from_slice::<&str>(&hex("4161")), Err(Error::Type));
    assert_eq!(from_slice::<&[u8]>(&hex("6161")), Err(Error::Type));
    assert_eq!(from_slice::<(u8, u8)>(&hex("83010203")), Err(Error::Length));
    assert_eq!(from_slice::<(u8, bool)>(&hex("8201f5")), Ok((1, true)));
    assert_eq!(from_slice::<Vec<u8>>(&hex("83010203")), Ok(vec![1, 2, 3]));
    assert_eq!(
        from_slice::<Vec<u8>>(&hex("8301021901ff")),
        Err(Error::Range)
    );
    assert_eq!(
        from_slice::<String>(&hex("6449455446")),
        Ok(String::from("IETF"))
    );
}

#[test]
fn decoder_null_and_peek() {
    let b = hex("f6");
    let mut d = Decoder::new(&b);
    assert_eq!(d.peek_major(), Ok(7));
    assert_eq!(d.null(), Ok(()));
    assert_eq!(d.peek_major(), Err(Error::Eof));
    assert!(d.is_empty());
    assert_eq!(d.position(), 1);
    assert_eq!(d.remaining(), 0);
    let mut d = Decoder::new(&[0xf5]);
    assert_eq!(d.null(), Err(Error::Type));
}

#[test]
fn raw_item_returns_exact_span() {
    let b = hex("82 a1 01 02 43 01 02 03 00");
    let mut d = Decoder::new(&b);
    assert_eq!(d.array(), Ok(2));
    assert_eq!(d.raw_item(), Ok(&b[1..4]));
    assert_eq!(d.raw_item(), Ok(&b[4..8]));
    assert_eq!(d.finish(), Err(Error::Trailing));
    assert_eq!(d.raw_item(), Ok(&b[8..9]));
    assert_eq!(d.finish(), Ok(()));
}

#[test]
fn value_encoder_sorts_keys_canonically() {
    let v = Value::Map(vec![
        (Value::Text("aa"), Value::Uint(4)),
        (Value::Uint(100), Value::Uint(3)),
        (Value::Nint(0), Value::Uint(2)),
        (Value::Text("b"), Value::Uint(5)),
        (Value::Uint(10), Value::Uint(1)),
    ]);
    let b = enc_value(&v);
    assert_eq!(b, hex("a5 0a 01 18 64 03 20 02 61 62 05 62 61 61 04"));
    assert_eq!(validate(&b), Ok(()));
}

#[test]
fn value_encoder_rejects_duplicate_keys() {
    let v = Value::Map(vec![
        (Value::Uint(1), Value::Null),
        (Value::Uint(1), Value::Bool(true)),
    ]);
    assert_eq!(v.to_vec(), Err(Error::KeyOrder));
    let nested = Value::Array(vec![v]);
    assert_eq!(nested.to_vec(), Err(Error::KeyOrder));
}

#[test]
fn from_slice_canonical_accepts_canonical() {
    assert_eq!(from_slice_canonical::<u64>(&hex("1864")), Ok(100));
    assert_eq!(
        from_slice_canonical::<u64>(&hex("1817")).err(),
        Some(Error::NonMinimal)
    );
}

fn sample_fields(entries: &[(u64, u64)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut e = Encoder::new(&mut out);
    e.map(entries.len());
    for (k, v) in entries {
        e.field(*k, v);
    }
    out
}

fn read_struct(bytes: &[u8]) -> Result<(u64, Option<u64>, u64), Error> {
    let mut d = Decoder::new(bytes);
    let mut f = d.fields()?;
    f.version(&mut d, 1)?;
    let a = f.opt(&mut d, 1)?;
    let b = f.req(&mut d, 3)?;
    f.finish(&mut d)?;
    d.finish()?;
    Ok((1, a, b))
}

#[test]
fn fields_reader_happy_paths() {
    assert_eq!(
        read_struct(&sample_fields(&[(0, 1), (1, 7), (3, 9)])),
        Ok((1, Some(7), 9))
    );
    assert_eq!(
        read_struct(&sample_fields(&[(0, 1), (3, 9)])),
        Ok((1, None, 9))
    );
}

#[test]
fn fields_reader_rejects_bad_structs() {
    assert_eq!(
        read_struct(&sample_fields(&[(0, 2), (3, 9)])),
        Err(Error::Version(2))
    );
    assert_eq!(
        read_struct(&sample_fields(&[(1, 2), (3, 9)])),
        Err(Error::MissingKey(0))
    );
    assert_eq!(
        read_struct(&sample_fields(&[(0, 1), (1, 2)])),
        Err(Error::MissingKey(3))
    );
    assert_eq!(
        read_struct(&sample_fields(&[(0, 1), (2, 2), (3, 9)])),
        Err(Error::UnknownKey(2))
    );
    assert_eq!(
        read_struct(&sample_fields(&[(0, 1), (3, 9), (4, 0)])),
        Err(Error::UnknownKey(4))
    );
    assert_eq!(
        read_struct(&sample_fields(&[(0, 1), (3, 9), (3, 9)])),
        Err(Error::KeyOrder)
    );
    assert_eq!(
        read_struct(&sample_fields(&[(0, 1), (3, 9), (1, 9)])),
        Err(Error::KeyOrder)
    );
    assert_eq!(read_struct(&hex("a2 00 01 61 61 00")), Err(Error::Type));
    assert_eq!(read_struct(&hex("a1 20 01")), Err(Error::Type));
    assert_eq!(read_struct(&hex("82 00 01")), Err(Error::Type));
    assert_eq!(read_struct(&hex("a2 00 01 03")), Err(Error::Eof));
}

#[test]
fn encoder_helpers() {
    let mut out = Vec::new();
    let mut e = Encoder::new(&mut out);
    assert!(e.is_empty());
    e.map(2);
    e.opt_field(0, Some(&5u8));
    e.opt_field::<u8>(1, None);
    e.field(2, "x");
    assert_eq!(e.len(), 6);
    e.raw(&[0xf6]);
    assert_eq!(e.as_slice(), hex("a2 00 05 02 61 78 f6").as_slice());
    e.put(&-1i8);
    assert_eq!(out, hex("a2 00 05 02 61 78 f6 20"));
}

#[test]
fn encode_into_reuses_buffer() {
    let mut buf = Vec::with_capacity(64);
    encode_into(&7u8, &mut buf);
    encode_into(&true, &mut buf);
    assert_eq!(buf, [0x07, 0xf5]);
}

#[test]
fn error_display_is_nonempty() {
    let all = [
        Error::Eof,
        Error::Trailing,
        Error::Indefinite,
        Error::Reserved,
        Error::NonMinimal,
        Error::Float,
        Error::Tag,
        Error::Simple,
        Error::Utf8,
        Error::KeyOrder,
        Error::Depth,
        Error::Type,
        Error::Length,
        Error::Range,
        Error::UnknownKey(3),
        Error::MissingKey(4),
        Error::Version(9),
        Error::Invalid,
        Error::NotCanonical,
    ];
    for e in all {
        assert!(!alloc::format!("{e}").is_empty());
    }
}

fn random_value<'a>(rng: &mut Rng, depth: u32, pool: &'a [u8], text: &'a str) -> Value<'a> {
    let pick = if depth >= 4 {
        rng.below(6)
    } else {
        rng.below(8)
    };
    match pick {
        0 => Value::Uint(match rng.below(4) {
            0 => rng.below(24),
            1 => rng.below(0x1_0000),
            2 => rng.below(0x1_0000_0000),
            _ => rng.next(),
        }),
        1 => Value::Nint(rng.next() >> rng.below(64)),
        2 => {
            let a = rng.below(pool.len() as u64) as usize;
            let b = a + rng.below((pool.len() - a) as u64 + 1) as usize;
            Value::Bytes(&pool[a..b])
        }
        3 => {
            let n = rng.below(text.len() as u64 + 1) as usize;
            Value::Text(&text[..n])
        }
        4 => Value::Bool(rng.bool()),
        5 => Value::Null,
        6 => Value::Array(
            (0..rng.below(6))
                .map(|_| random_value(rng, depth + 1, pool, text))
                .collect(),
        ),
        _ => {
            let mut keys: Vec<u64> = (0..rng.below(6)).map(|_| rng.below(40)).collect();
            keys.sort_unstable();
            keys.dedup();
            let mut entries: Vec<_> = keys
                .into_iter()
                .map(|k| {
                    let key = if k % 3 == 0 {
                        Value::Text(&text[..(k as usize % 7)])
                    } else {
                        Value::Uint(k * 1000)
                    };
                    (key, random_value(rng, depth + 1, pool, text))
                })
                .collect();
            entries.reverse();
            Value::Map(entries)
        }
    }
}

#[test]
fn property_value_roundtrip_is_fixpoint() {
    let mut rng = Rng::new(1);
    let pool: [u8; 300] = rng.bytes();
    let text = "abcdefghijklmnopqrstuvwxyz0123456789";
    let mut checked = 0;
    for _ in 0..3000 {
        let v = random_value(&mut rng, 0, &pool, text);
        let Ok(bytes) = v.to_vec() else { continue };
        assert_eq!(validate(&bytes), Ok(()));
        let back: Value<'_> = from_slice(&bytes).unwrap();
        assert_eq!(enc_value(&back), bytes);
        checked += 1;
    }
    assert!(checked > 2500);
}

#[test]
fn property_any_accepted_input_is_canonical() {
    let mut rng = Rng::new(2);
    let pool: [u8; 64] = rng.bytes();
    let text = "witness";
    let mut accepted = 0;
    for i in 0..20_000 {
        let mut bytes = if i % 2 == 0 {
            let v = random_value(&mut rng, 0, &pool, text);
            v.to_vec().unwrap_or_default()
        } else {
            let mut b = vec![0u8; rng.below(16) as usize];
            rng.fill(&mut b);
            b
        };
        if !bytes.is_empty() {
            for _ in 0..rng.below(3) {
                let at = rng.below(bytes.len() as u64) as usize;
                bytes[at] ^= 1 << rng.below(8);
            }
        }
        let v = validate(&bytes);
        let parsed = from_slice::<Value<'_>>(&bytes);
        assert_eq!(v.is_ok(), parsed.is_ok());
        if let Ok(val) = parsed {
            assert_eq!(enc_value(&val), bytes);
            accepted += 1;
        }
    }
    assert!(accepted > 1000);
}

#[test]
fn property_integers_roundtrip() {
    let mut rng = Rng::new(3);
    for _ in 0..5000 {
        let u = rng.next() >> rng.below(64);
        assert_eq!(from_slice_canonical::<u64>(&to_vec(&u)), Ok(u));
        let i = (rng.next() as i64) >> rng.below(64);
        assert_eq!(from_slice_canonical::<i64>(&to_vec(&i)), Ok(i));
        let s = i as i16;
        assert_eq!(from_slice_canonical::<i16>(&to_vec(&s)), Ok(s));
    }
}

#[test]
fn truncation_of_valid_input_never_validates() {
    let mut rng = Rng::new(4);
    let pool: [u8; 32] = rng.bytes();
    for _ in 0..500 {
        let v = random_value(&mut rng, 0, &pool, "truncate");
        let Ok(bytes) = v.to_vec() else { continue };
        for cut in 0..bytes.len() {
            assert!(validate(&bytes[..cut]).is_err());
        }
    }
}

#[derive(Debug, PartialEq)]
struct Lossy(u64);

impl Encode for Lossy {
    fn encode(&self, e: &mut Encoder<'_>) {
        e.u64(self.0 + 1);
    }
}

impl<'a> Decode<'a> for Lossy {
    fn decode(d: &mut Decoder<'a>) -> Result<Self, Error> {
        d.u64().map(Lossy)
    }
}

#[test]
fn from_slice_canonical_detects_non_roundtripping_types() {
    assert_eq!(from_slice::<Lossy>(&[0x01]), Ok(Lossy(1)));
    assert_eq!(
        from_slice_canonical::<Lossy>(&[0x01]),
        Err(Error::NotCanonical)
    );
}
