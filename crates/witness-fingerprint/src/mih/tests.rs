use super::permutation::generate;
use super::*;
use crate::hash::HASH_BITS;
use crate::testkit::Rng;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct MemoryStore {
    buckets: BTreeMap<u32, Vec<Posting>>,
    visits: core::cell::Cell<usize>,
}

impl PostingStore for MemoryStore {
    fn insert(&mut self, key: BucketKey, posting: Posting) {
        self.buckets.entry(key.packed()).or_default().push(posting);
    }

    fn visit(&self, key: BucketKey, mut sink: impl FnMut(Posting)) {
        self.visits.set(self.visits.get() + 1);
        if let Some(bucket) = self.buckets.get(&key.packed()) {
            bucket.iter().copied().for_each(&mut sink);
        }
    }
}

fn random_hash(rng: &mut Rng) -> PdqHash {
    PdqHash::from_words([rng.next(), rng.next(), rng.next(), rng.next()])
}

fn flip_permuted_bits(hash: &PdqHash, permuted_bits: &[usize]) -> PdqHash {
    let mut out = *hash;
    for &target in permuted_bits {
        let source = PERMUTATION
            .iter()
            .position(|&p| p as usize == target)
            .unwrap();
        out.0[31 - source / 8] ^= 1 << (source % 8);
    }
    out
}

fn flip_random_bits(rng: &mut Rng, hash: &PdqHash, count: usize) -> PdqHash {
    let mut bits = BTreeSet::new();
    while bits.len() < count {
        bits.insert(rng.below(HASH_BITS as u64) as usize);
    }
    let mut out = *hash;
    bits.into_iter()
        .for_each(|b| out.0[31 - b / 8] ^= 1 << (b % 8));
    out
}

fn found(store: &MemoryStore, query: &PdqHash, ix: u32) -> bool {
    Prober::new()
        .probe(store, query, usize::MAX)
        .candidates
        .iter()
        .any(|c| c.local_ix == ix)
}

#[test]
fn permutation_is_a_bijection() {
    let set: BTreeSet<u8> = PERMUTATION.iter().copied().collect();
    assert_eq!(set.len(), HASH_BITS);
    assert_ne!(PERMUTATION, generate(PERMUTATION_SEED ^ 1));
    assert!(
        PERMUTATION
            .iter()
            .enumerate()
            .filter(|&(i, &p)| i == p as usize)
            .count()
            < 8
    );
}

#[test]
fn permutation_is_published_and_frozen() {
    assert_eq!(PERMUTATION, generate(0x5749_544E_4553_5331));
    let fingerprint = PERMUTATION
        .iter()
        .fold(0u64, |acc, &p| acc.wrapping_mul(31).wrapping_add(p as u64));
    assert_eq!(fingerprint, FROZEN_PERMUTATION_FINGERPRINT);
    assert_eq!(&PERMUTATION[..8], &FROZEN_PERMUTATION_PREFIX);
}

const FROZEN_PERMUTATION_FINGERPRINT: u64 = 7_809_565_986_940_808_236;
const FROZEN_PERMUTATION_PREFIX: [u8; 8] = [187, 124, 136, 184, 195, 56, 52, 96];

#[test]
fn permute_preserves_popcount_and_distance() {
    let mut rng = Rng::new(1);
    for _ in 0..300 {
        let (a, b) = (random_hash(&mut rng), random_hash(&mut rng));
        let (pa, pb) = (permute(&a), permute(&b));
        assert_eq!(
            pa.iter().map(|w| w.count_ones()).sum::<u32>(),
            a.count_ones()
        );
        let permuted_distance: u32 = pa.iter().zip(pb).map(|(x, y)| (x ^ y).count_ones()).sum();
        assert_eq!(permuted_distance, a.distance(&b));
    }
}

#[test]
fn permute_moves_each_bit_to_its_published_slot() {
    for (source, &target) in PERMUTATION.iter().enumerate() {
        let mut hash = PdqHash::default();
        hash.set_bit(source);
        let target = target as usize;
        let mut expected = [0u64; 4];
        expected[target / 64] = 1 << (target % 64);
        assert_eq!(permute(&hash), expected);
    }
}

