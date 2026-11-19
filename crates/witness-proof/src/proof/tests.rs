use super::*;
use crate::DEFAULT_MAX_AGE_NS;
use crate::testkit::{TestLedger, TestSubnet};
use serde_bytes::ByteBuf;
use witness_format::{Freshness, KeyHistory, KeyId, Provisioning, SecurityLevel};

const CANISTER: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 7, 1, 1];
const NOW: u64 = 1_763_510_400_000_000_000;

#[derive(Clone)]
struct Rec {
    record_ix: u64,
    record_id: String,
    claim: Vec<u8>,
    c2pa_claim: Vec<u8>,
    cose: Vec<u8>,
    anchored_ns: u64,
    leaf_count: u64,
    mmr_proof: Vec<ByteBuf>,
    certificate: Vec<u8>,
}

impl Rec {
    fn parts(&self) -> RecordParts<'_, ByteBuf> {
        RecordParts {
            record_ix: self.record_ix,
            record_id: &self.record_id,
            claim: &self.claim,
            c2pa_claim: &self.c2pa_claim,
            cose: &self.cose,
            anchored_ns: self.anchored_ns,
            leaf_count: self.leaf_count,
            mmr_proof: &self.mmr_proof,
            certificate: &self.certificate,
        }
    }
}

fn key() -> KeyInfo {
    let pubkey = vec![0x30, 0x59, 1, 2, 3];
    KeyInfo {
        key_id: KeyId::from_spki(&pubkey),
        pubkey,
        security: SecurityLevel::Tee,
        provisioning: Provisioning::Rkp,
        freshness: Freshness::Fresh,
        patch: 202_511,
        enrolled_ns: NOW - 10,
        active_until_ns: NOW + 10,
        history: KeyHistory::new(NOW - 10),
        chain_serials: vec!["1f".into()],
    }
}

fn shard_lists() -> ([ShardEntry<'static>; 2], [ShardEntry<'static>; 1]) {
    (
        [
            ShardEntry {
                shard: 0,
                canister: &[1; 10],
                open: true,
            },
            ShardEntry {
                shard: 1,
                canister: &[2; 29],
                open: false,
            },
        ],
        [ShardEntry {
            shard: 0,
            canister: &[3; 10],
            open: true,
        }],
    )
}

struct World {
    ledger: TestLedger,
    records: Vec<Rec>,
    key: KeyInfo,
    map: Hash,
}

fn world(n: u64) -> World {
    let mut ledger = TestLedger::new(TestSubnet::new(11), &CANISTER, NOW);
    let mut records: Vec<Rec> = (0..n)
        .map(|i| {
            let claim = vec![0xa0 | (i as u8 & 0x0f), i as u8, 7];
            let r = Rec {
                record_ix: i,
                record_id: RecordId::from_claim_cbor(&claim).to_string(),
                c2pa_claim: vec![i as u8; 3],
                cose: vec![!(i as u8); 5],
                claim,
                anchored_ns: NOW - 1_000 + i,
                leaf_count: 0,
                mmr_proof: Vec::new(),
                certificate: Vec::new(),
            };
            ledger.append(&r.claim, &r.c2pa_claim, &r.cose, r.anchored_ns);
            r
        })
        .collect();
    let key = key();
    ledger.insert(key_of(&key.key_id), &key.leaf_hash());
    let (index, media) = shard_lists();
    let map = shard_map_hash(index, media).unwrap();
    ledger.insert(SHARDS_KEY, &map);
    let cert = ledger.certificate();
    for r in &mut records {
        r.leaf_count = n;
        r.mmr_proof = ledger.record_proof(r.record_ix);
        r.certificate = cert.clone();
    }
    World {
        ledger,
        records,
        key,
        map,
    }
}

