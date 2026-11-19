use super::*;
use crate::jumbf::{self, CBOR};
use crate::testkit::{Rng, jpeg, jpeg_with};
use alloc::vec;

fn store_of(len: usize, rng: &mut Rng) -> Vec<u8> {
    assert!(len >= 46);
    let mut out = Vec::new();
    jumbf::write_superbox(&mut out, &uuid::STORE, "c2pa", |o| {
        let fill = len - o.len() - 8;
        jumbf::write_box(o, CBOR, &rng.bytes(fill))
    })
    .unwrap();
    let pad = len - out.len();
    assert_eq!(pad, 0, "store sized exactly");
    out
}

fn embed(base: &[u8], store: &[u8], en: u16, chunk: usize) -> Vec<u8> {
    let l = locate(base).unwrap();
    let mut out = base[..l.insert_at].to_vec();
    write_packets(&mut out, store, en, chunk).unwrap();
    out.extend_from_slice(&base[l.insert_at..]);
    out
}

fn app11(contents: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    jpeg::write_segment(&mut v, marker::APP11, &[contents]).unwrap();
    v
}

fn jp(en: u16, z: u32, data: &[u8]) -> Vec<u8> {
    let mut c = b"JP".to_vec();
    c.extend_from_slice(&en.to_be_bytes());
    c.extend_from_slice(&z.to_be_bytes());
    c.extend_from_slice(data);
    app11(&c)
}

fn splice(base: &[u8], extra: &[u8]) -> Vec<u8> {
    let at = locate(base).unwrap().insert_at;
    [&base[..at], extra, &base[at..]].concat()
}

#[test]
fn packet_parses_header() {
    let p = packet(b"JP\x02\x11\x00\x00\x00\x03rest").unwrap();
    assert_eq!((p.en, p.z, p.data), (0x0211, 3, &b"rest"[..]));
    assert_eq!(packet(b"JP\x02\x11\x00\x00\x00\x03").unwrap().data, b"");
    assert_eq!(packet(b"JP\x02\x11\x00\x00\x00"), None);
    assert_eq!(packet(b"XX\x02\x11\x00\x00\x00\x03rest"), None);
}

#[test]
fn plain_jpeg_has_no_store() {
    let j = jpeg(1);
    let l = locate(&j).unwrap();
    assert!(l.store.is_none() && l.excluded.is_empty() && l.foreign_en.is_empty());
    assert_eq!(data_hash(&j).unwrap(), witness_format::crypto::sha256(&j));
}

#[test]
fn insert_point_follows_leading_app0_app1() {
    let j = jpeg(2);
    let first = jpeg::segments(&j).unwrap().next().unwrap().unwrap();
    assert_eq!(locate(&j).unwrap().insert_at, first.end());
    let mut rng = Rng::new(3);
    let j = jpeg_with(&mut rng, &[(0xe1, vec![1; 10]), (0xe2, vec![2; 10])], 32);
    let segs: Vec<_> = jpeg::segments(&j).unwrap().map(Result::unwrap).collect();
    assert_eq!(locate(&j).unwrap().insert_at, segs[1].end());
    let no_app = [0xff, 0xd8, 0xff, 0xdb, 0x00, 0x03, 0x00, 0xff, 0xd9];
    assert_eq!(locate(&no_app).unwrap().insert_at, 2);
}

#[test]
fn packetized_len_matches_written_bytes() {
    let mut rng = Rng::new(4);
    for _ in 0..300 {
        let chunk = MIN_CHUNK + rng.below(300);
        let len = 46 + rng.below(2000);
        let store = store_of(len, &mut rng);
        let mut out = Vec::new();
        write_packets(&mut out, &store, 7, chunk).unwrap();
        assert_eq!(
            out.len(),
            packetized_len(len, chunk),
            "len {len} chunk {chunk}"
        );
    }
}

#[test]
fn round_trip_at_chunk_boundaries() {
    let base = jpeg(5);
    let mut rng = Rng::new(6);
    let chunk = 100;
    for len in [46, 99, 100, 101, 199, 200, 201, 299, 300, 301, 1000] {
        let store = store_of(len, &mut rng);
        let f = embed(&base, &store, DEFAULT_EN, chunk);
        let l = locate(&f).unwrap();
        assert_eq!(l.store.as_deref(), Some(store.as_slice()), "len {len}");
        assert_eq!(l.excluded.len(), len.div_ceil(chunk));
        let total: usize = l.excluded.iter().map(|r| r.len()).sum();
        assert_eq!(total, packetized_len(len, chunk));
        assert_eq!(l.excluded[0].start, locate(&base).unwrap().insert_at);
        assert_eq!(hash_excluding(&f, &l.excluded), data_hash(&base).unwrap());
    }
}

