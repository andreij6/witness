use super::*;
use crate::client::tests::{media, registry, reply};
use crate::oracle::tests::{World, cert_now, probe};
use crate::seal::tests::{DeviceSigner, camera};
use crate::transport::mock::Mock;
use candid::decode_one;
use futures::executor::block_on;
use witness_canister_common::{CaseInfo, EraseAuth, MediaErr};
use witness_core::format::{Outcome, RecordId};
use witness_core::verdict::Reason;

fn witness(mock: Mock, ca_root: Option<&[u8]>) -> (Witness, Arc<Mock>) {
    witness_in(mock, ca_root, true)
}

fn witness_in(mock: Mock, ca_root: Option<&[u8]>, local: bool) -> (Witness, Arc<Mock>) {
    let mock = Arc::new(mock);
    let w = Witness::with_transport(mock.clone(), registry(), ca_root, local)
        .unwrap()
        .with_clock(cert_now);
    (w, mock)
}

fn count(m: &Mock, method: &str) -> usize {
    m.methods().iter().filter(|x| *x == method).count()
}

fn config() -> WitnessConfig {
    WitnessConfig {
        replica_url: "http://10.0.2.2:4943".into(),
        registry: registry(),
        local: true,
        ca_root: None,
        identity_seed: None,
        max_cert_age_s: None,
    }
}

#[test]
fn certificate_skew_is_configurable_and_never_disabled() {
    assert_eq!(max_cert_age_ns(None), DEFAULT_MAX_AGE_NS);
    assert_eq!(max_cert_age_ns(Some(60)), 60 * NS_PER_S);
    assert_eq!(max_cert_age_ns(Some(0)), 1);
    assert_eq!(max_cert_age_ns(Some(u64::MAX)), u64::MAX);
    let w = Witness::new(WitnessConfig {
        max_cert_age_s: Some(30),
        ..config()
    })
    .unwrap();
    assert_eq!(w.max_cert_age_ns, 30 * NS_PER_S);
    assert_eq!(
        Witness::new(config()).unwrap().max_cert_age_ns,
        DEFAULT_MAX_AGE_NS
    );
}

#[test]
fn new_builds_without_touching_the_network() {
    assert!(Witness::new(config()).is_ok());
    let seeded = WitnessConfig {
        identity_seed: Some(vec![3; 32]),
        ..config()
    };
    assert!(Witness::new(seeded).is_ok());
}

#[test]
fn new_rejects_bad_config() {
    let bad_seed = WitnessConfig {
        identity_seed: Some(vec![3; 31]),
        ..config()
    };
    let bad_url = WitnessConfig {
        replica_url: "10.0.2.2:4943".into(),
        ..config()
    };
    let bad_root = WitnessConfig {
        ca_root: Some(vec![0x30, 0x00]),
        ..config()
    };
    let cleartext_production = WitnessConfig {
        local: false,
        ca_root: None,
        ..config()
    };
    for c in [bad_seed, bad_url, bad_root, cleartext_production] {
        assert!(matches!(
            Witness::new(c),
            Err(WitnessError::InvalidInput { .. })
        ));
    }
}

#[test]
fn check_finds_the_sealed_original() {
    let world = World::new(1);
    let (w, m) = witness(world.mock(probe(&[], &[], &[])), None);
    let v = block_on(w.check(world.sealed.jpeg.clone(), None)).unwrap();
    assert_eq!(v.outcome, Outcome::Original);
    assert!(v.reasons.contains(&Reason::SealVerified));
    let reference = v.reference.unwrap();
    assert_eq!(reference.record_id, world.sealed.record_id);
    assert_eq!(reference.claim, world.sealed.claim);
    assert_eq!(reference.anchor.unwrap().record_ix, 0);
    assert_eq!(count(&m, "ca_root"), 1);
    block_on(w.check(world.sealed.jpeg.clone(), None)).unwrap();
    assert_eq!(count(&m, "ca_root"), 1);
    assert_eq!(count(&m, "shards"), 1);
}

#[test]
fn check_uses_a_pinned_root_without_asking_the_registry() {
    let world = World::new(2);
    let (w, m) = witness(world.mock(probe(&[], &[], &[])), Some(&world.ca.root_der));
    assert_eq!(
        block_on(w.check(world.sealed.jpeg.clone(), None))
            .unwrap()
            .outcome,
        Outcome::Original
    );
    assert_eq!(count(&m, "ca_root"), 0);
}

