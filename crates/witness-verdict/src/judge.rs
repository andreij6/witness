use crate::candidates::gather;
use crate::confirm::decide;
use crate::credentials::{self, Path};
use crate::exact;
use crate::options::Options;
use crate::oracle::Oracle;
use crate::query::{self, Query};
use crate::verdict::{Notes, Reason, Verdict};
use witness_format::Outcome;

fn no_record<O: Oracle + ?Sized>(oracle: &O, mut notes: Notes, reason: Reason) -> Verdict {
    notes.reason(reason);
    if oracle.index_pending() {
        notes.reason(Reason::IndexPending);
    }
    notes.no_record()
}

pub fn judge<O: Oracle + ?Sized>(input: &[u8], oracle: &O, options: &Options<'_>) -> Verdict {
    let mut notes = Notes::default();
    let (asset_hash, skip_exact) = match credentials::check(input, oracle, options, &mut notes) {
        Path::Decided(v) => return *v,
        Path::Continue {
            asset_hash,
            skip_exact,
        } => (asset_hash, skip_exact),
    };
    let decoded = query::decode(input, options.max_pixels).ok();
    if !skip_exact {
        let pixel_hash = decoded.as_ref().map(|d| &d.pixel_hash);
        if let Some(v) = exact::check(oracle, &asset_hash, pixel_hash, &mut notes) {
            return v;
        }
    }
    let Some(decoded) = decoded else {
        return no_record(oracle, notes, Reason::Undecodable);
    };
    let query = Query::new(decoded.image);
    let pool = gather(&query, oracle, options, &mut notes);
    if pool.candidates.is_empty() {
        return no_record(oracle, notes, Reason::NoCandidates);
    }
    let mut verdict = decide(&query, &pool, oracle, options, notes);
    if verdict.outcome == Outcome::NoRecord && oracle.index_pending() {
        verdict.reasons.insert(Reason::IndexPending);
    }
    verdict
}
