use super::*;
use crate::testkit::{World, record_wire};
use crate::wire::{B64, KeyWire, PdqHitWire, PdqWire};
use witness_core::verdict::Oracle;

fn world_with(n: u64) -> World {
    let mut w = World::new();
    for seed in 0..n {
        w.original(seed + 1, true);
    }
    w
}

fn fetched_records(w: &World) -> Fetched {
    let mut f = Fetched::default();
    for r in w.oracle.records() {
        f.records
            .insert(r.record_id.to_string(), Some(record_wire(r)));
    }
    f
}

#[test]
fn need_gates_later_stages_behind_earlier_ones() {
    let mut n = Need::default();
    assert!(n.is_empty());
    n.ask(STAGE_LOOKUP, |n| {
        n.keys.insert(Hex(vec![1]));
    });
    n.ask(STAGE_PIXELS, |n| {
        n.references.insert("A".into());
    });
    n.ask(STAGE_RESOLVE, |n| {
        n.record_ixs.insert(Dec(1));
    });
    assert!(n.references.is_empty() && n.record_ixs.is_empty());
    n.ask(STAGE_LOOKUP, |n| {
        n.keys.insert(Hex(vec![2]));
    });
    assert_eq!(n.keys.len(), 2);
    assert_eq!(n.stage(), Some(STAGE_LOOKUP));
}

#[test]
fn need_keeps_later_entries_when_an_earlier_stage_arrives() {
    let mut n = Need::default();
    n.ask(STAGE_DETAIL, |n| {
        n.fingerprints.insert(Dec(3));
    });
    n.ask(STAGE_LOOKUP, |n| {
        n.records.insert("X".into());
    });
    assert_eq!(n.stage(), Some(STAGE_LOOKUP));
    assert_eq!(n.fingerprints.len(), 1);
    assert!(!n.is_empty());
}

#[test]
fn need_serializes_every_field_in_camel_case() {
    let mut n = Need::default();
    n.ask(STAGE_LOOKUP, |n| {
        n.exact = Some(ExactNeed {
            asset: Hex(vec![0xaa]),
            pixel: None,
        });
        n.record_ixs.insert(Dec(7));
    });
    let v = serde_json::to_value(&n).unwrap();
    assert_eq!(v["exact"]["asset"], "aa");
    assert!(v["exact"]["pixel"].is_null());
    assert_eq!(v["recordIxs"][0], "7");
    for k in [
        "keys",
        "records",
        "watermark",
        "derivatives",
        "fingerprints",
        "references",
    ] {
        assert!(v[k].as_array().unwrap().is_empty(), "{k}");
    }
    assert!(v["pdq"].is_null());
    assert!(v.get("stage").is_none());
}

#[test]
fn answers_parse_empty_and_reject_bad_json() {
    assert_eq!(Answers::parse("{}").unwrap(), Answers::default());
    assert!(matches!(Answers::parse("{"), Err(Error::Json(_))));
    assert!(matches!(
        Answers::parse("{\"keys\":{\"zz\":null}}"),
        Err(Error::Json(_))
    ));
    assert_eq!(
        Answers::parse("{\"keys\":{\"00\":null}}"),
        Err(Error::Malformed("key id"))
    );
    assert_eq!(
        Answers::parse("{\"records\":{\"!\":null}}"),
        Err(Error::Malformed("record id"))
    );
    assert_eq!(
        Answers::parse("{\"references\":{\"?\":null}}"),
        Err(Error::Malformed("record id"))
    );
}

#[test]
fn answers_reject_keys_filed_under_another_id() {
    let w = World::new();
    let key = w.oracle.get_key(&w.dev.key_id()).unwrap();
    let mut f = Fetched::default();
    f.keys
        .insert(Hex(vec![9; 16]), Some(KeyWire::from_info(&key)));
    assert_eq!(Answers::from_fetched(f), Err(Error::Malformed("key id")));
}