#[test]
fn local_check_without_pinned_root_queries_the_registry_and_flags_dev() {
    let world = World::new(20);
    let (w, m) = witness(world.mock(probe(&[], &[], &[])), None);
    let v = block_on(w.check(world.sealed.jpeg.clone(), None)).unwrap();
    assert_eq!(v.outcome, Outcome::Original);
    assert!(v.dev_anchor && v.credentials_judged);
    assert_eq!(count(&m, "ca_root"), 1);
}

#[test]
fn pinned_root_is_not_dev() {
    let world = World::new(21);
    for local in [true, false] {
        let (w, m) = witness_in(
            world.mock(probe(&[], &[], &[])),
            Some(&world.ca.root_der),
            local,
        );
        let v = block_on(w.check(world.sealed.jpeg.clone(), None)).unwrap();
        assert_eq!(v.outcome, Outcome::Original);
        assert!(!v.dev_anchor && v.credentials_judged);
        assert_eq!(count(&m, "ca_root"), 0);
    }
}

#[test]
fn production_without_pinned_root_never_trusts_the_registry_query() {
    let world = World::new(22);
    for root in [None, Some(&[][..])] {
        let (w, m) = witness_in(world.mock(probe(&[], &[], &[])), root, false);
        let v = block_on(w.check(world.sealed.jpeg.clone(), None)).unwrap();
        assert_eq!(count(&m, "ca_root"), 0);
        assert!(!v.credentials_judged && !v.dev_anchor);
        assert!(v.reasons.contains(&Reason::UnverifiedCredentials));
        assert!(!v.reasons.contains(&Reason::SealVerified));
        assert_ne!(v.outcome, Outcome::Original);
        let again = block_on(w.check(camera(22), None)).unwrap();
        assert_eq!(again.outcome, Outcome::NoRecord);
        assert!(!again.credentials_judged);
    }
}

#[test]
fn empty_pinned_root_counts_as_missing_in_local_mode() {
    let world = World::new(23);
    let (w, m) = witness_in(world.mock(probe(&[], &[], &[])), Some(&[]), true);
    let v = block_on(w.check(world.sealed.jpeg.clone(), None)).unwrap();
    assert!(v.dev_anchor);
    assert_eq!(count(&m, "ca_root"), 1);
}

#[test]
fn unpinned_anchor_parses_and_differs_from_any_ca() {
    let a = parse_anchor(UNPINNED_ANCHOR_DER).unwrap();
    assert_ne!(a.key_id(), World::new(24).ca.anchor.key_id());
}

#[test]
fn case_limits_mirror_the_media_contract() {
    assert_eq!(
        case_limits(),
        CaseLimits {
            max_chunks: 16,
            max_bytes: 16 << 20,
            plus_max_chunks: 256,
            plus_max_bytes: 256 << 20,
            chunk_bytes: 1 << 20,
        }
    );
}

#[test]
fn case_info_queries_the_case_shard() {
    let info = CaseInfo {
        expires_ns: 9,
        bytes: 3,
        chunks: 2,
    };
    let (w, m) = witness(
        World::new(25)
            .mock(probe(&[], &[], &[]))
            .on(media(0), "case_info", move |a| {
                let id: String = decode_one(a).unwrap();
                reply(if id == "C" { Some(info) } else { None })
            }),
        None,
    );
    assert_eq!(block_on(w.case_info(0, "C".into())).unwrap(), Some(info));
    assert_eq!(block_on(w.case_info(0, "D".into())).unwrap(), None);
    assert!(block_on(w.case_info(7, "C".into())).is_err());
    let w = Arc::new(w);
    assert_eq!(block_on(case_info(w, 0, "C".into())).unwrap(), Some(info));
    assert_eq!(count(&m, "case_info"), 3);
}

#[test]
fn check_flags_registry_answers_that_fail_their_proofs() {
    let world = World::new(30);
    let mut forged = world.record();
    forged.anchored_ns -= 1;
    let (w, m) = witness(
        world.mock_serving(probe(&[(0, 0)], &[0], &[0]), forged),
        Some(&world.ca.root_der),
    );
    let v = block_on(w.check(world.sealed.jpeg.clone(), None)).unwrap();
    assert!(v.proofs_rejected);
    assert_ne!(v.outcome, Outcome::Original);
    let honest = World::new(30);
    let (w, _) = witness(
        honest.mock(probe(&[(0, 0)], &[0], &[0])),
        Some(&honest.ca.root_der),
    );
    let v = block_on(w.check(honest.sealed.jpeg.clone(), None)).unwrap();
    assert!(!v.proofs_rejected);
    assert_eq!(v.outcome, Outcome::Original);
    assert_eq!(count(&m, "get_key"), 1);
}

