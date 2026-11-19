use crate::options::Options;
use crate::oracle::{MAX_DERIVATIVES, MAX_WATERMARK, Oracle, Record};
use crate::query::Query;
use crate::verdict::{Notes, Reason};
use witness_fingerprint::HASH_BITS;
use witness_format::{FingerprintBundle, VerdictFlags};
use witness_watermark::{DEFAULT_DEAD_ZONE, Decoded, SENSOR_ASPECT, WatermarkError, Watermarker};

pub const MAX_PROBE_HITS: usize = 32;
pub const MAX_CANDIDATES: usize = 64;
pub const NO_DISTANCE: u32 = HASH_BITS as u32 + 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub record: Record,
    pub bundle: Option<FingerprintBundle>,
    pub distance: u32,
}

#[derive(Debug, Default)]
pub(crate) struct Pool {
    pub candidates: Vec<Candidate>,
    pub watermark: Option<Decoded>,
}

fn read_watermark(query: &Query, w: &dyn Watermarker) -> Result<Option<Decoded>, WatermarkError> {
    for view in query.views() {
        let crop = query.image.crop(view.x, view.y, view.width, view.height)?;
        if let Some(d) = w.read(crop, SENSOR_ASPECT, DEFAULT_DEAD_ZONE)? {
            return Ok(Some(d));
        }
    }
    Ok(None)
}

fn watermark(query: &Query, options: &Options<'_>, notes: &mut Notes) -> Option<Decoded> {
    let decoded = options.watermarker.map(|w| read_watermark(query, w));
    match decoded {
        Some(Ok(d)) => d,
        None | Some(Err(_)) => {
            notes.reason(Reason::WatermarkUnavailable);
            None
        }
    }
}

fn capped<T>(mut items: Vec<T>, cap: usize, notes: &mut Notes) -> Vec<T> {
    if items.len() > cap {
        items.truncate(cap);
        notes.flag(VerdictFlags::TRUNCATED_CANDIDATES);
    }
    items
}

fn dedupe(found: &mut Vec<(Record, u32)>) {
    found.sort_unstable_by_key(|(r, d)| (r.record_ix, *d));
    found.dedup_by_key(|(r, _)| r.record_ix);
}