#[test]
fn answers_reject_records_filed_under_another_id_or_index() {
    let w = world_with(2);
    let recs = w.oracle.records();
    let mut f = Fetched::default();
    f.records
        .insert(recs[1].record_id.to_string(), Some(record_wire(&recs[0])));
    assert_eq!(Answers::from_fetched(f), Err(Error::Malformed("record id")));
    let mut f = fetched_records(&w);
    let mut clash = record_wire(&recs[1]);
    clash.record_ix = Dec(recs[0].record_ix);
    f.records.insert(recs[1].record_id.to_string(), Some(clash));
    assert_eq!(
        Answers::from_fetched(f),
        Err(Error::Malformed("duplicate record index"))
    );
}

#[test]
fn unanswered_lookups_are_recorded() {
    let w = world_with(1);
    let rec = w.oracle.records()[0].clone();
    let answers = Answers::default();
    let o = ReplayOracle::new(&answers);
    assert_eq!(o.get_key(&w.dev.key_id()), None);
    assert_eq!(o.get_record(&rec.record_id), None);
    assert!(o.exact(&[1; 32], Some(&[2; 32])).is_empty());
    assert!(o.by_watermark(rec.claim.capture_id).is_empty());
    assert_eq!(o.pdq_probe(&[PdqHash([3; 32])]), PdqProbe::default());
    assert!(!o.index_pending());
    let n = o.into_need();
    assert_eq!(n.stage(), Some(STAGE_LOOKUP));
    assert_eq!(n.keys.iter().next().unwrap().0, w.dev.key_id().as_bytes());
    assert_eq!(n.records.iter().next().unwrap(), &rec.record_id.to_string());
    assert_eq!(n.exact.as_ref().unwrap().pixel, Some(Hex(vec![2; 32])));
    assert_eq!(
        n.watermark.iter().next().unwrap().0,
        rec.claim.capture_id.to_u64()
    );
    assert_eq!(n.pdq, Some(vec![Hex(vec![3; 32])]));
}

#[test]
fn later_stage_lookups_wait_for_earlier_answers() {
    let w = world_with(1);
    let rec = w.oracle.records()[0].clone();
    let answers = Answers::default();
    let o = ReplayOracle::new(&answers);
    o.get_key(&w.dev.key_id());
    assert!(o.derivatives(&rec).is_empty());
    assert_eq!(o.fingerprints(&rec), None);
    assert_eq!(o.reference_image(&rec), None);
    let n = o.into_need();
    assert!(n.derivatives.is_empty() && n.fingerprints.is_empty() && n.references.is_empty());
}

#[test]
fn detail_lookups_are_recorded_when_nothing_earlier_is_missing() {
    let w = world_with(1);
    let rec = w.oracle.records()[0].clone();
    let answers = Answers::default();
    let o = ReplayOracle::new(&answers);
    o.reference_image(&rec);
    o.fingerprints(&rec);
    o.derivatives(&rec);
    let n = o.into_need();
    assert_eq!(n.stage(), Some(STAGE_RESOLVE));
    assert_eq!(n.derivatives.iter().next(), Some(&Dec(rec.record_ix)));
    assert!(n.references.contains(&rec.record_id.to_string()));
}

