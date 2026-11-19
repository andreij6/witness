use super::*;
use candid::{Decode, Encode};
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_slice(&[seed; 32]).unwrap_or_else(|_| panic!("bad seed"))
}

const SPKI_P256_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

fn spki(k: &SigningKey) -> Vec<u8> {
    let mut v = SPKI_P256_PREFIX.to_vec();
    v.extend_from_slice(k.verifying_key().to_encoded_point(false).as_bytes());
    v
}

fn format_key() -> wf::KeyInfo {
    let pubkey = spki(&signing_key(3));
    let mut history = KeyHistory::new(1_000);
    history
        .push(wf::StatusChange {
            status: wf::KeyStatus::Retired,
            since_ns: 2_000,
        })
        .unwrap_or_else(|_| panic!("push"));
    wf::KeyInfo {
        key_id: KeyId::from_spki(&pubkey),
        pubkey,
        security: wf::SecurityLevel::Tee,
        provisioning: wf::Provisioning::Rkp,
        freshness: wf::Freshness::Late,
        patch: 202_511,
        enrolled_ns: 1_000,
        active_until_ns: 5_000,
        history,
        chain_serials: vec!["1f".into(), "abc".into()],
    }
}

fn round_trip<T: CandidType + for<'de> Deserialize<'de> + PartialEq + core::fmt::Debug>(v: &T) {
    let bytes = Encode!(v).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(Encode!(v).unwrap_or_else(|e| panic!("{e}")), bytes);
    let back = Decode!(&bytes, T).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(&back, v);
}

fn submission() -> Submission {
    Submission {
        claim: vec![1; 10],
        c2pa_claim: vec![2; 10],
        cose: vec![3; 10],
        fingerprints: Some(ByteBuf::from(vec![4; 10])),
    }
}

fn anchored() -> Anchored {
    Anchored {
        record_ix: 42,
        record_id: "SYXJWEC36ZHC9FGC".into(),
        anchored_ns: 7,
        media_shard: 3,
    }
}

#[test]
fn enums_mirror_format_in_both_directions() {
    for s in wf::SecurityLevel::ALL {
        assert_eq!(wf::SecurityLevel::from(Security::from(*s)), *s);
    }
    for p in wf::Provisioning::ALL {
        assert_eq!(wf::Provisioning::from(Prov::from(*p)), *p);
    }
    for f in wf::Freshness::ALL {
        assert_eq!(wf::Freshness::from(Fresh::from(*f)), *f);
    }
    for k in wf::KeyStatus::ALL {
        assert_eq!(wf::KeyStatus::from(KeyStatus::from(*k)), *k);
    }
}

#[test]
fn key_info_round_trips_through_format() {
    let k = format_key();
    let view = KeyInfo::from(&k);
    assert_eq!(view.key_id, k.key_id.as_bytes());
    assert_eq!(view.history.len(), 2);
    assert_eq!(view.history[1].status, KeyStatus::Retired);
    let back = wf::KeyInfo::try_from(view.clone()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, k);
    round_trip(&view);
}

#[test]
fn key_info_rejects_bad_key_id_length() {
    let mut v = KeyInfo::from(&format_key());
    v.key_id.pop();
    assert_eq!(wf::KeyInfo::try_from(v), Err(ContractError::BadId));
}

#[test]
fn key_info_rejects_mismatched_key_id() {
    let mut v = KeyInfo::from(&format_key());
    v.key_id[0] ^= 1;
    assert_eq!(wf::KeyInfo::try_from(v), Err(ContractError::Invalid));
}

#[test]
fn key_info_rejects_bad_history() {
    let mut v = KeyInfo::from(&format_key());
    v.history.clear();
    assert_eq!(wf::KeyInfo::try_from(v), Err(ContractError::Invalid));
    let mut v = KeyInfo::from(&format_key());
    v.history.swap(0, 1);
    assert_eq!(wf::KeyInfo::try_from(v), Err(ContractError::Invalid));
}