pub(crate) fn gather<O: Oracle + ?Sized>(
    query: &Query,
    oracle: &O,
    options: &Options<'_>,
    notes: &mut Notes,
) -> Pool {
    let watermark = watermark(query, options, notes);
    let mut found: Vec<(Record, u32)> = Vec::new();
    if let Some(d) = watermark {
        notes.reason(Reason::WatermarkFound);
        let hits = capped(oracle.by_watermark(d.capture_id), MAX_WATERMARK, notes);
        found.extend(hits.into_iter().map(|r| (r, NO_DISTANCE)));
    }
    if !query.hashes.is_empty() {
        let probe = oracle.pdq_probe(&query.hashes);
        if probe.truncated {
            notes.flag(VerdictFlags::TRUNCATED_CANDIDATES);
        }
        let mut hits: Vec<(Record, u32)> = probe
            .hits
            .into_iter()
            .map(|h| (h.record, h.distance))
            .collect();
        hits.sort_unstable_by_key(|(r, d)| (*d, r.record_ix));
        found.extend(capped(hits, MAX_PROBE_HITS, notes));
    }
    dedupe(&mut found);
    let roots: Vec<usize> = (0..found.len())
        .filter(|&i| found[i].0.claim.kind.is_original())
        .collect();
    let mut expanded: Vec<(Record, u32)> = Vec::new();
    for i in roots {
        let room = MAX_CANDIDATES
            .saturating_sub(found.len() + expanded.len())
            .min(MAX_DERIVATIVES);
        let mut derivatives = oracle.derivatives(&found[i].0);
        derivatives.retain(|d| {
            found
                .binary_search_by_key(&d.record_ix, |(r, _)| r.record_ix)
                .is_err()
        });
        let derivatives = capped(derivatives, room, notes);
        expanded.extend(derivatives.into_iter().map(|r| (r, NO_DISTANCE)));
    }
    found.append(&mut expanded);
    dedupe(&mut found);
    let mut candidates: Vec<Candidate> = found
        .into_iter()
        .map(|(record, probed)| {
            let bundle = oracle.fingerprints(&record);
            let computed = bundle
                .as_ref()
                .and_then(|b| query.min_distance(b.pdq()))
                .unwrap_or(NO_DISTANCE);
            Candidate {
                record,
                bundle,
                distance: probed.min(computed),
            }
        })
        .collect();
    candidates.sort_unstable_by_key(|c| (c.distance, c.record.record_ix));
    Pool {
        candidates: capped(candidates, MAX_CANDIDATES, notes),
        watermark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{ENROLLED_NS, FixedWatermarker, MemoryOracle, TestCa, photo};
    use witness_format::{Edit, Kind};

    fn setup() -> (TestCa, MemoryOracle) {
        (TestCa::new(3), MemoryOracle::new())
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn dedupe_keeps_smallest_distance_per_record() {
        let (ca, mut oracle) = setup();
        let dev = ca.device(9, 9);
        let a = oracle.anchor(dev.claim(1, Kind::Original, 1), 5);
        let b = oracle.anchor(dev.claim(2, Kind::Original, 2), 6);
        let mut found = vec![
            (b.clone(), 40),
            (a.clone(), 12),
            (b.clone(), 7),
            (a.clone(), NO_DISTANCE),
        ];
        dedupe(&mut found);
        assert_eq!(found, vec![(a, 12), (b, 7)]);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn capped_flags_only_when_cutting() {
        let mut notes = Notes::default();
        assert_eq!(capped(vec![1, 2, 3], 3, &mut notes), [1, 2, 3]);
        assert!(notes.flags.is_empty());
        assert_eq!(capped(vec![1, 2, 3, 4], 3, &mut notes), [1, 2, 3]);
        assert!(notes.flags.contains(VerdictFlags::TRUNCATED_CANDIDATES));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn missing_watermarker_is_reported_and_no_candidates_found() {
        let (ca, oracle) = setup();
        let query = Query::new(photo(160, 120, 1));
        let mut notes = Notes::default();
        let pool = gather(&query, &oracle, &Options::web(&ca.anchor), &mut notes);
        assert!(pool.candidates.is_empty() && pool.watermark.is_none());
        assert!(notes.reasons.contains(Reason::WatermarkUnavailable));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn watermark_hits_are_capped_at_thirty_two() {
        let (ca, mut oracle) = setup();
        let dev = ca.device(9, 9);
        let claims: Vec<_> = (0..40).map(|i| dev.claim(77, Kind::Original, i)).collect();
        let id = claims[0].capture_id;
        for (i, c) in claims.into_iter().enumerate() {
            oracle.anchor(c, ENROLLED_NS + 100 - i as u64);
        }
        let wm = FixedWatermarker::reading(id);
        let query = Query::new(photo(160, 120, 2));
        let mut notes = Notes::default();
        let pool = gather(
            &query,
            &oracle,
            &Options::web(&ca.anchor).with_watermarker(&wm),
            &mut notes,
        );
        assert_eq!(pool.candidates.len(), MAX_WATERMARK);
        assert!(notes.reasons.contains(Reason::WatermarkFound));
        assert_eq!(pool.watermark.map(|d| d.capture_id), Some(id));
        assert!(
            pool.candidates
                .iter()
                .all(|c| c.distance == NO_DISTANCE && c.bundle.is_none())
        );
        let ixs: Vec<u64> = pool.candidates.iter().map(|c| c.record.record_ix).collect();
        assert!(ixs.windows(2).all(|w| w[0] < w[1]));
        assert!(ixs.iter().all(|&ix| ix >= 8));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn derivatives_of_roots_are_expanded_within_budget() {
        let (ca, mut oracle) = setup();
        let dev = ca.device(9, 9);
        let root = oracle.anchor(dev.claim(5, Kind::Original, 0), 1);
        for i in 0..(MAX_CANDIDATES as u64 + 10) {
            let kind = Kind::Derivative {
                parent: root.record_id,
                root: root.claim.capture_id,
                edits: vec![Edit::Rotate {
                    deg_e2: (i % 1000) as i16,
                }],
            };
            let mut c = dev.claim(1000 + i, kind, i + 1);
            c.asset_hash = [i as u8; 32];
            oracle.anchor(c, 10 + i);
        }
        let wm = FixedWatermarker::reading(root.claim.capture_id);
        let query = Query::new(photo(160, 120, 3));
        let mut notes = Notes::default();
        let pool = gather(
            &query,
            &oracle,
            &Options::android(&ca.anchor).with_watermarker(&wm),
            &mut notes,
        );
        assert_eq!(pool.candidates.len(), MAX_CANDIDATES);
        assert!(notes.flags.contains(VerdictFlags::TRUNCATED_CANDIDATES));
        assert!(
            pool.candidates
                .iter()
                .any(|c| c.record.record_ix == root.record_ix)
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn probe_distances_rank_candidates() {
        let (ca, mut oracle) = setup();
        let dev = ca.device(9, 9);
        let img = photo(200, 150, 4);
        let near = oracle.anchor(dev.claim(1, Kind::Original, 1), 1);
        let far = oracle.anchor(dev.claim(2, Kind::Original, 2), 2);
        oracle.set_fingerprints(
            near.record_ix,
            witness_fingerprint::fingerprint(img.view()).unwrap(),
        );
        oracle.set_fingerprints(
            far.record_ix,
            witness_fingerprint::fingerprint(photo(200, 150, 5).view()).unwrap(),
        );
        oracle.pdq_radius = 256;
        let query = Query::new(img);
        let mut notes = Notes::default();
        let pool = gather(&query, &oracle, &Options::web(&ca.anchor), &mut notes);
        assert_eq!(pool.candidates.len(), 2);
        assert_eq!(pool.candidates[0].record.record_ix, near.record_ix);
        assert_eq!(pool.candidates[0].distance, 0);
        assert!(pool.candidates[1].distance > 0);
    }
}
