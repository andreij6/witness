use super::*;
use crate::ca::{NS_PER_S, finish, finish_root, root_to_sign};
use crate::config::{ENROLL_FREE, POLICY};
use crate::store::with;
use crate::types::LateOpening;
use ed25519_dalek::Signer as _;
use serde_bytes::ByteBuf;
use witness_attest::roots::AOSP_SOFT_EC;
use witness_attest::{issue_fresh, late_challenge};
use witness_c2pa::x509::verify_chain;
use witness_format::{Freshness, SecurityLevel};

#[path = "../../tests/support/attest.rs"]
pub(crate) mod attest;

pub const NOW_S: i64 = 1_763_510_400;
pub const NOW: u64 = NOW_S as u64 * NS_PER_S;
pub const SECRET: [u8; 32] = [5; 32];
const DIGESTS: &[[u8; 32]] = &[attest::DEBUG_DIGEST];
const ANCHORS: &[witness_attest::TrustAnchor<'static>] = &[AOSP_SOFT_EC];

type Tamper = Box<dyn Fn(&mut attest::Attestation)>;

pub fn policy() -> Policy<'static> {
    Policy {
        anchors: ANCHORS,
        release_digests: DIGESTS,
        ..POLICY
    }
}

pub fn ca_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[9; 32])
}

pub fn ready(st: &mut Stores) {
    let k = ca_key();
    let pk = k.verifying_key().to_bytes();
    let tbs = root_to_sign(NOW - 86_400 * NS_PER_S, &pk).unwrap();
    let root = finish_root(&tbs, &k.sign(&tbs).to_bytes(), &pk).unwrap();
    st.update_state(|s| {
        s.secret = Some(ByteBuf::from(SECRET.to_vec()));
        s.ca_root = Some(ByteBuf::from(root));
    });
}

pub fn request(seed: u64) -> (Enrollment, p256::ecdsa::SigningKey) {
    let device = attest::device_key(seed);
    let a = attest::Attestation::new(&issue_fresh(&SECRET, NOW - 10 * NS_PER_S), NOW_S);
    let req = Enrollment {
        chain: a.chain(&device).into_iter().map(ByteBuf::from).collect(),
        key_id: None,
        late: None,
    };
    (req, device)
}

pub fn enroll(st: &mut Stores, seed: u64) -> (KeyInfo, Vec<u8>) {
    let (req, _) = request(seed);
    let t = begin(st, &req, &policy(), NOW).unwrap();
    let leaf = finish(&t.tbs, &ca_key().sign(&t.tbs).to_bytes()).unwrap();
    commit(st, &t);
    (t.info, leaf)
}

#[test]
fn happy_path_issues_c2pa_leaf_and_certifies_key() {
    with(|st| {
        ready(st);
        let before = st.roots();
        let (req, device) = request(1);
        let t = begin(st, &req, &policy(), NOW).unwrap();
        assert_eq!(t.info.pubkey, attest::spki(&device));
        assert_eq!(t.info.provisioning, Provisioning::Dev);
        assert_eq!(t.info.security, SecurityLevel::Dev);
        assert_eq!(t.info.freshness, Freshness::Fresh);
        assert_eq!(t.info.enrolled_ns, NOW);
        assert_eq!(t.info.active_until_ns, NOW + POLICY.active_ns);
        assert!(st.pending.contains_key(&t.info.key_id.to_bytes()));
        assert_eq!(
            st.enroll_quota
                .get(&t.quota_key)
                .map(|b| EnrollWindow::from_bytes(&b).count),
            Some(1)
        );
        let leaf = finish(&t.tbs, &ca_key().sign(&t.tbs).to_bytes()).unwrap();
        let anchor = ca::anchor(st.state.get().ca_root().unwrap()).unwrap();
        let checked = verify_chain(&[&leaf], &anchor, Some(NOW_S as u64)).unwrap();
        assert_eq!(checked.spki_der, t.info.pubkey);
        let roots = commit(st, &t);
        assert!(st.pending.is_empty());
        assert_eq!(st.key(&t.info.key_id), Some(t.info.clone()));
        assert_ne!(roots.smt_root, before.smt_root);
        assert_eq!(roots.leaf_count, before.leaf_count);
    });
}

