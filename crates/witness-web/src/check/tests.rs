use super::*;
use crate::testkit::{World, jpeg_of, replay};
use crate::wire::Hex;
use witness_core::format::{Kind, Outcome};
use witness_core::image::strip_and_normalize_jpeg;
use witness_core::verdict::Reason;

#[test]
fn plan_of_a_sealed_photo_asks_for_key_and_record_first() {
    let mut w = World::new();
    let s = w.original(1, false);
    let step = step(&s.jpeg, &w.options(), &Answers::default()).unwrap();
    assert!(!step.done());
    assert!(
        step.need
            .keys
            .contains(&Hex(w.dev.key_id().as_bytes().to_vec()))
    );
    assert_eq!(step.verdict.outcome, Outcome::NoRecord);
}

#[test]
fn plan_json_reports_credentials_and_lookups() {
    let mut w = World::new();
    let s = w.original(2, false);
    let opts = serde_json::to_string(&w.options()).unwrap();
    let v: Value = serde_json::from_str(&plan_json(&s.jpeg, &opts).unwrap()).unwrap();
    assert_eq!(v["done"], false);
    assert_eq!(v["credentials"]["kind"], "witness");
    assert_eq!(v["credentials"]["recordId"], s.record_id.to_string());
    assert_eq!(v["credentials"]["bindingIntact"], true);
    assert_eq!(
        v["credentials"]["claim"]["keyId"],
        hex::encode(w.dev.key_id().as_bytes())
    );
    assert_eq!(v["need"]["keys"].as_array().unwrap().len(), 1);
    assert_eq!(v["verdict"]["outcome"], "NO_RECORD");
}

#[test]
fn plan_of_a_plain_photo_asks_for_exact_and_pdq() {
    let w = World::new();
    let step = step(&jpeg_of(3, 90), &w.options(), &Answers::default()).unwrap();
    let exact = step.need.exact.as_ref().expect("exact");
    assert_eq!(exact.asset.0.len(), 32);
    assert_eq!(exact.pixel.as_ref().map(|p| p.0.len()), Some(32));
    let pdq = step.need.pdq.as_ref().expect("pdq");
    assert!(!pdq.is_empty() && pdq.len() <= 16);
    assert!(step.need.keys.is_empty() && step.need.records.is_empty());
}

#[test]
fn credentials_classify_inputs() {
    let mut w = World::new();
    let opts = w.options();
    let s = w.original(4, false);
    assert_eq!(credentials(b"not an image", &opts).unwrap()["kind"], "none");
    assert_eq!(
        credentials(&jpeg_of(5, 90), &opts).unwrap()["kind"],
        "absent"
    );
    let stripped = strip_and_normalize_jpeg(&s.jpeg).unwrap();
    assert_eq!(credentials(&stripped, &opts).unwrap()["kind"], "absent");
    let mut truncated = s.jpeg.clone();
    truncated.truncate(40);
    let kind = credentials(&truncated, &opts).unwrap()["kind"].clone();
    assert!(kind == "invalid" || kind == "none", "{kind}");
    let other_ca = World::new_with_seed(9);
    let foreign = credentials(&s.jpeg, &other_ca.options()).unwrap();
    assert_ne!(foreign["kind"], "witness");
}

#[test]
fn tampered_pixels_break_the_binding() {
    let mut w = World::new();
    let s = w.original(6, false);
    let mut jpeg = s.jpeg.clone();
    let n = jpeg.len();
    jpeg[n - 40] ^= 0x55;
    let c = credentials(&jpeg, &w.options()).unwrap();
    if c["kind"] == "witness" {
        assert_eq!(c["bindingIntact"], false);
    }
}

