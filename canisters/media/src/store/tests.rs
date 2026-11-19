use super::*;
use crate::pagestore::PAGE;
use ic_stable_structures::VectorMemory;
use ic_stable_structures::memory_manager::{MemoryManager, VirtualMemory};
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use serde_bytes::ByteBuf;
use witness_canister_common::limits::{
    MAX_CASE_BYTES, MAX_CASE_CHUNKS, MAX_CHUNK_BYTES, MAX_PLUS_CASE_BYTES, MAX_PLUS_CASE_CHUNKS,
    MAX_PREVIEW_BYTES,
};
use witness_canister_common::{CaseInfo, EntitlementGrant, Tier, UploadTarget};
use witness_format::CaseId;
use witness_format::crypto::sha256;

type Mem = VirtualMemory<VectorMemory>;

const T0: u64 = 1_000 * DAY_NS + 5;
const SPKI_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

struct Env {
    mm: MemoryManager<VectorMemory>,
    media: Media<Mem>,
}

fn env() -> Env {
    let mm = MemoryManager::init(VectorMemory::default());
    let media = Media::init(|id| mm.get(id));
    Env { mm, media }
}

fn user() -> Principal {
    Principal::from_slice(&[7; 29])
}

fn ctx(now_ns: u64) -> Ctx {
    Ctx {
        now_ns,
        caller: user(),
        shard: 0,
        signer: Some(grant_key().verifying_key().to_bytes()),
    }
}

fn grant_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[0x57; 32])
}

struct Device {
    sk: SigningKey,
    spki: Vec<u8>,
    key_id: [u8; 16],
}

fn device(seed: u8) -> Device {
    let sk = SigningKey::from_slice(&[seed; 32]).unwrap_or_else(|e| panic!("{e}"));
    let mut spki = SPKI_PREFIX.to_vec();
    spki.extend_from_slice(sk.verifying_key().to_encoded_point(false).as_bytes());
    let key_id = KeyId::from_spki(&spki).to_bytes();
    Device { sk, spki, key_id }
}

fn rid(n: u8) -> RecordId {
    RecordId::from_bytes([n; 10])
}

fn cid(shard: u8, n: u8) -> CaseId {
    CaseId::new(shard, [n; 15])
}

fn bytes(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(13) ^ seed)
        .collect()
}

fn grant(expires_ns: u64, quota: u64) -> EntitlementGrant {
    let mut g = EntitlementGrant {
        ii_principal: user(),
        tier: Tier::Plus,
        quota_bytes: quota,
        expires_ns,
        signature: vec![],
    };
    g.signature = ed25519_dalek::Signer::sign(&grant_key(), &g.signing_bytes())
        .to_bytes()
        .to_vec();
    g
}

fn request(
    d: &Device,
    target: UploadTarget,
    data: &[u8],
    grant: Option<EntitlementGrant>,
) -> BeginUpload {
    let mut req = BeginUpload {
        target,
        size: data.len() as u64,
        sha256: sha256(data).to_vec(),
        key_id: d.key_id.to_vec(),
        signature: vec![],
        grant,
    };
    let msg = req.signing_bytes().unwrap_or_default();
    let sig: Signature = d.sk.sign(&msg);
    req.signature = sig.to_bytes().to_vec();
    req
}

fn preview(r: RecordId) -> UploadTarget {
    UploadTarget::Preview(r.to_string())
}

fn case_chunk(c: CaseId, ix: u32, count: u32) -> UploadTarget {
    UploadTarget::CaseChunk {
        case_id: c.to_string(),
        ix,
        count,
    }
}

impl Env {
    fn allow(&mut self, d: &Device, records: &[RecordId]) {
        let mut items = vec![Allow::Key {
            key_id: ByteBuf::from(d.key_id.to_vec()),
            pubkey: ByteBuf::from(d.spki.clone()),
        }];
        items.extend(records.iter().map(|r| Allow::Record {
            record_id: r.to_string(),
            key_id: ByteBuf::from(d.key_id.to_vec()),
        }));
        assert_eq!(self.media.sync(&items), items.len() as u32);
    }

    fn send(&mut self, id: u64, data: &[u8]) {
        for (ix, c) in data.chunks(MAX_CHUNK_BYTES).enumerate() {
            let chunk = Chunk {
                upload: id,
                ix: ix as u32,
                bytes: c.to_vec(),
            };
            assert_eq!(self.media.put_chunk(&chunk, &user(), T0), Ok(()));
        }
    }

    fn upload(&mut self, req: &BeginUpload, data: &[u8], now: u64) -> Result<String, MediaErr> {
        let b = self.media.begin(req, &ctx(now))?;
        self.send(b.upload, data);
        self.media.commit(b.upload, now)
    }

    fn fetch(&self, url: &str) -> Option<Vec<u8>> {
        let s = self.media.lookup(url)?;
        self.media.read(s.blob, 0, s.size)
    }
}

#[test]
fn preview_upload_is_served_and_certified() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(5000, 1);
    let req = request(&d, preview(rid(1)), &data, None);
    let b = e
        .media
        .begin(&req, &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(b.missing, vec![0]);
    e.send(b.upload, &data);
    let url = e.media.commit(b.upload, T0).unwrap_or_default();
    assert_eq!(url, format!("/p/{}", rid(1)));
    assert_eq!(e.fetch(&url), Some(data.clone()));
    let s = e.media.lookup(&url).unwrap_or_else(|| panic!("missing"));
    assert_eq!(s.size, 5000);
    let w = e
        .media
        .witness(&s.path)
        .unwrap_or_else(ic_certification::empty);
    assert_eq!(w.digest(), e.media.root_hash());
    assert!(e.media.tree_contains(&s.path));
}

#[test]
fn uncommitted_blobs_are_not_served() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(10, 1);
    let b = e
        .media
        .begin(&request(&d, preview(rid(1)), &data, None), &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    e.send(b.upload, &data);
    assert!(e.media.lookup(&format!("/p/{}", rid(1))).is_none());
    assert!(e.media.read(b.upload, 0, 1).is_none());
    assert!(e.media.blob_size(b.upload).is_none());
}

