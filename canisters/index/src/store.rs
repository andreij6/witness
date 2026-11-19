use crate::buckets::Buckets;
use crate::multimap::Multimap;
use crate::row::Row;
use crate::trace::Trace;
use ic_stable_structures::memory_manager::MemoryId;
use ic_stable_structures::{Memory, StableBTreeMap, StableCell, StableVec};
use serde_bytes::ByteBuf;
use witness_canister_common::index::{hash_key, probe_within_limits};
use witness_canister_common::limits::{MAX_EXACT_HITS, MAX_WATERMARK_HITS};
use witness_canister_common::memory::index as ids;
use witness_canister_common::{Fingerprints, PdqHit, Posting, ProbeResult};
use witness_fingerprint::{
    BucketKey, HASH_BYTES, MIH_RADIUS, PdqHash, Posting as MihPosting, Prober, index_keys,
};
use witness_format::FingerprintBundle;
use witness_format::cbor::from_slice_canonical;
use witness_format::fingerprint::GRID_BYTES;

pub const TOP_CANDIDATES: usize = 2000;
pub const MAX_PDQ_DISTANCE: u32 = MIH_RADIUS;
pub const MAX_PDQ_HITS: usize = 32;
pub const MAX_DERIVATIVE_HITS: usize = 32;
const ERASED_RECORD: u64 = u64::MAX;

type Grid = [u8; GRID_BYTES];

pub struct Index<M: Memory> {
    rows: StableVec<Row, M>,
    grids: StableVec<Grid, M>,
    buckets: Buckets<M>,
    watermark: Multimap<u64, M>,
    exact_asset: Multimap<[u8; 16], M>,
    exact_pixel: Multimap<[u8; 16], M>,
    derivatives: Multimap<u64, M>,
    locals: StableVec<u64, M>,
    traces: StableBTreeMap<u64, Trace, M>,
    indexed_upto: StableCell<u64, M>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub records: u64,
    pub rows: u64,
    pub blocks: u64,
    pub indexed_upto: u64,
}

fn decode_bundle(bytes: &[u8]) -> Option<FingerprintBundle> {
    from_slice_canonical::<FingerprintBundle>(bytes).ok()
}

fn as_hash(bytes: &[u8]) -> Option<PdqHash> {
    <[u8; HASH_BYTES]>::try_from(bytes).ok().map(PdqHash)
}

impl<M: Memory> Index<M> {
    pub fn init(memory: impl Fn(MemoryId) -> M) -> Self {
        Self {
            rows: StableVec::init(memory(ids::PDQ_ROWS)),
            grids: StableVec::init(memory(ids::GRIDS)),
            buckets: Buckets::init(memory(ids::POSTINGS)),
            watermark: Multimap::init(memory(ids::WATERMARK)),
            exact_asset: Multimap::init(memory(ids::EXACT_ASSET)),
            exact_pixel: Multimap::init(memory(ids::EXACT_PIXEL)),
            derivatives: Multimap::init(memory(ids::DERIVATIVES)),
            locals: StableVec::init(memory(ids::LOCAL_TO_RECORD)),
            traces: StableBTreeMap::init(memory(ids::RECORD_TO_LOCAL)),
            indexed_upto: StableCell::init(memory(ids::STATE), 0),
        }
    }

    pub fn indexed_upto(&self) -> u64 {
        *self.indexed_upto.get()
    }

    pub fn stats(&self) -> Stats {
        Stats {
            records: self.traces.len(),
            rows: self.locals.len(),
            blocks: self.buckets.blocks(),
            indexed_upto: self.indexed_upto(),
        }
    }

