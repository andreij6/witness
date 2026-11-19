use crate::outbox::Out;
use crate::proof::{Roots, SHARDS_KEY, shard_map_hash};
use crate::record::RecordEntry;
use crate::state::State;
use ic_stable_structures::{BTreeMap, Cell, Log, Vec as StableVec};
use std::cell::RefCell;
use witness_canister_common::memory::{CONFIG, local, memory, registry as ids};
use witness_canister_common::{Memory, RegistryInit, ShardInfo, Shards};
use witness_format::{KeyId, RecordId, cbor};
use witness_merkle::{Hash, Mmr, MmrStore, Smt, SmtStore};

pub type Map<K, V> = BTreeMap<K, V, Memory>;

pub struct MmrNodes<'a>(&'a StableVec<Hash, Memory>);

impl MmrStore for MmrNodes<'_> {
    fn size(&self) -> u64 {
        self.0.len()
    }

    fn get(&self, pos: u64) -> Option<Hash> {
        self.0.get(pos)
    }

    fn push(&mut self, hash: Hash) {
        self.0.push(&hash);
    }
}

pub struct SmtNodes<'a>(&'a mut Map<[u8; 17], Hash>);

pub fn smt_key(depth: u8, prefix: u128) -> [u8; 17] {
    let mut k = [0u8; 17];
    k[0] = depth;
    k[1..].copy_from_slice(&prefix.to_be_bytes());
    k
}

impl SmtStore for SmtNodes<'_> {
    fn get(&self, depth: u8, prefix: u128) -> Option<Hash> {
        self.0.get(&smt_key(depth, prefix))
    }

    fn set(&mut self, depth: u8, prefix: u128, hash: Hash) {
        self.0.insert(smt_key(depth, prefix), hash);
    }

    fn remove(&mut self, depth: u8, prefix: u128) {
        self.0.remove(&smt_key(depth, prefix));
    }
}

pub struct Stores {
    pub config: Cell<RegistryInit, Memory>,
    pub state: Cell<State, Memory>,
    pub mmr: StableVec<Hash, Memory>,
    pub smt: Map<[u8; 17], Hash>,
    pub records: Log<Vec<u8>, Memory, Memory>,
    pub by_id: Map<[u8; 10], u64>,
    pub keys: Map<[u8; 16], Vec<u8>>,
    pub pending: Map<[u8; 16], u64>,
    pub counters: Map<[u8; 24], u64>,
    pub by_capture: Map<[u8; 16], ()>,
    pub derivatives: Map<[u8; 16], ()>,
    pub tombstones: Map<u64, u64>,
    pub outbox: Map<u64, Vec<u8>>,
    pub enroll_quota: Map<[u8; 32], [u8; 24]>,
    pub submit_quota: Map<[u8; 16], [u8; 16]>,
}

pub const RECORD_DATA: u8 = 0;
pub const SUBMIT_QUOTA: u8 = 1;

thread_local! {
    static STORES: RefCell<Stores> = RefCell::new(Stores::open());
}

pub fn with<R>(f: impl FnOnce(&mut Stores) -> R) -> R {
    STORES.with_borrow_mut(f)
}

pub fn pair(a: u64, b: u64) -> [u8; 16] {
    let mut k = [0u8; 16];
    k[..8].copy_from_slice(&a.to_be_bytes());
    k[8..].copy_from_slice(&b.to_be_bytes());
    k
}

pub fn counter_key(key_id: &KeyId, counter: u64) -> [u8; 24] {
    let mut k = [0u8; 24];
    k[..16].copy_from_slice(key_id.as_bytes());
    k[16..].copy_from_slice(&counter.to_be_bytes());
    k
}

fn shard_list(canisters: &[candid::Principal]) -> Vec<ShardInfo> {
    let last = canisters.len().saturating_sub(1);
    canisters
        .iter()
        .take(usize::from(u8::MAX) + 1)
        .enumerate()
        .map(|(i, c)| ShardInfo {
            shard: i as u8,
            canister: *c,
            open: i == last,
        })
        .collect()
}