#[test]
fn key_info_rejects_bad_serials_and_window() {
    let mut v = KeyInfo::from(&format_key());
    v.chain_serials.push("0abc".into());
    assert_eq!(wf::KeyInfo::try_from(v), Err(ContractError::Invalid));
    let mut v = KeyInfo::from(&format_key());
    v.active_until_ns = 0;
    assert_eq!(wf::KeyInfo::try_from(v), Err(ContractError::Invalid));
}

#[test]
fn submission_limits() {
    assert!(submission().within_limits());
    let mut s = submission();
    s.fingerprints = None;
    assert!(s.within_limits());
    let mut s = submission();
    s.claim.clear();
    assert!(!s.within_limits());
    let mut s = submission();
    s.cose = vec![0; wf::Submission::MAX_PART + 1];
    assert!(!s.within_limits());
    let mut s = submission();
    s.c2pa_claim = vec![0; wf::Submission::MAX_PART];
    assert!(s.within_limits());
    let mut s = submission();
    s.fingerprints = Some(ByteBuf::new());
    assert!(!s.within_limits());
    let mut s = submission();
    s.fingerprints = Some(ByteBuf::from(vec![0; MAX_FINGERPRINTS_BYTES + 1]));
    assert!(!s.within_limits());
}

#[test]
fn submission_parts_round_trip() {
    let s = submission();
    let (f, fp) = s.clone().into_parts();
    assert_eq!(f.claim_cbor, s.claim);
    assert_eq!(f.cose, s.cose);
    assert_eq!(fp.as_deref(), Some(&[4u8; 10][..]));
    assert_eq!(f.record_id(), s.record_id());
    assert_eq!(Submission::from_parts(f, fp), s);
    round_trip(&s);
}

#[test]
fn submission_record_id_tracks_claim_only() {
    let a = submission();
    let mut b = submission();
    b.cose = vec![9; 3];
    b.fingerprints = None;
    assert_eq!(a.record_id(), b.record_id());
    b.claim[0] ^= 1;
    assert_ne!(a.record_id(), b.record_id());
}

#[test]
fn submit_results_round_trip_and_classify() {
    let all = [
        SubmitResult::Ok(anchored()),
        SubmitResult::Dup(anchored()),
        SubmitResult::Err(SubmitErr::Malformed("x".into())),
        SubmitResult::Err(SubmitErr::TooLarge),
        SubmitResult::Err(SubmitErr::BadSignature),
        SubmitResult::Err(SubmitErr::UnknownKey),
        SubmitResult::Err(SubmitErr::KeyDistrusted),
        SubmitResult::Err(SubmitErr::KeyExpired),
        SubmitResult::Err(SubmitErr::CounterReused),
        SubmitResult::Err(SubmitErr::ParentUnknown),
        SubmitResult::Err(SubmitErr::FingerprintMismatch),
        SubmitResult::Err(SubmitErr::RetryAfterNs(9)),
    ];
    for r in &all {
        round_trip(r);
    }
    assert_eq!(all[0].anchored(), Some(&anchored()));
    assert_eq!(all[1].anchored(), Some(&anchored()));
    assert!(all[2..].iter().all(|r| r.anchored().is_none()));
    assert!(SubmitErr::RetryAfterNs(0).is_retryable());
    assert!(!SubmitErr::KeyExpired.is_retryable());
    round_trip(&all.to_vec());
}

#[test]
fn certified_types_round_trip() {
    let rec = CertifiedRecord {
        record_ix: 1,
        record_id: "SYXJWEC36ZHC9FGC".into(),
        claim: vec![1, 2],
        c2pa_claim: vec![3],
        cose: vec![4],
        anchored_ns: 5,
        mmr_proof: vec![ByteBuf::from(vec![7; 32]), ByteBuf::from(vec![8; 32])],
        leaf_count: 2,
        certificate: vec![9; 100],
        media_shard: 255,
    };
    round_trip(&rec);
    round_trip(&Some(rec));
    round_trip(&CertifiedKey {
        key: KeyInfo::from(&format_key()),
        smt_proof: vec![1; 48],
        certificate: vec![],
    });
}

