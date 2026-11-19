use super::*;
use crate::transport::mock::Mock;
use candid::{decode_one, encode_one};
use std::future::Future;
use witness_canister_common::Anchored;

pub(crate) fn registry() -> Principal {
    Principal::from_slice(&[1])
}

pub(crate) fn index(n: u8) -> Principal {
    Principal::from_slice(&[0x10, n])
}

pub(crate) fn media(n: u8) -> Principal {
    Principal::from_slice(&[0x20, n])
}

pub(crate) fn shard(n: u8, canister: Principal) -> ShardInfo {
    ShardInfo {
        shard: n,
        canister,
        open: n == 0,
    }
}

pub(crate) fn reply<T: CandidType>(v: T) -> Result<Vec<u8>, WitnessError> {
    Ok(encode_one(v).unwrap())
}

pub(crate) fn run<F: Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

fn client(mock: Mock) -> (IcpClient, Arc<Mock>) {
    let mock = Arc::new(mock);
    (IcpClient::new(mock.clone(), registry()), mock)
}

fn record(ix: u64) -> CertifiedRecord {
    CertifiedRecord {
        record_ix: ix,
        record_id: format!("R{ix}"),
        claim: vec![1],
        c2pa_claim: vec![2],
        cose: vec![3],
        anchored_ns: 10 * ix,
        mmr_proof: vec![ByteBuf::from(vec![9; 32])],
        leaf_count: ix + 1,
        certificate: vec![4],
        media_shard: 0,
    }
}

fn probe_result(
    pdq: &[(u64, u32)],
    wm: &[u64],
    exact: &[u64],
    truncated: bool,
    upto: u64,
) -> ProbeResult {
    ProbeResult {
        pdq: pdq
            .iter()
            .map(|&(record_ix, distance)| PdqHit {
                record_ix,
                distance,
            })
            .collect(),
        watermark: wm.to_vec(),
        exact: exact.to_vec(),
        truncated_candidates: truncated,
        indexed_upto: upto,
    }
}

#[test]
fn challenge_is_a_registry_query_without_args() {
    let (c, m) = client(Mock::default().on(registry(), "challenge", |a| {
        assert_eq!(a, encode_args(()).unwrap());
        reply(ByteBuf::from(vec![5; 40]))
    }));
    assert_eq!(run(c.challenge()).unwrap(), vec![5; 40]);
    let calls = m.calls();
    assert_eq!(calls.len(), 1);
    assert!(!calls[0].update);
    assert_eq!(calls[0].canister, registry());
}

#[test]
fn ca_root_decodes_both_options() {
    let (c, _) = client(Mock::default().on(registry(), "ca_root", |_| {
        reply(Some(ByteBuf::from(vec![0x30])))
    }));
    assert_eq!(run(c.ca_root()).unwrap(), Some(vec![0x30]));
    let (c, _) = client(Mock::default().on(registry(), "ca_root", |_| reply(None::<ByteBuf>)));
    assert_eq!(run(c.ca_root()).unwrap(), None);
}

fn enrollment() -> Enrollment {
    Enrollment {
        key_id: Some(ByteBuf::from(vec![1; 16])),
        chain: vec![ByteBuf::from(vec![0x30, 0x01])],
        late: Some(LateOpening {
            install_nonce: vec![2; 16],
            wall_ms: 99,
        }),
    }
}

#[test]
fn enroll_returns_enrolled_and_sends_the_request() {
    let enrolled = Enrolled {
        key_id: vec![1; 16],
        c2pa_cert: vec![3],
        ca_cert: vec![4],
    };
    let sent = enrollment();
    let expected = enrolled.clone();
    let (c, m) = client(Mock::default().on(registry(), "enroll", move |a| {
        assert_eq!(decode_one::<Enrollment>(a).unwrap(), sent);
        reply(EnrollResult::Ok(expected.clone()))
    }));
    assert_eq!(run(c.enroll(enrollment())).unwrap(), enrolled);
    assert!(m.calls()[0].update);
}