#[test]
fn check_needs_the_root_key_and_fetches_it_once() {
    let world = World::new(31);
    let mock = world.mock(probe(&[], &[], &[]));
    let (w, _) = witness(mock, Some(&world.ca.root_der));
    block_on(w.check(camera(31), None)).unwrap();
    block_on(w.check(camera(32), None)).unwrap();
    assert_eq!(
        lock(&w.root_key).as_deref(),
        Some(&crate::oracle::tests::root_key())
    );
    let no_key = Mock::default().on(registry(), "shards", |_| {
        Ok(candid::encode_args((World::shards(), serde_bytes::ByteBuf::new())).unwrap())
    });
    let (w, _) = witness(no_key, Some(&world.ca.root_der));
    assert!(matches!(
        block_on(w.check(camera(31), None)),
        Err(WitnessError::Network { ref reason }) if reason.contains("root key")
    ));
}

#[test]
fn check_of_unknown_photo_is_no_record() {
    let world = World::new(3);
    let (w, _) = witness(world.mock(probe(&[], &[], &[])), None);
    let v = block_on(w.check(camera(99), None)).unwrap();
    assert_eq!(v.outcome, Outcome::NoRecord);
    assert!(v.reference.is_none());
}

#[test]
fn check_is_deterministic() {
    let world = World::new(4);
    let (w, _) = witness(world.mock(probe(&[], &[], &[])), None);
    let a = block_on(w.check(world.sealed.jpeg.clone(), None)).unwrap();
    let b = block_on(w.check(world.sealed.jpeg.clone(), None)).unwrap();
    assert_eq!(a, b);
}

#[test]
fn check_reports_network_failure_instead_of_no_record() {
    let world = World::new(5);
    let shards_only = Mock::default().on(registry(), "shards", |_| {
        Ok(candid::encode_args((World::shards(), serde_bytes::ByteBuf::new())).unwrap())
    });
    let (w, _) = witness(shards_only, Some(&world.ca.root_der));
    assert!(matches!(
        block_on(w.check(camera(5), None)),
        Err(WitnessError::Network { .. })
    ));
}

#[test]
fn check_without_registry_root_is_rejected() {
    let (w, _) = witness(
        Mock::default().on(registry(), "ca_root", |_| {
            reply(None::<serde_bytes::ByteBuf>)
        }),
        None,
    );
    assert!(matches!(
        block_on(w.check(vec![1], None)),
        Err(WitnessError::Rejected { .. })
    ));
}

#[test]
fn check_of_garbage_bytes_is_no_record_not_a_panic() {
    let world = World::new(6);
    let (w, _) = witness(world.mock(probe(&[], &[], &[])), None);
    for bytes in [vec![], vec![0xff, 0xd8, 0xff], vec![7; 1000]] {
        assert_eq!(
            block_on(w.check(bytes, None)).unwrap().outcome,
            Outcome::NoRecord
        );
    }
}

#[test]
fn shards_are_cached_until_refreshed() {
    let world = World::new(7);
    let (w, m) = witness(world.mock(probe(&[], &[1], &[])), None);
    assert_eq!(
        block_on(w.probe(vec![], vec![1], vec![]))
            .unwrap()
            .watermark,
        vec![1]
    );
    block_on(w.probe(vec![], vec![1], vec![])).unwrap();
    assert_eq!(count(&m, "shards"), 1);
    assert_eq!(block_on(w.shards()).unwrap(), World::shards());
    assert_eq!(count(&m, "shards"), 2);
}

#[test]
fn registry_passthroughs_decode() {
    let world = World::new(8);
    let (w, _) = witness(world.mock(probe(&[], &[], &[])), None);
    let id = world.sealed.record_id.to_string();
    assert_eq!(block_on(w.get(id)).unwrap(), Some(world.record()));
    assert_eq!(
        block_on(w.get_key(world.dev.key_id().as_bytes().to_vec())).unwrap(),
        Some(world.key())
    );
    assert_eq!(
        block_on(w.ca_root()).unwrap(),
        Some(world.ca.root_der.clone())
    );
}

fn late() -> u64 {
    cert_now() + DEFAULT_MAX_AGE_NS + 1
}

fn refused<T: std::fmt::Debug>(r: Result<T, WitnessError>) -> bool {
    matches!(r, Err(WitnessError::Rejected { .. }))
}

