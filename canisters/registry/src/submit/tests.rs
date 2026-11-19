use super::*;
use crate::enroll::tests::{NOW, attest, enroll, ready};
use crate::store::with;
use serde_bytes::ByteBuf;
use witness_c2pa::x509::verify_chain;
use witness_c2pa::{ManifestSpec, data_hash, seal_jpeg};
use witness_core::image::encode_jpeg;
use witness_format::{CaptureId, KeyInfo, PdqShape, StatusChange};
use witness_verdict::testkit::{CAPTURED, GENERATOR, TestDevice, photo};

pub struct Dev {
    pub device: TestDevice,
    pub anchor: TrustAnchor,
}

pub fn device(st: &mut Stores, seed: u64) -> Dev {
    let (info, leaf) = enroll(st, seed);
    let root = st.state.get().ca_root().unwrap().to_vec();
    let anchor = ca::anchor(&root).unwrap();
    let serial = verify_chain(&[&leaf], &anchor, None).unwrap().serial;
    let serial = serial.iter().fold(0u64, |a, b| (a << 8) | u64::from(*b));
    let device = TestDevice {
        key: attest::device_key(seed),
        spki: info.pubkey,
        serial,
        chain: vec![leaf, root],
    };
    Dev { device, anchor }
}

pub fn sealed(dev: &Dev, mut claim: WitnessClaim, seed: u64) -> Submission {
    let jpeg = encode_jpeg(&photo(32, 24, seed), 90).unwrap();
    claim.asset_hash = data_hash(&jpeg).unwrap();
    let cbor = claim.to_cbor().unwrap();
    let spec = ManifestSpec {
        id: [0x5a; 16],
        generator: GENERATOR,
        actions: CAPTURED,
        witness_claim: &cbor,
    };
    let s = seal_jpeg(&jpeg, &spec, &dev.device, &dev.anchor).unwrap();
    Submission {
        claim: cbor,
        c2pa_claim: s.c2pa_claim,
        cose: s.cose_sign1,
        fingerprints: None,
    }
}

fn original(dev: &Dev, capture: u64, counter: u64) -> Submission {
    sealed(
        dev,
        dev.device.claim(capture, Kind::Original, counter),
        capture,
    )
}

fn derivative(dev: &Dev, parent: &Submission, root: u64, capture: u64, counter: u64) -> Submission {
    let kind = Kind::Derivative {
        parent: parent.record_id(),
        root: CaptureId::new(root & CaptureId::RANDOM_MASK, 0).unwrap(),
        edits: vec![],
    };
    sealed(dev, dev.device.claim(capture, kind, counter), capture)
}

fn ok(r: SubmitResult) -> Anchored {
    match r {
        SubmitResult::Ok(a) => a,
        other => panic!("expected ok, got {other:?}"),
    }
}

fn err(r: SubmitResult) -> SubmitErr {
    match r {
        SubmitResult::Err(e) => e,
        other => panic!("expected err, got {other:?}"),
    }
}

fn one(st: &mut Stores, s: Submission) -> SubmitResult {
    let (mut r, _) = submit(st, vec![s], NOW).unwrap();
    r.remove(0)
}

fn outbox(st: &Stores) -> Vec<Out> {
    st.outbox
        .values()
        .map(|b| Out::decode(&b).unwrap())
        .collect()
}

#[test]
fn original_is_anchored_certified_and_queued() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let s = original(&dev, 77, 1);
        let id = s.record_id();
        let (r, roots) = submit(st, vec![s.clone()], NOW).unwrap();
        let a = ok(r[0].clone());
        assert_eq!((a.record_ix, a.anchored_ns), (0, NOW));
        assert_eq!(a.record_id, id.to_string());
        assert_eq!(roots.leaf_count, 1);
        assert_eq!(roots, st.roots());
        let e = st.record(0).unwrap();
        assert_eq!((e.claim, e.root_ix), (s.claim.clone(), 0));
        assert_eq!(st.record_ix(&id), Some(0));
        assert!(st.by_capture.contains_key(&pair(77, 0)));
        let out = outbox(st);
        assert_eq!(out.len(), 1);
        let Body::Put(p) = &out[0].body else {
            panic!("posting")
        };
        assert_eq!((p.record_ix, p.wm_root, p.root_ix), (0, 77, None));
        assert_eq!(out[0].dest, Dest::Index(0));
        assert_eq!(one(st, s), SubmitResult::Dup(a));
        assert_eq!(st.mmr().leaf_count(), 1);
    });
}