fn trust<T>(w: &World, f: impl FnOnce(&Trust<'_>) -> T) -> T {
    let root = w.ledger.subnet.public_der();
    f(&Trust {
        root_key: &root,
        canister: &CANISTER,
        now_ns: NOW,
        max_age_ns: DEFAULT_MAX_AGE_NS,
    })
}

#[test]
fn leaf_data_binds_every_part() {
    let base = leaf_data(b"a", b"b", b"c", 7);
    assert_eq!(&base[96..], &7u64.to_be_bytes());
    assert_eq!(&base[..32], &sha256(b"a"));
    assert_ne!(base, leaf_data(b"x", b"b", b"c", 7));
    assert_ne!(base, leaf_data(b"a", b"x", b"c", 7));
    assert_ne!(base, leaf_data(b"a", b"b", b"x", 7));
    assert_ne!(base, leaf_data(b"a", b"b", b"c", 8));
}

#[test]
fn every_record_reaches_the_certified_root_for_every_size() {
    for n in 1..=9 {
        let w = world(n);
        let expected = w.ledger.certified_data();
        for r in &w.records {
            assert_eq!(r.parts().root(), Ok(expected));
            let c = trust(&w, |t| t.record(&r.parts())).unwrap();
            assert_eq!(c.data, expected.to_vec());
            assert_eq!(c.time_ns, NOW);
        }
    }
}

#[test]
fn tampered_record_fields_are_rejected() {
    let w = world(4);
    let r = &w.records[2];
    let mut cases = vec![
        Rec {
            anchored_ns: 1,
            ..r.clone()
        },
        Rec {
            record_ix: 3,
            ..r.clone()
        },
        Rec {
            leaf_count: 5,
            ..r.clone()
        },
        Rec {
            cose: vec![1],
            ..r.clone()
        },
        Rec {
            c2pa_claim: vec![1],
            ..r.clone()
        },
    ];
    let mut smt = r.clone();
    *smt.mmr_proof.last_mut().unwrap() = ByteBuf::from(vec![8; 32]);
    cases.push(smt);
    let mut sibling = r.clone();
    sibling.mmr_proof[0][0] ^= 1;
    cases.push(sibling);
    let mut cert = r.clone();
    let n = cert.certificate.len();
    cert.certificate[n - 1] ^= 1;
    cases.push(cert);
    for c in &cases {
        assert!(trust(&w, |t| t.record(&c.parts())).is_err());
    }
}

#[test]
fn no_record_verifies_at_another_index_size_or_with_a_borrowed_or_padded_proof() {
    let rejected = |w: &World, r: &Rec| trust(w, |t| t.record(&r.parts())).is_err();
    for n in 1..=9 {
        let w = world(n);
        for r in &w.records {
            for ix in (0..n + 2).filter(|&i| i != r.record_ix) {
                let moved = Rec {
                    record_ix: ix,
                    ..r.clone()
                };
                assert!(rejected(&w, &moved), "{n}: {} as {ix}", r.record_ix);
            }
            for count in (0..n + 3).filter(|&c| c != n) {
                let resized = Rec {
                    leaf_count: count,
                    ..r.clone()
                };
                assert!(rejected(&w, &resized), "{n}: {} in {count}", r.record_ix);
            }
            for other in w.records.iter().filter(|o| o.record_ix != r.record_ix) {
                let borrowed = Rec {
                    mmr_proof: other.mmr_proof.clone(),
                    ..r.clone()
                };
                assert!(rejected(&w, &borrowed));
                let relabelled = Rec {
                    record_ix: other.record_ix,
                    mmr_proof: other.mmr_proof.clone(),
                    ..r.clone()
                };
                assert!(rejected(&w, &relabelled));
            }
            let last = r.mmr_proof.len() - 1;
            for at in [0, last] {
                let mut padded = r.clone();
                padded.mmr_proof.insert(at, ByteBuf::from(vec![0; 32]));
                assert!(rejected(&w, &padded), "{n}: pad at {at}");
            }
            if last > 0 {
                let mut short = r.clone();
                short.mmr_proof.remove(0);
                assert!(rejected(&w, &short));
            }
        }
    }
}

#[test]
fn claims_must_match_their_record_id() {
    let w = world(3);
    let r = &w.records[2];
    let other_id = Rec {
        record_id: w.records[1].record_id.clone(),
        ..r.clone()
    };
    assert_eq!(
        trust(&w, |t| t.record(&other_id.parts())),
        Err(Error::Malformed("record id"))
    );
    let mut flipped = r.clone();
    flipped.claim[0] ^= 1;
    assert_eq!(
        trust(&w, |t| t.record(&flipped.parts())),
        Err(Error::Malformed("record id"))
    );
    flipped.record_id = RecordId::from_claim_cbor(&flipped.claim).to_string();
    assert_eq!(
        trust(&w, |t| t.record(&flipped.parts())),
        Err(Error::Proof("certified data mismatch"))
    );
    let garbage = Rec {
        record_id: "not base32!".into(),
        ..r.clone()
    };
    assert_eq!(
        garbage.parts().record_id(),
        Err(Error::Malformed("record id"))
    );
}

#[test]
fn malformed_record_proofs_fail_cleanly() {
    let w = world(2);
    let mut empty = w.records[0].clone();
    empty.mmr_proof.clear();
    assert_eq!(empty.parts().root(), Err(Error::Proof("record")));
    let mut short = w.records[0].clone();
    short.mmr_proof[0].pop();
    assert_eq!(short.parts().root(), Err(Error::Malformed("record proof")));
    let mut short_smt = w.records[0].clone();
    short_smt.mmr_proof.last_mut().unwrap().push(0);
    assert_eq!(
        short_smt.parts().root(),
        Err(Error::Malformed("record proof"))
    );
    let mut hostile = w.records[0].clone();
    hostile.record_ix = u64::MAX;
    hostile.leaf_count = u64::MAX;
    hostile.mmr_proof = vec![ByteBuf::from(vec![0; 32]); 70];
    assert!(trust(&w, |t| t.record(&hostile.parts())).is_err());
    let mut beyond = w.records[1].clone();
    beyond.record_ix = 2;
    assert!(beyond.parts().root().is_err());
}

#[test]
fn key_proof_reaches_the_certified_root() {
    let w = world(3);
    let bundle = w.ledger.smt_bundle(key_of(&w.key.key_id));
    assert_eq!(key_root(&w.key, &bundle), Ok(w.ledger.certified_data()));
    let cert = w.ledger.certificate();
    assert!(trust(&w, |t| t.key(&w.key, &bundle, &cert)).is_ok());
}

#[test]
fn tampered_keys_are_rejected() {
    let w = world(1);
    let bundle = w.ledger.smt_bundle(key_of(&w.key.key_id));
    let cert = w.ledger.certificate();
    let mut patch = w.key.clone();
    patch.patch += 1;
    assert_eq!(
        trust(&w, |t| t.key(&patch, &bundle, &cert)),
        Err(Error::Proof("certified data mismatch"))
    );
    let mut serials = w.key.clone();
    serials.chain_serials.clear();
    assert!(trust(&w, |t| t.key(&serials, &bundle, &cert)).is_err());
    let mut other = w.key.clone();
    other.key_id = KeyId::from_spki(b"other");
    assert!(trust(&w, |t| t.key(&other, &bundle, &cert)).is_err());
    let mut cut = bundle.clone();
    cut.pop();
    assert!(trust(&w, |t| t.key(&w.key, &cut, &cert)).is_err());
    assert_eq!(
        key_root(&w.key, &bundle[..10]),
        Err(Error::Proof("smt bundle"))
    );
    assert_eq!(key_root(&w.key, &[]), Err(Error::Proof("smt bundle")));
    let mut count = bundle.clone();
    let n = count.len();
    count[n - 1] ^= 1;
    assert!(trust(&w, |t| t.key(&w.key, &count, &cert)).is_err());
    let mut mmr = bundle;
    mmr[n - ROOTS_LEN] ^= 1;
    assert!(trust(&w, |t| t.key(&w.key, &mmr, &cert)).is_err());
}

#[test]
fn shard_map_proof_reaches_the_certified_root() {
    let w = world(2);
    let bundle = w.ledger.shards_bundle(SHARDS_KEY);
    let c = trust(&w, |t| t.shards(&bundle, &w.map)).unwrap();
    assert_eq!(c.data, w.ledger.certified_data().to_vec());
    let (index, media) = shard_lists();
    let mut closed = index;
    closed[0].open = false;
    let moved = shard_map_hash(closed, media).unwrap();
    assert!(trust(&w, |t| t.shards(&bundle, &moved)).is_err());
    let swapped = shard_map_hash(media, index).unwrap();
    assert!(trust(&w, |t| t.shards(&bundle, &swapped)).is_err());
    let (cert, _) = split_shards_bundle(&bundle).unwrap();
    assert_eq!(cert, w.ledger.certificate().as_slice());
}

#[test]
fn malformed_shard_bundles_fail_cleanly() {
    let w = world(1);
    let bundle = w.ledger.shards_bundle(SHARDS_KEY);
    assert_eq!(
        shards_root(&bundle[..3], &w.map),
        Err(Error::Proof("shards bundle"))
    );
    let mut long = bundle.clone();
    long[..4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(
        shards_root(&long, &w.map),
        Err(Error::Proof("shards bundle"))
    );
    let mut empty_smt = bundle.clone();
    empty_smt.truncate(bundle.len() - 41);
    assert!(shards_root(&empty_smt, &w.map).is_err());
}

#[test]
fn shard_map_hash_is_deterministic_order_sensitive_and_bounded() {
    let (index, media) = shard_lists();
    let h = shard_map_hash(index, media).unwrap();
    assert_eq!(h, shard_map_hash(index, media).unwrap());
    let mut reversed = index;
    reversed.reverse();
    assert_ne!(h, shard_map_hash(reversed, media).unwrap());
    assert_ne!(h, shard_map_hash([], []).unwrap());
    let wide = [ShardEntry {
        shard: 0,
        canister: &[1; 30],
        open: true,
    }];
    assert_eq!(
        shard_map_hash(wide, []),
        Err(Error::Malformed("canister id"))
    );
    let many = core::iter::repeat_n(index[0], usize::from(u16::MAX) + 1);
    assert_eq!(shard_map_hash(many, []), Err(Error::Malformed("shard map")));
}

#[test]
fn wrong_trust_inputs_are_rejected() {
    let w = world(1);
    let r = &w.records[0];
    let other = TestSubnet::new(12).public_der();
    let base = Trust {
        root_key: &other,
        canister: &CANISTER,
        now_ns: NOW,
        max_age_ns: 0,
    };
    assert_eq!(
        base.record(&r.parts()),
        Err(Error::Certificate("signature"))
    );
    let key = w.ledger.subnet.public_der();
    let t = Trust {
        root_key: &key,
        canister: &[1; 10],
        ..base
    };
    assert_eq!(
        t.record(&r.parts()),
        Err(Error::Certificate("certified data"))
    );
    let t = Trust {
        root_key: &key,
        now_ns: NOW + DEFAULT_MAX_AGE_NS + 1,
        max_age_ns: DEFAULT_MAX_AGE_NS,
        ..base
    };
    assert_eq!(t.record(&r.parts()), Err(Error::Certificate("stale")));
    let t = Trust {
        now_ns: NOW - DEFAULT_MAX_AGE_NS,
        ..t
    };
    assert!(t.record(&r.parts()).is_ok());
}

#[test]
fn verification_is_deterministic() {
    let a = world(5);
    let b = world(5);
    assert_eq!(a.ledger.certificate(), b.ledger.certificate());
    for (x, y) in a.records.iter().zip(&b.records) {
        assert_eq!(
            trust(&a, |t| t.record(&x.parts())),
            trust(&b, |t| t.record(&y.parts()))
        );
    }
}
