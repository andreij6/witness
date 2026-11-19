use super::*;
use candid::{Decode, Encode};
use witness_format::{FingerprintBundle, PdqShape};

fn posting() -> Posting {
    Posting {
        record_ix: 9,
        wm_root: 0x0123_4567_89ab_cdef,
        asset_hash: (0..32).collect(),
        pixel_hash: (100..132).collect(),
        fingerprints: Some(ByteBuf::from(vec![1, 2, 3])),
        root_ix: Some(3),
    }
}

fn round_trip<T: CandidType + for<'de> Deserialize<'de> + PartialEq + core::fmt::Debug>(v: &T) {
    let bytes = Encode!(v).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(Encode!(v).unwrap_or_else(|e| panic!("{e}")), bytes);
    assert_eq!(&Decode!(&bytes, T).unwrap_or_else(|e| panic!("{e}")), v);
}

#[test]
fn posting_keys_are_hash_prefixes() {
    let p = posting();
    let expected: [u8; 16] = core::array::from_fn(|i| i as u8);
    assert_eq!(p.asset_key(), Some(expected));
    let expected: [u8; 16] = core::array::from_fn(|i| 100 + i as u8);
    assert_eq!(p.pixel_key(), Some(expected));
}

#[test]
fn hash_key_requires_exactly_32_bytes() {
    assert_eq!(hash_key(&[]), None);
    assert_eq!(hash_key(&[0; 31]), None);
    assert_eq!(hash_key(&[0; 33]), None);
    assert_eq!(hash_key(&[5; 32]), Some([5; 16]));
}

#[test]
fn malformed_posting_hashes_have_no_key() {
    let mut p = posting();
    p.asset_hash.truncate(16);
    p.pixel_hash.clear();
    assert_eq!(p.asset_key(), None);
    assert_eq!(p.pixel_key(), None);
}

#[test]
fn postings_round_trip() {
    round_trip(&posting());
    let mut p = posting();
    p.fingerprints = None;
    p.root_ix = None;
    round_trip(&p);
    round_trip(&vec![posting(), p]);
}

#[test]
fn probe_limits() {
    let h = |n: usize| vec![ByteBuf::from(vec![0u8; 32]); n];
    assert!(probe_within_limits(&[], &[], &[]));
    assert!(probe_within_limits(
        &h(MAX_PROBE_PDQ),
        &[1; MAX_PROBE_WATERMARKS],
        &h(MAX_PROBE_EXACT)
    ));
    assert!(!probe_within_limits(&h(MAX_PROBE_PDQ + 1), &[], &[]));
    assert!(!probe_within_limits(
        &[],
        &[0; MAX_PROBE_WATERMARKS + 1],
        &[]
    ));
    assert!(!probe_within_limits(&[], &[], &h(MAX_PROBE_EXACT + 1)));
    assert!(!probe_within_limits(
        &[ByteBuf::from(vec![0; 31])],
        &[],
        &[]
    ));
    assert!(!probe_within_limits(
        &[],
        &[],
        &[ByteBuf::from(vec![0; 33])]
    ));
}

#[test]
fn probe_result_helpers() {
    let mut r = ProbeResult::default();
    assert!(r.is_empty());
    assert!(!r.covers(0));
    r.indexed_upto = 10;
    assert!(r.covers(9));
    assert!(!r.covers(10));
    r.exact.push(4);
    assert!(!r.is_empty());
    r = ProbeResult {
        pdq: vec![PdqHit {
            record_ix: 1,
            distance: 31,
        }],
        ..ProbeResult::default()
    };
    assert!(!r.is_empty());
    round_trip(&r);
}

#[test]
fn pdq_hits_order_by_record_then_distance() {
    let mut v = [
        PdqHit {
            record_ix: 2,
            distance: 1,
        },
        PdqHit {
            record_ix: 1,
            distance: 9,
        },
        PdqHit {
            record_ix: 1,
            distance: 3,
        },
    ];
    v.sort();
    assert_eq!(
        v.iter()
            .map(|h| (h.record_ix, h.distance))
            .collect::<Vec<_>>(),
        [(1, 3), (1, 9), (2, 1)]
    );
}

#[test]
fn fingerprints_from_bundle() {
    let grid: [u16; 192] = core::array::from_fn(|i| i as u16 * 257);
    let bundle = FingerprintBundle::new(
        vec![
            PdqShape {
                tag: 5,
                hash: [5; 32],
            },
            PdqShape {
                tag: 1,
                hash: [1; 32],
            },
        ],
        80,
        grid,
        false,
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    let f = Fingerprints::from(&bundle);
    assert_eq!(f.grid, bundle.grid_bytes().to_vec());
    assert_eq!(f.grid.len(), 384);
    assert_eq!(
        f.shapes
            .iter()
            .map(|(t, h)| (*t, h[0], h.len()))
            .collect::<Vec<_>>(),
        [(1, 1, 32), (5, 5, 32)]
    );
    assert_eq!((f.quality, f.portrait), (80, false));
    round_trip(&f);
    round_trip(&Some(f));
}

#[test]
fn fingerprints_from_empty_bundle() {
    let bundle =
        FingerprintBundle::new(vec![], 0, [0; 192], true).unwrap_or_else(|e| panic!("{e:?}"));
    let f = Fingerprints::from(&bundle);
    assert!(f.shapes.is_empty());
    assert_eq!(f.grid, vec![0; 384]);
    assert_eq!((f.quality, f.portrait), (0, true));
}

#[test]
fn decoding_wrong_type_fails() {
    let bytes = Encode!(&posting()).unwrap_or_else(|e| panic!("{e}"));
    assert!(Decode!(&bytes, ProbeResult).is_err());
    assert!(Decode!(b"garbage", Posting).is_err());
}