#[test]
fn derivatives_link_to_root_and_get_shells() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let o = original(&dev, 5, 1);
        ok(one(st, o.clone()));
        let d1 = derivative(&dev, &o, 5, 6, 2);
        let a1 = ok(one(st, d1.clone()));
        let d2 = derivative(&dev, &d1, 5, 7, 3);
        let a2 = ok(one(st, d2));
        assert_eq!((a1.record_ix, a2.record_ix), (1, 2));
        assert_eq!(st.record(2).unwrap().root_ix, 0);
        assert!(st.derivatives.contains_key(&pair(0, 1)));
        assert!(st.derivatives.contains_key(&pair(1, 2)));
        let out = outbox(st);
        assert_eq!(out.len(), 7);
        let syncs: Vec<_> = out
            .iter()
            .filter_map(|o| match &o.body {
                Body::Sync { record_ix, .. } => Some((o.dest, *record_ix)),
                _ => None,
            })
            .collect();
        assert_eq!(
            syncs,
            vec![(Dest::Media(0), Some(1)), (Dest::Media(0), Some(2))]
        );
        let shells: Vec<_> = out
            .iter()
            .filter_map(|o| match &o.body {
                Body::Shell(s) => Some((o.dest, s.record_id.clone(), s.record_ix)),
                _ => None,
            })
            .collect();
        assert_eq!(
            shells,
            vec![
                (Dest::Pages, a1.record_id, 1),
                (Dest::Pages, a2.record_id, 2)
            ]
        );
        let Body::Put(p) = &out[2].body else {
            panic!("posting")
        };
        assert_eq!((p.wm_root, p.root_ix), (5, Some(0)));
    });
}

#[test]
fn derivative_rules() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let other = device(st, 2);
        let o = original(&dev, 5, 1);
        assert_eq!(
            err(one(st, derivative(&dev, &o, 5, 6, 2))),
            SubmitErr::ParentUnknown
        );
        ok(one(st, o.clone()));
        assert!(matches!(
            err(one(st, derivative(&dev, &o, 9, 6, 3))),
            SubmitErr::Malformed(_)
        ));
        assert!(matches!(
            err(one(st, derivative(&other, &o, 5, 6, 1))),
            SubmitErr::Malformed(_)
        ));
        ok(one(st, derivative(&dev, &o, 5, 6, 4)));
    });
}

#[test]
fn content_edited_parents_cannot_be_derived_again() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let o = original(&dev, 5, 1);
        ok(one(st, o.clone()));
        let blurred = sealed(
            &dev,
            dev.device.claim(
                6,
                Kind::Derivative {
                    parent: o.record_id(),
                    root: CaptureId::new(5, 0).unwrap(),
                    edits: vec![witness_format::Edit::Crop(witness_format::Rect {
                        x: 0,
                        y: 0,
                        w: 16,
                        h: 12,
                    })],
                },
                2,
            ),
            6,
        );
        ok(one(st, blurred.clone()));
        assert_eq!(
            err(one(st, derivative(&dev, &blurred, 5, 7, 3))),
            SubmitErr::Malformed("parent has content edits".into())
        );
        let resized = derivative(&dev, &o, 5, 8, 4);
        ok(one(st, resized.clone()));
        ok(one(st, derivative(&dev, &resized, 5, 9, 5)));
    });
}

#[test]
fn rejects_unknown_key_and_bad_seals() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let s = original(&dev, 1, 1);
        st.keys.remove(&dev.device.key_id().to_bytes());
        assert_eq!(err(one(st, s.clone())), SubmitErr::UnknownKey);
        let dev = device(st, 3);
        let s = original(&dev, 1, 1);
        let mut bad_cose = s.clone();
        let n = bad_cose.cose.len();
        bad_cose.cose[n - 3] ^= 1;
        assert_eq!(err(one(st, bad_cose)), SubmitErr::BadSignature);
        let mut bad_claim = s.clone();
        bad_claim.claim[3] ^= 0x40;
        assert!(matches!(
            err(one(st, bad_claim)),
            SubmitErr::Malformed(_) | SubmitErr::BadSignature
        ));
        let mut garbage = s.clone();
        garbage.c2pa_claim = vec![0xff; 40];
        assert!(matches!(
            err(one(st, garbage)),
            SubmitErr::Malformed(_) | SubmitErr::BadSignature
        ));
        let swapped = Submission {
            cose: original(&dev, 2, 2).cose,
            ..s.clone()
        };
        assert_eq!(err(one(st, swapped)), SubmitErr::BadSignature);
        assert_eq!(
            err(one(
                st,
                Submission {
                    claim: vec![],
                    ..s.clone()
                }
            )),
            SubmitErr::TooLarge
        );
        assert_eq!(
            err(one(
                st,
                Submission {
                    cose: vec![0; 70_000],
                    ..s.clone()
                }
            )),
            SubmitErr::TooLarge
        );
        let big_fp = Submission {
            fingerprints: Some(ByteBuf::from(vec![0; 9000])),
            ..s
        };
        assert_eq!(err(one(st, big_fp)), SubmitErr::TooLarge);
        assert_eq!(st.mmr().leaf_count(), 0);
        assert!(st.outbox.is_empty());
    });
}