#[test]
fn single_segment_store_is_borrowed() {
    let base = jpeg(7);
    let store = store_of(200, &mut Rng::new(8));
    let f = embed(&base, &store, DEFAULT_EN, MAX_CHUNK);
    assert!(matches!(locate(&f).unwrap().store, Some(Cow::Borrowed(_))));
    let f = embed(&base, &store, DEFAULT_EN, 64);
    assert!(matches!(locate(&f).unwrap().store, Some(Cow::Owned(_))));
}

#[test]
fn random_round_trips() {
    let mut rng = Rng::new(9);
    for i in 0..200 {
        let base = jpeg_with(&mut rng, &[], 64 + i);
        let len = 46 + rng.below(3000);
        let chunk = MIN_CHUNK + rng.below(700);
        let store = store_of(len, &mut rng);
        let en = rng.next() as u16;
        let f = embed(&base, &store, en, chunk);
        let l = locate(&f).unwrap();
        assert_eq!(l.store.as_deref(), Some(store.as_slice()));
        assert_eq!(hash_excluding(&f, &l.excluded), data_hash(&base).unwrap());
    }
}

#[test]
fn write_packets_validates_inputs() {
    let store = store_of(46, &mut Rng::new(10));
    let mut out = Vec::new();
    assert_eq!(
        write_packets(&mut out, &store[..31], 1, 40),
        Err(Error::TooLarge)
    );
    assert_eq!(write_packets(&mut out, &store, 1, 0), Err(Error::TooLarge));
    assert_eq!(
        write_packets(&mut out, &store, 1, MIN_CHUNK - 1),
        Err(Error::TooLarge)
    );
    assert!(write_packets(&mut out, &store, 1, MIN_CHUNK).is_ok());
    assert_eq!(
        write_packets(&mut out, &store, 1, MAX_CHUNK + 1),
        Err(Error::TooLarge)
    );
    assert!(write_packets(&mut out, &store, 1, MAX_CHUNK).is_ok());
}

#[test]
fn max_chunk_fills_segment_exactly() {
    let mut rng = Rng::new(11);
    let store = store_of(MAX_CHUNK * 2 + 5, &mut rng);
    let mut out = Vec::new();
    write_packets(&mut out, &store, 1, MAX_CHUNK).unwrap();
    let f = splice(&jpeg(12), &out);
    let segs: Vec<_> = jpeg::segments(&f)
        .unwrap()
        .map(Result::unwrap)
        .filter(|s| s.marker == marker::APP11)
        .collect();
    assert_eq!(segs.len(), 3);
    assert_eq!(segs[0].contents.len(), MAX_CHUNK + 8);
    assert_eq!(segs[1].contents.len(), MAX_SEGMENT_CONTENTS);
    assert_eq!(locate(&f).unwrap().store.as_deref(), Some(store.as_slice()));
}

fn packets(store: &[u8], chunk: usize, en: u16) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    write_packets(&mut out, store, en, chunk).unwrap();
    jpeg::segments(&[&[0xff, 0xd8][..], &out].concat())
        .unwrap()
        .map(|s| app11(s.unwrap().contents))
        .collect()
}

#[test]
fn out_of_order_or_missing_packets_are_rejected() {
    let base = jpeg(13);
    let store = store_of(300, &mut Rng::new(14));
    let p = packets(&store, 100, 5);
    assert_eq!(p.len(), 3);
    let bad = [
        [p[0].clone(), p[2].clone(), p[1].clone()].concat(),
        [p[0].clone(), p[1].clone()].concat(),
        [p[0].clone(), p[1].clone(), p[1].clone(), p[2].clone()].concat(),
        [p[0].clone(), p[1].clone(), p[2].clone(), p[2].clone()].concat(),
    ];
    for (i, b) in bad.iter().enumerate() {
        assert_eq!(
            locate(&splice(&base, b)),
            Err(Error::MalformedSegments),
            "case {i}"
        );
    }
}

#[test]
fn first_packet_must_have_sequence_one() {
    let base = jpeg(15);
    let store = store_of(60, &mut Rng::new(16));
    assert_eq!(
        locate(&splice(&base, &jp(1, 2, &store))),
        Err(Error::MalformedSegments)
    );
}

#[test]
fn continuation_header_must_repeat_store_header() {
    let base = jpeg(17);
    let store = store_of(200, &mut Rng::new(18));
    let mut p = packets(&store, 100, 5);
    let at = 4 + 8;
    p[1][at] ^= 1;
    assert_eq!(
        locate(&splice(&base, &p.concat())),
        Err(Error::MalformedSegments)
    );
}

