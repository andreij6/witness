use super::*;
use candid::{Decode, Encode};
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use witness_format::RecordId;

const NOW: u64 = 1_000_000;

fn caller() -> Principal {
    Principal::from_slice(&[42; 29])
}

fn grant_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[11; 32])
}

fn grant() -> EntitlementGrant {
    let mut g = EntitlementGrant {
        ii_principal: caller(),
        tier: Tier::Plus,
        quota_bytes: 25 << 30,
        expires_ns: NOW + 1,
        signature: vec![],
    };
    g.signature = grant_key().sign(&g.signing_bytes()).to_bytes().to_vec();
    g
}

fn signer() -> [u8; 32] {
    grant_key().verifying_key().to_bytes()
}

fn round_trip<T: CandidType + for<'de> Deserialize<'de> + PartialEq + core::fmt::Debug>(v: &T) {
    let bytes = Encode!(v).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(Encode!(v).unwrap_or_else(|e| panic!("{e}")), bytes);
    assert_eq!(&Decode!(&bytes, T).unwrap_or_else(|e| panic!("{e}")), v);
}

#[test]
fn grant_signing_bytes_layout() {
    let g = grant();
    let b = g.signing_bytes();
    let p = caller();
    let p = p.as_slice();
    assert_eq!(&b[..16], GRANT_DOMAIN);
    assert_eq!(b[16] as usize, p.len());
    assert_eq!(&b[17..17 + p.len()], p);
    let rest = &b[17 + p.len()..];
    assert_eq!(rest[0], 0);
    assert_eq!(&rest[1..9], &(25u64 << 30).to_be_bytes());
    assert_eq!(&rest[9..17], &(NOW + 1).to_be_bytes());
    assert_eq!(rest.len(), 17);
    assert_eq!(b, grant().signing_bytes());
}

#[test]
fn grant_verifies() {
    assert_eq!(grant().verify(&signer(), &caller(), NOW), Ok(()));
    round_trip(&grant());
}

#[test]
fn grant_rejects_expiry_caller_and_tampering() {
    let g = grant();
    assert_eq!(
        g.verify(&signer(), &caller(), NOW + 1),
        Err(ContractError::Expired)
    );
    assert_eq!(
        g.verify(&signer(), &Principal::anonymous(), NOW),
        Err(ContractError::WrongCaller)
    );
    let mut t = grant();
    t.quota_bytes += 1;
    assert_eq!(
        t.verify(&signer(), &caller(), NOW),
        Err(ContractError::BadSignature)
    );
    let mut t = grant();
    t.expires_ns += 1;
    assert_eq!(
        t.verify(&signer(), &caller(), NOW),
        Err(ContractError::BadSignature)
    );
    let mut t = grant();
    t.signature.truncate(10);
    assert_eq!(
        t.verify(&signer(), &caller(), NOW),
        Err(ContractError::BadSignature)
    );
    let other = ed25519_dalek::SigningKey::from_bytes(&[12; 32])
        .verifying_key()
        .to_bytes();
    assert_eq!(
        g.verify(&other, &caller(), NOW),
        Err(ContractError::BadSignature)
    );
    assert_eq!(
        g.verify(&[1, 2], &caller(), NOW),
        Err(ContractError::BadSignature)
    );
}

#[test]
fn grant_principal_length_prefix_prevents_ambiguity() {
    let mut a = grant();
    a.ii_principal = Principal::from_slice(&[1, 2]);
    let mut b = grant();
    b.ii_principal = Principal::from_slice(&[1, 2, 0]);
    assert_ne!(a.signing_bytes(), b.signing_bytes());
}