impl Stores {
    fn open() -> Self {
        Self {
            config: Cell::init(memory(CONFIG), RegistryInit::default()),
            state: Cell::init(memory(ids::STATE), State::default()),
            mmr: StableVec::init(memory(ids::MMR_NODES)),
            smt: BTreeMap::init(memory(ids::SMT_NODES)),
            records: Log::init(memory(ids::RECORDS), memory(local(RECORD_DATA))),
            by_id: BTreeMap::init(memory(ids::RECORD_BY_ID)),
            keys: BTreeMap::init(memory(ids::KEYS)),
            pending: BTreeMap::init(memory(ids::PENDING_KEYS)),
            counters: BTreeMap::init(memory(ids::COUNTERS)),
            by_capture: BTreeMap::init(memory(ids::BY_CAPTURE)),
            derivatives: BTreeMap::init(memory(ids::DERIVATIVES)),
            tombstones: BTreeMap::init(memory(ids::TOMBSTONES)),
            outbox: BTreeMap::init(memory(ids::OUTBOX)),
            enroll_quota: BTreeMap::init(memory(ids::QUOTAS)),
            submit_quota: BTreeMap::init(memory(local(SUBMIT_QUOTA))),
        }
    }

    pub fn mmr(&self) -> Mmr<MmrNodes<'_>> {
        Mmr::new(MmrNodes(&self.mmr)).expect("mmr store")
    }

    pub fn smt(&mut self) -> Smt<SmtNodes<'_>> {
        Smt::new(SmtNodes(&mut self.smt))
    }

    pub fn roots(&mut self) -> Roots {
        let mmr = self.mmr();
        let leaf_count = mmr.leaf_count();
        let mmr_root = mmr.root().expect("mmr root");
        Roots {
            mmr_root,
            leaf_count,
            smt_root: self.smt().root(),
        }
    }

    pub fn update_state<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        let mut s = self.state.get().clone();
        let r = f(&mut s);
        self.state.set(s);
        r
    }

    pub fn key(&self, id: &KeyId) -> Option<witness_format::KeyInfo> {
        self.keys
            .get(id.as_bytes())
            .and_then(|b| cbor::from_slice(&b).ok())
    }

    pub fn put_key(&mut self, info: &witness_format::KeyInfo) {
        self.keys.insert(info.key_id.to_bytes(), cbor::to_vec(info));
        let k = witness_merkle::key_of(&info.key_id);
        self.smt().insert(k, &info.leaf_hash());
    }

    pub fn record(&self, ix: u64) -> Option<RecordEntry> {
        self.records.get(ix).and_then(|b| RecordEntry::decode(&b))
    }

    pub fn record_ix(&self, id: &RecordId) -> Option<u64> {
        self.by_id.get(id.as_bytes())
    }

    pub fn shards(&self) -> Shards {
        let c = self.config.get();
        Shards {
            index: shard_list(&c.index),
            media: shard_list(&c.media),
        }
    }

    pub fn configure(&mut self, init: RegistryInit) {
        self.config.set(init);
        let h = shard_map_hash(&self.shards());
        self.smt().insert(SHARDS_KEY, &h);
    }

    pub fn enqueue(&mut self, out: &Out) {
        let lane = out.dest.lane();
        let seq = self
            .outbox
            .keys_range(lane..=lane | crate::outbox::SEQ_MASK)
            .next_back()
            .map_or(lane, |k| k + 1);
        self.outbox.insert(seq, out.encode());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outbox::{Body, Dest};
    use candid::Principal;
    use witness_format::{Freshness, KeyHistory, Provisioning, SecurityLevel};

    fn p(n: u8) -> Principal {
        Principal::from_slice(&[n; 10])
    }

    pub fn key_info(seed: u8) -> witness_format::KeyInfo {
        let pubkey = vec![seed; 91];
        witness_format::KeyInfo {
            key_id: KeyId::from_spki(&pubkey),
            pubkey,
            security: SecurityLevel::Tee,
            provisioning: Provisioning::Rkp,
            freshness: Freshness::Fresh,
            patch: 202_601,
            enrolled_ns: 5,
            active_until_ns: 10,
            history: KeyHistory::new(5),
            chain_serials: vec!["1".into()],
        }
    }

    #[test]
    fn keys_pack_big_endian() {
        assert_eq!(smt_key(3, 1)[0], 3);
        assert_eq!(smt_key(3, 1)[16], 1);
        assert!(pair(1, u64::MAX) < pair(2, 0));
        assert!(pair(1, 1) < pair(1, 2));
        let id = KeyId::from_bytes([7; 16]);
        assert!(counter_key(&id, 1) < counter_key(&id, 256));
        assert_eq!(&counter_key(&id, 1)[..16], &[7; 16]);
    }

    #[test]
    fn mmr_store_persists_appends() {
        with(|s| {
            assert_eq!(s.mmr().leaf_count(), 0);
            for i in 0..5u8 {
                s.mmr().append(&[i]).unwrap();
            }
            assert_eq!(s.mmr().leaf_count(), 5);
            let mut reference = witness_merkle::Mmr::new(Vec::new()).unwrap();
            for i in 0..5u8 {
                reference.append(&[i]).unwrap();
            }
            assert_eq!(s.mmr().root().unwrap(), reference.root().unwrap());
        });
    }

    #[test]
    fn smt_store_matches_reference() {
        with(|s| {
            let mut reference = Smt::new(std::collections::BTreeMap::new());
            for k in [0u128, 1, u128::MAX, 1 << 100] {
                s.smt().insert(k, &[k as u8; 32]);
                reference.insert(k, &[k as u8; 32]);
            }
            s.smt().remove(1);
            reference.remove(1);
            assert_eq!(s.smt().root(), reference.root());
            assert_eq!(s.smt().prove(0), reference.prove(0));
        });
    }

    #[test]
    fn keys_round_trip_and_update_smt() {
        with(|s| {
            let before = s.roots();
            let k = key_info(4);
            s.put_key(&k);
            assert_eq!(s.key(&k.key_id), Some(k.clone()));
            assert_ne!(s.roots().smt_root, before.smt_root);
            let proof = s.smt().prove(witness_merkle::key_of(&k.key_id));
            assert!(proof.verify(
                &s.roots().smt_root,
                witness_merkle::key_of(&k.key_id),
                Some(&k.leaf_hash())
            ));
            assert_eq!(s.key(&KeyId::from_bytes([0; 16])), None);
            s.keys.insert([1; 16], vec![0xff]);
            assert_eq!(s.key(&KeyId::from_bytes([1; 16])), None);
        });
    }

    #[test]
    fn records_round_trip() {
        with(|s| {
            let e = RecordEntry {
                anchored_ns: 1,
                root_ix: 0,
                index_shard: 0,
                media_shard: 0,
                claim: vec![1],
                c2pa_claim: vec![2],
                cose: vec![3],
            };
            assert_eq!(s.records.append(&e.encode()).unwrap(), 0);
            s.by_id.insert(e.record_id().to_bytes(), 0);
            assert_eq!(s.record(0), Some(e.clone()));
            assert_eq!(s.record(1), None);
            assert_eq!(s.record_ix(&e.record_id()), Some(0));
        });
    }

    #[test]
    fn shards_follow_config_and_certify() {
        with(|s| {
            assert_eq!(s.shards(), Shards::default());
            let r0 = s.roots().smt_root;
            s.configure(RegistryInit {
                index: vec![p(1), p(2)],
                media: vec![p(3)],
                pages: p(4),
            });
            let sh = s.shards();
            assert_eq!(sh.index.len(), 2);
            assert!(!sh.index[0].open && sh.index[1].open);
            assert_eq!(sh.index[1].shard, 1);
            assert_eq!(
                sh.media,
                vec![ShardInfo {
                    shard: 0,
                    canister: p(3),
                    open: true
                }]
            );
            let r1 = s.roots().smt_root;
            assert_ne!(r0, r1);
            assert!(
                s.smt()
                    .prove(SHARDS_KEY)
                    .verify(&r1, SHARDS_KEY, Some(&shard_map_hash(&sh)))
            );
        });
    }

    #[test]
    fn shard_list_caps_at_256() {
        let many = vec![p(1); 300];
        let l = shard_list(&many);
        assert_eq!(l.len(), 256);
        assert_eq!(l[255].shard, 255);
        assert!(shard_list(&[]).is_empty());
    }

    #[test]
    fn update_state_persists() {
        with(|s| {
            let serial = s.update_state(State::take_serial);
            assert_eq!(serial, 2);
            assert_eq!(s.state.get().next_serial, 3);
        });
    }

    #[test]
    fn enqueue_appends_in_order() {
        with(|s| {
            for ix in 0..3 {
                s.enqueue(&Out {
                    dest: Dest::Index(0),
                    body: Body::EraseIx(ix),
                });
            }
            let keys: Vec<u64> = s.outbox.keys().collect();
            assert_eq!(keys, vec![0, 1, 2]);
            s.outbox.remove(&0);
            s.enqueue(&Out {
                dest: Dest::Pages,
                body: Body::EraseId("x".into()),
            });
            assert_eq!(
                s.outbox.last_key_value().map(|(k, _)| k),
                Some(Dest::Pages.lane())
            );
            s.enqueue(&Out {
                dest: Dest::Index(0),
                body: Body::EraseIx(9),
            });
            assert!(s.outbox.contains_key(&3));
            s.enqueue(&Out {
                dest: Dest::Media(2),
                body: Body::EraseId("y".into()),
            });
            assert!(s.outbox.contains_key(&Dest::Media(2).lane()));
        });
    }
}
