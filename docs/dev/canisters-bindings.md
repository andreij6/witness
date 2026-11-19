

# ---- report ----
I've set up the canister foundation, and every check you asked for passes, but nothing has been deployed: the deploy script has only been dry-run against a fake `icp` binary.

**Checks run:**
- `cargo build --workspace` passes.
- `cargo test -p witness-canister-common` passes: 92 tests (89 unit, 3 in `tests/did.rs`).
- The wasm32 release build passes for all five crates, including registry with `--features dev-attestation`.
- Clippy with `-D warnings` and `--all-targets` is clean on the five crates, and on registry with `dev-attestation`. `cargo fmt --check` is clean.
- `cargo test -p witness-index --no-run` compiles pocket-ic 16.1. Its reqwest dependency pulls in aws-lc-sys, which compiled without the cmake tool.
- No getrandom, wasm-bindgen or js-sys in any canister's wasm32 dependency tree.
- `bash -n` passes on all four scripts.
- `scripts/build-canister.sh witness-registry dev-attestation` and `scripts/stage-web.sh` were run for real: the wasm carries the `candid:service` metadata, and the real webApp build was staged.
- The dry run produced the four `icp deploy` calls with the right init args and a valid `.icp-local/ids.json`.

**Files:**
- `canisters/common/` (package `witness-canister-common`): `common.did` is the contract; modules `error`, `registry`, `index`, `media`, `init`, `status`, `timers`, `inspect`, `limits`, `memory`, `runtime`; tests in `tests/did.rs`.
- `canisters/{registry,index,pages,media}` (`witness-registry`, `-index`, `-pages`, `-media`): `cdylib` + `rlib` stubs with `init`/`post_upgrade` taking the init args, a `status` query and `export_candid!`. The registry's `dev-attestation` feature turns on `witness-attest/dev-attestation`. pocket-ic 16.1.0 is a dev-dependency of each.
- Root `Cargo.toml`: the five members, plus workspace deps ic-cdk 0.20.3, ic-cdk-timers 1.0.0, ic-stable-structures 0.7.2, candid 0.10.41, candid_parser 0.4, ic-certification / ic-http-certification 4.0.1, serde, serde_bytes, pocket-ic 16.1.0. Common also uses ic0 1.2.0.
- `icp.yaml` (I chose icp CLI 1.7 over dfx.json), `scripts/{deploy-local,stop-local,build-canister,stage-web}.sh`, `canisters/web/{_redirects,dist-placeholder/}`, `canisters/README.md`.
- `.gitignore`: added `.icp-local/` and `canisters/web/dist/`.

## Shared Candid types (`canisters/common/common.did`)
The Rust name equals the Candid name for every type. `tests/did.rs` fails if a Rust type and the `.did` drift apart, or if a `.did` type has no Rust twin. All are re-exported from the crate root.
```candid
type Security = variant { StrongBox; Tee; Dev };
type Prov = variant { Rkp; Factory; Dev };
type Fresh = variant { Fresh; Late };
type KeyStatus = variant { Active; Retired; Suspended; Revoked };
type StatusChange = record { status : KeyStatus; since_ns : nat64 };
type KeyInfo = record { key_id : blob; pubkey : blob; security : Security; provisioning : Prov; freshness : Fresh; patch : nat32; enrolled_ns : nat64; active_until_ns : nat64; history : vec StatusChange; chain_serials : vec text };
type Submission = record { claim : blob; c2pa_claim : blob; cose : blob; fingerprints : opt blob };
type Anchored = record { record_ix : nat64; record_id : text; anchored_ns : nat64 };
type SubmitErr = variant { malformed : text; too_large; bad_signature; unknown_key; key_distrusted; key_expired; counter_reused; parent_unknown; fingerprint_mismatch; retry_after_ns : nat64 };
type SubmitResult = variant { ok : Anchored; dup : Anchored; err : SubmitErr };
type CertifiedRecord = record { record_ix : nat64; record_id : text; claim : blob; c2pa_claim : blob; cose : blob; anchored_ns : nat64; mmr_proof : vec blob; leaf_count : nat64; certificate : blob };
type CertifiedKey = record { key : KeyInfo; smt_proof : blob; certificate : blob };
type ShardInfo = record { shard : nat8; canister : principal; open : bool };
type Shards = record { index : vec ShardInfo; media : vec ShardInfo };
type EraseAuth = variant { device : blob; ii };
type EraseRequest = record { record_id : text; nonce : blob; auth : EraseAuth };
type Posting = record { record_ix : nat64; wm_root : nat64; asset_hash : blob; pixel_hash : blob; fingerprints : opt blob; root_ix : opt nat64 };
type PdqHit = record { record_ix : nat64; distance : nat32 };
type ProbeResult = record { pdq : vec PdqHit; watermark : vec nat64; exact : vec nat64; truncated_candidates : bool; indexed_upto : nat64 };
type Fingerprints = record { grid : blob; shapes : vec record { nat8; blob } };
type Tier = variant { Plus };
type EntitlementGrant = record { ii_principal : principal; tier : Tier; quota_bytes : nat64; expires_ns : nat64; signature : blob };
type UploadTarget = variant { preview : text; og : text; full : text; case_chunk : record { case_id : text; ix : nat32; count : nat32 } };
type BeginUpload = record { target : UploadTarget; size : nat64; sha256 : blob; key_id : blob; signature : blob; grant : opt EntitlementGrant };
type Chunk = record { upload : nat64; ix : nat32; bytes : blob };
type MediaErr = variant { malformed : text; unauthorized; not_found; too_large; quota_exceeded; grant_invalid; bad_chunk; hash_mismatch; incomplete; retry_after_ns : nat64 };
type TimerStatus = record { name : text; period_ns : nat64; last_run_ns : nat64; runs : nat64 };
type Status = record { version : text; cycles : nat; stable_bytes : nat64; timers : vec TimerStatus; metrics : vec record { text; nat64 }; flags : vec text };
type RegistryInit = record { index : vec principal; media : vec principal; pages : principal };
type PeerInit = record { registry : principal; shard : nat8 };
```

**Helpers on the types (all unit-tested):**
- **Conversions:** the four enums convert both ways with their `witness_format` twins. `KeyInfo` converts from `&witness_format::KeyInfo`, and back with a check that runs `KeyHistory::from_changes` and `validate`. `Fingerprints` converts from `&FingerprintBundle`.
- **Submissions:** `Submission::{within_limits, record_id, into_parts, from_parts}`, `SubmitErr::is_retryable`, `SubmitResult::anchored`, `Shards::{open_index, media_shard}`.
- **Erase:** `EraseRequest::{record_id, signing_bytes, verify_device}` and `erase_signing_bytes`. The signed bytes are `"witness/erase/v1" ‖ record_id (10 raw bytes) ‖ nonce (1–64 bytes)`, verified with ES256.
- **Index:** `Posting::{asset_key, pixel_key}` (the first 16 bytes of each hash), `hash_key`, `index::probe_within_limits`, and `ProbeResult::{is_empty, covers}`.
- **Media:**
  - `EntitlementGrant::{signing_bytes, verify(signer, caller, now)}`: `"witness/grant/v1" ‖ len ‖ principal ‖ tier ‖ quota (8 bytes, big-endian) ‖ expires (8 bytes, big-endian)`, verified with Ed25519.
  - `UploadTarget::{code, record_id, case_id, max_bytes}`.
  - `BeginUpload::{key_id, chunk_count, check_shape, signing_bytes, verify(spki)}`: `"witness/upload/v1" ‖ key_id ‖ target code ‖ target ids (and ix, count for case chunks) ‖ size (8 bytes, big-endian) ‖ sha256`, verified with ES256.
  - `Chunk::within_limits`, and `From<ContractError> for MediaErr`.
- **Init and status:** `RegistryInit`/`PeerInit` are `Storable` (Candid-encoded) with `Default` and `is_configured`. `Status::{new, metric, has_flag}` and `TimerStatus::is_overdue` (true after 2× the period).

**Runtime helpers:**
- `timers`: `TimerSpec { name, period, tick: fn() -> Tick }`, `validate_specs`, and a pure `TimerBook`.
- `inspect`: `MethodLimit`, `limit()`, `arg_allowed(limits, default_max: Option<usize>, method, len)`. With `None`, methods not in the list are rejected.
- `runtime` (IC-only): `set_certified_root`, `certificate`, `inspect` (reads the argument size without copying it), `start_timers(&'static [TimerSpec])` (call from both `init` and `post_upgrade`; ticks never overlap), `timer_statuses`, `status`.
- `limits`: the shared caps (batch 64, probe 16/8/4, chunk 1 MiB, case 16 MB, preview 150 KB, og 60 KB, and so on).

## MemoryId allocation (`common::memory`)
Each canister has its own manager, reached through `memory(id)`. Ids 0–7 are common: 0 is `CONFIG` (the init-args cell) and 1–7 are reserved. Ids 24–31 are spare, to be added in common. Ids 32–254 come from `memory::local(n)` and are private to the canister, so agents don't need to edit common for extra stores.

| Id | registry | index | pages | media |
|---|---|---|---|---|
| 8 | MMR_NODES | PDQ_ROWS | CERT_TREE | BLOBS |
| 9 | MMR_PEAKS | GRIDS | SHELLS | BLOB_INDEX |
| 10 | SMT_NODES | POSTINGS | STATE | UPLOADS |
| 11 | RECORDS | WATERMARK | | QUOTAS |
| 12 | RECORD_BY_ID | EXACT_ASSET | | GRANT_USAGE |
| 13 | KEYS | EXACT_PIXEL | | EXPIRY |
| 14 | PENDING_KEYS | DERIVATIVES | | CERT_TREE |
| 15 | COUNTERS | LOCAL_TO_RECORD | | STATE |
| 16 | BY_CAPTURE | RECORD_TO_LOCAL | | |
| 17 | DERIVATIVES | STATE | | |
| 18–23 | TOMBSTONES, OUTBOX, QUOTAS, REVOCATIONS, II_LINKS, STATE | | | |