#[test]
fn bad_options_and_answers_are_rejected() {
    let w = World::new();
    let jpeg = jpeg_of(7, 90);
    assert!(matches!(plan_json(&jpeg, "{"), Err(Error::Json(_))));
    assert_eq!(
        plan_json(&jpeg, "{\"anchor\":\"00\"}"),
        Err(Error::Malformed("anchor"))
    );
    let opts = serde_json::to_string(&w.options()).unwrap();
    assert!(judge_json(&jpeg, &opts, "nope").is_err());
    assert!(judge_json(&jpeg, &opts, "{}").is_ok());
}

#[test]
fn options_map_onto_verdict_options() {
    let w = World::new();
    let anchor = w.options().anchor().unwrap();
    let mut o = w.options();
    assert_eq!(o.options(&anchor).top_k, witness_core::verdict::WEB_TOP_K);
    o.top_k = Some(99);
    o.at_s = Some(123);
    o.max_pixels = Some(1000);
    let mapped = o.options(&anchor);
    assert_eq!(mapped.top_k, witness_core::verdict::MAX_TOP_K);
    assert_eq!(mapped.at_s, Some(123));
    assert_eq!(mapped.max_pixels, 1000);
}

#[test]
fn garbage_input_finishes_as_no_record() {
    let w = World::new();
    let opts = serde_json::to_string(&w.options()).unwrap();
    let first: Value = serde_json::from_str(&plan_json(&[0xff; 64], &opts).unwrap()).unwrap();
    assert_eq!(first["credentials"]["kind"], "none");
    let exact = &first["need"]["exact"];
    assert!(exact["pixel"].is_null());
    let fetched = "{\"exact\":[]}";
    let done: Value =
        serde_json::from_str(&judge_json(&[0xff; 64], &opts, fetched).unwrap()).unwrap();
    assert_eq!(done["done"], true);
    assert_eq!(done["verdict"]["outcome"], "NO_RECORD");
    assert!(
        done["verdict"]["reasons"]
            .as_array()
            .unwrap()
            .contains(&"Undecodable".into())
    );
}

#[test]
fn replay_converges_to_original_for_a_sealed_photo() {
    let mut w = World::new();
    let s = w.original(8, true);
    let (step, rounds, _) = replay(&s.jpeg, &w.options(), &w.oracle);
    assert_eq!(step.verdict.outcome, Outcome::Original);
    assert!(rounds <= 3, "{rounds}");
    let v = step.to_json();
    assert_eq!(v["verdict"]["recordId"], s.record_id.to_string());
    assert_eq!(v["done"], true);
}

#[test]
fn replay_matches_native_judge_for_a_derivative() {
    let mut w = World::new();
    let root = w.original(10, true);
    let edited = w.seal(
        &jpeg_of(10, 90),
        Kind::Derivative {
            parent: root.record_id,
            root: root.claim.capture_id,
            edits: vec![witness_core::format::Edit::Rotate { deg_e2: 50 }],
        },
    );
    w.anchor(&edited, true);
    let anchor = w.options().anchor().unwrap();
    let native = judge(&edited.jpeg, &w.oracle, &w.options().options(&anchor));
    let (step, _, _) = replay(&edited.jpeg, &w.options(), &w.oracle);
    assert_eq!(step.verdict, native);
    assert_eq!(native.outcome, Outcome::MatchesWithChanges);
}

#[test]
fn step_is_deterministic() {
    let mut w = World::new();
    let s = w.original(11, true);
    let a = replay(&s.jpeg, &w.options(), &w.oracle);
    let b = replay(&s.jpeg, &w.options(), &w.oracle);
    assert_eq!(a.0, b.0);
    assert_eq!(a.1, b.1);
    assert_eq!(a.0.to_json().to_string(), b.0.to_json().to_string());
}

fn without_anchor() -> CheckOptions {
    CheckOptions {
        anchor: Hex(vec![]),
        ..CheckOptions::default()
    }
}

