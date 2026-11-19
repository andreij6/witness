use crate::store::Stores;
use crate::types::{Allow, Removal, Shell};
use candid::{CandidType, Decode, Encode, Principal};
use serde::Deserialize;
use witness_canister_common::Posting;
use witness_canister_common::peer::{ERASE, PUT, PUT_SHELL, SYNC, TOMBSTONE};

#[derive(CandidType, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dest {
    Index(u8),
    Pages,
    Media(u8),
}

pub const SEQ_BITS: u32 = 48;
pub const SEQ_MASK: u64 = (1 << SEQ_BITS) - 1;

impl Dest {
    pub const fn lane(self) -> u64 {
        let (tag, shard) = match self {
            Self::Index(i) => (0u64, i),
            Self::Pages => (1, 0),
            Self::Media(i) => (2, i),
        };
        ((tag << 8) | shard as u64) << SEQ_BITS
    }
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Put(Posting),
    Shell(Shell),
    EraseIx(u64),
    EraseId(String),
    Tombstone(String),
    Sync {
        record_ix: Option<u64>,
        allow: Allow,
    },
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Out {
    pub dest: Dest,
    pub body: Body,
}

impl Out {
    pub fn encode(&self) -> Vec<u8> {
        Encode!(self).expect("candid encode")
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        Decode!(bytes, Self).ok()
    }

