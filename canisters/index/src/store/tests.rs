use super::*;
use crate::testkit::{bundle, flip, hash};
use ic_stable_structures::DefaultMemoryImpl;
use ic_stable_structures::memory_manager::{MemoryManager, VirtualMemory};
use witness_canister_common::limits::{MAX_PROBE_EXACT, MAX_PROBE_PDQ, MAX_PROBE_WATERMARKS};
use witness_fingerprint::MIN_INDEX_QUALITY;
use witness_format::PdqShape;
use witness_format::cbor::to_vec;

type Vm = VirtualMemory<DefaultMemoryImpl>;

fn fresh() -> (DefaultMemoryImpl, Index<Vm>) {
    let raw = DefaultMemoryImpl::default();
    let index = reopen(&raw);
    (raw, index)
}

fn reopen(raw: &DefaultMemoryImpl) -> Index<Vm> {
    let manager = MemoryManager::init(raw.clone());
    Index::init(|id| manager.get(id))
}

fn asset(ix: u64) -> Vec<u8> {
    hash(ix ^ 0xA55E7).0.to_vec()
}

fn pixel(ix: u64) -> Vec<u8> {
    hash(ix ^ 0xB1C5E1).0.to_vec()
}

fn posting(ix: u64, b: Option<&FingerprintBundle>) -> Posting {
    Posting {
        record_ix: ix,
        wm_root: ix.wrapping_add(1_000),
        asset_hash: asset(ix),
        pixel_hash: pixel(ix),
        fingerprints: b.map(|b| ByteBuf::from(to_vec(b))),
        root_ix: None,
    }
}

fn capture(ix: u64) -> Posting {
    posting(ix, Some(&bundle(ix, 17, 90, ix.is_multiple_of(2))))
}

fn buf(bytes: &[u8]) -> ByteBuf {
    ByteBuf::from(bytes.to_vec())
}

fn pdq(index: &Index<Vm>, queries: &[PdqHash]) -> ProbeResult {
    let q: Vec<ByteBuf> = queries.iter().map(|h| buf(&h.0)).collect();
    index.probe(&q, &[], &[]).unwrap()
}

fn shape_hash(seed: u64, shape: usize) -> PdqHash {
    PdqHash(bundle(seed, 17, 90, false).pdq()[shape].hash)
}

#[test]
fn put_indexes_every_kind_of_lookup() {
    let (_, mut index) = fresh();
    assert!(index.put(&capture(5)));
    let r = index
        .probe(
            &[buf(&shape_hash(5, 3).0)],
            &[1_005],
            &[buf(&asset(5)), buf(&pixel(5))],
        )
        .unwrap();
    assert_eq!(
        r.pdq,
        [PdqHit {
            record_ix: 5,
            distance: 0
        }]
    );
    assert_eq!(r.watermark, [5]);
    assert_eq!(r.exact, [5]);
    assert!(!r.truncated_candidates);
    assert_eq!(r.indexed_upto, 6);
    assert!(r.covers(5) && !r.covers(6));
}

#[test]
fn fingerprints_round_trip_the_bundle() {
    let (_, mut index) = fresh();
    let b = bundle(9, 17, 90, true);
    index.put(&posting(9, Some(&b)));
    assert_eq!(index.fingerprints(9), Some(Fingerprints::from(&b)));
    assert_eq!(index.fingerprints(10), None);
}

#[test]
fn put_is_idempotent() {
    let (_, mut index) = fresh();
    assert!(index.put(&capture(1)));
    let before = (index.stats(), pdq(&index, &[shape_hash(1, 0)]));
    assert!(!index.put(&capture(1)));
    let mut changed = capture(1);
    changed.wm_root = 77;
    assert!(!index.put(&changed));
    assert_eq!((index.stats(), pdq(&index, &[shape_hash(1, 0)])), before);
    assert!(index.probe(&[], &[77], &[]).unwrap().watermark.is_empty());
}

#[test]
fn put_all_counts_new_postings_only() {
    let (_, mut index) = fresh();
    let batch = [capture(1), capture(2), capture(1)];
    assert_eq!(index.put_all(&batch), 2);
    assert_eq!(index.put_all(&batch), 0);
    assert_eq!(index.stats().records, 2);
}

#[test]
fn posting_without_fingerprints_keeps_other_lookups() {
    let (_, mut index) = fresh();
    index.put(&posting(3, None));
    assert_eq!(index.fingerprints(3), None);
    assert_eq!(index.stats().rows, 0);
    let r = index.probe(&[], &[1_003], &[buf(&asset(3))]).unwrap();
    assert_eq!((r.watermark, r.exact), (vec![3], vec![3]));
}