#[test]
fn shards_lookup() {
    let p = |n: u8| Principal::from_slice(&[n; 10]);
    let s = Shards {
        index: vec![
            ShardInfo {
                shard: 0,
                canister: p(1),
                open: false,
            },
            ShardInfo {
                shard: 1,
                canister: p(2),
                open: true,
            },
        ],
        media: vec![ShardInfo {
            shard: 4,
            canister: p(3),
            open: true,
        }],
    };
    assert_eq!(s.open_index().map(|i| i.shard), Some(1));
    assert_eq!(s.media_shard(4).map(|m| m.canister), Some(p(3)));
    assert!(s.media_shard(0).is_none());
    assert!(Shards::default().open_index().is_none());
    round_trip(&s);
}

fn erase(seed: u8, nonce: Vec<u8>) -> (EraseRequest, Vec<u8>) {
    let k = signing_key(seed);
    let rid = RecordId::from_claim_cbor(b"claim");
    let msg = erase_signing_bytes(&rid, &nonce).unwrap_or_else(|e| panic!("{e}"));
    let sig: Signature = k.sign(&msg);
    (
        EraseRequest {
            record_id: rid.to_string(),
            nonce,
            auth: EraseAuth::Device(sig.to_bytes().to_vec()),
        },
        spki(&k),
    )
}

