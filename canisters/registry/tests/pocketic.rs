mod support;

use candid::{Decode, Encode};
use p256::ecdsa::signature::Signer as _;
use serde_bytes::ByteBuf;
use support::{Env, capture, quick, user};
use witness_canister_common::registry::erase_signing_bytes;
use witness_canister_common::{
    CertifiedKey, CertifiedRecord, EraseAuth, EraseRequest, Posting, Shards, Status, Submission,
    SubmitErr, SubmitResult,
};
use witness_format::{CaptureId, Kind, RecordId};
use witness_merkle::{ConsistencyProof, Hash, Mmr};
use witness_registry::proof::{key_certified, record_certified, shards_certified};
use witness_registry::record::leaf_data;
use witness_registry::types::{Allow, EnrollResult, EraseResult, Removal, Shell};

fn ok(r: &SubmitResult) -> &witness_canister_common::Anchored {
    match r {
        SubmitResult::Ok(a) => a,
        other => panic!("expected ok: {other:?}"),
    }
}

fn err(r: &SubmitResult) -> &SubmitErr {
    match r {
        SubmitResult::Err(e) => e,
        other => panic!("expected err: {other:?}"),
    }
}

fn derived(parent: &Submission, root: u64) -> Kind {
    Kind::Derivative {
        parent: parent.record_id(),
        root: CaptureId::new(root & CaptureId::RANDOM_MASK, 0).unwrap(),
        edits: vec![],
    }
}

fn erase_req(d: &support::Device, id: &RecordId) -> EraseRequest {
    let msg = erase_signing_bytes(id, b"n1").unwrap();
    let sig: p256::ecdsa::Signature = d.dev.key.sign(&msg);
    EraseRequest {
        record_id: id.to_string(),
        nonce: b"n1".to_vec(),
        auth: EraseAuth::Device(sig.to_bytes().to_vec()),
    }
}

fn status(env: &Env) -> Status {
    env.query("status", ())
}