#[test]
fn unknown_key_or_record_asks_to_retry() {
    let mut e = env();
    let d = device(1);
    let data = bytes(10, 1);
    let req = request(&d, preview(rid(1)), &data, None);
    assert_eq!(
        e.media.begin(&req, &ctx(T0)),
        Err(MediaErr::RetryAfterNs(SYNC_RETRY_NS))
    );
    e.allow(&d, &[]);
    assert_eq!(
        e.media.begin(&req, &ctx(T0)),
        Err(MediaErr::RetryAfterNs(SYNC_RETRY_NS))
    );
}

#[test]
fn record_of_another_key_is_unauthorized() {
    let mut e = env();
    let (a, b) = (device(1), device(2));
    e.allow(&a, &[rid(1)]);
    e.allow(&b, &[]);
    let req = request(&b, preview(rid(1)), &bytes(10, 1), None);
    assert_eq!(e.media.begin(&req, &ctx(T0)), Err(MediaErr::Unauthorized));
}

#[test]
fn bad_signature_and_shapes_are_rejected() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(10, 1);
    let mut req = request(&d, preview(rid(1)), &data, None);
    req.signature[5] ^= 1;
    assert_eq!(e.media.begin(&req, &ctx(T0)), Err(MediaErr::Unauthorized));
    let mut req = request(&d, preview(rid(1)), &data, None);
    req.size += 1;
    assert_eq!(e.media.begin(&req, &ctx(T0)), Err(MediaErr::Unauthorized));
    let mut req = request(&d, preview(rid(1)), &data, None);
    req.sha256.pop();
    assert!(matches!(
        e.media.begin(&req, &ctx(T0)),
        Err(MediaErr::Malformed(_))
    ));
    let big = vec![0u8; MAX_PREVIEW_BYTES as usize + 1];
    let req = request(&d, preview(rid(1)), &big, None);
    assert_eq!(e.media.begin(&req, &ctx(T0)), Err(MediaErr::TooLarge));
    let req = request(&d, UploadTarget::Preview("bad".into()), &data, None);
    assert!(matches!(
        e.media.begin(&req, &ctx(T0)),
        Err(MediaErr::Malformed(_))
    ));
    let mut req = request(&d, preview(rid(1)), &data, None);
    req.key_id.pop();
    assert!(matches!(
        e.media.begin(&req, &ctx(T0)),
        Err(MediaErr::Malformed(_))
    ));
}

#[test]
fn full_is_capped() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let mut req = request(
        &d,
        UploadTarget::Full(rid(1).to_string()),
        b"x",
        Some(grant(T0 + DAY_NS, 1 << 40)),
    );
    req.size = MAX_FULL_BYTES + 1;
    assert_eq!(e.media.begin(&req, &ctx(T0)), Err(MediaErr::TooLarge));
}

#[test]
fn begin_is_resumable_by_its_uploader() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(MAX_CHUNK_BYTES + 10, 3);
    let req = request(
        &d,
        UploadTarget::Full(rid(1).to_string()),
        &data,
        Some(grant(T0 + DAY_NS, 1 << 30)),
    );
    let first = e
        .media
        .begin(&req, &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(first.missing, vec![0, 1]);
    e.media
        .put_chunk(
            &Chunk {
                upload: first.upload,
                ix: 1,
                bytes: data[MAX_CHUNK_BYTES..].to_vec(),
            },
            &user(),
            T0,
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    let again = e
        .media
        .begin(&req, &ctx(T0 + 1))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        again,
        Begun {
            upload: first.upload,
            missing: vec![0]
        }
    );
    let chunk0 = Chunk {
        upload: first.upload,
        ix: 0,
        bytes: data[..MAX_CHUNK_BYTES].to_vec(),
    };
    let other = Principal::from_slice(&[1; 29]);
    assert_eq!(
        e.media.put_chunk(&chunk0, &other, T0),
        Err(MediaErr::Unauthorized)
    );
    assert_eq!(e.media.put_chunk(&chunk0, &user(), T0), Ok(()));
    assert!(e.media.commit(first.upload, T0).is_ok());
    assert_eq!(e.usage_used(), data.len() as u64);
}

fn as_caller(now: u64, caller: Principal) -> Ctx {
    Ctx { caller, ..ctx(now) }
}

#[test]
fn replayed_begin_cannot_take_over_an_active_upload() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(MAX_CHUNK_BYTES + 10, 3);
    let req = request(&d, preview(rid(1)), &data[..100], None);
    let first = e
        .media
        .begin(&req, &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let attacker = Principal::from_slice(&[0xAA; 29]);
    let replay = req.clone();
    assert_eq!(
        e.media.begin(&replay, &as_caller(T0 + 1, attacker)),
        Err(MediaErr::RetryAfterNs(STALE_UPLOAD_NS - 1))
    );
    let chunk = Chunk {
        upload: first.upload,
        ix: 0,
        bytes: data[..100].to_vec(),
    };
    assert_eq!(
        e.media.put_chunk(&chunk, &attacker, T0 + 2),
        Err(MediaErr::Unauthorized)
    );
    let older = request(&d, preview(rid(1)), &data[..50], None);
    assert_eq!(
        e.media.begin(&older, &as_caller(T0 + 3, attacker)),
        Err(MediaErr::RetryAfterNs(STALE_UPLOAD_NS - 3))
    );
    assert_eq!(e.media.put_chunk(&chunk, &user(), T0 + 4), Ok(()));
    assert_eq!(
        e.media.commit(first.upload, T0 + 5),
        Ok(format!("/p/{}", rid(1)))
    );
    assert_eq!(
        e.fetch(&format!("/p/{}", rid(1))),
        Some(data[..100].to_vec())
    );
    assert_eq!(
        e.media.begin(&replay, &as_caller(T0 + 6, attacker)),
        Ok(Begun {
            upload: first.upload,
            missing: vec![]
        })
    );
}