#[test]
fn malformed_or_noncanonical_fingerprints_are_ignored() {
    let (_, mut index) = fresh();
    let mut bad = posting(1, None);
    bad.fingerprints = Some(buf(&[0xFF, 0x00, 0x13]));
    assert!(index.put(&bad));
    let mut trailing = posting(2, None);
    let mut bytes = to_vec(&bundle(2, 3, 90, false));
    bytes.push(0);
    trailing.fingerprints = Some(ByteBuf::from(bytes));
    assert!(index.put(&trailing));
    assert_eq!(index.stats().rows, 0);
    assert_eq!(index.fingerprints(1), None);
    assert_eq!(index.indexed_upto(), 3);
}

#[test]
fn oversized_shape_set_is_not_stored() {
    let (_, mut index) = fresh();
    index.put(&posting(1, Some(&bundle(1, 32, 90, false))));
    assert_eq!(index.stats().rows, 0);
    assert_eq!(index.fingerprints(1), None);
}

#[test]
fn low_quality_rows_are_stored_but_not_probed() {
    let (_, mut index) = fresh();
    let b = bundle(4, 17, MIN_INDEX_QUALITY - 1, false);
    index.put(&posting(4, Some(&b)));
    assert!(index.fingerprints(4).is_some());
    assert_eq!(index.stats().blocks, 0);
    assert!(pdq(&index, &[PdqHash(b.pdq()[0].hash)]).pdq.is_empty());
}

#[test]
fn malformed_hash_lengths_skip_exact_entries() {
    let (_, mut index) = fresh();
    let mut p = posting(1, None);
    p.asset_hash = vec![1; 31];
    p.pixel_hash = Vec::new();
    index.put(&p);
    assert!(
        index
            .probe(&[], &[], &[buf(&[1; 32])])
            .unwrap()
            .exact
            .is_empty()
    );
}

#[test]
fn indexed_upto_is_exclusive_max_and_saturates() {
    let (_, mut index) = fresh();
    assert_eq!(index.indexed_upto(), 0);
    index.put(&posting(10, None));
    index.put(&posting(4, None));
    assert_eq!(index.indexed_upto(), 11);
    index.put(&posting(u64::MAX, None));
    assert_eq!(index.indexed_upto(), u64::MAX);
}

#[test]
fn watermark_keeps_32_earliest_against_floods() {
    let (_, mut index) = fresh();
    for ix in (0..200u64).rev() {
        let mut p = posting(ix, None);
        p.wm_root = 42;
        index.put(&p);
    }
    let r = index.probe(&[], &[42, 42, 43], &[]).unwrap();
    assert_eq!(r.watermark, (0..32).collect::<Vec<_>>());
}

#[test]
fn watermark_ids_merge_sorted_and_deduped() {
    let (_, mut index) = fresh();
    for ix in 0..6 {
        let mut p = posting(ix, None);
        p.wm_root = ix % 2;
        index.put(&p);
    }
    let r = index.probe(&[], &[1, 0], &[]).unwrap();
    assert_eq!(r.watermark, [0, 1, 2, 3, 4, 5]);
}

#[test]
fn exact_caps_at_eight_per_hash_prefix() {
    let (_, mut index) = fresh();
    for ix in 0..20 {
        let mut p = posting(ix, None);
        p.asset_hash = vec![7; 32];
        p.pixel_hash = vec![7; 32];
        index.put(&p);
    }
    let r = index.probe(&[], &[], &[buf(&[7; 32])]).unwrap();
    assert_eq!(r.exact, (0..8).collect::<Vec<_>>());
}

#[test]
fn exact_matches_on_sixteen_byte_prefix() {
    let (_, mut index) = fresh();
    index.put(&posting(1, None));
    let mut probe = asset(1);
    probe[31] ^= 0xFF;
    assert_eq!(index.probe(&[], &[], &[buf(&probe)]).unwrap().exact, [1]);
    probe[15] ^= 1;
    assert!(
        index
            .probe(&[], &[], &[buf(&probe)])
            .unwrap()
            .exact
            .is_empty()
    );
}

#[test]
fn derivatives_link_roots_earliest_first_capped() {
    let (_, mut index) = fresh();
    index.put(&capture(1));
    for ix in (2..50).rev() {
        let mut p = posting(ix, None);
        p.root_ix = Some(1);
        index.put(&p);
    }
    let d = index.derivatives(1);
    assert_eq!(d, (2..34).collect::<Vec<_>>());
    assert!(index.derivatives(2).is_empty());
}