    pub fn delivers_record(&self) -> Option<u64> {
        match &self.body {
            Body::Put(p) => Some(p.record_ix),
            Body::Shell(s) => Some(s.record_ix),
            Body::Sync { record_ix, .. } => *record_ix,
            Body::EraseIx(_) | Body::EraseId(_) | Body::Tombstone(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Put(Vec<Posting>),
    Shells(Vec<Shell>),
    EraseIx(u64),
    EraseId(String),
    Tombstones(Vec<String>),
    Sync(Vec<Allow>),
}

impl Call {
    pub const fn method(&self) -> &'static str {
        match self {
            Self::Put(_) => PUT,
            Self::Shells(_) => PUT_SHELL,
            Self::EraseIx(_) | Self::EraseId(_) => ERASE,
            Self::Tombstones(_) => TOMBSTONE,
            Self::Sync(_) => SYNC,
        }
    }

    pub fn arg(&self) -> Vec<u8> {
        match self {
            Self::Put(p) => Encode!(p),
            Self::Shells(s) => Encode!(s),
            Self::EraseIx(ix) => Encode!(ix),
            Self::EraseId(id) => Encode!(id),
            Self::Tombstones(ids) => Encode!(ids, &Removal::Erased),
            Self::Sync(a) => Encode!(a),
        }
        .expect("candid encode")
    }

    fn start(body: Body) -> Self {
        match body {
            Body::Put(p) => Self::Put(vec![p]),
            Body::Shell(s) => Self::Shells(vec![s]),
            Body::EraseIx(ix) => Self::EraseIx(ix),
            Body::EraseId(id) => Self::EraseId(id),
            Body::Tombstone(id) => Self::Tombstones(vec![id]),
            Body::Sync { allow, .. } => Self::Sync(vec![allow]),
        }
    }

    fn extend(&mut self, body: Body, max: usize) -> Result<(), Body> {
        match (self, body) {
            (Self::Put(v), Body::Put(p)) if v.len() < max => {
                v.push(p);
                Ok(())
            }
            (Self::Shells(v), Body::Shell(s)) if v.len() < max => {
                v.push(s);
                Ok(())
            }
            (Self::Tombstones(v), Body::Tombstone(id)) if v.len() < max => {
                v.push(id);
                Ok(())
            }
            (Self::Sync(v), Body::Sync { allow, .. }) if v.len() < max => {
                v.push(allow);
                Ok(())
            }
            (_, b) => Err(b),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub dest: Dest,
    pub call: Call,
    pub keys: Vec<u64>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Plan {
    pub batches: Vec<Batch>,
    pub dropped: Vec<u64>,
    pub resume: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub per_dest: usize,
    pub total: usize,
    pub batch: usize,
}

impl Budget {
    pub const fn unbounded(per_dest: usize, batch: usize) -> Self {
        Self {
            per_dest,
            total: usize::MAX,
            batch,
        }
    }
}

pub fn plan(st: &Stores, cursor: u64, budget: Budget) -> Plan {
    let mut plan = Plan::default();
    let mut closed: Vec<Dest> = Vec::new();
    let mut left = budget.total;
    let cursor = cursor & !SEQ_MASK;
    for (mut from, stop) in [(cursor, u64::MAX), (0, cursor)] {
        while left > 0 {
            let Some(first) = st.outbox.keys_range(from..).next().filter(|k| *k < stop) else {
                break;
            };
            let end = first | SEQ_MASK;
            let shared_lane = first & !SEQ_MASK == 0;
            let take = budget.per_dest.min(left);
            for e in st.outbox.range(first..=end).take(take) {
                left -= 1;
                let (seq, bytes) = e.into_pair();
                let Some(out) = Out::decode(&bytes) else {
                    plan.dropped.push(seq);
                    continue;
                };
                if out
                    .delivers_record()
                    .is_some_and(|ix| st.tombstones.contains_key(&ix))
                {
                    plan.dropped.push(seq);
                    continue;
                }
                if closed.contains(&out.dest) {
                    continue;
                }
                match plan.batches.iter_mut().find(|b| b.dest == out.dest) {
                    None => plan.batches.push(Batch {
                        dest: out.dest,
                        call: Call::start(out.body),
                        keys: vec![seq],
                    }),
                    Some(b) => match b.call.extend(out.body, budget.batch) {
                        Ok(()) => b.keys.push(seq),
                        Err(_) => {
                            closed.push(out.dest);
                            if !shared_lane {
                                break;
                            }
                        }
                    },
                }
            }
            let Some(next) = end.checked_add(1) else {
                break;
            };
            from = next;
            if left == 0 {
                plan.resume = next;
            }
        }
    }
    plan
}

pub fn resolve(st: &Stores, dest: Dest) -> Option<Principal> {
    let c = st.config.get();
    let p = match dest {
        Dest::Index(i) => c.index.get(usize::from(i)).copied(),
        Dest::Media(i) => c.media.get(usize::from(i)).copied(),
        Dest::Pages => Some(c.pages),
    }?;
    (p != Principal::anonymous()).then_some(p)
}

pub fn complete(st: &mut Stores, keys: &[u64]) {
    for k in keys {
        st.outbox.remove(k);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::with;
    use serde_bytes::ByteBuf;
    use witness_canister_common::RegistryInit;

    fn posting(ix: u64) -> Posting {
        Posting {
            record_ix: ix,
            wm_root: 1,
            asset_hash: vec![2; 32],
            pixel_hash: vec![3; 32],
            fingerprints: Some(ByteBuf::from(vec![4; 10])),
            root_ix: None,
        }
    }

    fn shell(ix: u64) -> Shell {
        Shell {
            record_id: format!("R{ix}"),
            record_ix: ix,
            anchored_ns: 0,
            claim: vec![],
            media_shard: 0,
            media: Principal::anonymous(),
            dev: false,
        }
    }

    fn allow(n: u8) -> Allow {
        Allow::Record {
            record_id: format!("R{n}"),
            key_id: ByteBuf::from(vec![n; 16]),
        }
    }

    fn put(st: &mut Stores, dest: Dest, body: Body) {
        st.enqueue(&Out { dest, body });
    }

    #[test]
    fn out_round_trips_and_rejects_garbage() {
        let o = Out {
            dest: Dest::Media(3),
            body: Body::EraseId("abc".into()),
        };
        assert_eq!(Out::decode(&o.encode()), Some(o.clone()));
        assert_eq!(o.encode(), o.encode());
        assert_eq!(Out::decode(b"junk"), None);
    }

    #[test]
    fn delivers_record_only_for_data() {
        let d = |body| {
            Out {
                dest: Dest::Pages,
                body,
            }
            .delivers_record()
        };
        assert_eq!(d(Body::Put(posting(4))), Some(4));
        assert_eq!(d(Body::Shell(shell(5))), Some(5));
        assert_eq!(d(Body::EraseIx(6)), None);
        assert_eq!(d(Body::EraseId("x".into())), None);
    }

    #[test]
    fn call_methods_and_args() {
        assert_eq!(Call::Put(vec![]).method(), "put");
        assert_eq!(Call::Shells(vec![]).method(), "put_shell");
        assert_eq!(Call::Tombstones(vec![]).method(), "tombstone");
        assert_eq!(Call::Sync(vec![]).method(), "sync");
        let (ids, why) = Decode!(
            &Call::Tombstones(vec!["a".into(), "b".into()]).arg(),
            Vec<String>,
            Removal
        )
        .unwrap();
        assert_eq!(
            (ids, why),
            (vec!["a".to_owned(), "b".to_owned()], Removal::Erased)
        );
        assert_eq!(
            Decode!(&Call::Sync(vec![allow(1)]).arg(), Vec<Allow>).unwrap(),
            vec![allow(1)]
        );
        assert_eq!(Call::EraseIx(1).method(), "erase");
        assert_eq!(Call::EraseId("a".into()).method(), "erase");
        let arg = Call::Put(vec![posting(1)]).arg();
        assert_eq!(Decode!(&arg, Vec<Posting>).unwrap(), vec![posting(1)]);
        assert_eq!(Decode!(&Call::EraseIx(9).arg(), u64).unwrap(), 9);
        assert_eq!(
            Decode!(&Call::EraseId("z".into()).arg(), String).unwrap(),
            "z"
        );
        assert_eq!(
            Decode!(&Call::Shells(vec![shell(2)]).arg(), Vec<Shell>).unwrap(),
            vec![shell(2)]
        );
    }

    #[test]
    fn plan_batches_per_destination_in_order() {
        with(|st| {
            for ix in 0..3 {
                put(st, Dest::Index(0), Body::Put(posting(ix)));
            }
            put(st, Dest::Pages, Body::Shell(shell(0)));
            put(st, Dest::Index(0), Body::EraseIx(1));
            put(st, Dest::Index(0), Body::Put(posting(9)));
            put(st, Dest::Media(0), Body::EraseId("m".into()));
            put(st, Dest::Pages, Body::Shell(shell(1)));
            let p = plan(st, 0, Budget::unbounded(100, 64));
            assert!(p.dropped.is_empty());
            assert_eq!(p.batches.len(), 3);
            assert_eq!(p.batches[0].dest, Dest::Index(0));
            assert_eq!(p.batches[0].call, Call::Put((0..3).map(posting).collect()));
            assert_eq!(p.batches[0].keys, vec![0, 1, 2]);
            assert_eq!(p.batches[1].call, Call::Shells(vec![shell(0), shell(1)]));
            assert_eq!(
                p.batches[1].keys,
                vec![Dest::Pages.lane(), Dest::Pages.lane() + 1]
            );
            assert_eq!(p.batches[2].call, Call::EraseId("m".into()));
        });
    }

    #[test]
    fn plan_respects_batch_and_scan_caps() {
        with(|st| {
            for ix in 0..10 {
                put(st, Dest::Index(0), Body::Put(posting(ix)));
            }
            let p = plan(st, 0, Budget::unbounded(100, 4));
            assert_eq!(p.batches.len(), 1);
            assert_eq!(p.batches[0].keys, vec![0, 1, 2, 3]);
            let p = plan(st, 0, Budget::unbounded(2, 64));
            assert_eq!(p.batches[0].keys, vec![0, 1]);
            assert_eq!(plan(st, 0, Budget::unbounded(0, 64)), Plan::default());
        });
    }

    #[test]
    fn plan_erase_is_never_batched() {
        with(|st| {
            put(st, Dest::Index(0), Body::EraseIx(1));
            put(st, Dest::Index(0), Body::EraseIx(2));
            let p = plan(st, 0, Budget::unbounded(100, 64));
            assert_eq!(p.batches.len(), 1);
            assert_eq!(p.batches[0].call, Call::EraseIx(1));
        });
    }

    #[test]
    fn plan_drops_tombstoned_and_garbage() {
        with(|st| {
            put(st, Dest::Index(0), Body::Put(posting(1)));
            put(st, Dest::Pages, Body::Shell(shell(1)));
            put(st, Dest::Index(0), Body::Put(posting(2)));
            st.outbox.insert(50, vec![0xde, 0xad]);
            put(st, Dest::Index(0), Body::EraseIx(1));
            st.tombstones.insert(1, 99);
            let p = plan(st, 0, Budget::unbounded(100, 64));
            assert_eq!(p.dropped, vec![0, 50, Dest::Pages.lane()]);
            assert_eq!(p.batches.len(), 1);
            assert_eq!(p.batches[0].call, Call::Put(vec![posting(2)]));
            complete(st, &p.dropped);
            complete(st, &p.batches[0].keys);
            let p = plan(st, 0, Budget::unbounded(100, 64));
            assert_eq!(p.batches[0].call, Call::EraseIx(1));
        });
    }

    #[test]
    fn lanes_are_disjoint_and_ordered() {
        let lanes: Vec<u64> = [
            Dest::Index(0),
            Dest::Index(255),
            Dest::Pages,
            Dest::Media(0),
            Dest::Media(255),
        ]
        .map(Dest::lane)
        .to_vec();
        assert!(lanes.windows(2).all(|w| w[0] | SEQ_MASK < w[1]));
        assert_eq!(Dest::Index(0).lane(), 0);
        assert_eq!(Dest::Media(7).lane() & SEQ_MASK, 0);
    }

    #[test]
    fn a_stuck_destination_does_not_starve_the_others() {
        with(|st| {
            for ix in 0..300 {
                put(st, Dest::Index(0), Body::EraseIx(ix));
            }
            put(st, Dest::Pages, Body::Tombstone("p".into()));
            put(st, Dest::Media(1), Body::EraseId("m".into()));
            let p = plan(st, 0, Budget::unbounded(256, 64));
            let dests: Vec<Dest> = p.batches.iter().map(|b| b.dest).collect();
            assert_eq!(dests, vec![Dest::Index(0), Dest::Pages, Dest::Media(1)]);
            assert_eq!(p.batches[0].call, Call::EraseIx(0));
            assert_eq!(p.batches[1].call, Call::Tombstones(vec!["p".into()]));
            assert_eq!(p.batches[2].call, Call::EraseId("m".into()));
        });
    }

    #[test]
    fn a_full_dedicated_lane_stops_scanning_and_leaves_budget_for_the_next() {
        with(|st| {
            for i in 0..200 {
                put(st, Dest::Pages, Body::Tombstone(format!("t{i}")));
            }
            put(st, Dest::Media(0), Body::EraseId("m".into()));
            let budget = Budget {
                per_dest: 256,
                total: 100,
                batch: 64,
            };
            let p = plan(st, 0, budget);
            let dests: Vec<Dest> = p.batches.iter().map(|b| b.dest).collect();
            assert_eq!(dests, vec![Dest::Pages, Dest::Media(0)]);
            assert_eq!(p.batches[0].keys.len(), 64);
            assert_eq!(p.resume, 0);
        });
    }

    #[test]
    fn a_full_shared_lane_keeps_scanning_for_legacy_entries() {
        with(|st| {
            for ix in 0..3 {
                put(st, Dest::Index(0), Body::Put(posting(ix)));
            }
            st.outbox.insert(
                3,
                Out {
                    dest: Dest::Pages,
                    body: Body::Tombstone("a".into()),
                }
                .encode(),
            );
            let p = plan(st, 0, Budget::unbounded(256, 2));
            assert_eq!(p.batches.len(), 2);
            assert_eq!(p.batches[0].keys, vec![0, 1]);
            assert_eq!(p.batches[1].call, Call::Tombstones(vec!["a".into()]));
        });
    }

    #[test]
    fn legacy_unlaned_entries_still_drain_in_order() {
        with(|st| {
            let legacy = |dest, body| Out { dest, body }.encode();
            st.outbox
                .insert(0, legacy(Dest::Pages, Body::Tombstone("a".into())));
            st.outbox
                .insert(1, legacy(Dest::Media(0), Body::EraseId("m".into())));
            put(st, Dest::Pages, Body::Tombstone("b".into()));
            let p = plan(st, 0, Budget::unbounded(256, 64));
            assert_eq!(p.batches.len(), 2);
            assert_eq!(
                p.batches[0].call,
                Call::Tombstones(vec!["a".into(), "b".into()])
            );
            assert_eq!(p.batches[0].keys, vec![0, Dest::Pages.lane()]);
            assert_eq!(p.batches[1].call, Call::EraseId("m".into()));
        });
    }

    #[test]
    fn resolve_uses_config() {
        with(|st| {
            assert_eq!(resolve(st, Dest::Pages), None);
            assert_eq!(resolve(st, Dest::Index(0)), None);
            let p = |n: u8| Principal::from_slice(&[n; 10]);
            st.configure(RegistryInit {
                index: vec![p(1)],
                media: vec![p(2), p(3)],
                pages: p(4),
            });
            assert_eq!(resolve(st, Dest::Index(0)), Some(p(1)));
            assert_eq!(resolve(st, Dest::Index(1)), None);
            assert_eq!(resolve(st, Dest::Media(1)), Some(p(3)));
            assert_eq!(resolve(st, Dest::Pages), Some(p(4)));
        });
    }

    #[test]
    fn delivers_record_for_record_sync_only() {
        let d = |body| {
            Out {
                dest: Dest::Media(0),
                body,
            }
            .delivers_record()
        };
        assert_eq!(
            d(Body::Sync {
                record_ix: Some(3),
                allow: allow(3)
            }),
            Some(3)
        );
        assert_eq!(
            d(Body::Sync {
                record_ix: None,
                allow: allow(4)
            }),
            None
        );
        assert_eq!(d(Body::Tombstone("t".into())), None);
    }

    #[test]
    fn plan_batches_syncs_and_tombstones_and_drops_erased_syncs() {
        with(|st| {
            let sync = |ix: Option<u64>, n| Body::Sync {
                record_ix: ix,
                allow: allow(n),
            };
            put(st, Dest::Media(0), sync(None, 1));
            put(st, Dest::Media(0), sync(Some(7), 2));
            put(st, Dest::Media(0), sync(Some(8), 3));
            put(st, Dest::Pages, Body::Tombstone("a".into()));
            put(st, Dest::Pages, Body::Tombstone("b".into()));
            st.tombstones.insert(7, 1);
            let p = plan(st, 0, Budget::unbounded(100, 64));
            let media = Dest::Media(0).lane();
            assert_eq!(p.dropped, vec![media + 1]);
            assert_eq!(
                p.batches[0].call,
                Call::Tombstones(vec!["a".into(), "b".into()])
            );
            assert_eq!(p.batches[1].call, Call::Sync(vec![allow(1), allow(3)]));
            assert_eq!(p.batches[1].keys, vec![media, media + 2]);
            assert_eq!(plan(st, 0, Budget::unbounded(100, 64)), p);
        });
    }

    fn all_dests() -> Vec<Dest> {
        (0..8)
            .map(Dest::Index)
            .chain([Dest::Pages])
            .chain((0..8).map(Dest::Media))
            .collect()
    }

    fn fill(st: &mut Stores, dests: &[Dest], per: usize) {
        for d in dests {
            for i in 0..per {
                put(st, *d, Body::Tombstone(format!("{i}")));
            }
        }
    }

    #[test]
    fn total_budget_caps_each_tick() {
        with(|st| {
            fill(st, &all_dests(), 10);
            let budget = Budget {
                per_dest: 4,
                total: 10,
                batch: 64,
            };
            let p = plan(st, 0, budget);
            let scanned: usize = p.batches.iter().map(|b| b.keys.len()).sum();
            assert_eq!(scanned, 10);
            let dests: Vec<Dest> = p.batches.iter().map(|b| b.dest).collect();
            assert_eq!(dests, vec![Dest::Index(0), Dest::Index(1), Dest::Index(2)]);
            assert_eq!(p.batches[2].keys.len(), 2);
            assert_eq!(p.resume, Dest::Index(3).lane());
            assert_eq!(plan(st, 0, budget), p);
        });
    }

    #[test]
    fn stuck_destinations_are_served_round_robin() {
        with(|st| {
            let dests = all_dests();
            fill(st, &dests, 10);
            let budget = Budget {
                per_dest: 4,
                total: 8,
                batch: 64,
            };
            let mut cursor = 0;
            let mut served = vec![0u32; dests.len()];
            for _ in 0..9 {
                let p = plan(st, cursor, budget);
                let scanned: usize = p.batches.iter().map(|b| b.keys.len()).sum();
                assert!(scanned <= budget.total);
                for b in &p.batches {
                    let i = dests.iter().position(|d| *d == b.dest).unwrap();
                    served[i] += 1;
                }
                cursor = p.resume;
            }
            assert!(served.iter().all(|n| *n >= 1), "{served:?}");
            let (lo, hi) = (served.iter().min().unwrap(), served.iter().max().unwrap());
            assert!(hi - lo <= 1, "{served:?}");
        });
    }

    #[test]
    fn draining_with_a_small_budget_empties_every_lane() {
        with(|st| {
            let dests = all_dests();
            fill(st, &dests, 5);
            let budget = Budget {
                per_dest: 3,
                total: 7,
                batch: 2,
            };
            let mut cursor = 0;
            let mut ticks = 0;
            while !st.outbox.is_empty() {
                let p = plan(st, cursor, budget);
                for b in &p.batches {
                    assert!(b.keys.len() <= budget.batch);
                    complete(st, &b.keys);
                }
                cursor = p.resume;
                ticks += 1;
                assert!(ticks < 100, "outbox never drained");
            }
            assert!(ticks >= dests.len() * 5 / budget.total);
        });
    }

    #[test]
    fn cursor_starts_mid_ring_and_wraps() {
        with(|st| {
            fill(st, &[Dest::Index(0), Dest::Pages, Dest::Media(0)], 1);
            let p = plan(st, Dest::Pages.lane() + 5, Budget::unbounded(256, 64));
            let dests: Vec<Dest> = p.batches.iter().map(|b| b.dest).collect();
            assert_eq!(dests, vec![Dest::Pages, Dest::Media(0), Dest::Index(0)]);
            assert_eq!(p.resume, 0);
            let p = plan(st, u64::MAX, Budget::unbounded(256, 64));
            assert_eq!(p.batches.len(), 3);
        });
    }

    #[test]
    fn zero_budget_plans_nothing() {
        with(|st| {
            fill(st, &all_dests(), 2);
            let p = plan(
                st,
                0,
                Budget {
                    per_dest: 4,
                    total: 0,
                    batch: 64,
                },
            );
            assert_eq!(p, Plan::default());
        });
    }

    #[test]
    fn call_methods_are_methods_of_the_peer_services() {
        let did = witness_canister_common::DID;
        let env = candid_parser::utils::CandidSource::Text(did)
            .load()
            .unwrap()
            .0;
        let methods = |svc: &str| -> Vec<String> {
            env.as_service(&env.find_type(svc).unwrap().clone())
                .unwrap()
                .iter()
                .map(|(m, _)| m.to_string())
                .collect()
        };
        let cases = [
            ("IndexPeer", Call::Put(vec![])),
            ("IndexPeer", Call::EraseIx(0)),
            ("PagesPeer", Call::Shells(vec![])),
            ("PagesPeer", Call::Tombstones(vec![])),
            ("MediaPeer", Call::Sync(vec![])),
            ("MediaPeer", Call::EraseId(String::new())),
        ];
        for (svc, call) in cases {
            assert!(
                methods(svc).contains(&call.method().to_string()),
                "{svc} {}",
                call.method()
            );
        }
    }
}