#[test]
fn activity_keeps_an_upload_bound_and_staleness_frees_it() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(2 * MAX_CHUNK_BYTES, 4);
    let req = request(&d, UploadTarget::Og(rid(1).to_string()), &data[..10], None);
    let b = e
        .media
        .begin(&req, &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let other = Principal::from_slice(&[2; 29]);
    let later = T0 + STALE_UPLOAD_NS - 1;
    let chunk = Chunk {
        upload: b.upload,
        ix: 0,
        bytes: data[..10].to_vec(),
    };
    assert_eq!(e.media.put_chunk(&chunk, &user(), later), Ok(()));
    assert_eq!(
        e.media.begin(&req, &as_caller(T0 + STALE_UPLOAD_NS, other)),
        Err(MediaErr::RetryAfterNs(STALE_UPLOAD_NS - 1))
    );
    assert_eq!(
        e.media.put_chunk(&chunk, &user(), T0),
        Ok(()),
        "an out-of-order clock never moves activity backwards"
    );
    let stale = later + STALE_UPLOAD_NS;
    assert_eq!(
        e.media.begin(&req, &as_caller(stale, other)),
        Ok(Begun {
            upload: b.upload,
            missing: vec![]
        })
    );
    assert_eq!(
        e.media.put_chunk(&chunk, &user(), stale),
        Err(MediaErr::Unauthorized)
    );
    assert_eq!(
        e.media.begin(&req, &ctx(stale + 1)),
        Err(MediaErr::RetryAfterNs(STALE_UPLOAD_NS - 1))
    );
    assert_eq!(e.media.put_chunk(&chunk, &other, stale + 2), Ok(()));
    assert!(e.media.commit(b.upload, stale + 3).is_ok());
}

#[test]
fn stale_upload_can_be_restarted_with_new_content_by_another_caller() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let a = bytes(10, 1);
    let b = bytes(20, 2);
    let first = e
        .media
        .begin(&request(&d, preview(rid(1)), &a, None), &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let other = Principal::from_slice(&[3; 29]);
    let swap = request(&d, preview(rid(1)), &b, None);
    assert!(matches!(
        e.media.begin(&swap, &as_caller(T0 + 1, other)),
        Err(MediaErr::RetryAfterNs(_))
    ));
    let restarted = e
        .media
        .begin(&swap, &as_caller(T0 + STALE_UPLOAD_NS, other))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_ne!(restarted.upload, first.upload);
    assert_eq!(e.media.commit(first.upload, T0), Err(MediaErr::NotFound));
}

#[test]
fn anonymous_callers_share_one_identity_but_cannot_steal_from_others() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1), rid(2)]);
    let data = bytes(10, 1);
    let anon = Principal::anonymous();
    let req = request(&d, preview(rid(1)), &data, None);
    let b = e
        .media
        .begin(&req, &as_caller(T0, anon))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(e.media.begin(&req, &as_caller(T0 + 1, anon)).is_ok());
    assert!(matches!(
        e.media.begin(&req, &ctx(T0 + 2)),
        Err(MediaErr::RetryAfterNs(_))
    ));
    let chunk = Chunk {
        upload: b.upload,
        ix: 0,
        bytes: data.clone(),
    };
    assert_eq!(e.media.put_chunk(&chunk, &anon, T0 + 3), Ok(()));
    let mine = request(&d, preview(rid(2)), &data, None);
    let m = e
        .media
        .begin(&mine, &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(matches!(
        e.media.begin(&mine, &as_caller(T0 + 1, anon)),
        Err(MediaErr::RetryAfterNs(_))
    ));
    assert_eq!(
        e.media.put_chunk(
            &Chunk {
                upload: m.upload,
                ix: 0,
                bytes: data
            },
            &anon,
            T0 + 2
        ),
        Err(MediaErr::Unauthorized)
    );
}

impl Env {
    fn usage_used(&self) -> u64 {
        self.media.usage_of(&user()).map_or(0, |u| u.used)
    }
}

#[test]
fn put_chunk_validation() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(100, 1);
    let b = e
        .media
        .begin(&request(&d, preview(rid(1)), &data, None), &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let chunk = |ix, bytes: Vec<u8>| Chunk {
        upload: b.upload,
        ix,
        bytes,
    };
    assert_eq!(
        e.media.put_chunk(&chunk(0, vec![]), &user(), T0),
        Err(MediaErr::BadChunk)
    );
    assert_eq!(
        e.media.put_chunk(&chunk(0, vec![1; 99]), &user(), T0),
        Err(MediaErr::BadChunk)
    );
    assert_eq!(
        e.media.put_chunk(&chunk(0, vec![1; 101]), &user(), T0),
        Err(MediaErr::BadChunk)
    );
    assert_eq!(
        e.media.put_chunk(&chunk(1, vec![1; 100]), &user(), T0),
        Err(MediaErr::BadChunk)
    );
    assert_eq!(
        e.media
            .put_chunk(&chunk(u32::MAX, vec![1; 100]), &user(), T0),
        Err(MediaErr::BadChunk)
    );
    let missing = Chunk {
        upload: 999,
        ix: 0,
        bytes: vec![1],
    };
    assert_eq!(
        e.media.put_chunk(&missing, &user(), T0),
        Err(MediaErr::NotFound)
    );
    assert_eq!(e.media.commit(999, T0), Err(MediaErr::NotFound));
}