#[test]
fn self_rooted_derivative_ignored() {
    let (_, mut index) = fresh();
    let mut p = posting(3, None);
    p.root_ix = Some(3);
    index.put(&p);
    assert!(index.derivatives(3).is_empty());
}

#[test]
fn erase_removes_all_traces() {
    let (_, mut index) = fresh();
    index.put(&capture(1));
    let mut d = capture(2);
    d.root_ix = Some(1);
    index.put(&d);
    let keep = capture(3);
    index.put(&keep);
    assert!(index.erase(2));
    assert!(index.erase(1));
    let r = index
        .probe(
            &[buf(&shape_hash(1, 0).0), buf(&shape_hash(2, 5).0)],
            &[1_001, 1_002],
            &[buf(&asset(1)), buf(&pixel(2))],
        )
        .unwrap();
    assert!(r.is_empty());
    assert_eq!(index.fingerprints(1), None);
    assert_eq!(index.fingerprints(2), None);
    assert!(index.derivatives(1).is_empty());
    assert_eq!(pdq(&index, &[shape_hash(3, 0)]).pdq[0].record_ix, 3);
    assert!(index.erase(3));
    assert_eq!(index.stats().blocks, 0);
    assert_eq!(index.stats().indexed_upto, 4);
}

#[test]
fn erase_is_idempotent_and_blocks_late_puts() {
    let (_, mut index) = fresh();
    index.put(&capture(1));
    assert!(index.erase(1));
    assert!(!index.erase(1));
    assert!(!index.put(&capture(1)));
    assert!(!index.erase(9));
    assert!(!index.put(&capture(9)));
    assert!(pdq(&index, &[shape_hash(9, 0)]).pdq.is_empty());
    assert!(
        index
            .probe(&[], &[1_009], &[])
            .unwrap()
            .watermark
            .is_empty()
    );
}

#[test]
fn derivative_of_erased_root_not_linked() {
    let (_, mut index) = fresh();
    index.put(&capture(1));
    index.erase(1);
    let mut d = posting(2, None);
    d.root_ix = Some(1);
    assert!(index.put(&d));
    assert!(index.derivatives(1).is_empty());
}

#[test]
fn erasing_from_shared_buckets_keeps_neighbours() {
    let (_, mut index) = fresh();
    let shared = bundle(77, 3, 90, false);
    for ix in 0..40 {
        index.put(&posting(ix, Some(&shared)));
    }
    for ix in (0..40).step_by(3) {
        index.erase(ix);
    }
    let fp = Row::from_bundle(&shared).unwrap().hash(1);
    let mut got: Vec<u64> = Vec::new();
    let r = pdq(&index, &[fp]);
    got.extend(r.pdq.iter().map(|h| h.record_ix));
    let expected: Vec<u64> = (0..40)
        .filter(|ix| ix % 3 != 0)
        .take(MAX_PDQ_HITS)
        .collect();
    assert_eq!(got, expected);
}

#[test]
fn near_duplicates_within_radius_always_found() {
    let (_, mut index) = fresh();
    for ix in 0..300 {
        index.put(&capture(ix));
    }
    for (seed, bits) in [
        (1u64, 1usize),
        (2, 8),
        (3, 16),
        (4, 24),
        (5, MIH_RADIUS as usize),
    ] {
        let target = 100 + seed;
        let q = flip(&shape_hash(target, seed as usize), bits, seed);
        let r = pdq(&index, &[q]);
        let hit = r
            .pdq
            .iter()
            .find(|h| h.record_ix == target)
            .expect("recall");
        assert!(hit.distance <= bits as u32);
    }
}

#[test]
fn far_hashes_are_not_reported() {
    let (_, mut index) = fresh();
    for ix in 0..50 {
        index.put(&capture(ix));
    }
    let q = flip(&shape_hash(7, 0), 64, 9);
    assert!(
        pdq(&index, &[q])
            .pdq
            .iter()
            .all(|h| h.distance <= MAX_PDQ_DISTANCE)
    );
    assert!(pdq(&index, &[hash(0xFEED_FACE)]).pdq.is_empty());
}

#[test]
fn multiple_queries_dedupe_to_min_distance() {
    let (_, mut index) = fresh();
    for ix in 0..20 {
        index.put(&capture(ix));
    }
    let base = shape_hash(4, 2);
    let r = pdq(&index, &[flip(&base, 9, 1), flip(&base, 2, 2), base]);
    let hits: Vec<_> = r.pdq.iter().filter(|h| h.record_ix == 4).collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].distance, 0);
}