#[test]
fn commit_pushes_key_to_every_media_shard() {
    with(|st| {
        ready(st);
        let (info, _) = enroll(st, 1);
        assert_eq!(st.outbox.len(), 0);
        let p = |n: u8| candid::Principal::from_slice(&[n; 10]);
        st.configure(witness_canister_common::RegistryInit {
            index: vec![p(1)],
            media: vec![p(2), p(3)],
            pages: p(4),
        });
        let (info2, _) = enroll(st, 2);
        assert_ne!(info.key_id, info2.key_id);
        let out: Vec<_> = st
            .outbox
            .values()
            .map(|b| crate::outbox::Out::decode(&b).unwrap())
            .collect();
        let want = |shard| crate::outbox::Out {
            dest: crate::outbox::Dest::Media(shard),
            body: crate::outbox::Body::Sync {
                record_ix: None,
                allow: crate::types::Allow::Key {
                    key_id: ByteBuf::from(info2.key_id.as_bytes().to_vec()),
                    pubkey: ByteBuf::from(info2.pubkey.clone()),
                },
            },
        };
        assert_eq!(out, vec![want(0), want(1)]);
    });
}

#[test]
fn not_ready_without_secret_or_ca() {
    with(|st| {
        let (req, _) = request(1);
        assert!(
            begin(st, &req, &policy(), NOW)
                .unwrap_err()
                .contains("secret")
        );
        st.update_state(|s| s.secret = Some(ByteBuf::from(SECRET.to_vec())));
        assert!(begin(st, &req, &policy(), NOW).unwrap_err().contains("ca"));
    });
}

#[test]
fn rejects_bad_or_mismatched_key_id() {
    with(|st| {
        ready(st);
        let (mut req, _) = request(1);
        req.key_id = Some(ByteBuf::from(vec![1; 15]));
        assert_eq!(begin(st, &req, &policy(), NOW).unwrap_err(), "bad key_id");
        req.key_id = Some(ByteBuf::from(vec![1; 16]));
        assert!(
            begin(st, &req, &policy(), NOW)
                .unwrap_err()
                .starts_with("attestation")
        );
        let device = attest::device_key(1);
        req.key_id = Some(ByteBuf::from(
            KeyId::from_spki(&attest::spki(&device)).as_bytes().to_vec(),
        ));
        assert!(begin(st, &req, &policy(), NOW).is_ok());
    });
}

#[test]
fn rejects_bad_attestation() {
    with(|st| {
        ready(st);
        let device = attest::device_key(1);
        let cases: Vec<Tamper> = vec![
            Box::new(|a| a.digest = [0; 32]),
            Box::new(|a| a.package = b"app.other".to_vec()),
            Box::new(|a| a.patch = 202_001),
            Box::new(|a| a.challenge = issue_fresh(&[6; 32], NOW).to_vec()),
            Box::new(|a| a.challenge = issue_fresh(&SECRET, NOW - 2 * 3600 * NS_PER_S).to_vec()),
            Box::new(|a| a.challenge = issue_fresh(&SECRET, NOW + 1).to_vec()),
            Box::new(|a| a.not_before = NOW_S + 100),
        ];
        for tamper in cases {
            let mut a = attest::Attestation::new(&issue_fresh(&SECRET, NOW), NOW_S);
            tamper(&mut a);
            let req = Enrollment {
                chain: a.chain(&device).into_iter().map(ByteBuf::from).collect(),
                key_id: None,
                late: None,
            };
            assert!(
                begin(st, &req, &policy(), NOW)
                    .unwrap_err()
                    .starts_with("attestation")
            );
        }
        assert!(st.pending.is_empty());
        assert!(st.enroll_quota.is_empty());
    });
}

#[test]
fn rejects_malformed_chains() {
    with(|st| {
        ready(st);
        let (req, _) = request(1);
        let mut flipped = req.clone();
        let leaf = flipped.chain[0].as_mut_slice();
        let n = leaf.len();
        leaf[n - 5] ^= 1;
        let cases = [
            Enrollment {
                chain: vec![],
                ..req.clone()
            },
            Enrollment {
                chain: req.chain[..1].to_vec(),
                ..req.clone()
            },
            Enrollment {
                chain: vec![ByteBuf::from(vec![0x30, 0x00]); 3],
                ..req.clone()
            },
            Enrollment {
                chain: vec![req.chain[0].clone(); 9],
                ..req.clone()
            },
            flipped,
        ];
        for c in cases {
            assert!(begin(st, &c, &policy(), NOW).is_err());
        }
    });
}

#[test]
fn production_policy_rejects_dev_chain_without_feature() {
    with(|st| {
        ready(st);
        let (req, _) = request(1);
        let r = begin(st, &req, &POLICY, NOW);
        assert_eq!(
            r.is_ok(),
            cfg!(feature = "dev-attestation")
                && POLICY.release_digests.contains(&attest::DEBUG_DIGEST)
        );
    });
}

#[test]
fn rejects_duplicate_and_pending_keys() {
    with(|st| {
        ready(st);
        let (req, _) = request(1);
        let t = begin(st, &req, &policy(), NOW).unwrap();
        assert_eq!(
            begin(st, &req, &policy(), NOW).unwrap_err(),
            "enrollment pending"
        );
        commit(st, &t);
        assert_eq!(
            begin(st, &req, &policy(), NOW).unwrap_err(),
            "already enrolled"
        );
    });
}

