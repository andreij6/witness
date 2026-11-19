use crate::oracle::{Oracle, Record};
use crate::verdict::{Evidence, Notes, Reason, Reference, Verdict};
use witness_format::{Outcome, VerdictFlags};

pub(crate) fn earliest(records: &[Record]) -> Option<&Record> {
    records.iter().min_by_key(|r| (r.anchored_ns, r.record_ix))
}

pub(crate) fn key_flags<O: Oracle + ?Sized>(oracle: &O, record: &Record, notes: &mut Notes) {
    notes.flag(record.flags);
    if let Some(key) = oracle.get_key(&record.claim.key.key_id) {
        if key.history.revoked_after(record.anchored_ns) {
            notes.flag(VerdictFlags::KEY_REVOKED_AFTER_ANCHOR);
        }
        if key.is_dev() {
            notes.flag(VerdictFlags::DEV_ATTESTATION);
        }
    }
}

pub(crate) fn check<O: Oracle + ?Sized>(
    oracle: &O,
    asset_hash: &[u8; 32],
    pixel_hash: Option<&[u8; 32]>,
    notes: &mut Notes,
) -> Option<Verdict> {
    let mut hits = oracle.exact(asset_hash, pixel_hash);
    hits.retain(|r| &r.claim.asset_hash == asset_hash || pixel_hash == Some(&r.claim.pixel_hash));
    let record = earliest(&hits)?;
    notes.reason(if &record.claim.asset_hash == asset_hash {
        Reason::ExactAssetHash
    } else {
        Reason::ExactPixelHash
    });
    key_flags(oracle, record, notes);
    let outcome = if record.claim.kind.presentation_only() {
        Outcome::Original
    } else {
        Outcome::MatchesWithChanges
    };
    Some(notes.verdict(
        outcome,
        Some(Reference::certain(record, Evidence::ExactHash)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{MemoryOracle, TestCa};
    use witness_format::Kind;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn earliest_breaks_ties_by_record_ix() {
        let ca = TestCa::new(1);
        let dev = ca.device(2, 2);
        let mut oracle = MemoryOracle::new();
        let a = oracle.anchor(dev.claim(1, Kind::Original, 1), 50);
        let b = oracle.anchor(dev.claim(2, Kind::Original, 2), 10);
        let c = oracle.anchor(dev.claim(3, Kind::Original, 3), 10);
        assert_eq!(earliest(&[a.clone(), c.clone(), b.clone()]), Some(&b));
        assert_eq!(earliest(core::slice::from_ref(&a)), Some(&a));
        assert_eq!(earliest(&[]), None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn no_hits_leave_notes_untouched() {
        let oracle = MemoryOracle::new();
        let mut notes = Notes::default();
        assert!(check(&oracle, &[1; 32], Some(&[2; 32]), &mut notes).is_none());
        assert_eq!(notes, Notes::default());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn unknown_key_adds_no_key_flags() {
        let ca = TestCa::new(1);
        let dev = ca.device(2, 2);
        let mut oracle = MemoryOracle::new();
        let mut claim = dev.claim(1, Kind::Original, 1);
        claim.asset_hash = [9; 32];
        oracle.anchor(claim, 1);
        let mut notes = Notes::default();
        let v = check(&oracle, &[9; 32], None, &mut notes).unwrap();
        assert_eq!(v.outcome, Outcome::Original);
        assert!(v.flags.is_empty());
    }
}
