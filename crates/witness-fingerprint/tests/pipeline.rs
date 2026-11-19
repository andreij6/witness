#[path = "../src/testkit.rs"]
mod testkit;

use std::collections::BTreeMap;
use std::sync::OnceLock;
use testkit::{grey, jpeg_round_trip, paste, scene};
use witness_fingerprint::{
    Anchor, Aspect, BucketKey, Grid, MIH_RADIUS, PdqHash, Posting, PostingStore, Prober, ShapeTag,
    align, fingerprint, insert, query_hashes,
};
use witness_format::{FingerprintBundle, Rect};

#[derive(Default)]
struct Shard {
    buckets: BTreeMap<u32, Vec<[u8; 5]>>,
    rows: Vec<FingerprintBundle>,
    dims: Vec<(u32, u32)>,
}

impl PostingStore for Shard {
    fn insert(&mut self, key: BucketKey, posting: Posting) {
        self.buckets
            .entry(key.packed())
            .or_default()
            .push(posting.encode());
    }

    fn visit(&self, key: BucketKey, mut sink: impl FnMut(Posting)) {
        if let Some(bucket) = self.buckets.get(&key.packed()) {
            bucket.iter().for_each(|bytes| sink(Posting::decode(bytes)));
        }
    }
}

impl Shard {
    fn put(&mut self, bundle: FingerprintBundle, dims: (u32, u32)) -> u32 {
        let local_ix = self.rows.len() as u32;
        if bundle.quality() >= witness_fingerprint::MIN_INDEX_QUALITY {
            for shape in bundle.pdq() {
                insert(
                    self,
                    &PdqHash(shape.hash),
                    Posting {
                        local_ix,
                        shape: shape.tag,
                    },
                );
            }
        }
        self.rows.push(bundle);
        self.dims.push(dims);
        local_ix
    }

    fn lookup(&self, query: &PdqHash) -> Vec<(u32, u32)> {
        let ranked = Prober::new().probe(self, query, 2000);
        let mut hits: Vec<(u32, u32)> = ranked
            .candidates
            .iter()
            .filter_map(|c| {
                let row = &self.rows[c.local_ix as usize];
                let best = row
                    .pdq()
                    .iter()
                    .map(|s| PdqHash(s.hash).distance(query))
                    .min()?;
                (best <= MIH_RADIUS).then_some((best, c.local_ix))
            })
            .collect();
        hits.sort_unstable();
        hits
    }
}

fn whole(w: u32, h: u32) -> Rect {
    Rect { x: 0, y: 0, w, h }
}

const W: u32 = 960;
const H: u32 = 720;

fn shard() -> &'static Shard {
    static SHARD: OnceLock<Shard> = OnceLock::new();
    SHARD.get_or_init(|| {
        let mut shard = Shard::default();
        for seed in 0..24u64 {
            let (w, h) = if seed % 3 == 0 { (H, W) } else { (W, H) };
            shard.put(fingerprint(scene(w, h, seed).view()).unwrap(), (w, h));
        }
        shard
    })
}

#[test]
fn platform_copies_are_found_and_confirmed() {
    let shard = shard();
    for seed in [1u64, 4, 7, 10] {
        let original = scene(W, H, seed);
        for tag in [
            ShapeTag::FULL,
            ShapeTag::new(Aspect::Square, Anchor::Center),
            ShapeTag::new(Aspect::Landscape16x9, Anchor::Start),
        ] {
            let r = tag.rect(W, H).unwrap();
            let copy = jpeg_round_trip(&original.crop(r.x, r.y, r.w, r.h).unwrap().to_image(), 72);
            let queries = query_hashes(copy.view(), &[whole(copy.width(), copy.height())]);
            let found: Vec<u32> = queries
                .iter()
                .flat_map(|q| shard.lookup(&q.hash))
                .map(|(_, ix)| ix)
                .collect();
            assert!(
                found.contains(&(seed as u32)),
                "seed {seed} {tag:?} found {found:?}"
            );
            let reference = Grid::from_bundle(&shard.rows[seed as usize]);
            let alignment =
                align(&reference, shard.dims[seed as usize], grey(&copy).view()).unwrap();
            assert!(
                alignment.accepted,
                "seed {seed} {tag:?} {:?}",
                alignment.best
            );
            assert_eq!(alignment.best.hypothesis.tag, tag);
        }
    }
}

#[test]
fn unrelated_photos_are_not_found() {
    let shard = shard();
    for seed in 500..510u64 {
        let other = scene(W, H, seed);
        let queries = query_hashes(other.view(), &[whole(W, H)]);
        assert!(
            queries.iter().all(|q| shard.lookup(&q.hash).is_empty()),
            "seed {seed}"
        );
    }
}

#[test]
fn edited_copy_is_confirmed_with_localised_changes() {
    let shard = shard();
    let seed = 5u64;
    let original = scene(W, H, seed);
    let edited = jpeg_round_trip(&paste(&original, &scene(240, 180, 4242), 360, 240), 80);
    let alignment = align(
        &Grid::from_bundle(&shard.rows[seed as usize]),
        (W, H),
        grey(&edited).view(),
    )
    .unwrap();
    assert!(alignment.accepted);
    let changed: Vec<usize> = (0..192)
        .filter(|&c| alignment.diff.distance(c).is_some_and(|d| d >= 64))
        .collect();
    assert!(!changed.is_empty());
    assert!(
        changed
            .iter()
            .all(|&c| (5..=11).contains(&(c % 16)) && (3..=8).contains(&(c / 16))),
        "{changed:?}"
    );
}

#[test]
fn mirrored_repost_is_found_through_mirror_query() {
    let shard = shard();
    let original = scene(W, H, 8);
    let mirrored = jpeg_round_trip(&witness_image::flip_horizontal(original.view()), 75);
    let queries = query_hashes(mirrored.view(), &[whole(W, H)]);
    assert!(
        queries
            .iter()
            .any(|q| shard.lookup(&q.hash).iter().any(|&(_, ix)| ix == 8))
    );
}