#[test]
fn rejects_seal_from_foreign_ca() {
    with(|st| {
        ready(st);
        device(st, 1);
        let ca = witness_verdict::testkit::TestCa::new(3);
        let fdev = Dev {
            device: ca.device(1, 2),
            anchor: ca.anchor.clone(),
        };
        assert_eq!(err(one(st, original(&fdev, 1, 1))), SubmitErr::BadSignature);
    });
}

#[test]
fn counters_must_be_unique() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        ok(one(st, original(&dev, 1, 7)));
        assert_eq!(err(one(st, original(&dev, 2, 7))), SubmitErr::CounterReused);
        ok(one(st, original(&dev, 2, 8)));
    });
}

#[test]
fn originals_after_window_are_expired_but_derivatives_are_not() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let o = original(&dev, 1, 1);
        ok(one(st, o.clone()));
        let key = st.key(&dev.device.key_id()).unwrap();
        let mut late = dev.device.claim(2, Kind::Original, 2);
        late.time.wall_ms = key.active_until_ns / 1_000_000 + 1;
        assert_eq!(
            err(one(st, sealed(&dev, late.clone(), 2))),
            SubmitErr::KeyExpired
        );
        late.time.wall_ms -= 1;
        ok(one(st, sealed(&dev, late, 2)));
        let mut d = dev.device.claim(
            3,
            Kind::Derivative {
                parent: o.record_id(),
                root: CaptureId::new(1, 0).unwrap(),
                edits: vec![],
            },
            3,
        );
        d.time.wall_ms = key.active_until_ns / 1_000_000 + 10_000;
        ok(one(st, sealed(&dev, d, 3)));
    });
}

#[test]
fn distrusted_keys_are_rejected() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let mut key: KeyInfo = st.key(&dev.device.key_id()).unwrap();
        key.history
            .push(StatusChange {
                status: witness_format::KeyStatus::Suspended,
                since_ns: NOW + 1,
            })
            .unwrap();
        st.put_key(&key);
        assert_eq!(err(one(st, original(&dev, 1, 1))), SubmitErr::KeyDistrusted);
        key.history
            .push(StatusChange {
                status: witness_format::KeyStatus::Revoked,
                since_ns: NOW + 2,
            })
            .unwrap();
        st.put_key(&key);
        assert_eq!(err(one(st, original(&dev, 1, 1))), SubmitErr::KeyDistrusted);
    });
}

fn bundle(tag: u8) -> FingerprintBundle {
    FingerprintBundle::new(
        vec![PdqShape {
            tag,
            hash: [tag; 32],
        }],
        50,
        [7; 192],
        false,
    )
    .unwrap()
}

#[test]
fn fingerprints_must_match_commit() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let b = bundle(1);
        let mut claim = dev.device.claim(1, Kind::Original, 1);
        claim.fp_commit = b.fp_commit();
        let mut s = sealed(&dev, claim, 1);
        s.fingerprints = Some(ByteBuf::from(cbor::to_vec(&bundle(2))));
        assert_eq!(err(one(st, s.clone())), SubmitErr::FingerprintMismatch);
        s.fingerprints = Some(ByteBuf::from(vec![0xa0, 0x00]));
        assert!(matches!(err(one(st, s.clone())), SubmitErr::Malformed(_)));
        let fp = cbor::to_vec(&b);
        s.fingerprints = Some(ByteBuf::from(fp.clone()));
        ok(one(st, s));
        let Body::Put(p) = &outbox(st)[0].body else {
            panic!("posting")
        };
        assert_eq!(p.fingerprints.as_deref().map(|b| b.to_vec()), Some(fp));
    });
}

#[test]
fn daily_quota_per_key() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let day = NOW / witness_attest::policy::DAY_NS;
        st.submit_quota.insert(
            dev.device.key_id().to_bytes(),
            Daily {
                day,
                count: SUBMITS_PER_DAY,
            }
            .to_bytes(),
        );
        let wait = match err(one(st, original(&dev, 1, 1))) {
            SubmitErr::RetryAfterNs(w) => w,
            e => panic!("{e:?}"),
        };
        assert!(wait > 0 && (NOW + wait).is_multiple_of(witness_attest::policy::DAY_NS));
        st.submit_quota.insert(
            dev.device.key_id().to_bytes(),
            Daily {
                day,
                count: SUBMITS_PER_DAY - 1,
            }
            .to_bytes(),
        );
        ok(one(st, original(&dev, 1, 1)));
        assert!(matches!(
            err(one(st, original(&dev, 2, 2))),
            SubmitErr::RetryAfterNs(_)
        ));
        let other = device(st, 2);
        ok(one(st, original(&other, 3, 1)));
    });
}