## Deployment
- `icp.yaml` uses script builds through `build-canister.sh` (cargo release, candid-extractor, `ic-wasm shrink`, Candid metadata), with registry built with `dev-attestation`. `web` uses the `@dfinity/static-site@v0.4.2` recipe; `stage-web.sh` falls back to the placeholder when the webApp build has no `index.html`.
- The gateway is on **127.0.0.1:8090**, because port 8000 is already taken by another project's running icp network.
- `deploy-local.sh`:
  1. Pings the network and starts it in the background if needed.
  2. Creates any missing canisters.
  3. Reads the canister ids.
  4. Deploys registry with `RegistryInit` and the three peers with `PeerInit { registry; shard = 0 }`, always with `--yes`. The same args are passed on upgrade.
  5. Deploys `web`.
  6. Writes `.icp-local/ids.json` (`{network, gateway, canisters: {name: {id, url}}}`) and prints the ids and URLs.
- The scripts avoid bash 4 features because macOS ships bash 3.2.

## Deviations from the architecture doc
- **`Submission` has an extra `fingerprints : opt blob` field** (the FingerprintBundle CBOR). Without it the registry can't check `fp_commit` or forward the bundle to `index`.
- **`CertifiedKey.smt_proof` is a single `blob`**, because `witness-merkle`'s `SmtProof` already has `to_bytes`/`from_bytes`. The doc says `vec blob`.
- **`Err` is renamed `SubmitErr`**, so it doesn't shadow `Result::Err` when glob-imported.
- **`KeyInfo` and `KeyStatus` have the same names as the `witness_format` types.** Import them with a module path.
- **`ProbeResult.indexed_upto` is exclusive:** "indexed" means `record_ix < indexed_upto`.
- **No HTTP certification helper:** ic-http-certification already covers it, and `pages` builds its own tree in stable memory.
- **`Enrollment` is left to the registry** because only the registry uses it.

## Open issues
- **No native tests for `runtime`:** the IC-only functions can't run outside a canister. The canisters' PocketIC tests will have to cover them.
- **New signing domains:** `witness/upload/v1` and `witness/grant/v1` live in common. I suggest moving them into `witness_format::crypto::domain` (the format crate owner's call).
- **Deploy-time download:** the static-site recipe is fetched from GitHub when `web` is first deployed.
- **icp CLI version:** the CLI reports that 1.8.0 is available; I stayed on 1.7.0.
- **Workspace members:** `crates/witness-android` and `crates/witness-web` are not workspace members. That isn't my lane, so I left it.

# ---- report ----
I've built the `pages` canister (`canisters/pages/`) and every check passes: 77 unit tests, 5 PocketIC tests, clippy with `-D warnings`, the wasm32 release build and the Candid export. Nothing was deployed or committed.

**Commands run in the repo root:**
- `cargo test -p witness-pages`: 77 unit and 5 PocketIC tests pass.
- `cargo clippy -p witness-pages --all-targets -- -D warnings` is clean, and so is `cargo fmt --check`.
- `cargo build -p witness-pages --target wasm32-unknown-unknown --release` produces a 1.35 MB wasm. There is no getrandom, wasm-bindgen, js-sys or aws-lc in its dependency tree.
- `candid-extractor` writes `canisters/pages/pages.did` with comment lines stripped. A unit test compares the file with the canister's exported interface.

**Certification tree:** it lives in stable memory and nothing is rebuilt on upgrade.
- The `/p/{record_id}` level is a binary trie over record ids, keyed by the 16-byte id text and stored in `StableBTreeMap` (`CERT_TREE`), with the root in `STATE`. Each node keeps its children's hashes.
- Insert, update, delete and witness each touch one root-to-leaf path. Depth is about log₂ n for random ids and never more than 128.
- The tree layout is the v2 format. Witnesses are ordinary `ic_certification` hash trees and include the "no more specific path" proofs that `ic-http-certification` uses.
- The static entries are `/.well-known/ic-domains`, the 404 fallback for every path and the `/v/*` case shell. They are small and fully revealed. `post_upgrade` only re-sets the certified root.
- `put_shell` and `tombstone` re-certify in the same update.

**Pages:**
- **`/p/{id}` shell:** rendered from the claim with OG and Twitter tags, canonical URL, the summary in `<main>`, the inline JSON and the `boot.mjs` script. It uses the proof-site CSP.
  - The headline matches the design, e.g. "Matches an original, with 2 declared blurs".
  - The coarse area is shown as "near 37.8° N, 122.4° W", and dates are in UTC.
  - Shells from dev keys get a DEV badge and are marked noindex.
- **`/v/*`:** one generic, noindex case shell with `Referrer-Policy: no-referrer` and the nonce + `'strict-dynamic'` CSP. The nonce is fixed per release.
- **Tombstones:** answered with 410 and a different text for erased, takedown and revoked. Once a record is tombstoned, `put_shell` is refused, even if the tombstone arrived before the shell.
- **Everything else:** a certified 404.

**Tests:**
- **Unit:**
  - About 4,000 random inserts and deletes, comparing the root hash and node count against a reference tree after every step.
  - Neighbour lookups against a `BTreeSet`, and absence proofs checked with the ic-certification lookup functions.
  - Entry hashes compared with `ic-http-certification`'s own tree.
  - Every 404 path checked for its absence proofs.
  - HTML escaping with hostile input, and determinism checks.
- **PocketIC:**
  - Every route is checked with `ic-response-verification` against the PocketIC root key.
  - A response with a forged body fails verification.
  - Only the registry can write, and batches over 64 are rejected.
  - After an upgrade, all 64 pages still verify.
  - The live HTTP gateway serves the right status, `x-robots-tag` and body. I didn't confirm that it actually checks certificates, so the cryptographic checks are the explicit verification calls.

**Candid** (`pages.did`, with the standard `HttpRequest`/`HttpResponse` types):
```candid
type Removal = variant { erased; takedown; revoked };
type ShellInput = record { claim : blob; media : principal; media_shard : nat8; anchored_ns : nat64; dev : bool };
type PageErr = variant { malformed : text; gone };
type PageResult = variant { ok : text; err : PageErr };
service : (PeerInit) -> {
  put_shell : (vec ShellInput) -> (vec PageResult);
  tombstone : (vec text, Removal) -> (vec PageResult);
  http_request : (HttpRequest) -> (HttpResponse) query;
  status : () -> (Status) query;
}
```
- `put_shell` takes at most 64 items and accepts calls only from the registry. The record id is computed from the claim.
- `tombstone` accepts lowercase or grouped ids and returns the canonical one.
- `status` reports the number of pages and tree nodes, plus an `unconfigured` flag.

**Contract changes needed in common:**
- **Move the four types into common:** `ShellInput`, `Removal`, `PageErr` and `PageResult` are defined in `witness-pages`. The registry can't depend on another canister crate without pulling its canister exports into its own wasm.
- **Registry obligations:**
  - Call `put_shell` with the media canister for the record's shard and the `dev` flag (set when the key's provisioning is Dev).
  - Call `tombstone` from `erase_mine`, takedown and revocation.

**Deviations:**
- **Lowercase ids get a 404:** only the canonical uppercase 16-character id path is certified, so links need the uppercase id.
- **Extra CSP rules:** I added `base-uri 'none'; frame-ancestors 'none'` to both policies. Tombstones and the 404 use a stricter policy with no scripts allowed.
- **Messages from anyone but the registry are refused:** this includes direct (ingress) calls. In practice the registry calls canister-to-canister, which skips that check.

**Open issues:**
- **Media route:** the `og:image` URL is `https://{media}.icp0.io/og/{RECORD_ID}`. The media canister has to serve that path.
- **Placeholder domains:** `witness.app` and `app.witness.app` are compiled in, so the boot script doesn't load on the local replica.
- **Case-shell import map:** the shell has no pinned import map yet. It needs a build-time file from the `web` canister.
- **Changing headers:** a page's certified hash covers its HTTP headers. Changing the shell's headers in a later release would need a re-certification pass that runs over several timer ticks, because there are too many pages for one upgrade.
- **Shared files touched:**
  - `Cargo.lock` gained the new dev-dependencies (`ic-response-verification`, `serde_cbor`, `base64`, `reqwest` blocking).
  - The PocketIC tests build the wasm into `target/pocket-pages`, so they don't block on the main target directory. You can point them at a prebuilt wasm with `WITNESS_PAGES_WASM`.

Everything is in `canisters/pages/`:
- `Cargo.toml`
- `pages.did`
- `src/`: `lib.rs`, `api.rs`, `critbit.rs`, `store.rs`, `cert.rs`, `site.rs`, `http.rs`, `html.rs`, `view.rs`
- `tests/pocket.rs`

# ---- report ----
The QA bug is fixed. A plain recompress of a sealed photo now comes back as `MATCHES_WITH_CHANGES` with no couldn't-compare flag and no regions, at every size from 96 to 2048 px and every quality from 50 to 95. §6.5 gives that verdict when a stripped-credential copy has identical content. The tests for false accusations and forged originals still pass. The golden digest changed, for a reason explained below.

## Root cause
It was not too few ORB keypoints: alignment confidence was about 0.98 and the compared fraction about 0.9, above the 0.75 bar for a full compare. The flag came from `Classification::Inconclusive` (`InBorderBand` or `NotRinged`), which sets `COULDNT_COMPARE` in `decide`. Two separate defects caused it:

1. **Alignment error.** For a pair that is really the identity, RANSAC still returns a slightly wrong transform. At 1024×768 it gave about 0.2° of rotation and a (−1.8, 2.0) px shift. At low quality, that misregistration turns JPEG artefacts along edges into changed components.
2. **Change-size threshold.** The smallest component that counts as a change scales with the image (half a grid cell, or 0.2% of the frame), but codec artefacts are a fixed size in pixels. I measured artefact components of up to 125 px even with perfect alignment. Below about 256 px wide that is above the threshold, so even perfectly aligned recompresses failed: 33 of 48 at 96×72, 8 of 48 at 160×120, 3 of 48 at 192×144.