#[test]
fn answered_lookups_resolve_records_by_index() {
    let w = world_with(2);
    let recs = w.oracle.records().to_vec();
    let key = w.oracle.get_key(&w.dev.key_id()).unwrap();
    let mut f = fetched_records(&w);
    f.keys.insert(
        Hex(key.key_id.as_bytes().to_vec()),
        Some(KeyWire::from_info(&key)),
    );
    f.exact = Some(vec![Dec(1)]);
    f.watermark
        .insert(Dec(recs[0].claim.capture_id.to_u64()), vec![Dec(0), Dec(1)]);
    f.pdq = Some(PdqWire {
        hits: vec![PdqHitWire {
            ix: Dec(1),
            distance: 4,
        }],
        truncated: true,
    });
    f.derivatives.insert(Dec(0), vec![Dec(1)]);
    f.references
        .insert(recs[0].record_id.to_string(), Some(B64(vec![1, 2])));
    f.references.insert(recs[1].record_id.to_string(), None);
    f.index_pending = true;
    let answers = Answers::from_fetched(f).unwrap();
    let o = ReplayOracle::new(&answers);
    assert_eq!(o.get_key(&key.key_id), Some(key));
    assert_eq!(o.get_record(&recs[0].record_id), Some(recs[0].clone()));
    assert_eq!(o.exact(&[0; 32], None), vec![recs[1].clone()]);
    assert_eq!(o.by_watermark(recs[0].claim.capture_id), recs);
    let probe = o.pdq_probe(&[]);
    assert!(probe.truncated);
    assert_eq!(
        probe.hits,
        vec![PdqHit {
            record: recs[1].clone(),
            distance: 4
        }]
    );
    assert_eq!(o.derivatives(&recs[0]), vec![recs[1].clone()]);
    assert_eq!(o.reference_image(&recs[0]), Some(vec![1, 2]));
    assert_eq!(o.reference_image(&recs[1]), None);
    assert!(o.index_pending());
    assert!(o.into_need().is_empty());
}

#[test]
fn explicit_absence_is_not_asked_again() {
    let w = world_with(1);
    let rec = w.oracle.records()[0].clone();
    let mut f = Fetched::default();
    f.keys.insert(Hex(w.dev.key_id().as_bytes().to_vec()), None);
    f.records.insert(rec.record_id.to_string(), None);
    f.exact = Some(vec![Dec(5)]);
    f.absent_ixs = vec![Dec(5)];
    f.fingerprints.insert(Dec(0), None);
    let answers = Answers::from_fetched(f).unwrap();
    let o = ReplayOracle::new(&answers);
    assert_eq!(o.get_key(&w.dev.key_id()), None);
    assert_eq!(o.get_record(&rec.record_id), None);
    assert!(o.exact(&[0; 32], None).is_empty());
    assert_eq!(o.fingerprints(&rec), None);
    assert!(o.into_need().is_empty());
}

#[test]
fn unknown_indexes_are_requested_for_resolution() {
    let f = Fetched {
        exact: Some(vec![Dec(3), Dec(4)]),
        pdq: Some(PdqWire {
            hits: vec![PdqHitWire {
                ix: Dec(4),
                distance: 1,
            }],
            truncated: false,
        }),
        ..Fetched::default()
    };
    let answers = Answers::from_fetched(f).unwrap();
    let o = ReplayOracle::new(&answers);
    assert!(o.exact(&[0; 32], None).is_empty());
    assert!(o.pdq_probe(&[]).hits.is_empty());
    let n = o.into_need();
    assert_eq!(n.stage(), Some(STAGE_RESOLVE));
    assert_eq!(n.record_ixs.iter().map(|d| d.0).collect::<Vec<_>>(), [3, 4]);
}

#[test]
fn tampered_fingerprints_are_dropped() {
    let mut w = World::new();
    let s = w.original(9, false);
    let rec = w.oracle.records()[0].clone();
    let mut wire = FingerprintsWire::from_bundle(&s.fingerprints);
    let mut f = Fetched::default();
    f.fingerprints.insert(Dec(0), Some(wire.clone()));
    let answers = Answers::from_fetched(f.clone()).unwrap();
    assert_eq!(
        ReplayOracle::new(&answers).fingerprints(&rec),
        Some(s.fingerprints)
    );
    wire.grid.0[0] ^= 0x80;
    f.fingerprints.insert(Dec(0), Some(wire));
    let answers = Answers::from_fetched(f).unwrap();
    let o = ReplayOracle::new(&answers);
    assert_eq!(o.fingerprints(&rec), None);
    assert!(o.into_need().is_empty());
}

#[test]
fn parsing_is_deterministic() {
    let w = world_with(2);
    let json = serde_json::to_string(&fetched_records(&w)).unwrap();
    assert_eq!(
        Answers::parse(&json).unwrap(),
        Answers::parse(&json).unwrap()
    );
}