#[test]
fn hits_sorted_by_distance_then_record() {
    let (_, mut index) = fresh();
    let base = hash(5);
    for (ix, bits) in [(10u64, 6usize), (11, 0), (12, 3), (13, 0)] {
        let shapes = vec![PdqShape {
            tag: 0,
            hash: flip(&base, bits, ix).0,
        }];
        let b = FingerprintBundle::new(shapes, 90, [0; 192], false).unwrap();
        index.put(&posting(ix, Some(&b)));
    }
    let r = pdq(&index, &[base]);
    let order: Vec<_> = r.pdq.iter().map(|h| (h.distance, h.record_ix)).collect();
    assert_eq!(order, [(0, 11), (0, 13), (3, 12), (6, 10)]);
}

#[test]
fn too_many_hits_truncate_with_flag() {
    let (_, mut index) = fresh();
    let shared = bundle(3, 1, 90, false);
    for ix in 0..(MAX_PDQ_HITS as u64 + 5) {
        index.put(&posting(ix, Some(&shared)));
    }
    let r = pdq(&index, &[PdqHash(shared.pdq()[0].hash)]);
    assert_eq!(r.pdq.len(), MAX_PDQ_HITS);
    assert!(r.truncated_candidates);
    assert_eq!(r.pdq[0].record_ix, 0);
}

#[test]
fn candidate_tie_at_top_cut_flags_truncation() {
    let (_, mut index) = fresh();
    let shared = bundle(8, 1, 90, false);
    for ix in 0..(TOP_CANDIDATES as u64 + 1) {
        index.put(&posting(ix, Some(&shared)));
    }
    let (nominated, truncated) = index.nominate(&[PdqHash(shared.pdq()[0].hash)]);
    assert_eq!(nominated.len(), TOP_CANDIDATES);
    assert!(truncated);
}

#[test]
fn probe_limits_rejected() {
    let (_, index) = fresh();
    let h = buf(&[0; 32]);
    assert!(
        index
            .probe(&vec![h.clone(); MAX_PROBE_PDQ], &[], &[])
            .is_some()
    );
    assert!(
        index
            .probe(&vec![h.clone(); MAX_PROBE_PDQ + 1], &[], &[])
            .is_none()
    );
    assert!(
        index
            .probe(&[], &[0; MAX_PROBE_WATERMARKS + 1], &[])
            .is_none()
    );
    assert!(
        index
            .probe(&[], &[], &vec![h.clone(); MAX_PROBE_EXACT + 1])
            .is_none()
    );
    assert!(index.probe(&[buf(&[0; 31])], &[], &[]).is_none());
    assert!(index.probe(&[], &[], &[buf(&[0; 33])]).is_none());
}

#[test]
fn empty_probe_reports_progress_only() {
    let (_, mut index) = fresh();
    index.put(&capture(2));
    let r = index.probe(&[], &[], &[]).unwrap();
    assert!(r.is_empty());
    assert_eq!(r.indexed_upto, 3);
}

#[test]
fn sixteen_queries_all_contribute() {
    let (_, mut index) = fresh();
    for ix in 0..16 {
        index.put(&capture(ix));
    }
    let queries: Vec<PdqHash> = (0..16).map(|ix| shape_hash(ix, 0)).collect();
    let r = pdq(&index, &queries);
    assert_eq!(r.pdq.len(), 16);
    assert!(r.pdq.iter().all(|h| h.distance == 0));
}

#[test]
fn reopen_needs_no_rebuild() {
    let (raw, mut index) = fresh();
    for ix in 0..30 {
        let mut p = capture(ix);
        p.root_ix = (ix > 0).then_some(0);
        index.put(&p);
    }
    index.erase(7);
    let probe = |i: &Index<Vm>| {
        (
            i.probe(&[buf(&shape_hash(3, 1).0)], &[1_004], &[buf(&asset(5))]),
            i.fingerprints(9),
            i.derivatives(0),
            i.stats(),
        )
    };
    let before = probe(&index);
    drop(index);
    let mut again = reopen(&raw);
    assert_eq!(probe(&again), before);
    assert!(!again.put(&capture(3)));
    assert!(again.put(&capture(30)));
    assert_eq!(again.indexed_upto(), 31);
}

#[test]
fn deterministic_across_instances() {
    let build = || {
        let (_, mut index) = fresh();
        for ix in 0..60 {
            index.put(&capture(ix * 7 % 60));
        }
        index.erase(13);
        (0..10)
            .map(|q| pdq(&index, &[flip(&shape_hash(q * 5, 2), 12, q)]))
            .collect::<Vec<_>>()
    };
    assert_eq!(build(), build());
}