## Fix
- **`witness-match` geometry:**
  - `Similarity::resize(from, to) -> Option<Similarity>` returns a pure-scale transform when the aspect ratio matches to within 1 px.
  - `Similarity::max_corner_shift(&other, w, h) -> f64` measures the worst corner distance between two transforms (infinite if either is not finite; uses `sqrt`, not `hypot`, so results match on wasm32).
- **`witness-verdict/src/confirm.rs`, `by_pixels`:** if the RANSAC transform lands within `snap_px` of the plain-resize transform (2% of the long edge, at least 3 px), the plain resize is also tried. Each hypothesis is diffed, and the one with the fewest changes wins (`Comparison::fit`). The plain resize is tried first and the loop stops at the first consistent result, so a clean recompress still costs one diff. Because both hypotheses are within 2% of each other, a real local edit shows up under either one.
- **`witness-match` classify/diff:**
  - New field `ClassifyParams.min_area_px` (default 0, so the grid path is unchanged).
  - `DiffParams::default()` sets it to the new public constant `MIN_CHANGE_AREA_PX = 256`, one 16×16 JPEG chroma block. It only matters for frames under about 362×272; at 480×360 and above the existing threshold is already larger.

## Tests
- **Count:** core crates 1,322 passing (was 1,298), 0 failing.
- **New tests in `witness-verdict`:**
  - the QA case at 192×144, q50 to q95;
  - sizes 96×72, 72×96, 128×96, 160×120, 192×144, 256×192, 640×480 and 1024×768 at q50, 65, 80 and 95;
  - 2048×1536 at q50, as a separate native-only test because it is slow on wasm;
  - 12 seeded small recompresses;
  - a 28×28 edit on a 192×144 photo is still `ALTERED`;
  - determinism;
  - unit tests for the new hypothesis and selection code.
- **New tests in `witness-match`:** the new geometry and classify helpers and their defaults, plus integration tests in `tests/diff.rs` across sizes and qualities, the 1024 alignment-jitter case, an above-threshold edit (reported as a change), and an edit below 256 px (not reported).
- **wasm32:** every `#[test]` in `witness-core` (`src/seal.rs`, `tests/e2e.rs`) and in `witness-verdict/src` now also runs as a wasm test via the two `cfg_attr` lines. I added `wasm-bindgen-test = "=0.3.80"` as a wasm32-only dev-dependency in `witness-verdict/Cargo.toml`.

## Commands run
- `cargo test` with `-p` for format, image, fingerprint, match, c2pa, merkle, attest, watermark, verdict and core: all pass. Canister crates were excluded.
- `cargo test -p witness-core --target wasm32-unknown-unknown`: unit 10, e2e 13, golden 1, all pass. The golden digest matches on both platforms.
- `cargo test -p witness-verdict --target wasm32-unknown-unknown`: 82 pass.
- `cargo clippy -p witness-match -p witness-verdict -p witness-core --all-targets -- -D warnings`: clean, both native and wasm32.
- `cargo fmt` on the three crates.

## Golden digest
It changed from `07ac345d…5da7708` to `0aaed78d65202ec96219dc38ce80176a19d791db8f89b14fb0f5255121659d89`. I ran the golden suite with the fix switched off and on: only the four q70 reference-recompress verdicts differ. They are exactly the QA case (192×144), going from `COULDNT_COMPARE` with changed area 17531 or 8338 to flags 0 and changed area 0. The other 12 golden verdicts, including the cropped copies that correctly stay couldn't-compare, are unchanged.

## Decisions for you
- **Small-image trade-off:** on frames under about 362×272, an edit smaller than 256 px is no longer reported as `ALTERED`; at that size it can't be told apart from codec artefacts, and no false accusation is the rule. Raise or lower `MIN_CHANGE_AREA_PX` if you want a different balance.
- **Public API change:** `ClassifyParams` has a new public field. No struct literal outside `witness-match` builds it without `..Default::default()`, so nothing breaks today.

## Open issues
- The RANSAC estimate is still imprecise for crops and rotations, where the plain-resize hypothesis doesn't apply. A low-quality copy that was also cropped can still be couldn't-compare. Fixing that properly needs a pixel-level refinement step.
- Workspace crates build without optimisation in dev, so the 2048 debug test takes about 25 s. Adding `[profile.dev.package.witness-match] opt-level = 2` to the root `Cargo.toml` would speed it up; that file is outside my lane.

## Files changed
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-match/src/{geometry.rs,lib.rs,classify/mod.rs,diff/mod.rs}`
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-match/tests/diff.rs`
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-verdict/Cargo.toml`
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-verdict/src/{confirm.rs,tests.rs,candidates.rs,exact.rs,query.rs,verdict.rs}`
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-core/src/seal.rs`
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-core/tests/{e2e.rs,golden.rs}`

Nothing was committed.

# ---- report ----
The `witness-index` canister is built and all the checks you listed pass. A 16-hash probe over 10,000 inserted captures (17 shapes each) costs **387M instructions**, measured with the ic-cdk performance counter inside PocketIC. That is about 19% of the 2B shard-close threshold. Nothing outside `canisters/index/` was edited.

## What I built (`canisters/index/`)
The storage logic is generic over `ic_stable_structures::Memory`, so it is unit-tested natively on `VectorMemory` / `MemoryManager`.

- **`row.rs`, `Row`:** one fixed 630-byte row per capture: shape count, quality, portrait flag, 19 tags and 19×32 hash bytes, all contiguous. It provides `from_bundle`, `shapes`, `min_distance` and `indexed()` (quality ≥ `MIN_INDEX_QUALITY`). Rows live in a `StableVec` at `PDQ_ROWS` and grids in a `StableVec<[u8;384]>` at `GRIDS`.
- **`buckets.rs`, `Buckets`:** the postings store, a `StableBTreeMap<u64, Blob<1020>>` keyed by `BucketKey::stable_key(sub)` from witness-fingerprint, i.e. `chunk<<32 | value<<16 | sub`.
  - Each block holds up to 204 packed 5-byte `(local_ix, shape)` entries.
  - Every block except the last stays full, so reads are point `get`s on sub 0, 1, … rather than range scans.
  - It implements witness-fingerprint's `PostingStore`, so `Prober`, `index_keys`, `probe_keys` and the permutation are reused unchanged.
  - Removal swaps the bucket's tail entry into the gap, and buckets are never truncated.
- **`multimap.rs`, `Multimap<K>`:** a `StableBTreeMap<(K, u64), ()>` with a cap applied at insert that keeps the earliest record_ix. It backs:
  - watermark: root capture_id → up to 32 records
  - asset and pixel hash prefixes: 16 bytes → up to 8 records each
  - derivatives: root → derivatives, not capped on insert, read capped at 32
- **`trace.rs`, `Trace`:** a fixed 53-byte entry per record at `RECORD_TO_LOCAL`, holding everything needed to erase it. An erased record keeps a data-free tombstone so that late or retried `put`s are ignored.
- **`store.rs`, `Index<M>`:** `init`, `put`, `put_all`, `erase`, `probe`, `fingerprints`, `derivatives`, `indexed_upto`, `stats`.
  - `put` is idempotent and decodes fingerprints with `from_slice_canonical`. Malformed or non-canonical fingerprints, or more than 19 shapes, mean no row is stored, but the other lookups are still indexed.
  - Each capture's bucket keys are sorted and grouped, so each bucket is written once.
  - `probe` runs MIH per query hash with the top 2,000 candidates and the tie-truncation flag. Candidates from all queries are merged by `local_ix` with a u16 mask of the queries that found them, so each row is read once. Hits with distance ≤ 31 (`MIH_RADIUS`) are kept at the minimum distance per record, sorted by `(distance, record_ix)`, and capped at 32 with `truncated_candidates` set when the cap cuts.
  - Exact lookups check each hash against both the asset and pixel tables. Watermark and exact results are sorted and de-duplicated.
- **`guard.rs`:** per-method argument limits and the registry-only caller check. `inspect_message` rejects ingress `put`/`erase` from anyone but the registry; the update methods also trap for other callers.
- **`lib.rs`:** the canister glue. `init`/`post_upgrade` store `PeerInit` in `CONFIG`, and nothing is rebuilt on upgrade.

## Candid (`canisters/index/index.did`, from candid-extractor)
```candid
service : (PeerInit) -> {
  derivatives : (nat64) -> (vec nat64) query;
  erase : (nat64) -> ();
  fingerprints : (nat64) -> (opt Fingerprints) query;
  probe : (vec blob, vec nat64, vec blob) -> (ProbeResult) query;
  put : (vec Posting) -> ();
  status : () -> (Status) query;
}
```
All types come from `common.did`. `status` reports the metrics `records`, `rows`, `posting_blocks`, `indexed_upto` and `shard`, plus an `unconfigured` flag. The `bench` cargo feature, which is off by default and absent from `index.did`, adds `probe_instructions` and `put_instructions` for measurement.

## Tests
- **77 tests:** 73 unit and 4 PocketIC.
- **Unit tests** cover:
  - recall of every near-duplicate up to distance 31
  - caps that resist floods
  - erase removing every trace, and tombstones blocking late puts
  - truncation at the 2,000-candidate tie
  - reopening from the same memory with no rebuild
  - determinism across instances
  - rejection of malformed input and probe limits
- **PocketIC tests** cover:
  - registry-only writes
  - malformed probe and oversized ingress being rejected
  - upgrade preserving data
  - the 10k benchmark

## Commands verified
- `cargo test -p witness-index`
- `cargo clippy -p witness-index --all-targets -- -D warnings`, also with `--features bench`
- `cargo fmt --check`
- `cargo build -p witness-index --target wasm32-unknown-unknown --release`
- `candid-extractor … > canisters/index/index.did`