#[test]
fn enroll_rejection_carries_registry_text() {
    let (c, _) = client(Mock::default().on(registry(), "enroll", |_| {
        reply(EnrollResult::Err("enrollment pending".into()))
    }));
    assert_eq!(
        run(c.enroll(enrollment())),
        Err(WitnessError::Rejected {
            reason: "enrollment pending".into()
        })
    );
}

#[test]
fn enroll_rejects_empty_chain_without_calling() {
    let (c, m) = client(Mock::default());
    let mut e = enrollment();
    e.chain.clear();
    assert!(matches!(
        run(c.enroll(e)),
        Err(WitnessError::InvalidInput { .. })
    ));
    assert!(m.calls().is_empty());
}

fn submission(n: u8) -> Submission {
    Submission {
        claim: vec![n],
        c2pa_claim: vec![n],
        cose: vec![n],
        fingerprints: None,
    }
}

#[test]
fn submit_validates_batch_size_locally() {
    let (c, m) = client(Mock::default());
    assert!(matches!(
        run(c.submit(vec![])),
        Err(WitnessError::InvalidInput { .. })
    ));
    let big = (0..=MAX_SUBMIT_BATCH as u8).map(submission).collect();
    assert!(run(c.submit(big)).is_err());
    assert!(m.calls().is_empty());
}

#[test]
fn submit_returns_results_in_order() {
    let anchored = Anchored {
        record_ix: 7,
        record_id: "X".into(),
        anchored_ns: 1,
        media_shard: 0,
    };
    let a = anchored.clone();
    let (c, m) = client(Mock::default().on(registry(), "submit", move |arg| {
        let batch: Vec<Submission> = decode_one(arg).unwrap();
        assert_eq!(batch.len(), 2);
        reply(vec![
            SubmitResult::Ok(a.clone()),
            SubmitResult::Err(witness_canister_common::SubmitErr::RetryAfterNs(5)),
        ])
    }));
    let r = run(c.submit(vec![submission(1), submission(2)])).unwrap();
    assert_eq!(r[0], SubmitResult::Ok(anchored));
    assert!(matches!(r[1], SubmitResult::Err(_)));
    assert!(m.calls()[0].update);
}

#[test]
fn submit_rejects_mismatched_result_count() {
    let (c, _) =
        client(Mock::default().on(registry(), "submit", |_| reply(Vec::<SubmitResult>::new())));
    assert!(matches!(
        run(c.submit(vec![submission(1)])),
        Err(WitnessError::Malformed { .. })
    ));
}

#[test]
fn reads_decode_registry_records() {
    let (c, m) = client(
        Mock::default()
            .on(registry(), "get", |a| {
                assert_eq!(decode_one::<String>(a).unwrap(), "R3");
                reply(Some(record(3)))
            })
            .on(registry(), "get_ix", |a| {
                let ix: u64 = decode_one(a).unwrap();
                reply((ix != 404).then(|| record(ix)))
            })
            .on(registry(), "by_capture", |a| {
                assert_eq!(decode_one::<u64>(a).unwrap(), 77);
                reply(vec!["A".to_string(), "B".to_string()])
            }),
    );
    assert_eq!(run(c.get("R3")).unwrap(), Some(record(3)));
    assert_eq!(run(c.get_ix(8)).unwrap(), Some(record(8)));
    assert_eq!(run(c.get_ix(404)).unwrap(), None);
    assert_eq!(run(c.by_capture(77)).unwrap(), vec!["A", "B"]);
    assert!(m.calls().iter().all(|c| !c.update));
}

#[test]
fn get_key_sends_blob() {
    let (c, _) = client(Mock::default().on(registry(), "get_key", |a| {
        assert_eq!(decode_one::<ByteBuf>(a).unwrap().into_vec(), vec![6; 16]);
        reply(None::<CertifiedKey>)
    }));
    assert_eq!(run(c.get_key(&[6; 16])).unwrap(), None);
}

#[test]
fn shards_decodes_the_certified_pair() {
    let s = Shards {
        index: vec![shard(0, index(0))],
        media: vec![shard(0, media(0))],
    };
    let expected = s.clone();
    let (c, _) = client(Mock::default().on(registry(), "shards", move |_| {
        Ok(encode_args((s.clone(), ByteBuf::from(vec![1, 2]))).unwrap())
    }));
    assert_eq!(run(c.shards()).unwrap(), expected);
}