#[test]
fn commit_checks_completeness_and_hash() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(2 * MAX_CHUNK_BYTES + 1, 2);
    let req = request(
        &d,
        UploadTarget::Full(rid(1).to_string()),
        &data,
        Some(grant(T0 + DAY_NS, 1 << 30)),
    );
    let b = e
        .media
        .begin(&req, &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(e.media.commit(b.upload, T0), Err(MediaErr::Incomplete));
    let mut bad = data.clone();
    bad[MAX_CHUNK_BYTES + 3] ^= 0xFF;
    e.send(b.upload, &bad);
    assert_eq!(e.media.commit(b.upload, T0), Err(MediaErr::HashMismatch));
    let resumed = e
        .media
        .begin(&req, &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(resumed.missing, vec![0, 1, 2]);
    e.send(b.upload, &data);
    let url = e.media.commit(b.upload, T0).unwrap_or_default();
    assert_eq!(e.fetch(&url), Some(data));
    assert_eq!(e.media.commit(b.upload, T0), Ok(url));
}

#[test]
fn committed_paths_are_immutable() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(10, 1);
    let req = request(&d, preview(rid(1)), &data, None);
    let url = e.upload(&req, &data, T0).unwrap_or_default();
    let again = e
        .media
        .begin(&req, &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(again.missing.is_empty());
    let other = bytes(10, 2);
    let req2 = request(&d, preview(rid(1)), &other, None);
    assert!(matches!(
        e.media.begin(&req2, &ctx(T0)),
        Err(MediaErr::Malformed(_))
    ));
    assert_eq!(e.fetch(&url), Some(data));
}

#[test]
fn pending_upload_can_be_replaced_by_same_key() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let a = bytes(10, 1);
    let b = bytes(PAGE as usize + 1, 2);
    let first = e
        .media
        .begin(&request(&d, preview(rid(1)), &a, None), &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let url = e
        .upload(&request(&d, preview(rid(1)), &b, None), &b, T0)
        .unwrap_or_default();
    assert_eq!(e.fetch(&url), Some(b));
    assert_eq!(e.media.commit(first.upload, T0), Err(MediaErr::NotFound));
}

#[test]
fn daily_preview_quota() {
    let mut e = env();
    let d = device(1);
    let records: Vec<RecordId> = (0..=PREVIEWS_PER_DAY as u8).map(rid).collect();
    e.allow(&d, &records);
    let data = bytes(10, 1);
    for r in &records[..PREVIEWS_PER_DAY as usize] {
        assert!(
            e.media
                .begin(&request(&d, preview(*r), &data, None), &ctx(T0))
                .is_ok()
        );
    }
    let last = request(&d, preview(records[PREVIEWS_PER_DAY as usize]), &data, None);
    assert_eq!(e.media.begin(&last, &ctx(T0)), Err(MediaErr::QuotaExceeded));
    let og = request(&d, UploadTarget::Og(records[0].to_string()), &data, None);
    assert!(e.media.begin(&og, &ctx(T0)).is_ok());
    assert!(e.media.begin(&last, &ctx(T0 + DAY_NS)).is_ok());
    assert_eq!(e.media.quota_entries(), 3);
    e.media.expire(T0 + DAY_NS, EXPIRY_BATCH);
    assert_eq!(e.media.quota_entries(), 1);
}

#[test]
fn quota_is_per_key() {
    let mut e = env();
    let (a, b) = (device(1), device(2));
    let ra: Vec<RecordId> = (0..PREVIEWS_PER_DAY as u8).map(rid).collect();
    e.allow(&a, &ra);
    e.allow(&b, &[rid(200)]);
    let data = bytes(10, 1);
    for r in &ra {
        assert!(
            e.media
                .begin(&request(&a, preview(*r), &data, None), &ctx(T0))
                .is_ok()
        );
    }
    assert!(
        e.media
            .begin(&request(&b, preview(rid(200)), &data, None), &ctx(T0))
            .is_ok()
    );
}

#[test]
fn case_upload_flow_and_limits() {
    let mut e = env();
    let (d, other) = (device(1), device(2));
    e.allow(&d, &[]);
    e.allow(&other, &[]);
    let c = cid(0, 1);
    let first = bytes(MAX_CHUNK_BYTES + 7, 1);
    let second = bytes(100, 2);
    let u0 = e
        .upload(&request(&d, case_chunk(c, 0, 2), &first, None), &first, T0)
        .unwrap_or_default();
    assert_eq!(u0, format!("/c/{c}.0"));
    let mismatch = request(&d, case_chunk(c, 1, 3), &second, None);
    assert_eq!(
        e.media.begin(&mismatch, &ctx(T0)),
        Err(MediaErr::Unauthorized)
    );
    let stranger = request(&other, case_chunk(c, 1, 2), &second, None);
    assert_eq!(
        e.media.begin(&stranger, &ctx(T0)),
        Err(MediaErr::Unauthorized)
    );
    let u1 = e
        .upload(
            &request(&d, case_chunk(c, 1, 2), &second, None),
            &second,
            T0,
        )
        .unwrap_or_default();
    assert_eq!(e.fetch(&u0), Some(first.clone()));
    assert_eq!(e.fetch(&u1), Some(second.clone()));
    let meta = e.media.case_of(&c).unwrap_or_default();
    assert_eq!(
        (meta.live, meta.reserved, meta.count),
        (2, (first.len() + second.len()) as u64, 2)
    );
    assert_eq!(meta.expires_ns, T0 + CASE_TTL_NS);
}

#[test]
fn case_wrong_shard_and_size_cap() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    let req = request(&d, case_chunk(cid(1, 1), 0, 1), b"x", None);
    assert!(matches!(
        e.media.begin(&req, &ctx(T0)),
        Err(MediaErr::Malformed(_))
    ));
    let c = cid(0, 2);
    let mut a = request(&d, case_chunk(c, 0, 2), b"x", None);
    a.size = MAX_CASE_BYTES - 10;
    let a = BeginUpload {
        signature: vec![],
        ..a
    };
    let a = resign(&d, a);
    assert!(e.media.begin(&a, &ctx(T0)).is_ok());
    let mut b = request(&d, case_chunk(c, 1, 2), b"x", None);
    b.size = 11;
    let b = resign(&d, b);
    assert_eq!(e.media.begin(&b, &ctx(T0)), Err(MediaErr::QuotaExceeded));
    let mut b = request(&d, case_chunk(c, 1, 2), b"x", None);
    b.size = 10;
    assert!(e.media.begin(&resign(&d, b), &ctx(T0)).is_ok());
}

fn sized(d: &Device, target: UploadTarget, size: u64, g: Option<EntitlementGrant>) -> BeginUpload {
    let mut req = request(d, target, b"x", g);
    req.size = size;
    resign(d, req)
}