The PocketIC test builds its own bench wasm into `target/pocket-ic` so it doesn't touch the deploy build. PocketIC uses the server binary the pocket-ic crate fetches itself.

## Instructions (10,000 captures × 17 shapes, 970k posting blocks)
| Operation | Instructions |
|---|---|
| Probe, 1 hash | 31.2M |
| Probe, 16 hashes | 387M |
| Probe, 16 hashes + 8 watermark ids + 4 exact hashes | 389M |
| Probe, 16 hashes with no matches | 387M |
| Put, per capture | 36M |
| Put, per batch of 64 | 2.3B average, 2.54B max |

Almost all of the probe cost is the 272 B-tree point reads per query hash, about 90k instructions each at roughly 1M keys. I tried other block sizes:

| Block size | Probe, 16 hashes | Put, per capture |
|---|---|---|
| 4 KiB | 420M | 46M |
| 1 KiB (chosen) | 387M | 36M |
| 512 B | 350M | 33M |

I chose 1 KiB so a bucket still fits in one block at the target of about 500k captures per shard.

## Deviations and contract changes needed in common
1. **New `derivatives : (nat64) -> (vec nat64) query`.** It isn't in the §7.3 service, but the verdict's `Oracle::derivatives` needs it. Suggest adding `MAX_DERIVATIVE_HITS = 32` to `common::limits`; it is defined locally for now.
2. **`Fingerprints` has no `quality` or `portrait` field,** so a client can't rebuild a `FingerprintBundle` for `Oracle::fingerprints`. Suggest adding `quality : nat8; portrait : bool` to `common.did`. The index already stores both in the row header.
3. **The probe returns only hits within distance 31,** capped at 32 per shard. That matches the verdict's own cap of 32 and is still exact after merging shards. Suggest adding `MAX_PDQ_HITS` to common.
4. **Probe blobs are checked against both exact tables,** because the `vec blob` argument doesn't say whether a hash is an asset or pixel hash.
5. **`local_ix` is assigned by the index in `put` order,** and only for postings that carry valid fingerprints. There is no `shard_base` in `PeerInit`.
6. **`indexed_upto` is the maximum `record_ix` put plus one.** This is correct only if the registry drains each shard in `record_ix` order.
7. **The probe has no replicated-update variant yet.** §7.3 mentions a rate-limited update re-issue for a definitive "No record"; it's left out.

## Open issues
- **Capacity:** 10k captures uses about 19% of the 2B budget. At 500k captures each bucket read gets deeper and some buckets need two blocks, so the 2B shard-close point needs measuring at larger scale (canbench, M2). The biggest available improvement is to replace the posting B-tree with directly addressed bucket slots (16×65,536 entries). That would remove most of the probe cost, but it changes the key layout the spec publishes, so it's your call.
- **A batch of 64 at 2.5B instructions** is far below the 40B update limit, but the registry drain should expect it.
- **Erase is O(shapes × 16) bucket operations,** about 300 reads and writes per erase.

# ---- report ----
I built the `witness-registry` canister (enroll, submit, certified reads, shard map, erase and outbox) and every check you asked for passes. I changed nothing outside `canisters/registry/` except the shared `Cargo.lock`, which picked up the new dependencies.

**Commands run, all passing**
- `cargo test -p witness-registry`: 108 tests. 101 are unit tests and 7 are PocketIC integration tests. The 101 unit tests also pass with `--features dev-attestation`.
- `cargo clippy -p witness-registry --all-targets -- -D warnings`: clean, with and without `dev-attestation`, and also for `--target wasm32-unknown-unknown`.
- `cargo fmt -p witness-registry --check`: clean.
- `cargo build -p witness-registry --target wasm32-unknown-unknown --release`: builds with and without `dev-attestation`. The wasm is 2.09 MB after `ic-wasm shrink`.
- The wasm32 dependency tree has no getrandom, wasm-bindgen, js-sys, tokio or reqwest.
- `candid-extractor` output matches `canisters/registry/registry.did` for both feature sets. A unit test also fails if `export_candid!` and the `.did` file drift apart.

**Candid** (shared types come from `common.did`)
```candid
type LateOpening = record { install_nonce : blob; wall_ms : nat64 };
type Enrollment = record { key_id : opt blob; chain : vec blob; late : opt LateOpening };
type Enrolled = record { key_id : blob; c2pa_cert : blob; ca_cert : blob };
type EnrollResult = variant { ok : Enrolled; err : text };
type EraseResult = variant { ok; err : text };
service : (RegistryInit) -> {
  by_capture : (nat64) -> (vec text) query;
  ca_root : () -> (opt blob) query;
  challenge : () -> (blob) query;
  consistency : (nat64) -> (vec blob) query;
  enroll : (Enrollment) -> (EnrollResult);
  erase_mine : (EraseRequest) -> (EraseResult);
  get : (text) -> (opt CertifiedRecord) query;
  get_ix : (nat64) -> (opt CertifiedRecord) query;
  get_key : (blob) -> (opt CertifiedKey) query;
  shards : () -> (Shards, blob) query;
  status : () -> (Status) query;
  submit : (vec Submission) -> (vec SubmitResult);
}
```

**How it works**
- **Startup:** `init` and `post_upgrade` both store the config, commit the shard map, set `certified_data` and start the timers. Nothing is rebuilt. A one-shot timer fetches `raw_rand` for the challenge secret and builds the Witness CA root with `schnorr_public_key` and `sign_with_schnorr` (Ed25519, key `dfx_test_key`). It retries every 60 s until both exist.
- **Enroll:**
  - Attestation is checked with `witness_attest::verify`.
  - The pending key, the quota reservation and the leaf certificate serial are written before the signing await. They are rolled back if signing fails, and a second enroll of a pending key gets `"enrollment pending"`.
  - The leaf certificate comes from the witness-c2pa builder.
  - On success the key goes into the SMT and `certified_data` is updated.
- **Submit:**
  - Each submission is checked in this order: size limits, duplicate `record_id`, `verify_seal` against the stored root, then key status and the 60-day window for originals.
  - For derivatives the parent must exist and share the same key and root capture.
  - Then come `(key_id, counter)` uniqueness, `fp_commit`, and the limit of 2,000 per key per day.
  - Accepted records go into the stable MMR and a record log. The registry updates its lookup indexes and queues postings for the index shard and shells for pages (derivatives only).
  - `certified_data` is set once per batch.
- **Outbox:** a stable map drained every 5 s. Each tick sends at most one batch per destination with bounded-wait calls, in order. Failed batches stay queued, and postings or shells for erased records are dropped.
- **Erase:** checks the device signature, writes a tombstone, removes the derivative link, and queues `erase` to the index (plus pages and media for derivatives). The permanent record stays readable through `get`.
- **Publication stub:** an hourly timer records the latest `(leaf_count, root)` in state. It makes no external call.
- **Ingress filter:** `canister_inspect_message` caps `submit` at 2 MiB, `enroll` at 32 KiB and `erase_mine` at 1 KiB, and rejects every other ingress update.