#[test]
fn erase_maps_ok_and_err() {
    let req = EraseRequest {
        record_id: "R".into(),
        nonce: vec![1],
        auth: witness_canister_common::EraseAuth::Ii,
    };
    let (c, _) = client(Mock::default().on(registry(), "erase_mine", |_| reply(EraseResult::Ok)));
    assert_eq!(run(c.erase_mine(req.clone())), Ok(()));
    let (c, _) = client(Mock::default().on(registry(), "erase_mine", |_| {
        reply(EraseResult::Err("not yours".into()))
    }));
    assert_eq!(
        run(c.erase_mine(req)),
        Err(WitnessError::Rejected {
            reason: "not yours".into()
        })
    );
}

#[test]
fn records_skip_missing_and_keep_order() {
    let (c, _) = client(Mock::default().on(registry(), "get_ix", |a| {
        let ix: u64 = decode_one(a).unwrap();
        reply((ix % 2 == 1).then(|| record(ix)))
    }));
    let got = run(c.records(&[5, 2, 3, 4, 1])).unwrap();
    assert_eq!(
        got.iter().map(|r| r.record_ix).collect::<Vec<_>>(),
        [5, 3, 1]
    );
    assert!(run(c.records(&[])).unwrap().is_empty());
}

#[test]
fn records_propagate_transport_errors() {
    let (c, _) = client(Mock::default());
    assert!(matches!(
        run(c.records(&[1])),
        Err(WitnessError::Network { .. })
    ));
}

#[test]
fn malformed_replies_are_malformed_errors() {
    let (c, _) = client(Mock::default().on(registry(), "challenge", |_| Ok(b"DIDL junk".to_vec())));
    assert!(matches!(
        run(c.challenge()),
        Err(WitnessError::Malformed { .. })
    ));
    let (c, _) = client(Mock::default().on(registry(), "challenge", |_| reply(5u64)));
    assert!(matches!(
        run(c.challenge()),
        Err(WitnessError::Malformed { .. })
    ));
}

#[test]
fn merge_probes_dedupes_sorts_and_keeps_min_distance() {
    let a = probe_result(&[(5, 20), (2, 9)], &[7, 3], &[4], false, 100);
    let b = probe_result(&[(5, 4), (8, 9)], &[3, 1], &[4, 2], true, 40);
    let m = merge_probes(vec![a.clone(), b.clone()]);
    assert_eq!(
        m.pdq,
        vec![
            PdqHit {
                record_ix: 5,
                distance: 4
            },
            PdqHit {
                record_ix: 2,
                distance: 9
            },
            PdqHit {
                record_ix: 8,
                distance: 9
            },
        ]
    );
    assert_eq!(m.watermark, vec![1, 3, 7]);
    assert_eq!(m.exact, vec![2, 4]);
    assert!(m.truncated_candidates);
    assert_eq!(m.indexed_upto, 40);
    assert_eq!(merge_probes(vec![b, a]), m);
}

#[test]
fn merge_probes_caps_hits_and_flags_truncation() {
    let hits: Vec<(u64, u32)> = (0..40).map(|i| (i, 40 - i as u32)).collect();
    let m = merge_probes(vec![probe_result(&hits, &[], &[], false, 1)]);
    assert_eq!(m.pdq.len(), MAX_PDQ_HITS);
    assert_eq!(
        m.pdq[0],
        PdqHit {
            record_ix: 39,
            distance: 1
        }
    );
    assert!(m.truncated_candidates);
    let exact: Vec<(u64, u32)> = (0..MAX_PDQ_HITS as u64).map(|i| (i, 1)).collect();
    assert!(!merge_probes(vec![probe_result(&exact, &[], &[], false, 1)]).truncated_candidates);
}

#[test]
fn merge_of_nothing_is_empty_and_unindexed() {
    let m = merge_probes(vec![]);
    assert!(m.is_empty());
    assert_eq!(m.indexed_upto, 0);
    assert!(!m.truncated_candidates);
}