#[test]
fn get_and_get_key_refuse_answers_that_fail_their_proofs() {
    let world = World::new(21);
    let id = world.sealed.record_id.to_string();
    let key_id = world.dev.key_id().as_bytes().to_vec();
    let other_id = RecordId::from_claim_cbor(b"x").to_string();
    let honest = || world.mock(probe(&[], &[], &[]));
    let (w, _) = witness(honest(), None);
    assert_eq!(block_on(w.get(other_id.clone())).unwrap(), None);
    assert_eq!(block_on(w.get_key(vec![9; 16])).unwrap(), None);
    for tamper in [
        |r: &mut CertifiedRecord| r.anchored_ns += 1,
        |r: &mut CertifiedRecord| r.claim[3] ^= 1,
        |r: &mut CertifiedRecord| r.record_ix += 1,
        |r: &mut CertifiedRecord| r.mmr_proof[0][0] ^= 1,
        |r: &mut CertifiedRecord| r.certificate.truncate(16),
    ] {
        let mut r = world.record();
        tamper(&mut r);
        let (w, _) = witness(world.mock_serving(probe(&[], &[], &[]), r), None);
        assert!(refused(block_on(w.get(id.clone()))));
    }
    let r = world.record();
    let (w, _) = witness(
        honest().on(registry(), "get", move |_| reply(Some(r.clone()))),
        None,
    );
    assert!(refused(block_on(w.get(other_id))));
    let mut k = world.key();
    k.key.patch += 1;
    let (w, _) = witness(
        honest().on(registry(), "get_key", move |_| reply(Some(k.clone()))),
        None,
    );
    assert!(refused(block_on(w.get_key(key_id.clone()))));
    let k = world.key();
    let (w, _) = witness(
        honest().on(registry(), "get_key", move |_| reply(Some(k.clone()))),
        None,
    );
    assert!(refused(block_on(w.get_key(vec![9; 16]))));
    let stale = Witness::with_transport(Arc::new(honest()), registry(), None, true)
        .unwrap()
        .with_clock(late);
    assert!(refused(block_on(stale.get(id))));
    assert!(refused(block_on(stale.get_key(key_id))));
    assert!(matches!(
        block_on(w.get("not an id".into())),
        Err(WitnessError::InvalidInput { .. })
    ));
}

fn upload_mock() -> Mock {
    World::new(9)
        .mock(probe(&[], &[], &[]))
        .on(media(0), "begin_upload", |a| {
            let req: BeginUpload = decode_one(a).unwrap();
            let world = World::new(9);
            req.verify(&world.dev.spki).unwrap();
            reply(Ok::<Begun, MediaErr>(Begun {
                missing: vec![0],
                upload: 4,
            }))
        })
        .on(media(0), "put_chunk", |_| reply(Ok::<(), MediaErr>(())))
        .on(media(0), "commit", |_| {
            reply(Ok::<String, MediaErr>("/p/X".into()))
        })
}

#[test]
fn begin_upload_signs_with_the_device_key() {
    let world = World::new(9);
    let (w, m) = witness(upload_mock(), None);
    let begun = block_on(w.begin_upload(
        0,
        UploadTarget::Preview(world.sealed.record_id.to_string()),
        3,
        vec![0xab; 32],
        world.dev.key_id().as_bytes().to_vec(),
        DeviceSigner::new(world.dev.clone()),
        None,
    ))
    .unwrap();
    assert_eq!(
        begun,
        Begun {
            missing: vec![0],
            upload: 4
        }
    );
    block_on(w.put_chunk(0, 4, 0, vec![1, 2, 3])).unwrap();
    assert_eq!(block_on(w.commit(0, 4)).unwrap(), "/p/X");
    assert_eq!(count(&m, "shards"), 1);
}