#[test]
fn erase_signing_bytes_layout() {
    let rid = RecordId::from_bytes([7; 10]);
    let b = erase_signing_bytes(&rid, &[1, 2, 3]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(&b[..16], b"witness/erase/v1");
    assert_eq!(&b[16..26], &[7; 10]);
    assert_eq!(&b[26..], &[1, 2, 3]);
    assert_eq!(b, erase_signing_bytes(&rid, &[1, 2, 3]).unwrap_or_default());
}

#[test]
fn erase_nonce_bounds() {
    let rid = RecordId::from_bytes([0; 10]);
    assert_eq!(
        erase_signing_bytes(&rid, &[]),
        Err(ContractError::BadLength)
    );
    assert!(erase_signing_bytes(&rid, &[0; MAX_NONCE_BYTES]).is_ok());
    assert_eq!(
        erase_signing_bytes(&rid, &[0; MAX_NONCE_BYTES + 1]),
        Err(ContractError::BadLength)
    );
}

#[test]
fn erase_device_signature_verifies() {
    let (req, pk) = erase(5, vec![1; 16]);
    assert_eq!(req.verify_device(&pk), Ok(()));
    round_trip(&req);
}

#[test]
fn erase_accepts_grouped_and_lowercase_ids() {
    let (mut req, pk) = erase(5, vec![1; 16]);
    req.record_id = RecordId::parse(&req.record_id)
        .unwrap_or_else(|e| panic!("{e}"))
        .grouped()
        .to_lowercase();
    assert_eq!(req.verify_device(&pk), Ok(()));
}

#[test]
fn erase_rejects_tampering() {
    let (req, pk) = erase(5, vec![1; 16]);
    let (_, other) = erase(6, vec![1; 16]);
    assert_eq!(req.verify_device(&other), Err(ContractError::BadSignature));
    let mut r = req.clone();
    r.nonce[0] ^= 1;
    assert_eq!(r.verify_device(&pk), Err(ContractError::BadSignature));
    let mut r = req.clone();
    r.record_id = RecordId::from_bytes([1; 10]).to_string();
    assert_eq!(r.verify_device(&pk), Err(ContractError::BadSignature));
    let mut r = req.clone();
    r.auth = EraseAuth::Ii;
    assert_eq!(r.verify_device(&pk), Err(ContractError::BadSignature));
    let mut r = req.clone();
    r.auth = EraseAuth::Device(vec![0; 3]);
    assert_eq!(r.verify_device(&pk), Err(ContractError::BadSignature));
    assert_eq!(
        req.verify_device(&[1, 2, 3]),
        Err(ContractError::BadSignature)
    );
}

#[test]
fn erase_rejects_bad_record_id() {
    let (mut req, pk) = erase(5, vec![1; 16]);
    req.record_id = "not-an-id!".into();
    assert_eq!(req.record_id(), Err(ContractError::BadId));
    assert_eq!(req.verify_device(&pk), Err(ContractError::BadId));
    req.record_id = String::new();
    assert_eq!(req.signing_bytes(), Err(ContractError::BadId));
}

#[test]
fn decoding_garbage_fails() {
    assert!(Decode!(b"DIDL\x00\x01\x71\x00", Submission).is_err());
    assert!(Decode!(&[0u8; 4], KeyInfo).is_err());
    let wrong = Encode!(&anchored()).unwrap_or_else(|e| panic!("{e}"));
    assert!(Decode!(&wrong, CertifiedRecord).is_err());
}

#[derive(CandidType, Deserialize, Debug, PartialEq)]
struct PreShardAnchored {
    record_ix: u64,
    record_id: String,
    anchored_ns: u64,
}

#[derive(CandidType, Deserialize, Debug, PartialEq)]
enum PreShardSubmitResult {
    #[serde(rename = "ok")]
    Ok(PreShardAnchored),
    #[serde(rename = "dup")]
    Dup(PreShardAnchored),
    #[serde(rename = "err")]
    Err(SubmitErr),
}

#[derive(CandidType, Deserialize, Debug, PartialEq)]
struct PreShardCertifiedRecord {
    record_ix: u64,
    record_id: String,
    #[serde(with = "serde_bytes")]
    claim: Vec<u8>,
    #[serde(with = "serde_bytes")]
    c2pa_claim: Vec<u8>,
    #[serde(with = "serde_bytes")]
    cose: Vec<u8>,
    anchored_ns: u64,
    mmr_proof: Vec<ByteBuf>,
    leaf_count: u64,
    #[serde(with = "serde_bytes")]
    certificate: Vec<u8>,
}

#[test]
fn clients_built_before_media_shard_still_decode_replies() {
    let old = |a: &Anchored| PreShardAnchored {
        record_ix: a.record_ix,
        record_id: a.record_id.clone(),
        anchored_ns: a.anchored_ns,
    };
    let a = anchored();
    let bytes = Encode!(&SubmitResult::Ok(a.clone())).unwrap_or_else(|e| panic!("{e}"));
    let back = Decode!(&bytes, PreShardSubmitResult).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, PreShardSubmitResult::Ok(old(&a)));
    let bytes = Encode!(&SubmitResult::Dup(a.clone())).unwrap_or_else(|e| panic!("{e}"));
    let back = Decode!(&bytes, PreShardSubmitResult).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, PreShardSubmitResult::Dup(old(&a)));
    let rec = CertifiedRecord {
        record_ix: 9,
        record_id: "SYXJWEC36ZHC9FGC".into(),
        claim: vec![1, 2],
        c2pa_claim: vec![3],
        cose: vec![4],
        anchored_ns: 5,
        mmr_proof: vec![ByteBuf::from(vec![7; 32])],
        leaf_count: 10,
        certificate: vec![9; 40],
        media_shard: 7,
    };
    let bytes = Encode!(&Some(rec.clone())).unwrap_or_else(|e| panic!("{e}"));
    let back = Decode!(&bytes, Option<PreShardCertifiedRecord>).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        back,
        Some(PreShardCertifiedRecord {
            record_ix: rec.record_ix,
            record_id: rec.record_id,
            claim: rec.claim,
            c2pa_claim: rec.c2pa_claim,
            cose: rec.cose,
            anchored_ns: rec.anchored_ns,
            mmr_proof: rec.mmr_proof,
            leaf_count: rec.leaf_count,
            certificate: rec.certificate,
        })
    );
}