fn two_index_mock() -> Mock {
    Mock::default()
        .on(index(0), "probe", |a| {
            let (pdq, wm, exact): (Vec<ByteBuf>, Vec<u64>, Vec<ByteBuf>) = decode_args(a).unwrap();
            assert_eq!((pdq.len(), wm, exact.len()), (1, vec![9], 1));
            reply(probe_result(&[(1, 3)], &[1], &[], false, 10))
        })
        .on(index(1), "probe", |_| {
            reply(probe_result(&[(2, 1)], &[2], &[2], false, 20))
        })
}

#[test]
fn probe_fans_out_to_every_index_shard() {
    let (c, m) = client(two_index_mock());
    let shards = [shard(0, index(0)), shard(1, index(1))];
    let r = run(c.probe(&shards, &[vec![0; 32]], &[9], &[vec![1; 32]])).unwrap();
    assert_eq!(
        r.pdq[0],
        PdqHit {
            record_ix: 2,
            distance: 1
        }
    );
    assert_eq!(r.watermark, vec![1, 2]);
    assert_eq!(r.exact, vec![2]);
    assert_eq!(r.indexed_upto, 10);
    let mut hit: Vec<_> = m.calls().into_iter().map(|c| c.canister).collect();
    hit.sort();
    assert_eq!(hit, vec![index(0), index(1)]);
}

#[test]
fn probe_fails_if_any_shard_fails() {
    let (c, _) = client(two_index_mock());
    let shards = [shard(0, index(0)), shard(2, index(2))];
    assert!(matches!(
        run(c.probe(&shards, &[vec![0; 32]], &[9], &[vec![1; 32]])),
        Err(WitnessError::Network { .. })
    ));
}

#[test]
fn probe_enforces_limits_before_calling() {
    let (c, m) = client(two_index_mock());
    let shards = [shard(0, index(0))];
    assert!(run(c.probe(&shards, &vec![vec![0; 32]; 17], &[], &[])).is_err());
    assert!(run(c.probe(&shards, &[vec![0; 31]], &[], &[])).is_err());
    assert!(run(c.probe(&shards, &[], &[0; 9], &[])).is_err());
    assert!(run(c.probe(&shards, &[], &[], &vec![vec![0; 32]; 5])).is_err());
    assert!(m.calls().is_empty());
}

#[test]
fn probe_with_no_shards_is_empty() {
    let (c, _) = client(Mock::default());
    assert!(run(c.probe(&[], &[], &[1], &[])).unwrap().is_empty());
}

#[test]
fn derivatives_union_sorted_and_capped() {
    let (c, _) = client(
        Mock::default()
            .on(index(0), "derivatives", |_| reply(vec![9u64, 3, 5]))
            .on(index(1), "derivatives", |_| reply(vec![3u64, 1])),
    );
    let shards = [shard(0, index(0)), shard(1, index(1))];
    assert_eq!(
        run(c.derivatives(&shards, 1, 32)).unwrap(),
        vec![1, 3, 5, 9]
    );
    assert_eq!(run(c.derivatives(&shards, 1, 2)).unwrap(), vec![1, 3]);
}

#[test]
fn fingerprints_take_the_shard_that_has_them() {
    let f = Fingerprints {
        grid: vec![0; 384],
        shapes: vec![(0, ByteBuf::from(vec![1; 32]))],
        quality: 80,
        portrait: false,
    };
    let expected = f.clone();
    let (c, _) = client(
        Mock::default()
            .on(index(0), "fingerprints", |_| reply(None::<Fingerprints>))
            .on(index(1), "fingerprints", move |_| reply(Some(f.clone()))),
    );
    let shards = [shard(0, index(0)), shard(1, index(1))];
    assert_eq!(run(c.fingerprints(&shards, 4)).unwrap(), Some(expected));
    assert_eq!(run(c.fingerprints(&shards[..1], 4)).unwrap(), None);
}

fn http(status: u16, body: &[u8]) -> Result<Vec<u8>, WitnessError> {
    reply(HttpResponse {
        status_code: status,
        body: body.to_vec(),
    })
}