#[test]
fn chunks_split_permuted_bits() {
    let mut rng = Rng::new(2);
    for _ in 0..100 {
        let hash = random_hash(&mut rng);
        let words = permute(&hash);
        let parts = chunks(&hash);
        for (c, &value) in parts.iter().enumerate() {
            assert_eq!(value as u64, (words[c / 4] >> ((c % 4) * 16)) & 0xFFFF);
        }
        let sum: u32 = parts.iter().map(|v| v.count_ones()).sum();
        assert_eq!(sum, hash.count_ones());
    }
}

#[test]
fn single_bit_flip_changes_one_chunk_by_one_bit() {
    let mut rng = Rng::new(3);
    let hash = random_hash(&mut rng);
    for (bit, &slot) in PERMUTATION.iter().enumerate() {
        let mut flipped = hash;
        flipped.0[31 - bit / 8] ^= 1 << (bit % 8);
        let (a, b) = (chunks(&hash), chunks(&flipped));
        let changed: Vec<usize> = (0..MIH_CHUNKS).filter(|&c| a[c] != b[c]).collect();
        assert_eq!(changed.len(), 1);
        assert_eq!((a[changed[0]] ^ b[changed[0]]).count_ones(), 1);
        assert_eq!(changed[0], slot as usize / 16);
    }
}

#[test]
fn probe_keys_are_exact_plus_single_flips() {
    let mut rng = Rng::new(4);
    let hash = random_hash(&mut rng);
    let keys = probe_keys(&hash);
    let index = index_keys(&hash);
    assert_eq!(keys.len(), 272);
    assert_eq!(keys.iter().collect::<BTreeSet<_>>().len(), 272);
    for (chunk, group) in keys.chunks_exact(PROBES_PER_CHUNK).enumerate() {
        assert_eq!(group[0], index[chunk]);
        assert!(group.iter().all(|k| k.chunk as usize == chunk));
        for (flip, key) in group[1..].iter().enumerate() {
            assert_eq!(key.value ^ group[0].value, 1 << flip);
        }
    }
}

#[test]
fn bucket_key_layouts() {
    let key = BucketKey {
        chunk: 15,
        value: 0xBEEF,
    };
    assert_eq!(key.packed(), 0x000F_BEEF);
    assert_eq!(key.stable_key(7), 0x0000_000F_BEEF_0007);
    assert_eq!(
        BucketKey::from_stable_key(key.stable_key(0xFFFF)),
        (key, 0xFFFF)
    );
    let a = BucketKey {
        chunk: 1,
        value: 0xFFFF,
    }
    .stable_key(0xFFFF);
    let b = BucketKey { chunk: 2, value: 0 }.stable_key(0);
    assert!(a < b);
}

#[test]
fn posting_encoding_round_trips() {
    let mut rng = Rng::new(5);
    for _ in 0..200 {
        let posting = Posting {
            local_ix: rng.next() as u32,
            shape: rng.next() as u8,
        };
        assert_eq!(Posting::decode(&posting.encode()), posting);
    }
    assert_eq!(
        Posting {
            local_ix: 0x0102_0304,
            shape: 9
        }
        .encode(),
        [1, 2, 3, 4, 9]
    );
}

#[test]
fn exact_match_ranks_first_with_all_chunks() {
    let mut rng = Rng::new(6);
    let mut store = MemoryStore::default();
    let hashes: Vec<PdqHash> = (0..200).map(|_| random_hash(&mut rng)).collect();
    for (ix, hash) in hashes.iter().enumerate() {
        insert(
            &mut store,
            hash,
            Posting {
                local_ix: ix as u32,
                shape: 1,
            },
        );
    }
    let ranked = Prober::new().probe(&store, &hashes[42], 10);
    assert_eq!(
        ranked.candidates[0],
        Candidate {
            local_ix: 42,
            matched_chunks: 16
        }
    );
    assert!(!ranked.truncated);
    assert_eq!(store.visits.get(), PROBES_PER_HASH);
}