**PocketIC tests** (`canisters/registry/tests/pocketic.rs`)
- **Setup:** the tests use the real registry wasm built with `dev-attestation`. Index, pages and media are a small recording stub in `tests/stub`, a separate crate with its own `[workspace]` that can be switched off to test retries. The test harness builds both wasms into `target/registry-test`; set `WITNESS_REGISTRY_WASM` to use a prebuilt one.
- **Enrollment fixture:** each test enrolls an emulator-style dev chain (leaf signed by the AOSP intermediate key from witness-attest's test data, using a live challenge).
- **Captures:** they are sealed with `witness_core::seal`.
- **Certificates:** every `get`, `get_ix`, `get_key` and `shards` proof is checked against the IC certificate, including the BLS signature and the delegation.
- **Covered:** by_capture, consistency proofs, erase, upgrade (old challenge, root CA, records and dedupe all survive), two concurrent enrolls of one key, the dev enrollment backoff, outbox retry, and malformed or abusive input.

**Changes needed in common (I only worked around them)**
1. **Proofs can't be checked against `certified_data` as the types stand.** `CertifiedRecord` has no `smt_root`, and `CertifiedKey` has no `mmr_root` or `leaf_count`. My stopgap:
   - The last element of `mmr_proof` is the SMT root.
   - `smt_proof` is the SMT proof bytes followed by `mmr_root ‖ leaf_count` (8 bytes, big-endian).
   - I suggest adding explicit fields instead.
2. **The shard map has no proof slot either.** I committed it in the SMT under key `u128::MAX` as `hash_ds("witness/shards/v1", …)`. The `shards` blob is `u32 length ‖ certificate ‖ SMT proof bundle`.
3. **The MMR leaf format needs a shared home.** It is `sha256(claim) ‖ sha256(c2pa_claim) ‖ sha256(cose) ‖ anchored_ns` (8 bytes, big-endian), in `record::leaf_data`. Clients must reproduce it exactly, so it belongs in witness-merkle or witness-format. The verifiers in `proof.rs` (`record_certified`, `key_certified`, `shards_certified`) should move with it.
4. **Peer methods the registry calls:**
   - index: `put(vec Posting)`, `erase(nat64)`
   - pages: `put_shells(vec Shell)`, `erase(text)`, where `Shell = record { record_id; record_ix; anchored_ns; claim; media_shard }`
   - media: `erase(text)`
   - The index, pages and media agents need to export these, and `Shell` should move to common.
5. **My additions to the registry's own interface:** `Enrolled.ca_cert` and the `ca_root` query, because the root CA is generated at runtime on a local replica.
6. **Memory:** `local(0)` holds the record log data and `local(1)` the per-key submit quota. `MMR_PEAKS`, `REVOCATIONS` and `II_LINKS` are unused.

**Deviations**
- **Dev chains get progressive backoff, not a hard cap of 5.** Every emulator shares the AOSP intermediate certificate, so a hard cap would lock out all local devices. Factory chains get the same backoff; RKP chains get the hard 5-per-30-days cap.
- **A derivative must be signed by its parent's key.** The lost-key path (parent hash as a C2PA ingredient) is not built.
- **No revocation sync.** There is no outcall, and enrollment checks against an empty revocation list.
- **Erase via Internet Identity is refused.** There is no link method yet.
- **No stable `draining` flag.** common's serial interval timer already prevents overlapping ticks.
- **Low-S is not enforced.** Deduplication by `record_id` makes signature malleability harmless.
- **A key that is already enrolled cannot re-fetch its certificate.** If the enroll response is lost, the device must enroll a new key.

**Open issues**
- **Release signing digest is a placeholder.** It defaults to all zeros unless `WITNESS_RELEASE_DIGEST` is set at build time, so a production build won't enroll real devices until it is filled in. `WITNESS_DEBUG_DIGEST` defaults to `[0xd1; 32]` (the test value) and only applies with `dev-attestation`. `WITNESS_SCHNORR_KEY` defaults to `dfx_test_key`.
- **PocketIC needs two things for these tests.**
  - `dfx_test_key` only exists with `with_test_threshold_keys_subnet()`.
  - PocketIC's clock starts in 2021, so the tests set it to the current time.
- **Timer warning when time jumps.** Advancing PocketIC time by an hour logs "too many concurrent timer calls" from ic-cdk-timers. The tests still pass.
- **Erase reaches the index one drain tick later.** Each destination's queue is first-in first-out, so the index `erase` goes out on the tick after any pending `put` batch.

Files are in `/Users/andrejones/Desktop/workspace/projects/witness/canisters/registry/`:
- `Cargo.toml`, `registry.did`
- `src/`: `lib.rs`, `ic.rs`, `config.rs`, `types.rs`, `state.rs`, `store.rs`, `record.rs`, `proof.rs`, `quota.rs`, `ca.rs`, `enroll.rs`, `enroll/tests.rs`, `submit.rs`, `submit/tests.rs`, `erase.rs`, `read.rs`, `outbox.rs`, `status.rs`
- `tests/`: `pocketic.rs`, `support/mod.rs`, `support/attest.rs`, `stub/` (`Cargo.toml`, `Cargo.lock`, `src/lib.rs`)

# ---- report ----
I've built `witness-media`, and it passes all three checks you asked for: tests, clippy and the wasm32 build. I generated `media.did` from that build. The strongest check is a test that runs PocketIC's live HTTP gateway against the canister. It fetched a preview and a streamed 2 MiB+ blob and the gateway accepted the certification on both. Nothing was deployed and nothing was committed.

The canister only works end to end once the registry pushes keys and records to it. That push isn't in the shared contract yet; it's the first item under contract changes below.

## Design choice: the registry pushes an allowlist
I picked the push design. Uploads cost **zero round-trips** to the registry, every update method stays synchronous with no awaits, and a lagging push just returns `retry_after_ns` (5 s). The registry calls a registry-only `sync`, which takes three kinds of entry:
- **`key`**: on enrollment, sent to every media shard. Case chunks have no record id, so they need it.
- **`record`**: one per anchored derivative, sent to that record's shard.
- **`revoke`**: when a key is distrusted.

`begin_upload` then checks the ES256 upload signature against the pushed key. A preview, og or full upload is only accepted if its record was pushed with that same key.

## Candid service (`canisters/media/media.did`)
```candid
type Allow = variant { key : record { key_id : blob; pubkey : blob }; "record" : record { key_id : blob; record_id : text }; revoke : blob };
type Begun = record { missing : vec nat32; upload : nat64 };
type StreamingToken = record { "blob" : nat64; offset : nat64 };
service : (PeerInit) -> {
  begin_upload : (BeginUpload) -> (variant { Ok : Begun; Err : MediaErr });
  put_chunk : (Chunk) -> (variant { Ok; Err : MediaErr });
  commit : (nat64) -> (variant { Ok : text; Err : MediaErr });
  erase : (text) -> (variant { Ok : nat32; Err : MediaErr });
  sync : (vec Allow) -> (nat32);
  count_checks : (vec text) -> (nat32);
  checks : (vec text) -> (vec nat64) query;
  http_request : (HttpRequest) -> (HttpResponse) query;
  http_request_streaming_callback : (StreamingToken) -> (StreamingCallbackHttpResponse) query;
  status : () -> (Status) query;
}
```
- **Paths:** `commit` returns the path. Previews are `/p/{record_id}`, og images `/o/{record_id}`, full-res `/f/{record_id}`, and case chunks `/c/{case_id}.{ix}`.
- **Registry-only:** `erase` and `sync` trap for any other caller, and `canister_inspect_message` turns them away at ingress unless the caller is the registry.

## How it works
- **Uploads (resumable):** calling `begin_upload` again with the same request returns the same upload id and the chunks still missing. Chunks are bound to the caller who began the upload; a later valid `begin_upload` rebinds them.
- **Commit:** it hashes the stored pages and compares against the declared SHA-256. On a mismatch it resets the received chunks and returns `hash_mismatch`.
- **Storage:** blobs live in 64 KiB stable-memory pages with an O(1) free list.
- **Certification:**
  - I couldn't reuse the `pages` tree, so I wrote a crit-bit tree in stable memory instead. It uses the `ic-http-certification` v2 layout and keeps its node hashes, so nothing is rebuilt on upgrade.
  - `post_upgrade` only re-sets the certified root.
  - The certified headers are `content-type` and `cache-control: immutable`. CORS headers are added uncertified, for witness.app and case.witness.app, plus `http://localhost[:port]` under `local-dev`.
- **Streaming:** responses over 2 MiB stream in 2 MiB chunks through the callback.
- **Quotas:**
  - **Free tier:** 50 previews, 50 og images and 20 cases per key per day, and at most 16 MB per case.
  - **Plus:** the grant's signature, principal, expiry and `quota_bytes` are checked. The signer's public key is compiled in only under the `local-dev` feature (test seed `[0x57;32]`).
- **Expiry timer (every 10 minutes, 256 per tick):**
  - It deletes expired cases and uploads left uncommitted for over 24 hours.
  - Plus media is kept until the grant expires plus 30 days. A renewed grant pushes that date back.
  - It also purges old daily quota counters.
- **Check counter:** at most 32 ids per call, only for known records, with a budget of 100k per 10-minute window.
- Full-resolution uploads are capped at 64 MiB, so hashing at commit stays well inside the instruction limit.

## Tests and commands
- **Tests:** 110 in total, all passing.
  - 100 unit tests (also passing with `--features local-dev`). They include checks that the tree's root hash matches `HttpCertificationTree` and that its absence lookups work. The interface file `media.did` is also checked against the exported interface.
  - 10 PocketIC tests (`tests/pocketic.rs`). These cover the full flow, local response verification, registry-only rejection, oversized ingress, erase, streaming, the expiry timer, upgrade, the check counter, and the live gateway.
- **Commands run:**
  - `cargo test -p witness-media`
  - `cargo clippy -p witness-media --all-targets -- -D warnings`, also with `--features local-dev`
  - `cargo fmt --check`
  - `cargo build -p witness-media --target wasm32-unknown-unknown --release`
  - `candid-extractor`, written to `media.did`
  - `scripts/build-canister.sh witness-media`, which gives a 1.46 MB shrunk wasm
- The wasm has no getrandom, wasm-bindgen, js-sys, reqwest or tokio in its dependency tree.
- I added dev-dependencies `ic-response-verification` 4.0.1 and `reqwest` (blocking), plus `sha2` as a normal dependency, so `Cargo.lock` changed.

## Contract changes needed (other lanes)
1. **Registry must push** `sync` (`key` on enrollment to every media shard, `record` per derivative, `revoke` on distrust) and call `media.erase(record_id)` when erasing. I suggest moving `Allow` into `common.did`.
2. **`BeginUpload` has no case expiry**, so cases use a fixed 30-day TTL. I suggest adding a signed `expires_ns : opt nat64`.
3. **`BeginUpload` has no content type**, so full-res files are served as `application/octet-stream`.
4. **Local deploy:** `icp.yaml` builds media without `local-dev`, so on the local network Plus is disabled and localhost CORS is off. `status` shows a `plus-disabled` flag. The build should pass `local-dev`.
5. **Production grant signer key:** none exists yet, so without `local-dev` Plus is turned off.

## Open issues
- **Missing blobs return an uncertified 404.** I didn't build absence proofs, so a gateway answers missing or erased media with a 503 instead of a 404.
- **No early case deletion:** neither the contract nor the canister has a way for an owner to delete a case before it expires.
- **Timer burst after a time jump (common runtime):** after a large clock jump, the interval timers in `runtime::start_timers` flood with "too many concurrent timer calls". I saw it in PocketIC when switching to live mode after install. The tests now avoid it by advancing time one day at a time and by installing after going live.
- **Generated `.did` comments:** `candid-extractor` copies `ic-http-certification`'s doc comments into the interface, so I strip comment lines when writing `media.did`. The shared `build-canister.sh` still embeds them in the `candid:service` metadata.

Files are in `/Users/andrejones/Desktop/workspace/projects/witness/canisters/media`:
- Cargo.toml
- media.did
- tests/pocketic.rs
- src/lib.rs
- src/store.rs, src/store/tests.rs
- src/certtree.rs, src/certtree/tests.rs
- src/http.rs, src/http/tests.rs
- src/pagestore.rs
- src/paths.rs
- src/records.rs
- src/policy.rs

# ---- report ----
I built the `witness-android` UniFFI library. It has 100 host tests, which pass; clippy is clean with `-D warnings` and fmt is clean. It builds for arm64-v8a and x86_64, and the Kotlin bindings are generated. I also compiled the generated Kotlin with the cached Kotlin 2.4.21 compiler plus JNA, and ran a small JVM program against the host library; error mapping and object construction both worked. Nothing was deployed or committed, and no Gradle files were touched.

## What I built (`crates/witness-android`)
Each module has one job:
- **`ffi.rs`**: exposes the existing types from `witness-canister-common`, `witness-format` and `witness-verdict` to Kotlin as UniFFI "remote" types, so none are redefined. Principal, record ids and capture ids cross as text or numbers; the two bitsets cross as plain integers.
- **`claim.rs`**: `CaptureMeta`, the input for sealing, and `ClaimView`, a readable claim for Kotlin.
- **`seal.rs`**: the seal pipeline and the `KeystoreSigner` callback trait.
- **`transport.rs`**: a `Transport` port with the real ic-agent implementation and a mock used by the tests.
- **`client.rs`**: typed Candid calls to the canisters, with probe fan-out and merging.
- **`oracle.rs`**: the network-backed oracle that `judge` uses.
- **`witness.rs`**: the `Witness` handle Kotlin talks to.
- **`trustmark.rs`**: the watermark model wrapper; ONNX is behind the `trustmark` feature.
- **`verdict.rs`**: `VerdictView`, the readable result.

## Public API
- **`KeystoreSigner`** (Kotlin implements it): `sign(message) -> bytes` and `certChain() -> [leaf, root]`. Kotlin can return the raw Keystore DER signature; Rust converts it to the 64-byte low-S form.
- **Sealing**:
  - `prepare(capture, meta: CaptureMeta, watermark: Trustmark?) -> PreparedHandle`. The claim is validated here, before any signing.
  - `PreparedHandle.finish(counter, signer, caRoot) -> SealedCapture { jpeg, recordId, claim, submission, watermarked }`. The submission carries the canonical fingerprints, ready for `registry.submit`.
  - `PreparedHandle` also has `width()`, `height()`, `watermarked()`, `assetHash()` and `pixelHash()`.
  - If signing fails, `finish` can be retried. Once it succeeds, the handle returns `Consumed`.
- **`decodeClaim(cbor) -> ClaimView`**.
- **`Witness(WitnessConfig { replicaUrl, registry, local, caRoot?, identitySeed? })`**:
  - Registry calls: `challenge`, `caRoot`, `enroll`, `submit`, `get`, `getKey`, `byCapture`, `shards`, `eraseMine(recordId, nonce, signer)`.
  - `probe`: sent to all index shards in parallel, then merged (deduplicated, minimum distance kept, at most 32 hits, lowest `indexed_upto`).
  - Media uploads: `beginUpload(shard, target, size, sha256, keyId, signer, grant?)`, `putChunk` and `commit`.
  - `check(image, trustmark?) -> VerdictView`.
- **Errors**: one `WitnessError` enum with `InvalidInput`, `Image`, `Seal`, `Signer`, `Network`, `Rejected`, `Media{MediaErr}`, `Malformed`, `WatermarkUnavailable` and `Consumed`.

## Verified
- **Tests**: `cargo test -p witness-android` passes all 100 tests (claim 13, client 29, error 6, oracle 13, seal 13, transport 4, trustmark 3, verdict 3, witness 16). They cover:
  - seal round-trips checked with the existing C2PA verification code, determinism, DER signatures, signer failure and retry, and malformed input;
  - every client call against the mock transport, including probe limits and merging;
  - an end-to-end `check` that returns ORIGINAL;
  - a network failure coming back as `Err` rather than a false "no record";
  - erase and upload signatures checked with the shared `EraseRequest` and `BeginUpload` verifiers.
- **Lint and format**: clippy with `-D warnings` is clean for the default build and with `--features trustmark,bindgen`. fmt is clean.
- **TrustMark feature**: `cargo ndk -t arm64-v8a check --features trustmark` compiles. It uses ort `2.0.0-rc.15` with `load-dynamic`, so nothing is downloaded at build time.
- **Android build**: `scripts/build-android-native.sh` writes stripped libraries to `jniLibs/arm64-v8a` (6.0 MB) and `jniLibs/x86_64` (6.9 MB), and the bindings to `src/main/kotlin/uniffi/witness_android/witness_android.kt`.
- **TLS**: no aws-lc in the tree; TLS uses ring through rustls.
- **Canister check**: `scripts/check-canister-deps.sh` passes for all 5 canister crates. I also confirmed its pattern does flag tokio, reqwest, ort and wasm-bindgen.
- **Model script**: `scripts/fetch-models.sh` downloads nothing without `--download`, and refuses to run until both SHA-256 values are pinned. The digests are TODO placeholders you need to fill in.

## Deviations
- **ic-agent is pinned to 0.45.0, not the latest 0.49.3.** Every ic-agent from 0.46 on pulls in reqwest 0.13, whose rustls feature needs aws-lc, which needs cmake. 0.45 is the last version that uses ring.
- **Exported methods are blocking, not async.** Each call runs on the current-thread tokio runtime owned by the `Witness` handle, so Kotlin should call it from `Dispatchers.IO`. UniFFI's tokio async mode would spin up its own global multi-thread runtime, which conflicts with "owned current-thread runtime".
- **The build script forces panic=unwind.** The workspace release profile uses `panic=abort`, which would turn any Rust panic into an app crash instead of a Kotlin exception.
- **Derivative manifests use `c2pa.created` with no source type.** There is no shared "edited" action and no ingredient support yet.
- **Certified-proof checking is not done in the client.** It relies on ic-agent's query signature checks; the registry's proof verifiers would need to move into a shared crate first.
- **Fingerprints rebuilt from the index get `quality = 0`, and `portrait` is taken from the claim's dimensions**, because the `Fingerprints` type in common has neither field.

## Contract changes needed (other lanes)
1. The registry's `Enrollment`, `Enrolled`, `EnrollResult`, `EraseResult` and the media `Begun` type should move into common; I declared them locally.
2. Add `quality` and `portrait` to `Fingerprints`, which the index team also asked for.
3. Move the registry proof verifiers and the leaf format into a shared crate so the client can verify certified reads.
4. The verdict `Oracle` needs a batch `fingerprints` call. Each candidate is currently one blocking round-trip, up to 64 per check, so a check can take several seconds on a real network.
5. `index_pending` always returns false. Implementing it needs the record's `record_ix` or a shard-routing rule.
6. Nothing sets the late-anchor or clock-mismatch flags on records; the registry would need to send them.

## What Gradle must add (I did not edit it)
- `implementation("net.java.dev.jna:jna:5.17.0@aar")`. The bindings compiled against 5.12.1; any version from that up works.
- The existing `uniffi.**` and `com.sun.jna.**` keep rules stay. The `org.rustls.platformverifier` rule can go, since built-in root certificates are used.
- No `rustls-platform-verifier` dependency or `Tls.init` call is needed.
- With `trustmark` enabled, the app must also ship `libonnxruntime.so` (for example from `com.microsoft.onnxruntime:onnxruntime-android`), because the library loads it at runtime.
- Optionally, make `preBuild` run `scripts/build-android-native.sh`.
- The NDK target is API 26; the app's minSdk of 28 is compatible.

Shared files I changed: the workspace `Cargo.toml` (added the crate to members) and `Cargo.lock`.

Files are in `/Users/andrejones/Desktop/workspace/projects/witness/`:
- crates/witness-android/Cargo.toml
- crates/witness-android/src/: lib.rs, ffi.rs, error.rs, claim.rs, seal.rs, trustmark.rs, transport.rs, client.rs, client/tests.rs, oracle.rs, oracle/tests.rs, verdict.rs, witness.rs, witness/tests.rs
- crates/witness-android/src/bin/uniffi-bindgen.rs
- client/native/witness/src/main/jniLibs/arm64-v8a/libwitness_android.so
- client/native/witness/src/main/jniLibs/x86_64/libwitness_android.so
- client/native/witness/src/main/kotlin/uniffi/witness_android/witness_android.kt
- scripts/build-android-native.sh, scripts/fetch-models.sh, scripts/check-canister-deps.sh

# ---- report ----
I built `crates/witness-web` and everything I checked passes. The `.wasm` is 1,436,802 bytes after `wasm-bindgen --target web`, under the 1.5 MB target, and 1,315,234 bytes after `wasm-opt -Oz`. A replayed check gives exactly the same verdict as the native `judge` on 13 scenarios built with the witness-core testkit, natively and on wasm32. One real-certificate test also passes: a live registry's shard map, with its delegated application-subnet certificate, verifies against the PocketIC root key. Nothing was deployed or committed. Outside `crates/witness-web` I changed only the root `Cargo.toml` (one members line), and cargo updated the shared `Cargo.lock` when it added the new dependencies.

## How the check works
The `Oracle` in witness-verdict is synchronous, so the browser runs `judge` several times, each time with more fetched data. An internal oracle answers from whatever JS has fetched so far and records anything missing as a "need". The verdict is final when nothing is missing. No judge logic is duplicated; it calls `witness_verdict::judge` itself.

Needs come in four stages, and a later stage is only asked for once the earlier ones are answered:
1. Keys, records by id, exact hashes, PDQ hashes (exact, PDQ and watermark lookups come from one index `probe` call).
2. Records by index, and derivatives.
3. Fingerprints.
4. Reference images.

On the 13 golden scenarios this took 2–6 `judge` runs (the plan is the first). Fingerprints sent by JS are checked against the claim's `fp_commit`, so tampered index data is dropped.

## JS API
| Export | What it does |
|---|---|
| `init()` | Default export generated by wasm-bindgen |
| `version()` | Crate version |
| `queryPlan(bytes, optionsJson)` | Returns `{done, need, verdict, credentials}`. `credentials` is parsed from the embedded C2PA: kind, recordId, bindingIntact, assetHash, claim |
| `judge(bytes, optionsJson, fetchedJson)` | Returns `{done, need, verdict}` |
| `verifyCertifiedRecord(json, rootKey, canisterId, nowNs, maxAgeNs)` | IC certificate (BLS, delegation, canister ranges, freshness) plus MMR proof |
| `verifyCertifiedKey(...)`, `verifyShards(...)` | Same certificate checks, plus the SMT proof |
| `parseCaseBundle(bytes)` | Returns `{title, summary, items:[{recordId, jpeg:{offset,len}, submission}]}`. JPEGs are pointed to by offset, not copied |
| `new CaseOpener(fragment, caseId, count, bundleHash)` | `.add(ix, chunk)`, `.missing()`, `.finish()`. AES-256-GCM, decrypts in place, rejects any missing chunk or bad AAD, then checks the bundle SHA-256 |

**JSON conventions:**
- Field names are camelCase.
- Byte fields are hex, except reference images, which are base64.
- All u64 values are decimal strings.
- `rootKey` and `canisterId` are `Uint8Array`s (`Principal.toUint8Array()`).
- `nowNs` and `maxAgeNs` are `bigint`; `maxAgeNs = 0` skips the freshness check.
- Options are `{anchor: hex DER, atS?, topK?, maxPixels?}`.
- The fetched-data format is the `Fetched` struct in `src/wire.rs`.

BLS verification runs in Rust (`ic-verify-bls-signature` 0.6 with `ic_bls12_381`, no getrandom), so JS doesn't need to verify certificates.

## Size
| Stage | Bytes |
|---|---|
| wasm-bindgen output, name section removed | 1,436,802 |
| after `wasm-opt -Oz` | 1,315,234 |
| gzip | 515 KB |
| brotli | 399 KB |

Certificate verification is the largest optional part: BLS about 160 KB, serde_cbor about 98 KB and ic-certification about 20 KB, before wasm-opt. That is the piece to split out later if needed. The final wasm imports only 3 `wbg` functions. Its wasm32 dependency tree has no getrandom, js-sys, tokio, reqwest, ring or aws-lc.

## Tests
- **Native: 96.** 91 unit tests, 1 golden test, 3 tests against the registry's own proof builders (`tests/registry_compat.rs`), and 1 PocketIC test against a live registry.
- **wasm32 under node: 6.** 5 tests of the exported API and the same golden test. The golden digest is identical on both platforms.
- **Node smoke test** (`ts/smoke.mjs`) on the final wasm-opt'd package, using a fixture from `examples/fixture.rs`.
- **TypeScript check** (`ts/check.ts`, typescript 5.9.3) against `pkg/witness_web.d.ts`.

## Commands verified
- `cargo test -p witness-web`
- `cargo test -p witness-web --target wasm32-unknown-unknown`
- `cargo clippy -p witness-web --all-targets --all-features -- -D warnings`, native and wasm32
- `cargo fmt -p witness-web --check`
- `crates/witness-web/build.sh` (`--profile wasm`, `wasm-bindgen --target web` into `pkg/`, then `wasm-opt` if installed)
- `node ts/smoke.mjs <fixture>`
- `npx tsc -p ts`

## Changes needed outside my lane
1. **Duplicated verification logic.** The MMR leaf layout, record/key/shard proof checks and `shard_map_hash` are reimplemented here, because I can't link a canister crate into the browser build. A test checks they match the registry's versions. They should move into one shared crate (witness-merkle) used by both.
2. **Case bundle format.** No `CaseBundle` CBOR format existed, so I defined one in `src/bundle.rs`:
   - top level `{0: 1, 1: title, 2?: summary, 3: [items]}`
   - each item `{0: record_id (10 bytes), 1: jpeg, 2?: Submission}`
   It should move to witness-format. Kotlin's `FakeCaseBundleCodec` needs to adopt it.
3. **The `witness-bridges` package doesn't match.** Specific points:
   - `verify.mjs` expects a single `judge(bytes, onProgress)`, but this API is the two-step plan/judge loop.
   - `icp.mjs` reads `canister_id`, `base_ix` and `indexed_upto` from shard entries, but `common.did` has `{shard, canister, open}`.
   - The bridge never checks the MMR or SMT proofs. It should call the `verify*` exports.
4. **Index `Fingerprints` has no `quality` or `portrait` field.** I work around it by trying all 101 quality values against the claim's commitment. Adding the fields to the index output would remove that.
5. **Clock flags are never computed.** Nothing derives `LATE_ANCHOR` or `CLOCK_MISMATCH`, so records arrive with no flags.
6. **The Witness CA root (`anchor`) is supplied by the caller.** That's because a local replica generates it at runtime. A production build should compile it in.
7. **The `wasm` profile keeps the function-name section** (308 KB). `build.sh` strips it; setting `strip = true` in the root profile would do it at the source.

## Open issues
- **No watermark on the web.** The verdict reports "watermark unavailable", so a heavily cropped copy gives NO_RECORD on the web. The golden "cropped" scenario expects exactly that, unlike the Android path.
- **One round could be saved.** JS can fetch the record for `credentials.recordId` alongside the key in the first round instead of waiting to be asked.

Files are in `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-web/`:
- `src/`: `lib.rs`, `api.rs`, `wire.rs`, `oracle.rs`, `check.rs`, `render.rs`, `certificate.rs`, `proof.rs`, `case.rs`, `bundle.rs`, `error.rs`, `testkit.rs`, plus a `tests.rs` per module
- `tests/`: `golden.rs`, `api.rs`, `registry_compat.rs`, `pocketic.rs`
- `examples/fixture.rs`, `build.sh`, `ts/`, `pkg/`

# ---- report ----
The local deploy works and `scripts/smoke-local.sh` passes all 25 steps against the live local canisters, in about 18–19 s. It ran green 6 times in a row, and the local network is still running. Getting there took one canister-code fix. The registry was calling the wrong method names on pages and never pushed allowlist entries to media, so derivative pages, uploads and tombstones could not work. Nothing was committed, and nothing went anywhere except the local replica.

## Deployed (local only)
Gateway: `http://localhost:8090`. Ids are in `/Users/andrejones/Desktop/workspace/projects/witness/.icp-local/ids.json`.

| canister | id | url |
|---|---|---|
| registry | 4caro-hl777-77775-aaaba-cai | http://4caro-hl777-77775-aaaba-cai.localhost:8090/ |
| index_0 | 4fbx2-kt777-77775-aaabq-cai | http://4fbx2-kt777-77775-aaabq-cai.localhost:8090/ |
| pages | 4qggx-l3777-77775-aaaca-cai | http://4qggx-l3777-77775-aaaca-cai.localhost:8090/ |
| media_0 | 4xhad-gd777-77775-aaacq-cai | http://4xhad-gd777-77775-aaacq-cai.localhost:8090/ |
| web | 46el7-ql777-77775-aaada-cai | http://46el7-ql777-77775-aaada-cai.localhost:8090/ (returns 200) |

`deploy-local.sh` itself needed no changes. A full deploy takes about 64 s with a cold wasm build and about 25 s when only upgrading.

## Fixes
1. **Registry outbox now matches the real peer interfaces** (`canisters/registry/src/{types,outbox,submit,erase,enroll}.rs`). Before, its calls to pages were rejected forever and media was never told about keys or records.
   - **Pages, new shells:** the registry was calling `put_shells`, which pages doesn't have. It now calls `put_shell`. `Shell` gained `media: principal` and `dev: bool`, so it decodes as pages' `ShellInput`; the two extra fields are ignored by pages.
   - **Pages, erase:** the registry was calling `erase`, which pages doesn't have. It now calls `tombstone(vec text, variant { erased })`.
   - **Media, keys:** each enrollment now sends `sync` with `Allow::key` to every media shard.
   - **Media, records:** each derivative now sends `sync` with `Allow::record` to its media shard. If the record is erased before delivery, that entry is dropped.
   - `registry.did` is unchanged; these are internal outbox types.
2. **`icp.yaml`:** media is now built with `local-dev`, as the media agent asked, which turns on Plus and localhost CORS.

## New: `tools/witness-smoke` (added to the workspace) and `scripts/smoke-local.sh`
- **What it uses:** ic-agent 0.45 with the fetched root key. It reuses the `witness_android::Witness` client (its network-backed oracle calls `witness_verdict::judge`), `witness_web::proof` for certificate, MMR and SMT checks, `ic-response-verification` for HTTP certification, the registry's dev-attestation fixture (included via `#[path]`), the `witness-core` seal, and `witness-verdict` testkit images.
- **Flow:** challenge → enroll → `get_key` (certificate + SMT proof verified) → seal original → submit → `get` (BLS certificate + MMR proof verified) → probe finds it by PDQ (distance 0), watermark id and exact hash → judge sealed file = Original → judge q80 recompress = MatchesWithChanges → seal and submit a derivative → pages `/p/{id}` certified 200 (agent and curl) → media preview upload → certified fetch and curl, bytes match → `erase_mine` on both → probe no longer finds either → pages certified 410 (agent and curl) → media preview returns 503 → the permanent record is still readable and verifies.
- **Device reuse:** the enrolled test device is saved in `.icp-local/smoke-device.json` and reused, because all emulator chains share one enrollment quota (5 per 30 days, then backoff). Set `WITNESS_SMOKE_FRESH_DEVICE=1` to force a new enrollment.
- **Output:** a table on stdout and a JSON report at `.icp-local/smoke.json`. Set `WITNESS_SMOKE_SKIP_DEPLOY=1` to skip the deploy step.

**Timings (first run):**

| step | time |
|---|---|
| enroll | 437 ms |
| seal | about 270 ms |
| submit | about 200 ms |
| verified get | about 30 ms |
| index visible after drain | 1.6–2.6 s |
| judge Original | 13 ms |
| judge recompress | about 285 ms |
| page visible | about 4.6 s |
| media upload | about 650 ms |
| erase reaching the index | about 8.5 s (first-in first-out per destination) |
| total | about 17.6–19 s |

## Tests and commands verified
- `cargo test -p witness-registry`: 113 tests (106 unit, 7 PocketIC). Tests were updated to the new contract and 5 were added; none were deleted.
- `cargo test -p witness-smoke`: 25 unit tests.
- `cargo test -p witness-web --test registry_compat`: 3 passed.
- Clippy with `-D warnings` is clean for smoke and registry: `--all-targets`, `--features dev-attestation`, and wasm32. `cargo fmt --check` is clean for the whole workspace.
- `scripts/check-canister-deps.sh` passes.
- `scripts/deploy-local.sh` and `scripts/smoke-local.sh` both run end to end.

## Open issues
- **Recompressed copies often get `CouldntCompare`.** About half of 640×480 q80 recompresses come back flagged when there is no public reference image. The outcome is still MatchesWithChanges. I reproduced this offline with `MemoryOracle` (6 of 12 seeds), and the index's quality=0 workaround is not the cause. This is for the matcher owner; the smoke only asserts the outcome.
- **Shared types to move into `common.did`:** `Allow`, `Removal`, `ShellInput`/`Shell` (now with `media` and `dev`), and the `sync`, `put_shell` and `tombstone` peer methods.
- **Media has no revoke push:** the registry never sends `Allow::revoke`, because there is no distrust path yet.
- **Erased media answers 503 through the gateway** rather than a certified 404, because media has no absence proofs (already in media's report).
- **Pages hardcode witness.app domains**, so the page shell's `boot.mjs` doesn't load on the local network.

Files:
- `/Users/andrejones/Desktop/workspace/projects/witness/tools/witness-smoke/{Cargo.toml,src/main.rs,src/capture.rs,src/device.rs,src/net.rs,src/steps.rs}`
- `/Users/andrejones/Desktop/workspace/projects/witness/scripts/smoke-local.sh`
- `/Users/andrejones/Desktop/workspace/projects/witness/icp.yaml`
- Root `Cargo.toml` (member added) and `Cargo.lock`
- Registry: `/Users/andrejones/Desktop/workspace/projects/witness/canisters/registry/src/{types.rs,outbox.rs,submit.rs,submit/tests.rs,erase.rs,enroll.rs,enroll/tests.rs}` and `canisters/registry/tests/{pocketic.rs,stub/src/lib.rs}`

# ---- report ----
## QA report: workspace gate and adversarial review

Everything passes after my fixes. I found 5 real bugs and fixed each one with a regression test. Nothing was deployed or committed. The local replica is still running the old registry and media wasm. The outbox change can upgrade it in place (old entries still drain), but `deploy-local.sh` and `smoke-local.sh` were not re-run.

### Gate results
- `cargo fmt --all --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. Also clean for registry with `dev-attestation`, media with `local-dev`, android with `trustmark,bindgen`, and the wasm32 clippy of core, web and all canisters.
- `cargo test --workspace`: **2,024 passed, 0 failed.**
- wasm32 tests: witness-core 10 + 13 + 1, witness-web 5 + 1 golden, all pass.
- wasm32 release builds: core, witness-web, registry (with and without `dev-attestation`), index, pages, media (with and without `local-dev`) all build.
- `scripts/check-core-deps.sh` and `scripts/check-canister-deps.sh`: clean. No c2pa, ort, tokio, reqwest, wasm-bindgen or getrandom/js in any canister tree.
- `scripts/build-android-native.sh`: rebuilt both `.so` files (arm64 6.2 MB, x86_64 7.1 MB) and regenerated the Kotlin bindings, which now have 29 `suspend fun`s.

### Test totals per crate
| Crate | Tests |
|---|---|
| witness-registry | 109 unit (+3) + 7 PocketIC |
| witness-media | 101 unit (+1) + 10 PocketIC |
| witness-index | 73 unit + 4 PocketIC |
| witness-pages | 77 unit + 5 PocketIC |
| witness-canister-common | 89 |
| witness-android | 102 (+2) |
| witness-web | 92 unit (+1) + 3 registry_compat + 1 PocketIC + 1 golden + 5 api |
| witness-smoke | 25 |

Core crates, unit tests:

| Crate | Tests |
|---|---|
| format | 135 |
| image | 143 |
| fingerprint | 118 |
| match | 164 |
| c2pa | 168 |
| merkle | 81 |
| attest | 151 |
| watermark | 104 |
| verdict | 83 |
| core | 10 |

The core crates also have about 180 integration tests (align, decode, diff, e2e, golden, pipeline, and others).

### Bugs found and fixed
1. **Registry outbox: one stuck destination blocked delivery to all the others** (`canisters/registry/src/outbox.rs`, `store.rs`).
   - **Cause:** the drain looked at only the first 256 entries of one shared queue. If one peer was down, out of cycles or not configured, 256 queued messages for it hid every other destination's messages.
   - **Fix:** queue keys now carry the destination in their top 16 bits, so each destination has its own range and is scanned separately (first in, first out per destination is unchanged).
   - **Compatibility:** keys are still `u64`, so the stable layout is the same. Entries queued before the change all land in the first range but are batched by their own destination, so they still drain.
   - **Tests:** `a_stuck_destination_does_not_starve_the_others`, `legacy_unlaned_entries_still_drain_in_order`, `lanes_are_disjoint_and_ordered`. Existing tests that relied on global ordering were updated to the new order; none were deleted.
2. **Unconfigured media canister accepted `sync` and `erase` from the anonymous principal** (`canisters/media/src/lib.rs`). The check compared the caller with the default registry id, which is anonymous. It now also requires the canister to be configured, as index and pages already do. Test: `unconfigured_canister_trusts_no_registry`.
3. **witness-android blocked the caller's thread on every network call.** ARCHITECTURE §10.2 says these exports should be async.
   - **Fix:** all network methods and `check` are now `async`, so they become Kotlin `suspend fun`s. They run on a dedicated one-thread tokio runtime named `witness-io`. `check` runs `judge` in `spawn_blocking`, and the oracle uses `Handle::block_on`.
   - **Crash avoided:** this also removes the "runtime inside a runtime" panic you got by calling from inside a tokio context.
   - **Panics:** panics in those tasks now come back as an error instead of aborting. Panics in other exported functions are caught by UniFFI as long as `panic=unwind` is set, which the build script forces.
   - **Callers updated:** the tests and `tools/witness-smoke` now await the calls.
   - **Tests:** `network_calls_run_off_the_callers_thread`, `check_can_be_awaited_inside_another_tokio_runtime`.
4. **witness-web output depended on which crates were built together.** `cargo test --workspace` failed the web golden test (`08c0d89c…` instead of `9a2aad9f…`).
   - **Cause:** c2pa, a test-only dependency, turns on serde_json's `preserve_order`, which changes the key order of the rendered verdict JSON. The verdicts themselves were identical.
   - **Fix:** `render::{verdict, claim, edit}` now sort their keys, so the output and digest are the same in every build, native and wasm32. The golden value is unchanged.
   - **Test:** `rendered_json_has_canonical_key_order_under_any_feature_set`.
5. **Code comments removed.** I deleted comment lines from `design/tools/*.sh`, `design/tools/**/*.py` and `video/tools/*.py`, and checked that every Python file still parses. The Rust, Kotlin, JS and TS code had no comments. I kept two files on purpose: the `webpackIgnore` directive (it changes webpack's behaviour) and the Gradle-generated `.properties` header.

`.gitignore` now also covers `crates/witness-web/pkg/` and `*.so`. `.icp-local/`, `.icp/` and `.dfx/` were already covered.

### Reviewed and found sound
- **Caller checks:** registry-only methods in index, pages and media check the caller inside the method, not only in `inspect_message`.
- **Query limits:** query arguments are bounded inside the code (probe limits, at most 32 checks, `by_capture` cap), which matters because `inspect_message` does not run for queries.
- **Enroll across the signing await:** state is written before the await and rolled back if signing fails.
- **Drain timer:** it can't overlap itself and uses bounded-wait calls. Peers handle repeated delivery safely.
- **`certified_data`:** reset after every certified change in registry, pages and media.
- **Pages HTML:** every value is escaped, and the inline JSON contains only base32 text and a number.
- **Media:** CORS uses an exact allowlist; upload sizes and offsets can't overflow.

### Left open
- **Stuck enrollments:** if code after the signing await were to trap, the key would stay marked "pending" forever. That code currently returns errors rather than panicking, so the risk is low. The device would have to enroll a new key.
- **Outbox scan cost:** each tick scans every destination that has queued messages (up to 256 each). Fine for a handful of shards; it needs a cap if shard counts grow large.
- **Case-page nonce:** the CSP nonce is fixed per release, which is acceptable only because the case page is static.
- **Local CORS:** media allows only `http://localhost[:port]`, so the web app served from `*.localhost:8090` on the local replica can't fetch media.
- **Contract changes to move into common.** These came from the earlier agent reports and are still open:
  - `Allow`, `Shell`, `Removal`, `PageResult`.
  - `quality` and `portrait` on `Fingerprints`.
  - A shared crate for the proof verifiers and the MMR leaf format.
  - A batch `fingerprints` call on `Oracle`.

### Files changed
In `/Users/andrejones/Desktop/workspace/projects/witness/`:
- **Registry:** `canisters/registry/src/{outbox.rs,store.rs,erase.rs,submit/tests.rs}`
- **Media:** `canisters/media/src/lib.rs`
- **Android:** `crates/witness-android/{Cargo.toml,src/witness.rs,src/oracle.rs,src/oracle/tests.rs,src/witness/tests.rs}`
- **Web:** `crates/witness-web/src/{render.rs,render/tests.rs}`
- **Smoke tool:** `tools/witness-smoke/{Cargo.toml,src/main.rs}`
- **Comments removed:** `design/tools/*.sh`, `design/tools/*.py`, `design/tools/screens/*.py`, `video/tools/{make_voice.py,say.py}`
- **Repo config:** `.gitignore`, `Cargo.lock` (adds `futures-executor`)
- **Regenerated:** `client/native/witness/src/main/{jniLibs/*/libwitness_android.so,kotlin/uniffi/witness_android/witness_android.kt}`