#[test]
fn free_cases_are_capped_at_sixteen_chunks_and_sixteen_mib() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    let over = sized(&d, case_chunk(cid(0, 1), 0, MAX_CASE_CHUNKS + 1), 1, None);
    assert_eq!(e.media.begin(&over, &ctx(T0)), Err(MediaErr::TooLarge));
    let c = cid(0, 2);
    for ix in 0..MAX_CASE_CHUNKS {
        let req = sized(
            &d,
            case_chunk(c, ix, MAX_CASE_CHUNKS),
            MAX_CHUNK_BYTES as u64,
            None,
        );
        assert!(e.media.begin(&req, &ctx(T0)).is_ok(), "{ix}");
    }
    let info = e
        .media
        .case_info(&c.to_string())
        .unwrap_or_else(|| panic!("info"));
    assert_eq!(info.bytes, MAX_CASE_BYTES);
    let c = cid(0, 3);
    let big = sized(&d, case_chunk(c, 0, 2), MAX_CASE_BYTES, None);
    assert!(e.media.begin(&big, &ctx(T0)).is_ok());
    let more = sized(&d, case_chunk(c, 1, 2), 1, None);
    assert_eq!(e.media.begin(&more, &ctx(T0)), Err(MediaErr::QuotaExceeded));
}

#[test]
fn plus_cases_reach_256_chunks_and_charge_the_grant() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    let g = grant(T0 + DAY_NS, MAX_PLUS_CASE_BYTES);
    let over = sized(&d, case_chunk(cid(0, 1), 0, MAX_PLUS_CASE_CHUNKS), 1, None);
    assert_eq!(e.media.begin(&over, &ctx(T0)), Err(MediaErr::TooLarge));
    let mut beyond = request(&d, case_chunk(cid(0, 1), 0, 1), b"x", Some(g.clone()));
    beyond.target = case_chunk(cid(0, 1), 0, MAX_PLUS_CASE_CHUNKS + 1);
    assert!(matches!(
        e.media.begin(&beyond, &ctx(T0)),
        Err(MediaErr::Malformed(_))
    ));
    let c = cid(0, 2);
    let first = sized(
        &d,
        case_chunk(c, 0, MAX_PLUS_CASE_CHUNKS),
        MAX_CASE_BYTES,
        Some(g.clone()),
    );
    assert!(e.media.begin(&first, &ctx(T0)).is_ok());
    assert_eq!(e.usage_used(), MAX_CASE_BYTES);
    let free_chunk = sized(&d, case_chunk(c, 1, MAX_PLUS_CASE_CHUNKS), 1, None);
    assert_eq!(
        e.media.begin(&free_chunk, &ctx(T0)),
        Err(MediaErr::TooLarge)
    );
    let rest = MAX_PLUS_CASE_BYTES - MAX_CASE_BYTES;
    let mut ix = 1;
    let mut left = rest;
    while left > 0 {
        let req = sized(
            &d,
            case_chunk(c, ix, MAX_PLUS_CASE_CHUNKS),
            MAX_CASE_BYTES,
            Some(g.clone()),
        );
        assert!(e.media.begin(&req, &ctx(T0)).is_ok(), "{ix}");
        left -= MAX_CASE_BYTES;
        ix += 1;
    }
    assert_eq!(e.usage_used(), MAX_PLUS_CASE_BYTES);
    let one_more = sized(
        &d,
        case_chunk(c, ix, MAX_PLUS_CASE_CHUNKS),
        1,
        Some(grant(T0 + DAY_NS, 1 << 40)),
    );
    assert_eq!(
        e.media.begin(&one_more, &ctx(T0)),
        Err(MediaErr::QuotaExceeded)
    );
    let info = e
        .media
        .case_info(&c.to_string())
        .unwrap_or_else(|| panic!("info"));
    assert_eq!(
        info,
        CaseInfo {
            expires_ns: T0 + CASE_TTL_NS,
            bytes: MAX_PLUS_CASE_BYTES,
            chunks: MAX_PLUS_CASE_CHUNKS,
        }
    );
}

#[test]
fn plus_case_storage_is_bounded_by_grant_quota() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    let quota = 3 * MAX_CHUNK_BYTES as u64;
    let g = grant(T0 + DAY_NS, quota);
    let c = cid(0, 4);
    for ix in 0..3 {
        let req = sized(
            &d,
            case_chunk(c, ix, 40),
            MAX_CHUNK_BYTES as u64,
            Some(g.clone()),
        );
        assert!(e.media.begin(&req, &ctx(T0)).is_ok(), "{ix}");
    }
    let req = sized(&d, case_chunk(c, 3, 40), 1, Some(g.clone()));
    assert_eq!(e.media.begin(&req, &ctx(T0)), Err(MediaErr::QuotaExceeded));
    assert_eq!(e.usage_used(), quota);
    let expired = sized(&d, case_chunk(cid(0, 5), 0, 40), 1, Some(grant(T0, quota)));
    assert_eq!(
        e.media.begin(&expired, &ctx(T0)),
        Err(MediaErr::GrantInvalid)
    );
    let mut forged = grant(T0 + DAY_NS, quota);
    forged.quota_bytes += 1;
    let forged = sized(&d, case_chunk(cid(0, 5), 0, 40), 1, Some(forged));
    assert_eq!(
        e.media.begin(&forged, &ctx(T0)),
        Err(MediaErr::GrantInvalid)
    );
    let theirs = sized(&d, case_chunk(cid(0, 5), 0, 40), 1, Some(g));
    assert_eq!(
        e.media
            .begin(&theirs, &as_caller(T0, Principal::from_slice(&[4; 29]))),
        Err(MediaErr::Unauthorized)
    );
    let no_signer = Ctx {
        signer: None,
        ..ctx(T0)
    };
    let req = sized(
        &d,
        case_chunk(cid(0, 6), 0, 40),
        1,
        Some(grant(T0 + DAY_NS, quota)),
    );
    assert_eq!(e.media.begin(&req, &no_signer), Err(MediaErr::GrantInvalid));
}