#[test]
fn pigeonhole_guarantee_for_adversarial_spreads() {
    let mut rng = Rng::new(7);
    for trial in 0..200 {
        let base = random_hash(&mut rng);
        let mut store = MemoryStore::default();
        insert(
            &mut store,
            &base,
            Posting {
                local_ix: 5,
                shape: 0,
            },
        );
        let lucky = rng.below(16) as usize;
        let bits: Vec<usize> = (0..MIH_CHUNKS)
            .flat_map(|c| {
                let first = rng.below(16) as usize;
                let second = (first + 1 + rng.below(15) as usize) % 16;
                let picks = if c == lucky {
                    vec![first]
                } else {
                    vec![first, second]
                };
                picks.into_iter().map(move |b| c * 16 + b)
            })
            .collect();
        assert_eq!(bits.len(), 31);
        let query = flip_permuted_bits(&base, &bits);
        assert_eq!(query.distance(&base), 31);
        assert!(found(&store, &query, 5), "trial {trial}");
    }
}

#[test]
fn radius_is_tight_at_two_bits_per_chunk() {
    let mut rng = Rng::new(8);
    let base = random_hash(&mut rng);
    let mut store = MemoryStore::default();
    insert(
        &mut store,
        &base,
        Posting {
            local_ix: 1,
            shape: 0,
        },
    );
    let bits: Vec<usize> = (0..MIH_CHUNKS).flat_map(|c| [c * 16, c * 16 + 9]).collect();
    let query = flip_permuted_bits(&base, &bits);
    assert_eq!(query.distance(&base), 32);
    assert!(!found(&store, &query, 1));
    assert_eq!(MIH_RADIUS, 31);
}

#[test]
fn every_pair_within_radius_is_found_property() {
    let mut rng = Rng::new(9);
    let mut store = MemoryStore::default();
    let mut stored: Vec<PdqHash> = Vec::new();
    for ix in 0..400u32 {
        let hash = if ix % 4 == 0 || stored.is_empty() {
            random_hash(&mut rng)
        } else {
            let parent = stored[rng.below(stored.len() as u64) as usize];
            let distance = rng.below(48) as usize;
            flip_random_bits(&mut rng, &parent, distance)
        };
        insert(
            &mut store,
            &hash,
            Posting {
                local_ix: ix,
                shape: (ix % 19) as u8,
            },
        );
        stored.push(hash);
    }
    let mut prober = Prober::with_capacity(64);
    for _ in 0..150 {
        let anchor = stored[rng.below(stored.len() as u64) as usize];
        let count = rng.below(40) as usize;
        let query = flip_random_bits(&mut rng, &anchor, count);
        let candidates: BTreeSet<u32> = prober
            .probe(&store, &query, usize::MAX)
            .candidates
            .iter()
            .map(|c| c.local_ix)
            .collect();
        for (ix, hash) in stored.iter().enumerate() {
            if hash.distance(&query) <= MIH_RADIUS {
                assert!(
                    candidates.contains(&(ix as u32)),
                    "missed {ix} at {}",
                    hash.distance(&query)
                );
            }
        }
    }
}

#[test]
fn shapes_of_one_capture_are_deduplicated() {
    let mut rng = Rng::new(10);
    let base = random_hash(&mut rng);
    let mut store = MemoryStore::default();
    for shape in 0..19u8 {
        insert(&mut store, &base, Posting { local_ix: 3, shape });
    }
    let ranked = Prober::new().probe(&store, &base, 100);
    assert_eq!(
        ranked.candidates,
        vec![Candidate {
            local_ix: 3,
            matched_chunks: 16
        }]
    );
}

#[test]
fn matched_chunk_count_equals_chunks_within_one_bit() {
    let mut rng = Rng::new(11);
    for _ in 0..100 {
        let base = random_hash(&mut rng);
        let mut store = MemoryStore::default();
        insert(
            &mut store,
            &base,
            Posting {
                local_ix: 77,
                shape: 2,
            },
        );
        let count = rng.below(60) as usize;
        let query = flip_random_bits(&mut rng, &base, count);
        let expected = chunks(&base)
            .iter()
            .zip(chunks(&query))
            .filter(|(a, b)| (**a ^ *b).count_ones() <= 1)
            .count() as u8;
        let ranked = Prober::new().probe(&store, &query, 10);
        let got = ranked.candidates.first().map_or(0, |c| c.matched_chunks);
        assert_eq!(got, expected);
    }
}