#[test]
fn store_length_must_fit_file_and_data() {
    let base = jpeg(19);
    let mut store = store_of(60, &mut Rng::new(20));
    store[..4].copy_from_slice(&50u32.to_be_bytes());
    assert_eq!(
        locate(&splice(&base, &jp(1, 1, &store))),
        Err(Error::MalformedSegments)
    );
    store[..4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(
        locate(&splice(&base, &jp(1, 1, &store))),
        Err(Error::MalformedSegments)
    );
    store[..4].copy_from_slice(&1u32.to_be_bytes());
    assert_eq!(
        locate(&splice(&base, &jp(1, 1, &store))),
        Err(Error::MalformedSegments)
    );
}

#[test]
fn second_store_is_rejected() {
    let base = jpeg(21);
    let store = store_of(60, &mut Rng::new(22));
    let two = [jp(1, 1, &store), jp(2, 1, &store)].concat();
    assert_eq!(locate(&splice(&base, &two)), Err(Error::MultipleStores));
    let same_en = [jp(1, 1, &store), jp(1, 1, &store)].concat();
    assert_eq!(locate(&splice(&base, &same_en)), Err(Error::MultipleStores));
}

#[test]
fn foreign_app11_segments_are_kept_and_hashed() {
    let base = jpeg(23);
    let store = store_of(300, &mut Rng::new(24));
    let p = packets(&store, 100, 5);
    let foreign_jumbf = {
        let mut d = Vec::new();
        jumbf::write_superbox(&mut d, &uuid::CBOR, "other", |o| {
            jumbf::write_box(o, CBOR, &[1])
        })
        .unwrap();
        jp(9, 1, &d)
    };
    let other_app11 = app11(b"XXnot jumbf at all");
    let f = splice(
        &base,
        &[
            foreign_jumbf.clone(),
            p[0].clone(),
            other_app11.clone(),
            p[1].clone(),
            jp(9, 2, b"12345678"),
            p[2].clone(),
        ]
        .concat(),
    );
    let l = locate(&f).unwrap();
    assert_eq!(l.store.as_deref(), Some(store.as_slice()));
    assert_eq!(l.excluded.len(), 3);
    assert_eq!(l.foreign_en, vec![9, 9]);
    let kept = splice(
        &base,
        &[foreign_jumbf, other_app11, jp(9, 2, b"12345678")].concat(),
    );
    assert_eq!(hash_excluding(&f, &l.excluded), data_hash(&kept).unwrap());
}

#[test]
fn same_en_before_store_is_foreign() {
    let base = jpeg(25);
    let store = store_of(60, &mut Rng::new(26));
    let f = splice(&base, &[jp(5, 1, b"abcdefgh"), jp(5, 1, &store)].concat());
    let l = locate(&f).unwrap();
    assert_eq!(l.excluded.len(), 1);
    assert_eq!(l.foreign_en, vec![5]);
}

#[test]
fn free_en_avoids_foreign_instances() {
    let j = jpeg(27);
    let mut l = locate(&j).unwrap();
    assert_eq!(l.free_en(), DEFAULT_EN);
    l.foreign_en = vec![DEFAULT_EN, 1, 2];
    assert_eq!(l.free_en(), 3);
    l.foreign_en = vec![3, DEFAULT_EN];
    assert_eq!(l.free_en(), 1);
}

#[test]
fn free_en_is_fast_for_many_foreign_segments() {
    let j = jpeg(28);
    let mut l = locate(&j).unwrap();
    l.foreign_en = (0..=u16::MAX).filter(|&e| e != 40_000).collect();
    assert_eq!(l.free_en(), 40_000);
}

#[test]
fn malformed_jpeg_propagates() {
    assert_eq!(locate(b"nope"), Err(Error::NotJpeg));
    assert_eq!(data_hash(&[0xff, 0xd8, 0x12]), Err(Error::MalformedJpeg));
}

#[test]
fn hash_excluding_skips_exact_ranges() {
    let buf: Vec<u8> = (0..100).collect();
    let h = hash_excluding(&buf, &[3..10, 50..51, 99..100]);
    let kept: Vec<u8> = buf
        .iter()
        .enumerate()
        .filter(|(i, _)| !((3..10).contains(i) || *i == 50 || *i == 99))
        .map(|(_, b)| *b)
        .collect();
    assert_eq!(h, witness_format::crypto::sha256(&kept));
    assert_eq!(
        hash_excluding(&buf, &[]),
        witness_format::crypto::sha256(&buf)
    );
}

#[test]
fn random_corruption_of_embedded_store_never_panics() {
    let base = jpeg(29);
    let store = store_of(500, &mut Rng::new(30));
    let f = embed(&base, &store, 3, 90);
    let mut rng = Rng::new(31);
    for _ in 0..3000 {
        let mut d = f.clone();
        let i = rng.below(d.len());
        d[i] = rng.next() as u8;
        if let Ok(l) = locate(&d) {
            for r in &l.excluded {
                assert!(r.end <= d.len());
            }
        }
    }
}