#[test]
fn begin_upload_rejects_bad_requests_before_signing() {
    let world = World::new(9);
    let (w, m) = witness(upload_mock(), None);
    let signer = DeviceSigner::new(world.dev.clone());
    let key = world.dev.key_id().as_bytes().to_vec();
    let target = UploadTarget::Preview(world.sealed.record_id.to_string());
    assert!(matches!(
        block_on(w.begin_upload(
            0,
            target.clone(),
            3,
            vec![0; 31],
            key.clone(),
            signer.clone(),
            None
        )),
        Err(WitnessError::InvalidInput { .. })
    ));
    assert!(
        block_on(w.begin_upload(
            0,
            target.clone(),
            0,
            vec![0; 32],
            key.clone(),
            signer.clone(),
            None
        ))
        .is_err()
    );
    assert!(
        block_on(w.begin_upload(
            0,
            UploadTarget::Og("bad".into()),
            3,
            vec![0; 32],
            key.clone(),
            signer.clone(),
            None
        ))
        .is_err()
    );
    assert!(
        block_on(w.begin_upload(
            0,
            target.clone(),
            3,
            vec![0; 32],
            vec![1; 3],
            signer.clone(),
            None
        ))
        .is_err()
    );
    assert_eq!(signer.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(matches!(
        block_on(w.begin_upload(7, target, 3, vec![0; 32], key, signer, None)),
        Err(WitnessError::InvalidInput { .. })
    ));
    assert_eq!(count(&m, "begin_upload"), 0);
}

#[test]
fn erase_mine_sends_a_verifiable_device_signature() {
    let world = World::new(10);
    let spki = world.dev.spki.clone();
    let (w, _) = witness(
        Mock::default().on(registry(), "erase_mine", move |a| {
            let req: EraseRequest = decode_one(a).unwrap();
            assert!(matches!(req.auth, EraseAuth::Device(_)));
            req.verify_device(&spki).unwrap();
            reply(crate::client::EraseResult::Ok)
        }),
        None,
    );
    block_on(w.erase_mine(
        world.sealed.record_id.to_string(),
        vec![1; 16],
        DeviceSigner::new(world.dev.clone()),
    ))
    .unwrap();
}

#[test]
fn erase_mine_rejects_bad_ids_and_nonces_locally() {
    let world = World::new(11);
    let (w, m) = witness(Mock::default(), None);
    let signer = DeviceSigner::new(world.dev.clone());
    let id = RecordId::from_claim_cbor(b"x").to_string();
    for (record, nonce) in [
        ("bad".to_string(), vec![1]),
        (id.clone(), vec![]),
        (id, vec![1; 65]),
    ] {
        assert!(matches!(
            block_on(w.erase_mine(record, nonce, signer.clone())),
            Err(WitnessError::InvalidInput { .. })
        ));
    }
    assert!(m.calls().is_empty());
}

#[test]
fn enroll_and_submit_pass_through() {
    let world = World::new(12);
    let enrolled = Enrolled {
        key_id: world.dev.key_id().as_bytes().to_vec(),
        c2pa_cert: world.dev.chain[0].clone(),
        ca_cert: world.ca.root_der.clone(),
    };
    let e = enrolled.clone();
    let anchored = witness_canister_common::Anchored {
        record_ix: 0,
        record_id: world.sealed.record_id.to_string(),
        anchored_ns: 1,
        media_shard: 0,
    };
    let a = anchored.clone();
    let (w, _) = witness(
        Mock::default()
            .on(registry(), "enroll", move |_| {
                reply(crate::client::EnrollResult::Ok(e.clone()))
            })
            .on(registry(), "submit", move |_| {
                reply(vec![SubmitResult::Dup(a.clone())])
            })
            .on(registry(), "challenge", |_| {
                reply(serde_bytes::ByteBuf::from(vec![1; 8]))
            })
            .on(registry(), "by_capture", |_| reply(Vec::<String>::new())),
        None,
    );
    let enrollment = Enrollment {
        key_id: None,
        chain: vec![serde_bytes::ByteBuf::from(world.dev.chain[0].clone())],
        late: None,
    };
    assert_eq!(block_on(w.enroll(enrollment)).unwrap(), enrolled);
    assert_eq!(
        block_on(w.submit(vec![world.sealed.submission.clone()])).unwrap(),
        vec![SubmitResult::Dup(anchored)]
    );
    assert_eq!(block_on(w.challenge()).unwrap(), vec![1; 8]);
    assert!(block_on(w.by_capture(5)).unwrap().is_empty());
}

#[test]
fn network_calls_run_off_the_callers_thread() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = seen.clone();
    let (w, _) = witness(
        Mock::default().on(registry(), "challenge", move |_| {
            log.lock()
                .unwrap()
                .push(std::thread::current().name().map(str::to_owned));
            reply(serde_bytes::ByteBuf::from(vec![1; 8]))
        }),
        None,
    );
    assert_eq!(block_on(w.challenge()).unwrap(), vec![1; 8]);
    assert_eq!(*seen.lock().unwrap(), vec![Some("witness-io".to_owned())]);
}

#[test]
fn check_can_be_awaited_inside_another_tokio_runtime() {
    let world = World::new(13);
    let (w, _) = witness(world.mock(probe(&[], &[], &[])), None);
    let outer = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let v = outer
        .block_on(w.check(world.sealed.jpeg.clone(), None))
        .unwrap();
    assert_eq!(v.outcome, Outcome::Original);
    assert_eq!(
        outer.block_on(w.ca_root()).unwrap(),
        Some(world.ca.root_der.clone())
    );
}