#[test]
fn ranking_orders_by_matched_chunks_then_index() {
    let mut rng = Rng::new(12);
    let base = random_hash(&mut rng);
    let mut store = MemoryStore::default();
    let variants = [(9u32, 0usize), (4, 40), (2, 20), (8, 20), (1, 0)];
    for &(ix, distance) in &variants {
        let hash = flip_permuted_bits(
            &base,
            &(0..distance)
                .map(|i| (i % 16) * 16 + i / 16 * 2 + 4)
                .collect::<Vec<_>>(),
        );
        insert(
            &mut store,
            &hash,
            Posting {
                local_ix: ix,
                shape: 0,
            },
        );
    }
    let ranked = Prober::new().probe(&store, &base, 10).candidates;
    let order: Vec<u32> = ranked.iter().map(|c| c.local_ix).collect();
    assert_eq!(&order[..2], &[1, 9]);
    assert!(
        ranked
            .windows(2)
            .all(|w| (w[0].matched_chunks, core::cmp::Reverse(w[0].local_ix))
                >= (w[1].matched_chunks, core::cmp::Reverse(w[1].local_ix)))
    );
}

#[test]
fn truncation_flags_ties_at_the_cut() {
    let mut rng = Rng::new(13);
    let base = random_hash(&mut rng);
    let mut store = MemoryStore::default();
    for ix in 0..10u32 {
        insert(
            &mut store,
            &base,
            Posting {
                local_ix: ix,
                shape: 0,
            },
        );
    }
    let mut prober = Prober::new();
    let cut = prober.probe(&store, &base, 4);
    assert_eq!(cut.candidates.len(), 4);
    assert!(cut.truncated);
    assert_eq!(
        cut.candidates
            .iter()
            .map(|c| c.local_ix)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert!(!prober.probe(&store, &base, 10).truncated);
    assert!(prober.probe(&store, &base, 0).truncated);
    let far = flip_permuted_bits(
        &base,
        &(0..15)
            .flat_map(|c| [c * 16, c * 16 + 1])
            .collect::<Vec<_>>(),
    );
    insert(
        &mut store,
        &far,
        Posting {
            local_ix: 100,
            shape: 0,
        },
    );
    let untied = prober.probe(&store, &base, 10);
    assert_eq!(untied.candidates.len(), 10);
    assert!(!untied.truncated);
}

#[test]
fn empty_store_returns_nothing() {
    let store = MemoryStore::default();
    let ranked = Prober::new().probe(&store, &PdqHash::default(), 2000);
    assert_eq!(ranked, Ranked::default());
}

#[test]
fn prober_reuse_matches_fresh_prober() {
    let mut rng = Rng::new(14);
    let mut store = MemoryStore::default();
    let hashes: Vec<PdqHash> = (0..300).map(|_| random_hash(&mut rng)).collect();
    for (ix, hash) in hashes.iter().enumerate() {
        insert(
            &mut store,
            hash,
            Posting {
                local_ix: ix as u32 * 997 % 5000,
                shape: 0,
            },
        );
    }
    let mut reused = Prober::new();
    for hash in hashes.iter().take(40) {
        let query = flip_random_bits(&mut rng, hash, 20);
        assert_eq!(
            reused.probe(&store, &query, 50),
            Prober::new().probe(&store, &query, 50)
        );
    }
}

#[test]
fn large_local_indices_are_handled() {
    let mut rng = Rng::new(15);
    let base = random_hash(&mut rng);
    let mut store = MemoryStore::default();
    insert(
        &mut store,
        &base,
        Posting {
            local_ix: 3_000_000,
            shape: 0,
        },
    );
    insert(
        &mut store,
        &base,
        Posting {
            local_ix: 0,
            shape: 0,
        },
    );
    let ranked = Prober::with_capacity(16).probe(&store, &base, 5);
    assert_eq!(ranked.candidates.len(), 2);
    assert_eq!(ranked.candidates[1].local_ix, 3_000_000);
}