#[test]
fn trust_follows_the_configured_anchor() {
    let w = World::new();
    assert_eq!(w.options().trust(), AnchorTrust::Pinned);
    let dev = CheckOptions {
        dev_anchor: true,
        ..w.options()
    };
    assert_eq!(dev.trust(), AnchorTrust::Dev);
    assert_eq!(without_anchor().trust(), AnchorTrust::Missing);
    let dev_without = CheckOptions {
        dev_anchor: true,
        ..without_anchor()
    };
    assert_eq!(dev_without.trust(), AnchorTrust::Missing);
    let parsed = CheckOptions::parse("{}").unwrap();
    assert_eq!(parsed.trust(), AnchorTrust::Missing);
    let parsed = CheckOptions::parse(&format!(
        "{{\"anchor\":\"{}\",\"devAnchor\":true}}",
        hex::encode(&w.ca.root_der)
    ))
    .unwrap();
    assert_eq!(parsed.trust(), AnchorTrust::Dev);
    assert!(
        !serde_json::to_string(&w.options())
            .unwrap()
            .contains("devAnchor")
    );
    assert_eq!(
        [AnchorTrust::Pinned, AnchorTrust::Dev, AnchorTrust::Missing].map(AnchorTrust::name),
        ["pinned", "dev", "missing"]
    );
}

#[test]
fn unpinned_anchor_is_a_valid_root_that_signs_nothing_we_issue() {
    let a = TrustAnchor::from_der(UNPINNED_ANCHOR_DER).unwrap();
    assert_ne!(a.key_id(), World::new().ca.anchor.key_id());
    assert_eq!(
        without_anchor().anchor().map(|x| x.key_id().to_vec()),
        Ok(a.key_id().to_vec())
    );
}

#[test]
fn without_a_pinned_anchor_credentials_are_never_judged() {
    let mut w = World::new();
    let s = w.original(12, true);
    let opts = without_anchor();
    assert_eq!(credentials(&s.jpeg, &opts).unwrap()["kind"], "unverified");
    assert_eq!(
        credentials(&jpeg_of(13, 90), &opts).unwrap()["kind"],
        "absent"
    );
    let first = step(&s.jpeg, &opts, &Answers::default()).unwrap();
    assert!(first.need.keys.is_empty() && first.need.records.is_empty());
    assert!(first.need.exact.is_some());
    let (done, _, _) = replay(&s.jpeg, &opts, &w.oracle);
    assert_eq!(done.anchor, AnchorTrust::Missing);
    assert!(done.verdict.reasons.contains(Reason::UnverifiedCredentials));
    assert!(!done.verdict.reasons.contains(Reason::SealVerified));
    let v = done.to_json();
    assert_eq!(v["anchor"], "missing");
    let pinned = replay(&s.jpeg, &w.options(), &w.oracle).0;
    assert_eq!(pinned.anchor, AnchorTrust::Pinned);
    assert_eq!(pinned.to_json()["anchor"], "pinned");
    assert!(pinned.verdict.reasons.contains(Reason::SealVerified));
}

#[test]
fn unseen_photo_without_anchor_is_no_record() {
    let w = World::new();
    let (done, _, _) = replay(&jpeg_of(14, 90), &without_anchor(), &w.oracle);
    assert_eq!(done.verdict.outcome, Outcome::NoRecord);
}

#[test]
fn dev_anchor_judges_credentials_and_says_so() {
    let mut w = World::new();
    let s = w.original(15, true);
    let opts = CheckOptions {
        dev_anchor: true,
        ..w.options()
    };
    let (done, _, _) = replay(&s.jpeg, &opts, &w.oracle);
    assert_eq!(done.verdict.outcome, Outcome::Original);
    assert_eq!(done.to_json()["anchor"], "dev");
    let plan: Value =
        serde_json::from_str(&plan_json(&s.jpeg, &serde_json::to_string(&opts).unwrap()).unwrap())
            .unwrap();
    assert_eq!(plan["anchor"], "dev");
    assert_eq!(plan["credentials"]["kind"], "witness");
}