fn device() -> SigningKey {
    SigningKey::from_slice(&[9; 32]).unwrap_or_else(|_| panic!("key"))
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

fn device_spki() -> Vec<u8> {
    spki(&device())
}

fn record_text() -> String {
    RecordId::from_bytes([3; 10]).to_string()
}

fn case_text() -> String {
    CaseId::new(2, [8; 15]).to_string()
}

fn begin(target: UploadTarget, size: u64) -> BeginUpload {
    let mut b = BeginUpload {
        target,
        size,
        sha256: vec![6; 32],
        key_id: KeyId::from_spki(&device_spki()).as_bytes().to_vec(),
        signature: vec![],
        grant: None,
    };
    let msg = b.signing_bytes().unwrap_or_else(|e| panic!("{e}"));
    let sig: Signature = device().sign(&msg);
    b.signature = sig.to_bytes().to_vec();
    b
}

fn targets() -> Vec<UploadTarget> {
    vec![
        UploadTarget::Preview(record_text()),
        UploadTarget::Og(record_text()),
        UploadTarget::Full(record_text()),
        UploadTarget::CaseChunk {
            case_id: case_text(),
            ix: 1,
            count: 3,
        },
    ]
}

#[test]
fn target_codes_are_distinct() {
    let codes: Vec<u8> = targets().iter().map(UploadTarget::code).collect();
    assert_eq!(codes, [0, 1, 2, 3]);
}

#[test]
fn target_ids_parse() {
    let t = targets();
    assert_eq!(t[0].record_id(), Ok(Some(RecordId::from_bytes([3; 10]))));
    assert_eq!(t[0].case_id(), Ok(None));
    assert_eq!(t[3].record_id(), Ok(None));
    assert_eq!(t[3].case_id().map(|c| c.map(|c| c.shard())), Ok(Some(2)));
}

#[test]
fn target_rejects_bad_ids_and_indices() {
    assert_eq!(
        UploadTarget::Og("??".into()).record_id(),
        Err(ContractError::BadId)
    );
    let bad_case = |case_id: String, ix, count| UploadTarget::CaseChunk { case_id, ix, count };
    assert_eq!(
        bad_case("x".into(), 0, 1).case_id(),
        Err(ContractError::BadId)
    );
    assert_eq!(
        bad_case(case_text(), 0, 0).case_id(),
        Err(ContractError::Invalid)
    );
    assert_eq!(
        bad_case(case_text(), 3, 3).case_id(),
        Err(ContractError::Invalid)
    );
    assert_eq!(
        bad_case(case_text(), u32::MAX, u32::MAX).case_id(),
        Err(ContractError::Invalid)
    );
    assert_eq!(
        bad_case(case_text(), 0, MAX_PLUS_CASE_CHUNKS + 1).case_id(),
        Err(ContractError::Invalid)
    );
    assert!(
        bad_case(case_text(), MAX_PLUS_CASE_CHUNKS - 1, MAX_PLUS_CASE_CHUNKS)
            .case_id()
            .is_ok()
    );
}

#[test]
fn case_limits_are_chunk_multiples() {
    use crate::limits::{MAX_CASE_CHUNKS, MAX_PLUS_CASE_BYTES};
    assert_eq!(
        MAX_CASE_BYTES,
        MAX_CASE_CHUNKS as u64 * MAX_CHUNK_BYTES as u64
    );
    assert_eq!(
        MAX_PLUS_CASE_BYTES,
        MAX_PLUS_CASE_CHUNKS as u64 * MAX_CHUNK_BYTES as u64
    );
    assert_eq!((MAX_CASE_CHUNKS, MAX_PLUS_CASE_CHUNKS), (16, 256));
    assert_eq!(MAX_PLUS_CASE_BYTES, 256 << 20);
}

#[test]
fn case_info_round_trips() {
    let i = CaseInfo {
        expires_ns: u64::MAX,
        bytes: 7,
        chunks: 3,
    };
    round_trip(&i);
    round_trip(&Some(i));
    round_trip(&None::<CaseInfo>);
}

#[test]
fn target_size_caps() {
    let t = targets();
    assert_eq!(t[0].max_bytes(), Some(MAX_PREVIEW_BYTES));
    assert_eq!(t[1].max_bytes(), Some(MAX_OG_BYTES));
    assert_eq!(t[2].max_bytes(), None);
    assert_eq!(t[3].max_bytes(), Some(MAX_CASE_BYTES));
}

#[test]
fn begin_upload_verifies_for_every_target() {
    for t in targets() {
        let b = begin(t, 1000);
        assert_eq!(b.verify(&device_spki()), Ok(()));
        round_trip(&b);
    }
}

#[test]
fn begin_upload_signing_bytes_bind_every_field() {
    let base = begin(targets()[3].clone(), 1000);
    let msg = base.signing_bytes().unwrap_or_default();
    assert!(msg.starts_with(UPLOAD_DOMAIN));
    let variants = [
        BeginUpload {
            size: 1001,
            ..base.clone()
        },
        BeginUpload {
            sha256: vec![7; 32],
            ..base.clone()
        },
        BeginUpload {
            target: UploadTarget::CaseChunk {
                case_id: case_text(),
                ix: 2,
                count: 3,
            },
            ..base.clone()
        },
        BeginUpload {
            target: UploadTarget::CaseChunk {
                case_id: case_text(),
                ix: 1,
                count: 4,
            },
            ..base.clone()
        },
        BeginUpload {
            target: UploadTarget::Preview(record_text()),
            ..base.clone()
        },
    ];
    for v in &variants {
        assert_ne!(v.signing_bytes().unwrap_or_default(), msg);
        assert_eq!(v.verify(&device_spki()), Err(ContractError::BadSignature));
    }
    let a = begin(UploadTarget::Preview(record_text()), 10).signing_bytes();
    let b = begin(UploadTarget::Og(record_text()), 10).signing_bytes();
    assert_ne!(a, b);
}

#[test]
fn begin_upload_shape_errors() {
    let base = begin(targets()[0].clone(), 1000);
    let bad_hash = BeginUpload {
        sha256: vec![0; 31],
        ..base.clone()
    };
    assert_eq!(bad_hash.check_shape(), Err(ContractError::BadLength));
    let empty = BeginUpload {
        size: 0,
        ..base.clone()
    };
    assert_eq!(empty.check_shape(), Err(ContractError::BadLength));
    let big = BeginUpload {
        size: MAX_PREVIEW_BYTES + 1,
        ..base.clone()
    };
    assert_eq!(big.signing_bytes(), Err(ContractError::TooLarge));
    let bad_key = BeginUpload {
        key_id: vec![1; 15],
        ..base.clone()
    };
    assert_eq!(bad_key.check_shape(), Err(ContractError::BadId));
    assert_eq!(bad_key.verify(&device_spki()), Err(ContractError::BadId));
    let full = begin(UploadTarget::Full(record_text()), u64::MAX);
    assert_eq!(full.check_shape(), Ok(()));
}

#[test]
fn begin_upload_rejects_wrong_key() {
    let b = begin(targets()[0].clone(), 10);
    let other = spki(&SigningKey::from_slice(&[10; 32]).unwrap_or_else(|_| panic!("key")));
    assert_eq!(b.verify(&other), Err(ContractError::BadKey));
    let mut t = b.clone();
    t.signature[5] ^= 1;
    assert_eq!(t.verify(&device_spki()), Err(ContractError::BadSignature));
}

#[test]
fn chunk_counts() {
    let mut b = begin(targets()[2].clone(), 1);
    assert_eq!(b.chunk_count(), 1);
    b.size = MAX_CHUNK_BYTES as u64;
    assert_eq!(b.chunk_count(), 1);
    b.size += 1;
    assert_eq!(b.chunk_count(), 2);
    b.size = 0;
    assert_eq!(b.chunk_count(), 0);
    b.size = u64::MAX;
    assert_eq!(b.chunk_count(), u64::MAX / MAX_CHUNK_BYTES as u64 + 1);
}

#[test]
fn chunk_limits() {
    let c = |n: usize| Chunk {
        upload: 1,
        ix: 0,
        bytes: vec![0; n],
    };
    assert!(!c(0).within_limits());
    assert!(c(1).within_limits());
    assert!(c(MAX_CHUNK_BYTES).within_limits());
    assert!(!c(MAX_CHUNK_BYTES + 1).within_limits());
    round_trip(&c(10));
}

#[test]
fn media_errors_from_contract_errors() {
    assert_eq!(MediaErr::from(ContractError::TooLarge), MediaErr::TooLarge);
    assert_eq!(
        MediaErr::from(ContractError::BadKey),
        MediaErr::Unauthorized
    );
    assert_eq!(
        MediaErr::from(ContractError::BadSignature),
        MediaErr::Unauthorized
    );
    assert_eq!(
        MediaErr::from(ContractError::WrongCaller),
        MediaErr::Unauthorized
    );
    assert_eq!(
        MediaErr::from(ContractError::Expired),
        MediaErr::GrantInvalid
    );
    assert!(matches!(
        MediaErr::from(ContractError::BadId),
        MediaErr::Malformed(_)
    ));
    assert!(matches!(
        MediaErr::from(ContractError::BadLength),
        MediaErr::Malformed(_)
    ));
    assert!(matches!(
        MediaErr::from(ContractError::Invalid),
        MediaErr::Malformed(_)
    ));
}

#[test]
fn media_errors_round_trip() {
    for e in [
        MediaErr::Malformed("m".into()),
        MediaErr::Unauthorized,
        MediaErr::NotFound,
        MediaErr::TooLarge,
        MediaErr::QuotaExceeded,
        MediaErr::GrantInvalid,
        MediaErr::BadChunk,
        MediaErr::HashMismatch,
        MediaErr::Incomplete,
        MediaErr::RetryAfterNs(3),
    ] {
        round_trip(&e);
    }
}