#[test]
fn free_chunks_cannot_ride_on_a_plus_case_and_grants_do_not_transfer() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    let g = grant(T0 + DAY_NS, MAX_PLUS_CASE_BYTES);
    let c = cid(0, 7);
    let first = sized(
        &d,
        case_chunk(c, 0, MAX_CASE_CHUNKS),
        MAX_CASE_BYTES,
        Some(g.clone()),
    );
    assert!(e.media.begin(&first, &ctx(T0)).is_ok());
    let free = sized(&d, case_chunk(c, 1, MAX_CASE_CHUNKS), 1, None);
    assert_eq!(e.media.begin(&free, &ctx(T0)), Err(MediaErr::QuotaExceeded));
    let regrown = sized(
        &d,
        case_chunk(c, 1, MAX_PLUS_CASE_CHUNKS),
        1,
        Some(g.clone()),
    );
    assert_eq!(
        e.media.begin(&regrown, &ctx(T0)),
        Err(MediaErr::Unauthorized)
    );
    let replayed = sized(&d, case_chunk(c, 1, MAX_CASE_CHUNKS), 1, Some(g.clone()));
    for thief in [Principal::from_slice(&[4; 29]), Principal::anonymous()] {
        assert_eq!(
            e.media.begin(&replayed, &as_caller(T0, thief)),
            Err(MediaErr::Unauthorized)
        );
    }
    let mut inflated = g.clone();
    inflated.quota_bytes = u64::MAX;
    let inflated = sized(&d, case_chunk(c, 1, MAX_CASE_CHUNKS), 1, Some(inflated));
    assert_eq!(
        e.media.begin(&inflated, &ctx(T0)),
        Err(MediaErr::GrantInvalid)
    );
    let huge = grant(T0 + DAY_NS, u64::MAX);
    let wrap = sized(
        &d,
        case_chunk(cid(0, 8), 0, 2),
        u64::MAX,
        Some(huge.clone()),
    );
    assert_eq!(e.media.begin(&wrap, &ctx(T0)), Err(MediaErr::TooLarge));
    let past = sized(
        &d,
        case_chunk(cid(0, 8), 0, 2),
        MAX_CASE_BYTES + 1,
        Some(huge),
    );
    assert_eq!(e.media.begin(&past, &ctx(T0)), Err(MediaErr::TooLarge));
    assert_eq!(e.usage_used(), MAX_CASE_BYTES);
    assert_eq!(
        e.media.case_info(&c.to_string()).map(|i| i.bytes),
        Some(MAX_CASE_BYTES)
    );
}

#[test]
fn plus_case_chunks_expire_with_the_case_and_release_usage() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    let c = cid(0, 7);
    let data = bytes(100, 7);
    let g = grant(T0 + 365 * DAY_NS, 1 << 30);
    let url = e
        .upload(
            &request(&d, case_chunk(c, 0, 20), &data, Some(g)),
            &data,
            T0,
        )
        .unwrap_or_default();
    assert_eq!(e.usage_used(), 100);
    assert_eq!(e.media.expire(T0 + CASE_TTL_NS - 1, EXPIRY_BATCH), 0);
    assert_eq!(e.media.expire(T0 + CASE_TTL_NS, EXPIRY_BATCH), 1);
    assert!(e.fetch(&url).is_none());
    assert_eq!(e.usage_used(), 0);
    assert_eq!(e.media.case_info(&c.to_string()), None);
}

#[test]
fn case_info_reports_expiry_size_and_chunks() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    let c = cid(0, 8);
    assert_eq!(e.media.case_info(&c.to_string()), None);
    assert_eq!(e.media.case_info("not a case"), None);
    assert_eq!(e.media.case_info(""), None);
    let data = bytes(500, 8);
    e.upload(&request(&d, case_chunk(c, 0, 3), &data, None), &data, T0)
        .unwrap_or_else(|e| panic!("{e:?}"));
    let want = CaseInfo {
        expires_ns: T0 + CASE_TTL_NS,
        bytes: 500,
        chunks: 3,
    };
    assert_eq!(e.media.case_info(&c.to_string()), Some(want));
    assert_eq!(e.media.case_info(&c.to_string().to_lowercase()), Some(want));
    e.media.expire(T0 + CASE_TTL_NS, EXPIRY_BATCH);
    assert_eq!(e.media.case_info(&c.to_string()), None);
}

fn resign(d: &Device, mut req: BeginUpload) -> BeginUpload {
    let msg = req.signing_bytes().unwrap_or_default();
    let sig: Signature = d.sk.sign(&msg);
    req.signature = sig.to_bytes().to_vec();
    req
}

#[test]
fn daily_case_quota_counts_cases_not_chunks() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    for n in 0..CASES_PER_DAY as u8 {
        for ix in 0..3 {
            let req = request(&d, case_chunk(cid(0, n), ix, 3), b"abc", None);
            assert!(e.media.begin(&req, &ctx(T0)).is_ok(), "{n} {ix}");
        }
    }
    let req = request(&d, case_chunk(cid(0, 250), 0, 1), b"abc", None);
    assert_eq!(e.media.begin(&req, &ctx(T0)), Err(MediaErr::QuotaExceeded));
}

#[test]
fn cases_expire_and_free_pages() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    let c = cid(0, 3);
    let data = bytes(3 * PAGE as usize, 4);
    let url = e
        .upload(&request(&d, case_chunk(c, 0, 1), &data, None), &data, T0)
        .unwrap_or_default();
    let root = e.media.root_hash();
    assert_eq!(e.media.expire(T0 + CASE_TTL_NS - 1, EXPIRY_BATCH), 0);
    assert!(e.fetch(&url).is_some());
    assert_eq!(e.media.expire(T0 + CASE_TTL_NS, EXPIRY_BATCH), 1);
    assert!(e.fetch(&url).is_none());
    assert!(e.media.case_of(&c).is_none());
    assert_ne!(e.media.root_hash(), root);
    assert_eq!(e.media.root_hash(), env().media.root_hash());
    let m = e.media.metrics();
    let get = |k: &str| m.iter().find(|(n, _)| n == k).map(|(_, v)| *v);
    assert_eq!(get("pages_free"), get("pages_total"));
    assert_eq!(get("blobs"), Some(0));
}