#[test]
fn http_get_returns_body_only_on_200() {
    let (c, _) = client(
        Mock::default()
            .on(media(0), "http_request", |a| {
                let req: HttpRequest = decode_one(a).unwrap();
                assert_eq!((req.method.as_str(), req.url.as_str()), ("GET", "/p/X"));
                http(200, b"jpeg")
            })
            .on(media(1), "http_request", |_| http(404, b"nope")),
    );
    assert_eq!(
        run(c.http_get(media(0), "/p/X")).unwrap(),
        Some(b"jpeg".to_vec())
    );
    assert_eq!(run(c.http_get(media(1), "/p/X")).unwrap(), None);
    let shards = [shard(1, media(1)), shard(0, media(0))];
    assert_eq!(
        run(c.http_get_any(&shards, "/p/X")).unwrap(),
        Some(b"jpeg".to_vec())
    );
}

#[test]
fn http_response_decodes_with_extra_gateway_fields() {
    #[derive(CandidType)]
    struct Full {
        status_code: u16,
        headers: Vec<(String, String)>,
        body: ByteBuf,
        upgrade: Option<bool>,
    }
    let (c, _) = client(Mock::default().on(media(0), "http_request", |_| {
        reply(Full {
            status_code: 200,
            headers: vec![("content-type".into(), "image/jpeg".into())],
            body: ByteBuf::from(vec![7]),
            upgrade: None,
        })
    }));
    assert_eq!(run(c.http_get(media(0), "/p/X")).unwrap(), Some(vec![7]));
}

fn begin() -> BeginUpload {
    BeginUpload {
        target: witness_canister_common::UploadTarget::Preview("R".into()),
        size: 3,
        sha256: vec![0; 32],
        key_id: vec![1; 16],
        signature: vec![2; 64],
        grant: None,
    }
}

#[test]
fn media_upload_flow_maps_results() {
    let (c, m) = client(
        Mock::default()
            .on(media(0), "begin_upload", |_| {
                reply(Ok::<Begun, MediaErr>(Begun {
                    missing: vec![0],
                    upload: 9,
                }))
            })
            .on(media(0), "put_chunk", |a| {
                let chunk: Chunk = decode_one(a).unwrap();
                assert_eq!((chunk.upload, chunk.ix, chunk.bytes), (9, 0, vec![1, 2, 3]));
                reply(Ok::<(), MediaErr>(()))
            })
            .on(media(0), "commit", |_| {
                reply(Ok::<String, MediaErr>("/p/R".into()))
            }),
    );
    assert_eq!(run(c.begin_upload(media(0), begin())).unwrap().upload, 9);
    run(c.put_chunk(
        media(0),
        Chunk {
            upload: 9,
            ix: 0,
            bytes: vec![1, 2, 3],
        },
    ))
    .unwrap();
    assert_eq!(run(c.commit(media(0), 9)).unwrap(), "/p/R");
    assert!(m.calls().iter().all(|c| c.update));
}

#[test]
fn media_errors_surface_as_media_variant() {
    let (c, _) = client(
        Mock::default()
            .on(media(0), "begin_upload", |_| {
                reply(Err::<Begun, MediaErr>(MediaErr::QuotaExceeded))
            })
            .on(media(0), "commit", |_| {
                reply(Err::<String, MediaErr>(MediaErr::RetryAfterNs(5)))
            }),
    );
    assert_eq!(
        run(c.begin_upload(media(0), begin())),
        Err(WitnessError::Media {
            err: MediaErr::QuotaExceeded
        })
    );
    assert_eq!(
        run(c.commit(media(0), 1)),
        Err(WitnessError::Media {
            err: MediaErr::RetryAfterNs(5)
        })
    );
}

#[test]
fn put_chunk_checks_size_locally() {
    let (c, m) = client(Mock::default());
    for bytes in [vec![], vec![0; MAX_CHUNK_BYTES + 1]] {
        let chunk = Chunk {
            upload: 1,
            ix: 0,
            bytes,
        };
        assert!(matches!(
            run(c.put_chunk(media(0), chunk)),
            Err(WitnessError::InvalidInput { .. })
        ));
    }
    assert!(m.calls().is_empty());
}