#[test]
fn enroll_submit_get_with_certified_proofs() {
    let env = Env::new();
    let d = env.enroll(1);
    let o1 = capture(&d, 11, Kind::Original, 1);
    let o2 = capture(&d, 12, Kind::Original, 2);
    let r = env.submit(vec![o1.clone(), o2.clone()]);
    assert_eq!((ok(&r[0]).record_ix, ok(&r[1]).record_ix), (0, 1));
    let der = capture(&d, 13, derived(&o1, 11), 3);
    let r2 = env.submit(vec![der.clone(), o1.clone()]);
    assert_eq!(ok(&r2[0]).record_ix, 2);
    assert_eq!(r2[1], SubmitResult::Dup(ok(&r[0]).clone()));

    for (ix, s) in [&o1, &o2, &der].into_iter().enumerate() {
        let id = s.record_id().to_string();
        let rec: Option<CertifiedRecord> = env.query("get", (id.clone(),));
        let rec = rec.expect("record");
        assert_eq!((rec.record_ix, rec.leaf_count), (ix as u64, 3));
        assert_eq!(rec.claim, s.claim);
        let certified = record_certified(&rec).expect("proof");
        env.verify_certificate(&rec.certificate, &certified);
        let by_ix: Option<CertifiedRecord> = env.query("get_ix", (ix as u64,));
        assert_eq!(by_ix.map(|r| r.record_id), Some(id));
    }
    let missing: Option<CertifiedRecord> = env.query("get_ix", (3u64,));
    assert!(missing.is_none());

    let key: Option<CertifiedKey> = env.query("get_key", (ByteBuf::from(d.key_id.clone()),));
    let key = key.expect("key");
    assert_eq!(key.key.provisioning, witness_canister_common::Prov::Dev);
    env.verify_certificate(&key.certificate, &key_certified(&key).expect("key proof"));

    let raw = env.query_raw("shards", Encode!().unwrap()).unwrap();
    let (shards, bundle) = Decode!(&raw, Shards, ByteBuf).unwrap();
    assert_eq!(shards.index[0].canister, env.index);
    assert_eq!(shards.media[0].canister, env.media);
    let (cert, certified) = shards_certified(&shards, &bundle).expect("shards proof");
    env.verify_certificate(cert, &certified);

    let path: Vec<ByteBuf> = env.query("consistency", (1u64,));
    let path: Vec<Hash> = path
        .iter()
        .map(|h| h.as_slice().try_into().unwrap())
        .collect();
    let mut local = Mmr::new(Vec::new()).unwrap();
    for ix in 0..3u64 {
        let r: CertifiedRecord = env
            .query::<Option<CertifiedRecord>>("get_ix", (ix,))
            .unwrap();
        local
            .append(&leaf_data(&r.claim, &r.c2pa_claim, &r.cose, r.anchored_ns))
            .unwrap();
    }
    let proof = ConsistencyProof {
        old_size: 1,
        new_size: 3,
        path,
    };
    assert!(proof.verify(&local.root_at(1).unwrap(), &local.root().unwrap()));
    assert!(
        env.query_raw("consistency", Encode!(&4u64).unwrap())
            .is_err()
    );

    let ids: Vec<String> = env.query("by_capture", (11u64,));
    assert_eq!(ids, vec![o1.record_id().to_string()]);
    let none: Vec<String> = env.query("by_capture", (99u64,));
    assert!(none.is_empty());

    env.drain();
    let index_calls = env.calls(env.index);
    let postings: Vec<Posting> = index_calls
        .iter()
        .filter(|(m, _)| m == "put")
        .flat_map(|(_, a)| Decode!(a, Vec<Posting>).unwrap())
        .collect();
    assert_eq!(
        postings.iter().map(|p| p.record_ix).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(postings[2].root_ix, Some(0));
    assert_eq!(postings[2].wm_root, 11);
    assert!(postings.iter().all(|p| p.fingerprints.is_some()));
    let shells: Vec<Shell> = env
        .calls(env.pages)
        .iter()
        .filter(|(m, _)| m == "put_shell")
        .flat_map(|(_, a)| Decode!(a, Vec<Shell>).unwrap())
        .collect();
    assert_eq!(shells.len(), 1);
    assert_eq!(shells[0].record_id, der.record_id().to_string());
    assert!(shells[0].dev);
    assert_eq!(shells[0].media, env.media);
    let media = env.calls(env.media);
    assert!(media.iter().all(|(m, _)| m == "sync"), "{media:?}");
    let allows: Vec<Allow> = media
        .iter()
        .flat_map(|(_, a)| Decode!(a, Vec<Allow>).unwrap())
        .collect();
    assert!(allows.iter().any(|a| matches!(a, Allow::Key { .. })));
    assert!(
        allows.contains(&Allow::Record {
            record_id: der.record_id().to_string(),
            key_id: ByteBuf::from(
                witness_format::WitnessClaim::from_cbor(&shells[0].claim)
                    .unwrap()
                    .key
                    .key_id
                    .as_bytes()
                    .to_vec()
            ),
        })
    );
    assert_eq!(status(&env).metric("outbox"), Some(0));
    assert_eq!(status(&env).metric("records"), Some(3));
}

#[test]
fn concurrent_enrolls_are_reentrancy_safe() {
    let env = Env::new();
    let req = env.enrollment(1);
    let call = |r| {
        env.pic
            .submit_call(env.registry, user(1), "enroll", Encode!(&r).unwrap())
            .unwrap()
    };
    let a = call(req.clone());
    let b = call(req);
    let other = call(env.enrollment(2));
    let ra = Decode!(&env.pic.await_call(a).unwrap(), EnrollResult).unwrap();
    let rb = Decode!(&env.pic.await_call(b).unwrap(), EnrollResult).unwrap();
    let ro = Decode!(&env.pic.await_call(other).unwrap(), EnrollResult).unwrap();
    let oks = [&ra, &rb]
        .iter()
        .filter(|r| matches!(r, EnrollResult::Ok(_)))
        .count();
    assert_eq!(oks, 1, "{ra:?} {rb:?}");
    let errs: Vec<_> = [&ra, &rb]
        .iter()
        .filter_map(|r| match r {
            EnrollResult::Err(e) => Some(e.clone()),
            EnrollResult::Ok(_) => None,
        })
        .collect();
    assert_eq!(errs, vec!["enrollment pending".to_owned()]);
    assert!(matches!(ro, EnrollResult::Ok(_)));
    let s = status(&env);
    assert_eq!(
        (s.metric("keys"), s.metric("pending_keys")),
        (Some(2), Some(0))
    );
    let again: EnrollResult = env.update("enroll", (env.enrollment(1),));
    assert_eq!(again, EnrollResult::Err("already enrolled".into()));
}

#[test]
fn enroll_quota_backs_off_dev_chains() {
    let env = Env::new();
    for seed in 0..5 {
        env.enroll(100 + seed);
    }
    let r: EnrollResult = env.update("enroll", (env.enrollment(200),));
    let EnrollResult::Err(e) = r else {
        panic!("expected quota error")
    };
    assert!(e.starts_with("quota: retry after"), "{e}");
    env.pic.advance_time(std::time::Duration::from_secs(3601));
    env.pic.tick();
    env.enroll(200);
    assert_eq!(status(&env).metric("keys"), Some(6));
}

#[test]
fn erase_mine_tombstones_and_fans_out() {
    let env = Env::new();
    let d = env.enroll(1);
    let o = quick(&d, d.dev.claim(1, Kind::Original, 1), 1);
    let der = quick(&d, d.dev.claim(2, derived(&o, 1), 2), 2);
    let r = env.submit(vec![o.clone(), der.clone()]);
    ok(&r[1]);
    let mut bad = erase_req(&d, &der.record_id());
    bad.nonce = b"n2".to_vec();
    let res: EraseResult = env.update("erase_mine", (bad,));
    assert_eq!(res, EraseResult::Err("bad signature".into()));
    let res: EraseResult = env.update("erase_mine", (erase_req(&d, &der.record_id()),));
    assert_eq!(res, EraseResult::Ok);
    let res: EraseResult = env.update("erase_mine", (erase_req(&d, &der.record_id()),));
    assert_eq!(res, EraseResult::Ok);
    env.drain();
    let index = env.calls(env.index);
    let put: Vec<Posting> = index
        .iter()
        .filter(|(m, _)| m == "put")
        .flat_map(|(_, a)| Decode!(a, Vec<Posting>).unwrap())
        .collect();
    assert_eq!(put.iter().map(|p| p.record_ix).collect::<Vec<_>>(), vec![0]);
    let erased: Vec<u64> = index
        .iter()
        .filter(|(m, _)| m == "erase")
        .map(|(_, a)| Decode!(a, u64).unwrap())
        .collect();
    assert_eq!(erased, vec![1]);
    let text = der.record_id().to_string();
    let pages = env.calls(env.pages);
    assert_eq!(pages.len(), 1, "{pages:?}");
    assert_eq!(pages[0].0, "tombstone");
    assert_eq!(
        Decode!(&pages[0].1, Vec<String>, Removal).unwrap(),
        (vec![text.clone()], Removal::Erased)
    );
    let media = env.calls(env.media);
    let erases: Vec<_> = media.iter().filter(|(m, _)| m != "sync").collect();
    assert_eq!(erases.len(), 1, "{media:?}");
    assert_eq!(erases[0].0, "erase");
    assert_eq!(Decode!(&erases[0].1, String).unwrap(), text);
    let rec: Option<CertifiedRecord> = env.query("get", (text,));
    assert!(rec.is_some());
    assert_eq!(status(&env).metric("tombstones"), Some(1));
}

#[test]
fn upgrade_preserves_state() {
    let env = Env::new();
    let challenge: ByteBuf = env.query("challenge", ());
    let d = env.enroll(1);
    let o = quick(&d, d.dev.claim(1, Kind::Original, 1), 1);
    ok(&env.submit(vec![o.clone()])[0]);
    let before: CertifiedRecord = env
        .query::<Option<CertifiedRecord>>("get_ix", (0u64,))
        .unwrap();
    let ca_before: Option<ByteBuf> = env.query("ca_root", ());
    env.upgrade();
    let after: CertifiedRecord = env
        .query::<Option<CertifiedRecord>>("get_ix", (0u64,))
        .unwrap();
    assert_eq!(record_certified(&after), record_certified(&before));
    env.verify_certificate(&after.certificate, &record_certified(&after).unwrap());
    assert_eq!(env.query::<Option<ByteBuf>>("ca_root", ()), ca_before);
    let now_s = (env.now() / 1_000_000_000) as i64;
    let a = support::attest::Attestation::new(&challenge, now_s);
    let req = witness_registry::types::Enrollment {
        chain: a
            .chain(&support::attest::device_key(2))
            .into_iter()
            .map(ByteBuf::from)
            .collect(),
        key_id: None,
        late: None,
    };
    let d2 = env.device_from(2, env.update("enroll", (req,)));
    ok(&env.submit(vec![quick(&d2, d2.dev.claim(5, Kind::Original, 1), 5)])[0]);
    assert_eq!(
        env.submit(vec![o])[0],
        SubmitResult::Dup(witness_canister_common::Anchored {
            record_ix: 0,
            record_id: before.record_id.clone(),
            anchored_ns: before.anchored_ns,
            media_shard: before.media_shard,
        })
    );
    let s = status(&env);
    assert_eq!((s.metric("records"), s.metric("keys")), (Some(2), Some(2)));
    assert!(s.timers.iter().any(|t| t.name == "drain"));
    assert!(s.has_flag("dev_attestation"));
}

#[test]
fn outbox_retries_until_peer_recovers() {
    let env = Env::new();
    let d = env.enroll(1);
    env.set_down(env.index, true);
    ok(&env.submit(vec![quick(&d, d.dev.claim(1, Kind::Original, 1), 1)])[0]);
    env.drain();
    env.drain();
    assert_eq!(status(&env).metric("outbox"), Some(1));
    env.set_down(env.index, false);
    env.drain();
    assert_eq!(status(&env).metric("outbox"), Some(0));
    assert_eq!(env.calls(env.index).len(), 1);
}

#[test]
fn rejects_malformed_and_abusive_input() {
    let env = Env::new();
    let d = env.enroll(1);
    let good = quick(&d, d.dev.claim(1, Kind::Original, 1), 1);
    let mut tampered = good.clone();
    let n = tampered.cose.len();
    tampered.cose[n - 2] ^= 1;
    let garbage = Submission {
        claim: vec![0xa0],
        c2pa_claim: vec![1],
        cose: vec![2],
        fingerprints: None,
    };
    let mut expired = d.dev.claim(2, Kind::Original, 2);
    expired.time.wall_ms = env.now() / 1_000_000 + 61 * 86_400_000;
    let reused = quick(&d, d.dev.claim(3, Kind::Original, 1), 3);
    let orphan = quick(
        &d,
        d.dev.claim(
            4,
            derived(&quick(&d, d.dev.claim(9, Kind::Original, 9), 9), 9),
            4,
        ),
        4,
    );
    let r = env.submit(vec![
        tampered,
        garbage,
        quick(&d, expired, 2),
        good.clone(),
        reused,
        orphan,
        Submission {
            cose: vec![],
            ..good.clone()
        },
    ]);
    assert_eq!(err(&r[0]), &SubmitErr::BadSignature);
    assert!(matches!(err(&r[1]), SubmitErr::Malformed(_)));
    assert_eq!(err(&r[2]), &SubmitErr::KeyExpired);
    ok(&r[3]);
    assert_eq!(err(&r[4]), &SubmitErr::CounterReused);
    assert_eq!(err(&r[5]), &SubmitErr::ParentUnknown);
    assert_eq!(err(&r[6]), &SubmitErr::TooLarge);

    let stranger = witness_verdict::testkit::TestCa::new(7);
    let foreign = support::Device {
        dev: stranger.device(3, 2),
        anchor: stranger.anchor.clone(),
        key_id: vec![],
    };
    let r = env.submit(vec![quick(
        &foreign,
        foreign.dev.claim(1, Kind::Original, 1),
        1,
    )]);
    assert_eq!(err(&r[0]), &SubmitErr::BadSignature);

    assert!(
        env.update_raw("submit", Encode!(&vec![good.clone(); 65]).unwrap())
            .is_err()
    );
    assert!(env.update_raw("submit", b"not candid".to_vec()).is_err());
    let big = vec![0u8; 2_000];
    let oversized = EraseRequest {
        record_id: "x".into(),
        nonce: big,
        auth: EraseAuth::Ii,
    };
    assert!(
        env.update_raw("erase_mine", Encode!(&oversized).unwrap())
            .is_err()
    );
    assert!(env.update_raw("status", Encode!().unwrap()).is_err());

    let res: EraseResult = env.update(
        "erase_mine",
        (EraseRequest {
            record_id: good.record_id().to_string(),
            nonce: vec![1],
            auth: EraseAuth::Ii,
        },),
    );
    assert!(matches!(res, EraseResult::Err(_)));
    let res: EraseResult = env.update(
        "erase_mine",
        (EraseRequest {
            record_id: "garbage".into(),
            nonce: vec![1],
            auth: EraseAuth::Device(vec![]),
        },),
    );
    assert!(matches!(res, EraseResult::Err(_)));

    let bad_chain = witness_registry::types::Enrollment {
        chain: vec![ByteBuf::from(vec![0x30, 0x03, 0x02, 0x01, 0x00]); 3],
        key_id: None,
        late: None,
    };
    let r: EnrollResult = env.update("enroll", (bad_chain,));
    assert!(matches!(r, EnrollResult::Err(e) if e.starts_with("attestation")));
    let mut wrong_key = env.enrollment(5);
    wrong_key.key_id = Some(ByteBuf::from(vec![0; 16]));
    let r: EnrollResult = env.update("enroll", (wrong_key,));
    assert!(matches!(r, EnrollResult::Err(_)));
    let huge = witness_registry::types::Enrollment {
        chain: vec![ByteBuf::from(vec![0; 40_000])],
        key_id: None,
        late: None,
    };
    assert!(env.update_raw("enroll", Encode!(&huge).unwrap()).is_err());
    assert_eq!(status(&env).metric("records"), Some(1));
}