#[test]
fn abandoned_uploads_expire() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    let c = cid(0, 3);
    let req = request(&d, case_chunk(c, 0, 1), &bytes(10, 1), None);
    let b = e
        .media
        .begin(&req, &ctx(T0))
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(e.media.expire(T0 + PENDING_TTL_NS - 1, EXPIRY_BATCH), 0);
    assert_eq!(e.media.expire(T0 + PENDING_TTL_NS, EXPIRY_BATCH), 1);
    assert_eq!(e.media.commit(b.upload, T0), Err(MediaErr::NotFound));
    assert!(e.media.case_of(&c).is_none());
}

#[test]
fn expire_respects_budget() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[]);
    for n in 0..5 {
        let req = request(&d, case_chunk(cid(0, n), 0, 1), b"a", None);
        assert!(e.media.begin(&req, &ctx(T0)).is_ok());
    }
    let later = T0 + PENDING_TTL_NS;
    assert_eq!(e.media.expire(later, 2), 2);
    assert_eq!(e.media.expire(later, 2), 2);
    assert_eq!(e.media.expire(later, 2), 1);
    assert_eq!(e.media.expire(later, 2), 0);
}

#[test]
fn plus_grant_checks() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(10, 1);
    let full = UploadTarget::Full(rid(1).to_string());
    let req = |g| request(&d, full.clone(), &data, g);
    assert_eq!(
        e.media.begin(&req(None), &ctx(T0)),
        Err(MediaErr::GrantInvalid)
    );
    let mut no_signer = ctx(T0);
    no_signer.signer = None;
    assert_eq!(
        e.media.begin(&req(Some(grant(T0 + 1, 100))), &no_signer),
        Err(MediaErr::GrantInvalid)
    );
    assert_eq!(
        e.media.begin(&req(Some(grant(T0, 100))), &ctx(T0)),
        Err(MediaErr::GrantInvalid)
    );
    let mut forged = grant(T0 + 1, 100);
    forged.quota_bytes = 1 << 40;
    assert_eq!(
        e.media.begin(&req(Some(forged)), &ctx(T0)),
        Err(MediaErr::GrantInvalid)
    );
    let mut wrong = ctx(T0);
    wrong.caller = Principal::anonymous();
    assert_eq!(
        e.media.begin(&req(Some(grant(T0 + 1, 100))), &wrong),
        Err(MediaErr::Unauthorized)
    );
    assert_eq!(
        e.media.begin(&req(Some(grant(T0 + 1, 9))), &ctx(T0)),
        Err(MediaErr::QuotaExceeded)
    );
    assert!(
        e.media
            .begin(&req(Some(grant(T0 + 1, 10))), &ctx(T0))
            .is_ok()
    );
}

#[test]
fn plus_quota_accumulates_and_releases() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1), rid(2)]);
    let g = grant(T0 + DAY_NS, 15);
    let a = bytes(10, 1);
    e.upload(
        &request(
            &d,
            UploadTarget::Full(rid(1).to_string()),
            &a,
            Some(g.clone()),
        ),
        &a,
        T0,
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    let b = bytes(10, 2);
    let req = request(&d, UploadTarget::Full(rid(2).to_string()), &b, Some(g));
    assert_eq!(e.media.begin(&req, &ctx(T0)), Err(MediaErr::QuotaExceeded));
    assert_eq!(e.media.erase_record(&rid(1).to_string()), Ok(1));
    assert_eq!(e.usage_used(), 0);
    assert!(e.media.begin(&req, &ctx(T0)).is_ok());
}

#[test]
fn plus_media_survives_grace_and_renewal() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1), rid(2)]);
    let expires = T0 + DAY_NS;
    let a = bytes(10, 1);
    let url = e
        .upload(
            &request(
                &d,
                UploadTarget::Full(rid(1).to_string()),
                &a,
                Some(grant(expires, 1 << 20)),
            ),
            &a,
            T0,
        )
        .unwrap_or_default();
    assert_eq!(e.media.expire(expires + PLUS_GRACE_NS - 1, EXPIRY_BATCH), 0);
    assert!(e.fetch(&url).is_some());
    let renewed = expires + 10 * DAY_NS;
    let b = bytes(10, 2);
    e.upload(
        &request(
            &d,
            UploadTarget::Full(rid(2).to_string()),
            &b,
            Some(grant(renewed, 1 << 20)),
        ),
        &b,
        T0 + 2 * DAY_NS,
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(e.media.expire(expires + PLUS_GRACE_NS, EXPIRY_BATCH), 0);
    assert!(e.fetch(&url).is_some());
    assert_eq!(e.media.expire(renewed + PLUS_GRACE_NS, EXPIRY_BATCH), 2);
    assert!(e.fetch(&url).is_none());
    assert_eq!(e.usage_used(), 0);
}

#[test]
fn erase_removes_record_media_and_allowlist() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1), rid(2)]);
    let data = bytes(10, 1);
    let p = e
        .upload(&request(&d, preview(rid(1)), &data, None), &data, T0)
        .unwrap_or_default();
    let o = e
        .upload(
            &request(&d, UploadTarget::Og(rid(1).to_string()), &data, None),
            &data,
            T0,
        )
        .unwrap_or_default();
    let keep = e
        .upload(&request(&d, preview(rid(2)), &data, None), &data, T0)
        .unwrap_or_default();
    e.media.count_checks(&[rid(1).to_string()], T0);
    assert_eq!(e.media.erase_record(&rid(1).to_string()), Ok(2));
    assert!(e.fetch(&p).is_none() && e.fetch(&o).is_none());
    assert!(e.fetch(&keep).is_some());
    assert_eq!(e.media.checks(&[rid(1).to_string()]), vec![0]);
    let again = request(&d, preview(rid(1)), &data, None);
    assert_eq!(
        e.media.begin(&again, &ctx(T0)),
        Err(MediaErr::RetryAfterNs(SYNC_RETRY_NS))
    );
    assert_eq!(e.media.erase_record(&rid(1).to_string()), Ok(0));
    assert!(matches!(
        e.media.erase_record("nope"),
        Err(MediaErr::Malformed(_))
    ));
}

