use super::*;
use crate::cbor::{from_slice, from_slice_canonical, to_vec};
use crate::testutil::Rng;
use alloc::vec;

fn shape(tag: u8, fill: u8) -> PdqShape {
    PdqShape {
        tag,
        hash: [fill; 32],
    }
}

fn grid_seq() -> [u16; GRID_CELLS] {
    core::array::from_fn(|i| (i as u16) * 257)
}

fn bundle() -> FingerprintBundle {
    FingerprintBundle::new(vec![shape(3, 0x33), shape(1, 0x11)], 80, grid_seq(), false).unwrap()
}

#[test]
fn constructor_sorts_by_tag() {
    let b = bundle();
    assert_eq!(b.pdq().iter().map(|s| s.tag).collect::<Vec<_>>(), [1, 3]);
    assert_eq!(b.shape(3).map(|s| s.hash[0]), Some(0x33));
    assert_eq!(b.shape(2), None);
    assert_eq!(b.quality(), 80);
    assert!(!b.portrait());
    assert_eq!(b.grid()[191], 191 * 257);
}

#[test]
fn constructor_rejects_invalid() {
    let g = grid_seq();
    assert_eq!(
        FingerprintBundle::new(vec![shape(1, 0), shape(1, 1)], 50, g, false),
        Err(Error::KeyOrder)
    );
    assert_eq!(
        FingerprintBundle::new(vec![], 101, g, false),
        Err(Error::Range)
    );
    let many: Vec<PdqShape> = (0..=MAX_PDQ_SHAPES as u8).map(|t| shape(t, t)).collect();
    assert_eq!(
        FingerprintBundle::new(many, 50, g, false),
        Err(Error::Range)
    );
    let max: Vec<PdqShape> = (0..MAX_PDQ_SHAPES as u8).map(|t| shape(t, t)).collect();
    assert!(FingerprintBundle::new(max, 100, g, true).is_ok());
    assert!(FingerprintBundle::new(vec![], 0, g, false).is_ok());
}

#[test]
fn grid_geometry() {
    let land = bundle();
    assert_eq!(land.grid_dims(), (16, 12));
    assert_eq!(land.cell(0, 0), Some(0));
    assert_eq!(land.cell(15, 0), Some(15 * 257));
    assert_eq!(land.cell(0, 1), Some(16 * 257));
    assert_eq!(land.cell(15, 11), Some(191 * 257));
    assert_eq!(land.cell(16, 0), None);
    assert_eq!(land.cell(0, 12), None);
    let port = FingerprintBundle::new(vec![], 0, grid_seq(), true).unwrap();
    assert_eq!(port.grid_dims(), (12, 16));
    assert_eq!(port.cell(0, 1), Some(12 * 257));
    assert_eq!(port.cell(11, 15), Some(191 * 257));
    assert_eq!(port.cell(12, 0), None);
    assert_eq!(port.cell(0, 16), None);
}

#[test]
fn golden_encoding_prefix_and_grid_layout() {
    let b = bundle();
    let enc = to_vec(&b);
    let mut want = vec![0xa5, 0x00, 0x01, 0x01, 0x82, 0x82, 0x01, 0x58, 0x20];
    want.extend([0x11; 32]);
    want.extend([0x82, 0x03, 0x58, 0x20]);
    want.extend([0x33; 32]);
    want.extend([0x02, 0x18, 0x50, 0x03, 0x59, 0x01, 0x80]);
    assert_eq!(&enc[..want.len()], want.as_slice());
    let grid = &enc[want.len()..want.len() + GRID_BYTES];
    assert_eq!(&grid[..4], &[0x00, 0x00, 0x01, 0x01]);
    assert_eq!(&grid[GRID_BYTES - 2..], &(191u16 * 257).to_be_bytes());
    assert_eq!(&enc[want.len() + GRID_BYTES..], &[0x04, 0xf4]);
    assert_eq!(enc.len(), want.len() + GRID_BYTES + 2);
}

#[test]
fn roundtrip_and_commit_is_hash_of_encoding() {
    let b = bundle();
    let enc = to_vec(&b);
    assert_eq!(
        from_slice_canonical::<FingerprintBundle>(&enc),
        Ok(b.clone())
    );
    assert_eq!(b.fp_commit(), crate::crypto::sha256(&enc));
    let reordered =
        FingerprintBundle::new(vec![shape(1, 0x11), shape(3, 0x33)], 80, grid_seq(), false)
            .unwrap();
    assert_eq!(reordered.fp_commit(), b.fp_commit());
}