    pub fn put_all<'a>(&mut self, postings: impl IntoIterator<Item = &'a Posting>) -> usize {
        postings.into_iter().filter(|p| self.put(p)).count()
    }

    pub fn put(&mut self, p: &Posting) -> bool {
        let ix = p.record_ix;
        if self.traces.contains_key(&ix) {
            return false;
        }
        let bundle = p.fingerprints.as_ref().and_then(|b| decode_bundle(b));
        let local_ix = bundle.as_ref().and_then(|b| self.store_fingerprints(ix, b));
        let asset = p.asset_key();
        let pixel = p.pixel_key();
        self.watermark.insert(p.wm_root, ix, MAX_WATERMARK_HITS);
        if let Some(k) = asset {
            self.exact_asset.insert(k, ix, MAX_EXACT_HITS);
        }
        if let Some(k) = pixel {
            self.exact_pixel.insert(k, ix, MAX_EXACT_HITS);
        }
        let root_ix = p
            .root_ix
            .filter(|&root| root != ix && !self.is_erased(root));
        if let Some(root) = root_ix {
            self.derivatives.insert(root, ix, usize::MAX);
        }
        self.traces.insert(
            ix,
            Trace {
                local_ix,
                wm_root: p.wm_root,
                asset,
                pixel,
                root_ix,
                erased: false,
            },
        );
        let upto = ix.saturating_add(1).max(self.indexed_upto());
        self.indexed_upto.set(upto);
        true
    }

    fn is_erased(&self, ix: u64) -> bool {
        self.traces.get(&ix).is_some_and(|t| t.erased)
    }

    fn store_fingerprints(&mut self, ix: u64, bundle: &FingerprintBundle) -> Option<u32> {
        let row = Row::from_bundle(bundle)?;
        let local_ix = u32::try_from(self.locals.len()).expect("local_ix space exhausted");
        self.rows.push(&row);
        self.grids.push(&bundle.grid_bytes());
        self.locals.push(&ix);
        if row.indexed() {
            self.post(local_ix, &row);
        }
        Some(local_ix)
    }

    fn keyed(local_ix: u32, row: &Row) -> Vec<(BucketKey, MihPosting)> {
        let mut out = Vec::with_capacity(row.len() * witness_fingerprint::MIH_CHUNKS);
        for (shape, hash) in row.shapes() {
            let posting = MihPosting { local_ix, shape };
            out.extend(index_keys(&hash).into_iter().map(|k| (k, posting)));
        }
        out.sort_unstable();
        out
    }

    fn post(&mut self, local_ix: u32, row: &Row) {
        let keyed = Self::keyed(local_ix, row);
        for run in keyed.chunk_by(|a, b| a.0 == b.0) {
            self.buckets.append(run[0].0, run.iter().map(|e| e.1));
        }
    }

    fn unpost(&mut self, local_ix: u32, row: &Row) {
        for (key, posting) in Self::keyed(local_ix, row) {
            self.buckets.remove(key, posting);
        }
    }

    pub fn erase(&mut self, ix: u64) -> bool {
        let trace = self.traces.get(&ix);
        self.traces.insert(ix, Trace::ERASED);
        let Some(trace) = trace.filter(|t| !t.erased) else {
            return false;
        };
        if let Some(local) = trace.local_ix {
            let at = u64::from(local);
            if let Some(row) = self.rows.get(at).filter(Row::indexed) {
                self.unpost(local, &row);
            }
            self.rows.set(at, &Row::EMPTY);
            self.grids.set(at, &[0; GRID_BYTES]);
            self.locals.set(at, &ERASED_RECORD);
        }
        self.watermark.remove(trace.wm_root, ix);
        if let Some(k) = trace.asset {
            self.exact_asset.remove(k, ix);
        }
        if let Some(k) = trace.pixel {
            self.exact_pixel.remove(k, ix);
        }
        if let Some(root) = trace.root_ix {
            self.derivatives.remove(root, ix);
        }
        self.derivatives.remove_all(ix);
        true
    }

    pub fn fingerprints(&self, ix: u64) -> Option<Fingerprints> {
        let at = u64::from(self.traces.get(&ix)?.local_ix?);
        let row = self.rows.get(at)?;
        let grid = self.grids.get(at)?;
        Some(Fingerprints {
            grid: grid.to_vec(),
            shapes: row
                .shapes()
                .map(|(tag, hash)| (tag, ByteBuf::from(hash.0.to_vec())))
                .collect(),
            quality: row.quality(),
            portrait: row.portrait(),
        })
    }

    pub fn derivatives(&self, root: u64) -> Vec<u64> {
        self.derivatives.get(root, MAX_DERIVATIVE_HITS)
    }

    pub fn probe(&self, pdq: &[ByteBuf], wm: &[u64], exact: &[ByteBuf]) -> Option<ProbeResult> {
        if !probe_within_limits(pdq, wm, exact) {
            return None;
        }
        let queries: Vec<PdqHash> = pdq.iter().filter_map(|h| as_hash(h)).collect();
        let (pdq, truncated_candidates) = self.pdq_hits(&queries);
        Some(ProbeResult {
            pdq,
            watermark: self.watermark_hits(wm),
            exact: self.exact_hits(exact),
            truncated_candidates,
            indexed_upto: self.indexed_upto(),
        })
    }

    fn watermark_hits(&self, ids: &[u64]) -> Vec<u64> {
        let mut out = Vec::new();
        for &id in ids {
            self.watermark.extend_into(id, MAX_WATERMARK_HITS, &mut out);
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    fn exact_hits(&self, hashes: &[ByteBuf]) -> Vec<u64> {
        let mut out = Vec::new();
        for key in hashes.iter().filter_map(|h| hash_key(h)) {
            self.exact_asset.extend_into(key, MAX_EXACT_HITS, &mut out);
            self.exact_pixel.extend_into(key, MAX_EXACT_HITS, &mut out);
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    fn nominate(&self, queries: &[PdqHash]) -> (Vec<(u32, u16)>, bool) {
        let mut prober = Prober::with_capacity(self.locals.len() as u32);
        let mut nominated: Vec<(u32, u16)> = Vec::new();
        let mut truncated = false;
        for (q, query) in queries.iter().enumerate() {
            let ranked = prober.probe(&self.buckets, query, TOP_CANDIDATES);
            truncated |= ranked.truncated;
            nominated.extend(ranked.candidates.iter().map(|c| (c.local_ix, 1u16 << q)));
        }
        nominated.sort_unstable_by_key(|n| n.0);
        (nominated, truncated)
    }

    fn pdq_hits(&self, queries: &[PdqHash]) -> (Vec<PdqHit>, bool) {
        if queries.is_empty() {
            return (Vec::new(), false);
        }
        let (nominated, mut truncated) = self.nominate(queries);
        let mut hits = Vec::new();
        for run in nominated.chunk_by(|a, b| a.0 == b.0) {
            let at = u64::from(run[0].0);
            let mask = run.iter().fold(0u16, |m, n| m | n.1);
            let Some(record_ix) = self.locals.get(at).filter(|&r| r != ERASED_RECORD) else {
                continue;
            };
            let Some(row) = self.rows.get(at) else {
                continue;
            };
            let distance = queries
                .iter()
                .enumerate()
                .filter(|(q, _)| mask >> q & 1 == 1)
                .filter_map(|(_, query)| row.min_distance(query))
                .min();
            if let Some(distance) = distance.filter(|&d| d <= MAX_PDQ_DISTANCE) {
                hits.push(PdqHit {
                    record_ix,
                    distance,
                });
            }
        }
        hits.sort_unstable_by_key(|h| (h.distance, h.record_ix));
        if hits.len() > MAX_PDQ_HITS {
            hits.truncate(MAX_PDQ_HITS);
            truncated = true;
        }
        (hits, truncated)
    }
}

#[cfg(test)]
mod tests;