#[test]
fn sync_validates_entries() {
    let mut e = env();
    let d = device(1);
    let other = device(2);
    let bad = [
        Allow::Key {
            key_id: ByteBuf::from(d.key_id[..15].to_vec()),
            pubkey: ByteBuf::from(d.spki.clone()),
        },
        Allow::Key {
            key_id: ByteBuf::from(d.key_id.to_vec()),
            pubkey: ByteBuf::from(other.spki.clone()),
        },
        Allow::Key {
            key_id: ByteBuf::from(KeyId::from_spki(b"junk").to_bytes().to_vec()),
            pubkey: ByteBuf::from(b"junk".to_vec()),
        },
        Allow::Record {
            record_id: "bad".into(),
            key_id: ByteBuf::from(d.key_id.to_vec()),
        },
        Allow::Record {
            record_id: rid(1).to_string(),
            key_id: ByteBuf::from(vec![1; 3]),
        },
        Allow::Revoke(ByteBuf::from(vec![1; 17])),
    ];
    assert_eq!(e.media.sync(&bad), 0);
    let huge: Vec<Allow> = (0..MAX_SYNC_BATCH + 5)
        .map(|_| Allow::Revoke(ByteBuf::from(vec![0; 16])))
        .collect();
    assert_eq!(e.media.sync(&huge), MAX_SYNC_BATCH as u32);
}

#[test]
fn revoked_keys_cannot_upload() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    assert_eq!(
        e.media
            .sync(&[Allow::Revoke(ByteBuf::from(d.key_id.to_vec()))]),
        1
    );
    let req = request(&d, preview(rid(1)), b"x", None);
    assert_eq!(
        e.media.begin(&req, &ctx(T0)),
        Err(MediaErr::RetryAfterNs(SYNC_RETRY_NS))
    );
}

#[test]
fn check_counter_is_batched_and_budgeted() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1), rid(2)]);
    let ids = vec![
        rid(1).to_string(),
        rid(1).to_string(),
        rid(2).to_string(),
        rid(3).to_string(),
        "junk".into(),
    ];
    assert_eq!(e.media.count_checks(&ids, T0), 2);
    assert_eq!(e.media.count_checks(&ids, T0), 2);
    assert_eq!(e.media.checks(&ids), vec![2, 2, 2, 0, 0]);
    let many: Vec<String> = (0..MAX_CHECK_BATCH + 10).map(|_| "x".into()).collect();
    assert_eq!(e.media.checks(&many).len(), MAX_CHECK_BATCH);
    let mut st = *e.media.state.get();
    st.check_window = T0 / EXPIRY_PERIOD_NS;
    st.check_used = CHECK_BUDGET;
    e.media.state.set(st);
    assert_eq!(e.media.count_checks(&ids, T0), 0);
    assert_eq!(e.media.count_checks(&ids, T0 + EXPIRY_PERIOD_NS), 2);
}

#[test]
fn check_batch_is_capped() {
    let mut e = env();
    let d = device(1);
    let records: Vec<RecordId> = (0..40).map(rid).collect();
    e.allow(&d, &records);
    let ids: Vec<String> = records.iter().map(|r| r.to_string()).collect();
    assert_eq!(e.media.count_checks(&ids, T0), MAX_CHECK_BATCH as u32);
}

#[test]
fn read_bounds() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(3 * PAGE as usize + 9, 5);
    let url = e
        .upload(
            &request(
                &d,
                UploadTarget::Full(rid(1).to_string()),
                &data,
                Some(grant(T0 + 1, 1 << 30)),
            ),
            &data,
            T0,
        )
        .unwrap_or_default();
    let s = e.media.lookup(&url).unwrap_or_else(|| panic!("missing"));
    assert_eq!(e.media.blob_size(s.blob), Some(data.len() as u64));
    assert_eq!(
        e.media.read(s.blob, PAGE - 2, 4),
        Some(data[PAGE as usize - 2..PAGE as usize + 2].to_vec())
    );
    assert_eq!(e.media.read(s.blob, data.len() as u64, 0), Some(vec![]));
    assert_eq!(e.media.read(s.blob, data.len() as u64 - 1, 2), None);
    assert_eq!(e.media.read(s.blob, u64::MAX, 2), None);
    assert_eq!(e.media.read(999, 0, 1), None);
    assert!(e.media.lookup("/p/garbage").is_none());
    assert!(e.media.lookup(&format!("/o/{}", rid(1))).is_none());
}

#[test]
fn state_survives_reinit_without_rebuild() {
    let mut e = env();
    let d = device(1);
    e.allow(&d, &[rid(1)]);
    let data = bytes(PAGE as usize * 2, 7);
    let url = e
        .upload(
            &request(&d, preview(rid(1)), &data[..1000], None),
            &data[..1000],
            T0,
        )
        .unwrap_or_default();
    let root = e.media.root_hash();
    let reloaded: Media<Mem> = Media::init(|id| e.mm.get(id));
    assert_eq!(reloaded.root_hash(), root);
    let s = reloaded.lookup(&url).unwrap_or_else(|| panic!("missing"));
    assert_eq!(
        reloaded.read(s.blob, 0, s.size),
        Some(data[..1000].to_vec())
    );
}

#[test]
fn identical_operations_are_deterministic() {
    let run = || {
        let mut e = env();
        let d = device(1);
        e.allow(&d, &[rid(1), rid(2)]);
        for (r, seed) in [(rid(1), 1), (rid(2), 2)] {
            let data = bytes(100, seed);
            e.upload(&request(&d, preview(r), &data, None), &data, T0)
                .unwrap_or_default();
        }
        let c = cid(0, 9);
        let data = bytes(10, 3);
        e.upload(&request(&d, case_chunk(c, 0, 1), &data, None), &data, T0)
            .unwrap_or_default();
        (e.media.root_hash(), e.media.metrics())
    };
    assert_eq!(run(), run());
}

#[test]
fn metrics_are_named() {
    let e = env();
    let names: Vec<String> = e.media.metrics().into_iter().map(|(k, _)| k).collect();
    for n in [
        "blobs",
        "pages_total",
        "pages_free",
        "cert_nodes",
        "keys",
        "records",
        "cases",
        "expiry_queue",
        "plus_accounts",
    ] {
        assert!(names.iter().any(|x| x == n), "{n}");
    }
}