#[test]
fn commit_changes_with_any_field() {
    let base = bundle();
    let c = base.fp_commit();
    let mut g = grid_seq();
    g[100] ^= 1;
    for other in [
        FingerprintBundle::new(vec![shape(3, 0x33), shape(1, 0x12)], 80, grid_seq(), false),
        FingerprintBundle::new(vec![shape(3, 0x33), shape(2, 0x11)], 80, grid_seq(), false),
        FingerprintBundle::new(vec![shape(3, 0x33), shape(1, 0x11)], 81, grid_seq(), false),
        FingerprintBundle::new(vec![shape(3, 0x33), shape(1, 0x11)], 80, g, false),
        FingerprintBundle::new(vec![shape(3, 0x33), shape(1, 0x11)], 80, grid_seq(), true),
        FingerprintBundle::new(vec![shape(3, 0x33)], 80, grid_seq(), false),
    ] {
        assert_ne!(other.unwrap().fp_commit(), c);
    }
}

#[test]
fn decoder_rejects_malformed() {
    let enc = to_vec(&bundle());
    let mut unsorted = enc.clone();
    unsorted[6] = 0x05;
    assert_eq!(
        from_slice::<FingerprintBundle>(&unsorted),
        Err(Error::KeyOrder)
    );
    let mut dup = enc.clone();
    dup[6] = 0x03;
    assert_eq!(from_slice::<FingerprintBundle>(&dup), Err(Error::KeyOrder));
    let mut version = enc.clone();
    version[2] = 0x02;
    assert_eq!(
        from_slice::<FingerprintBundle>(&version),
        Err(Error::Version(2))
    );
    let q = enc.iter().position(|&b| b == 0x50).unwrap();
    let mut quality = enc.clone();
    quality[q] = 0x65;
    assert_eq!(from_slice::<FingerprintBundle>(&quality), Err(Error::Range));
    let mut truncated = enc.clone();
    truncated.truncate(enc.len() - 1);
    assert_eq!(from_slice::<FingerprintBundle>(&truncated), Err(Error::Eof));
    let mut short_hash = vec![0xa5, 0x00, 0x01, 0x01, 0x81, 0x82, 0x01, 0x41, 0x00];
    short_hash.extend([0x02, 0x00, 0x03, 0x41, 0x00, 0x04, 0xf4]);
    assert_eq!(
        from_slice::<FingerprintBundle>(&short_hash),
        Err(Error::Length)
    );
    let mut short_grid = vec![
        0xa5, 0x00, 0x01, 0x01, 0x80, 0x02, 0x00, 0x03, 0x59, 0x01, 0x7e,
    ];
    short_grid.extend([0u8; 382]);
    short_grid.extend([0x04, 0xf4]);
    assert_eq!(
        from_slice::<FingerprintBundle>(&short_grid),
        Err(Error::Length)
    );
    let mut trailing = enc.clone();
    trailing.push(0);
    assert_eq!(
        from_slice::<FingerprintBundle>(&trailing),
        Err(Error::Trailing)
    );
    let mut extra = enc;
    extra[0] = 0xa6;
    extra.extend([0x05, 0x00]);
    assert_eq!(
        from_slice::<FingerprintBundle>(&extra),
        Err(Error::UnknownKey(5))
    );
}

#[test]
fn property_random_bundles_roundtrip() {
    let mut rng = Rng::new(50);
    for _ in 0..300 {
        let n = rng.below(MAX_PDQ_SHAPES as u64 + 1) as usize;
        let mut tags: Vec<u8> = (0..=255u8).collect();
        for i in 0..n {
            let j = i + rng.below((256 - i) as u64) as usize;
            tags.swap(i, j);
        }
        let shapes: Vec<PdqShape> = tags[..n]
            .iter()
            .map(|&t| PdqShape {
                tag: t,
                hash: rng.bytes(),
            })
            .collect();
        let grid: [u16; GRID_CELLS] = core::array::from_fn(|_| rng.next() as u16);
        let b = FingerprintBundle::new(shapes, rng.below(101) as u8, grid, rng.bool()).unwrap();
        let enc = to_vec(&b);
        assert_eq!(
            from_slice_canonical::<FingerprintBundle>(&enc),
            Ok(b.clone())
        );
        assert_eq!(b.fp_commit(), crate::crypto::sha256(&enc));
        assert!(b.pdq().windows(2).all(|w| w[0].tag < w[1].tag));
    }
}

#[test]
fn property_mutations_never_panic() {
    let mut rng = Rng::new(51);
    let enc = to_vec(&bundle());
    for _ in 0..3000 {
        let mut m = enc.clone();
        let at = rng.below(m.len() as u64) as usize;
        m[at] = rng.next() as u8;
        if let Ok(b) = from_slice::<FingerprintBundle>(&m) {
            assert_eq!(to_vec(&b), m);
        }
    }
}