#[test]
fn stuck_pending_enrollment_can_be_retried_after_ttl() {
    with(|st| {
        ready(st);
        let (req, _) = request(1);
        let stuck = begin(st, &req, &policy(), NOW).unwrap();
        let id = stuck.info.key_id.to_bytes();
        st.pending.insert(id, NOW - PENDING_TTL_NS + 1);
        assert_eq!(
            begin(st, &req, &policy(), NOW).unwrap_err(),
            "enrollment pending"
        );
        st.pending.insert(id, NOW + 1);
        assert_eq!(
            begin(st, &req, &policy(), NOW).unwrap_err(),
            "enrollment pending"
        );
        st.pending.insert(id, NOW - PENDING_TTL_NS);
        let retry = begin(st, &req, &policy(), NOW).unwrap();
        assert_eq!(retry.info.key_id, stuck.info.key_id);
        assert_ne!(retry.tbs, stuck.tbs);
        assert_eq!(st.pending.get(&id), Some(NOW));
        assert_eq!(
            begin(st, &req, &policy(), NOW).unwrap_err(),
            "enrollment pending"
        );
        commit(st, &retry);
        assert!(st.pending.is_empty());
        assert_eq!(
            begin(st, &req, &policy(), NOW).unwrap_err(),
            "already enrolled"
        );
    });
}

#[test]
fn pending_ttl_is_ten_minutes() {
    assert_eq!(PENDING_TTL_NS, 600 * NS_PER_S);
}

#[test]
fn rollback_releases_reservation() {
    with(|st| {
        ready(st);
        let (req, _) = request(1);
        let t = begin(st, &req, &policy(), NOW).unwrap();
        rollback(st, &t);
        assert!(st.pending.is_empty());
        assert_eq!(
            st.enroll_quota
                .get(&t.quota_key)
                .map(|b| EnrollWindow::from_bytes(&b).count),
            Some(0)
        );
        assert!(st.keys.is_empty());
        let again = begin(st, &req, &policy(), NOW).unwrap();
        assert_ne!(again.tbs, t.tbs);
        rollback(st, &again);
        st.enroll_quota.remove(&t.quota_key);
        rollback(st, &again);
        assert!(st.enroll_quota.is_empty());
    });
}

#[test]
fn dev_chains_get_progressive_backoff() {
    with(|st| {
        ready(st);
        for seed in 0..ENROLL_FREE {
            enroll(st, seed + 10);
        }
        let (req, _) = request(99);
        assert!(
            begin(st, &req, &policy(), NOW)
                .unwrap_err()
                .starts_with("quota: retry after")
        );
        let later = NOW + 3600 * NS_PER_S;
        let device = attest::device_key(99);
        let a = attest::Attestation::new(&issue_fresh(&SECRET, later - NS_PER_S), NOW_S + 3600);
        let req = Enrollment {
            chain: a.chain(&device).into_iter().map(ByteBuf::from).collect(),
            key_id: None,
            late: None,
        };
        assert!(begin(st, &req, &policy(), later).is_ok());
    });
}

#[test]
fn late_keys_are_accepted_with_opening() {
    with(|st| {
        ready(st);
        let device = attest::device_key(3);
        let nonce = [4u8; 16];
        let wall_ms = (NOW_S as u64 - 3600) * 1000;
        let a = attest::Attestation::new(&late_challenge(&nonce, wall_ms), NOW_S);
        let mut req = Enrollment {
            chain: a.chain(&device).into_iter().map(ByteBuf::from).collect(),
            key_id: None,
            late: Some(LateOpening {
                install_nonce: nonce.to_vec(),
                wall_ms,
            }),
        };
        let t = begin(st, &req, &policy(), NOW).unwrap();
        assert_eq!(t.info.freshness, Freshness::Late);
        rollback(st, &t);
        req.late = Some(LateOpening {
            install_nonce: nonce.to_vec(),
            wall_ms: wall_ms + 1,
        });
        assert!(begin(st, &req, &policy(), NOW).is_err());
        req.late = None;
        assert!(begin(st, &req, &policy(), NOW).is_err());
    });
}

#[test]
fn serials_are_unique_per_issue() {
    with(|st| {
        ready(st);
        let anchor = ca::anchor(st.state.get().ca_root().unwrap()).unwrap();
        let mut serials = Vec::new();
        for seed in 0..3 {
            let (_, leaf) = enroll(st, seed + 40);
            serials.push(verify_chain(&[&leaf], &anchor, None).unwrap().serial);
        }
        assert_eq!(serials, vec![vec![2], vec![3], vec![4]]);
    });
}