#[test]
fn batch_limits_and_readiness() {
    with(|st| {
        let none: Vec<Submission> = vec![];
        assert!(
            submit(st, none.clone(), NOW)
                .unwrap_err()
                .contains("not ready")
        );
        ready(st);
        let dev = device(st, 1);
        let s = original(&dev, 1, 1);
        assert!(submit(st, vec![s.clone(); MAX_SUBMIT_BATCH + 1], NOW).is_err());
        let (r, _) = submit(st, vec![s; MAX_SUBMIT_BATCH], NOW).unwrap();
        assert!(matches!(r[0], SubmitResult::Ok(_)));
        assert!(r[1..].iter().all(|x| matches!(x, SubmitResult::Dup(_))));
        let (r, roots) = submit(st, none, NOW).unwrap();
        assert!(r.is_empty());
        assert_eq!(roots.leaf_count, 1);
    });
}

#[test]
fn batch_processes_in_order_with_mixed_results() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        let o = original(&dev, 1, 1);
        let d = derivative(&dev, &o, 1, 2, 2);
        let (r, _) = submit(st, vec![d.clone(), o, d], NOW).unwrap();
        assert_eq!(err(r[0].clone()), SubmitErr::ParentUnknown);
        assert_eq!(ok(r[1].clone()).record_ix, 0);
        assert_eq!(ok(r[2].clone()).record_ix, 1);
    });
}

#[test]
fn by_capture_is_capped_at_earliest() {
    with(|st| {
        ready(st);
        let dev = device(st, 1);
        for i in 0..MAX_PER_CAPTURE as u64 {
            st.by_capture.insert(pair(9, 1000 + i), ());
        }
        ok(one(st, original(&dev, 9, 1)));
        assert!(!st.by_capture.contains_key(&pair(9, 0)));
        ok(one(st, original(&dev, 10, 2)));
        assert!(st.by_capture.contains_key(&pair(10, 1)));
    });
}

#[test]
fn shards_route_postings() {
    with(|st| {
        ready(st);
        let p = |n: u8| candid::Principal::from_slice(&[n; 10]);
        st.configure(witness_canister_common::RegistryInit {
            index: vec![p(1), p(2)],
            media: vec![p(3), p(4)],
            pages: p(5),
        });
        assert_eq!(st.shards().open_index().map(|s| s.shard), Some(1));
        let dev = device(st, 1);
        let o = original(&dev, 1, 1);
        assert_eq!(ok(one(st, o.clone())).media_shard, 1);
        let d = derivative(&dev, &o, 1, 2, 2);
        assert_eq!(ok(one(st, d.clone())).media_shard, 1);
        let dup = one(st, d);
        assert!(matches!(dup, SubmitResult::Dup(_)));
        assert_eq!(dup.anchored().map(|a| a.media_shard), Some(1));
        let e = st.record(1).unwrap();
        assert_eq!((e.index_shard, e.media_shard), (1, 1));
        let r = crate::read::record(st, 1, vec![]).unwrap();
        assert_eq!(r.media_shard, 1);
        let out = outbox(st);
        assert_eq!(out[0].dest, Dest::Index(1));
        assert_eq!(out[1].dest, Dest::Index(1));
        let Body::Shell(s) = &out[2].body else {
            panic!("shell")
        };
        assert_eq!((s.media_shard, s.media), (1, p(4)));
        assert!(s.dev);
        assert_eq!(out[3].dest, Dest::Media(0));
        assert_eq!(out[4].dest, Dest::Media(1));
        assert!(matches!(
            &out[4].body,
            Body::Sync {
                record_ix: None,
                allow: crate::types::Allow::Key { .. }
            }
        ));
        assert_eq!(out[5].dest, Dest::Media(1));
    });
}

#[test]
fn seal_error_mapping() {
    assert_eq!(seal_err(C2paError::Signature), SubmitErr::BadSignature);
    assert_eq!(seal_err(C2paError::KeyMismatch), SubmitErr::BadSignature);
    assert!(matches!(
        seal_err(C2paError::MalformedCose),
        SubmitErr::Malformed(_)
    ));
    assert!(matches!(
        seal_err(C2paError::TooLarge),
        SubmitErr::Malformed(_)
    ));
}
