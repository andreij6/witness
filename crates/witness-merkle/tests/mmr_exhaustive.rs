mod common;

use common::{Rng, record};
use witness_merkle::hash::{mmr_bag, mmr_leaf, mmr_node, mmr_root};
use witness_merkle::{ConsistencyProof, Hash, InclusionProof, Mmr, ZERO};

const MAX: u64 = 300;

fn perfect(leaves: &[Hash]) -> Hash {
    match leaves {
        [one] => *one,
        _ => {
            let (l, r) = leaves.split_at(leaves.len() / 2);
            mmr_node(&perfect(l), &perfect(r))
        }
    }
}

fn reference_root(leaves: &[Hash]) -> Hash {
    let n = leaves.len();
    let mut peaks = Vec::new();
    let mut start = 0;
    for h in (0..usize::BITS).rev() {
        if n & (1 << h) != 0 {
            peaks.push(perfect(&leaves[start..start + (1 << h)]));
            start += 1 << h;
        }
    }
    let bag = peaks.iter().rev().fold(None, |acc: Option<Hash>, p| {
        Some(acc.map_or(*p, |a| mmr_bag(p, &a)))
    });
    mmr_root(n as u64, &bag.unwrap_or(ZERO))
}

fn build() -> (Mmr<Vec<Hash>>, Vec<Hash>) {
    let mut mmr = Mmr::new(Vec::new()).unwrap();
    let mut roots = vec![mmr.root().unwrap()];
    for i in 0..MAX {
        assert_eq!(mmr.append(&record(i)).unwrap(), i);
        roots.push(mmr.root().unwrap());
    }
    (mmr, roots)
}

#[test]
fn roots_match_independent_reference() {
    let (_, roots) = build();
    let leaves: Vec<Hash> = (0..MAX).map(|i| mmr_leaf(&record(i))).collect();
    for n in 0..=MAX as usize {
        assert_eq!(roots[n], reference_root(&leaves[..n]), "n={n}");
    }
}

#[test]
fn every_leaf_of_every_size_verifies_and_tamper_fails() {
    let (mmr, roots) = build();
    let mut rng = Rng::new(0xabc);
    for n in 1..=MAX {
        let root = roots[n as usize];
        for i in 0..n {
            let p = mmr.prove_at(i, n).unwrap();
            assert!(p.verify(&root, &record(i)), "n={n} i={i}");
            assert!(!p.verify(&root, &record(i + MAX)));
            if !p.path.is_empty() {
                let mut t = p.clone();
                let k = rng.below(t.path.len() as u64) as usize;
                t.path[k][rng.below(32) as usize] ^= 1 << rng.below(8);
                assert!(!t.verify(&root, &record(i)));
            }
            let wrong_index = InclusionProof {
                index: (i + 1) % n,
                ..p.clone()
            };
            if n > 1 {
                assert!(!wrong_index.verify(&root, &record(i)));
            }
            let wrong_count = InclusionProof {
                leaf_count: n + 1,
                ..p.clone()
            };
            assert!(!wrong_count.verify(&root, &record(i)));
            if n < MAX {
                assert!(!p.verify(&roots[n as usize + 1], &record(i)));
            }
        }
    }
}

#[test]
fn consistency_exhaustive_small_and_sampled_large() {
    let (mmr, roots) = build();
    for new in 0..=96u64 {
        for old in 0..=new {
            let p = mmr.prove_consistency(old, new).unwrap();
            assert!(
                p.verify(&roots[old as usize], &roots[new as usize]),
                "{old}->{new}"
            );
        }
    }
    let mut rng = Rng::new(0xdef);
    for _ in 0..3000 {
        let new = rng.below(MAX + 1);
        let old = rng.below(new + 1);
        let p = mmr.prove_consistency(old, new).unwrap();
        let (ro, rn) = (roots[old as usize], roots[new as usize]);
        assert!(p.verify(&ro, &rn), "{old}->{new}");
        if !p.path.is_empty() {
            let mut t = p.clone();
            let k = rng.below(t.path.len() as u64) as usize;
            t.path[k][rng.below(32) as usize] ^= 1 << rng.below(8);
            assert!(!t.verify(&ro, &rn));
        }
        if old != new {
            let swapped = ConsistencyProof {
                old_size: new,
                new_size: old,
                path: p.path.clone(),
            };
            assert!(!swapped.verify(&rn, &ro));
            let other = rng.below(MAX + 1);
            if roots[other as usize] != rn {
                assert!(!p.verify(&ro, &roots[other as usize]));
            }
        }
    }
}

#[test]
fn consistency_chains_compose() {
    let (mmr, roots) = build();
    let mut rng = Rng::new(0x123);
    for _ in 0..200 {
        let c = rng.below(MAX + 1);
        let b = rng.below(c + 1);
        let a = rng.below(b + 1);
        for (x, y) in [(a, b), (b, c), (a, c)] {
            assert!(
                mmr.prove_consistency(x, y)
                    .unwrap()
                    .verify(&roots[x as usize], &roots[y as usize])
            );
        }
    }
}

#[test]
fn reopened_store_produces_identical_proofs() {
    let (mmr, _) = build();
    let reopened = Mmr::new(mmr.store().clone()).unwrap();
    for i in (0..MAX).step_by(7) {
        assert_eq!(mmr.prove(i).unwrap(), reopened.prove(i).unwrap());
        assert_eq!(
            mmr.prove_consistency(i, MAX).unwrap(),
            reopened.prove_consistency(i, MAX).unwrap()
        );
    }
}
