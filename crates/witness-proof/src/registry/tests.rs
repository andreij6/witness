use super::*;
use crate::DEFAULT_MAX_AGE_NS;
use crate::testkit::{TestLedger, TestSubnet};
use witness_format::{Freshness, KeyHistory, KeyId, Provisioning, RecordId, SecurityLevel};
use witness_merkle::key_of;

const CANISTER: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 9, 1, 1];
const NOW: u64 = 1_763_510_400_000_000_000;

fn info() -> KeyInfo {
    let pubkey = vec![0x30, 0x59, 9, 9];
    KeyInfo {
        key_id: KeyId::from_spki(&pubkey),
        pubkey,
        security: SecurityLevel::StrongBox,
        provisioning: Provisioning::Factory,
        freshness: Freshness::Late,
        patch: 1,
        enrolled_ns: 5,
        active_until_ns: 6,
        history: KeyHistory::new(5),
        chain_serials: vec![],
    }
}

fn certified() -> (TestLedger, CertifiedRecord, CertifiedKey) {
    let mut l = TestLedger::new(TestSubnet::new(3), &CANISTER, NOW);
    let claim = vec![1, 2, 3];
    l.append(&claim, b"c2pa", b"cose", 42);
    l.append(b"other", b"", b"", 43);
    let info = info();
    l.insert(key_of(&info.key_id), &info.leaf_hash());
    let record = CertifiedRecord {
        record_ix: 0,
        record_id: RecordId::from_claim_cbor(&claim).to_string(),
        claim,
        c2pa_claim: b"c2pa".to_vec(),
        cose: b"cose".to_vec(),
        anchored_ns: 42,
        mmr_proof: l.record_proof(0),
        leaf_count: l.leaf_count(),
        certificate: l.certificate(),
        media_shard: 3,
    };
    let key = CertifiedKey {
        key: (&info).into(),
        smt_proof: l.smt_bundle(key_of(&info.key_id)),
        certificate: l.certificate(),
    };
    (l, record, key)
}

fn with<T>(l: &TestLedger, f: impl FnOnce(&Trust<'_>) -> T) -> T {
    let root = l.subnet.public_der();
    f(&Trust {
        root_key: &root,
        canister: &CANISTER,
        now_ns: NOW,
        max_age_ns: DEFAULT_MAX_AGE_NS,
    })
}

#[test]
fn record_parts_borrow_every_proof_field() {
    let (_, r, _) = certified();
    let p = record_parts(&r);
    assert_eq!(
        (p.record_ix, p.anchored_ns, p.leaf_count),
        (r.record_ix, r.anchored_ns, r.leaf_count)
    );
    assert_eq!(p.record_id, r.record_id);
    assert_eq!(
        (p.claim, p.c2pa_claim, p.cose),
        (&r.claim[..], &r.c2pa_claim[..], &r.cose[..])
    );
    assert_eq!(p.mmr_proof, r.mmr_proof.as_slice());
    assert_eq!(p.certificate, r.certificate.as_slice());
}

#[test]
fn certified_registry_replies_verify() {
    let (l, r, k) = certified();
    let c = with(&l, |t| t.certified_record(&r)).unwrap();
    assert_eq!(c.data, l.certified_data().to_vec());
    let (info, kc) = with(&l, |t| t.certified_key(&k)).unwrap();
    assert_eq!(info, self::info());
    assert_eq!(kc, c);
    let shard_hint = CertifiedRecord {
        media_shard: 0,
        ..r
    };
    assert!(with(&l, |t| t.certified_record(&shard_hint)).is_ok());
}

#[test]
fn tampered_registry_replies_are_rejected() {
    let (l, r, k) = certified();
    let mut claim = r.clone();
    claim.claim[0] ^= 1;
    assert_eq!(
        with(&l, |t| t.certified_record(&claim)),
        Err(Error::Malformed("record id"))
    );
    let leaf = CertifiedRecord {
        anchored_ns: 41,
        ..r.clone()
    };
    assert_eq!(
        with(&l, |t| t.certified_record(&leaf)),
        Err(Error::Proof("certified data mismatch"))
    );
    let mut proof = r.clone();
    proof.mmr_proof[0][5] ^= 1;
    assert!(with(&l, |t| t.certified_record(&proof)).is_err());
    let mut cert = r;
    cert.certificate[20] ^= 1;
    assert!(with(&l, |t| t.certified_record(&cert)).is_err());
    let mut key = k.clone();
    key.key.patch = 2;
    assert_eq!(
        with(&l, |t| t.certified_key(&key)),
        Err(Error::Proof("certified data mismatch"))
    );
    let mut bad = k;
    bad.key.key_id.pop();
    assert_eq!(key_info(&bad), Err(Error::Malformed("key")));
    assert_eq!(
        with(&l, |t| t.certified_key(&bad)),
        Err(Error::Malformed("key"))
    );
}
