# Closeout wave: lane reports

Fix lanes, verification lanes and the final gate from the closeout workflow. Open issues are tracked in `docs/dev/STATUS.md`.

# Fix lanes

## rust contracts
#### Report: rust-contracts lane

All seven items are done. Locally redeployed with `scripts/smoke-local.sh`: **SMOKE OK, 32 steps** (up from 28). Every crate I touched passes its tests, clippy and wasm32 builds.

`cargo test --workspace --no-fail-fast` did not run to the end. I stopped it after about 16 minutes because another agent's work-in-progress test, `witness-verdict` `tests::zz_probe_false_altered`, was hanging. Before that point all my crates had passed in that run. During the same window the witness-core and witness-web golden digests failed because of that agent's in-flight `witness-verdict` edits. Once they settled, both goldens passed again on aarch64 and wasm32.

Task 2 (pages rejects `http://` origins unless built with `local-dev`) was already implemented and enabled in `icp.yaml`. I checked it and its test and changed nothing.

### Changes (files)
- **canisters/common**
  - `common.did`: `media_shard` added to `Anchored` and `CertifiedRecord`; new `CaseInfo` type.
  - `src/registry.rs`: Rust twins of those two records.
  - `src/media.rs`: `CaseInfo`. `UploadTarget::case_id` now rejects `count > 256`.
  - `src/limits.rs`: `MAX_CASE_CHUNKS = 16`, `MAX_PLUS_CASE_BYTES = 256 MiB`, `MAX_PLUS_CASE_CHUNKS = 256`. `MAX_CASE_BYTES` stays 16 MiB.
  - `src/lib.rs`: exports `UNPINNED_ANCHOR_DER`.
  - New file `unpinned_anchor.der`, explained under task 6 below.
  - Tests: `tests/did.rs`, `src/{media,registry}/tests.rs`.
- **canisters/registry**: `src/submit.rs` and `src/read.rs` fill `media_shard` from the stored `RecordEntry`. Its stored layout already held that byte, so no migration is needed. `registry.did` regenerated. Tests: `src/submit/tests.rs`, `src/proof.rs`, `tests/pocketic.rs`.
- **canisters/media**
  - `src/policy.rs`: `STALE_UPLOAD_NS = 15 min`.
  - `src/records.rs`: `Meta.touched_ns` is appended at the end of the stored value. Values written before this change decode with `touched_ns = created_ns`, so upgrades are safe.
  - `src/store.rs`: upload sessions are bound to the caller (task 3), Plus cases (task 5), `case_info`, and renewal on grant expiry now applies only to `Full` uploads.
  - `src/lib.rs`: `put_chunk` now receives the current time; new `case_info` query. `media.did` regenerated.
  - Tests: `src/store/tests.rs`, `tests/pocketic.rs`.
- **crates/witness-web**
  - `src/wire.rs`: `RecordWire.media_shard`; `CheckOptions.anchor` now defaults to empty; new `devAnchor`.
  - `src/proof.rs`: verified-record JSON includes `mediaShard`.
  - `src/check.rs`: `AnchorTrust` (pinned, dev, missing), `UNPINNED_ANCHOR_DER`, and the step output now carries `anchor`.
  - `src/case.rs`: the opener accepts up to 256 chunks.
  - Tests: `src/{case,check,proof,wire}/tests.rs`, `tests/registry_compat.rs`.
- **crates/witness-android**
  - `src/ffi.rs`: UniFFI records carry `media_shard` (default 0); new `CaseInfo`.
  - `src/client.rs`: `case_info` query.
  - `src/witness.rs`: anchor modes, top-level `case_info`, `case_limits`.
  - `src/verdict.rs`: `dev_anchor` and `credentials_judged`; `src/lib.rs` exports.
  - Tests: `src/witness/tests.rs`, `src/client/tests.rs`, `src/oracle/tests.rs`.
- **tools/witness-smoke** (`src/{main,net,steps}.rs`): pins `ca_root` from `ids.json` and adds three steps: media_shard routing, takeover refused, case upload plus `case_info`. If the daily case quota is used up, the case step reports "skipped" and still passes; it suggests `WITNESS_SMOKE_FRESH_DEVICE=1`.
- **scripts**
  - `deploy-local.sh`: reads `ca_root` with `icp canister call registry ca_root '()' --query -o hex`, decodes it, and writes it to `.icp-local/ids.json` as `"ca_root"` (hex DER). It now writes `ids.json` *before* `icp deploy web`; before, the web staging step read the previous run's file.
  - `stage-web.sh`: local config gets `anchor` (empty means fall back to the DEV query). With `WITNESS_WEB_ENV` set to anything other than local, staging fails unless `WITNESS_CA_ROOT` and `WITNESS_REGISTRY_ID` are set.
- **Regenerated** via `scripts/build-android-native.sh`: `client/native/witness/src/main/kotlin/uniffi/witness_android/witness_android.kt` and the jniLibs.

### What the tasks now do
- **Task 3 (upload takeover):** a `begin_upload` from a different caller on an uncommitted session that has been active in the last 15 minutes gets `retry_after_ns(remaining)`. That applies to the same signed request and to one with different content. "Active" means the uploader's last `begin` or `put_chunk`, and that time never moves backwards. After 15 idle minutes another caller can take the session over. The anonymous principal is treated as one caller. A committed path still returns `Ok` with nothing missing.
- **Task 5 (Plus cases):** without a grant, a case is limited to 16 chunks (`too_large` above that) and 16 MiB in total (`quota_exceeded`). With a valid grant presented at begin, a case can have up to 256 chunks and 256 MiB. Each chunk is charged to the grant's usage, so total Plus storage stays under `quota_bytes`. A grant that is invalid, expired, belongs to another caller, or has no signer is an error, not a silent fallback to free. Plus case chunks expire with the case (30 days) and release their usage when deleted.
- **Task 6 (trust anchor):**
  - Web: an empty anchor gives `anchor: "missing"`. Content Credentials are judged against a null anchor, so a Witness manifest shows as `{"kind":"unverified"}` and reason `UnverifiedCredentials`, and the check continues on the exact-hash/PDQ path.
  - Android: with `local=false` and no pinned `ca_root` (or an empty one), the client never queries the registry and returns `credentials_judged=false`. With `local=true` and no pinned root, it queries the registry once and returns `dev_anchor=true`.
  - **The null anchor:** `unpinned_anchor.der` is a valid self-signed Ed25519 CA certificate. I generated it once from `/dev/urandom` and never stored the private key, so nothing can chain to it. I used it because `witness-verdict` `Options.anchor` requires a `&TrustAnchor` and that crate is outside my lane.

### New contract fields for the client agent
- **Candid**
  - `Anchored`: `media_shard : nat8`.
  - `CertifiedRecord`: `media_shard : nat8`.
  - New: `type CaseInfo = record { expires_ns : nat64; bytes : nat64; chunks : nat32 }`.
  - Media: `case_info : (text) -> (opt CaseInfo) query`. This is a plain query and is not certified.
  - Media `begin_upload` can now also return `retry_after_ns` when another caller holds the session; `too_large` for too many chunks.
  - `media_shard` is **not** part of the MMR leaf. It is an unauthenticated routing hint.
- **UniFFI (Kotlin)**
  - `Anchored.mediaShard: UByte = 0u` (last field).
  - `CertifiedRecord.mediaShard: UByte = 0u` (last field).
  - `VerdictView.devAnchor: Boolean = false` and `credentialsJudged: Boolean = true` (last fields).
  - New `data class CaseInfo(expiresNs: ULong, bytes: ULong, chunks: UInt)`.
  - New `data class CaseLimits(maxChunks, maxBytes, plusMaxChunks, plusMaxBytes, chunkBytes)`.
  - New top-level `suspend fun caseInfo(witness: Witness, shard: UByte, caseId: String): CaseInfo?` and `fun caseLimits(): CaseLimits`.
  - `caseInfo` is deliberately not a method on `Witness`, so the existing `FakeWitness : WitnessInterface` still compiles.
  - All new fields have defaults, so existing positional constructors in Kotlin tests still compile.
- **JSON (wasm)**
  - `RecordWire` accepts an optional `mediaShard` (defaults to 0, so older payloads still parse). `verifyCertifiedRecord` output includes `"mediaShard"`.
  - `CheckOptions`: `anchor` is optional (empty or missing means missing), plus a new `"devAnchor": bool`.
  - `queryPlan` and `judge` output a top-level `"anchor": "pinned" | "dev" | "missing"`.
  - `credentials.kind` can now be `"unverified"` (only when no anchor is set).
  - `CaseOpener` accepts `count` up to 256.

### Tests
- **Changed expectations (intended behaviour changes):**
  - media `begin_is_resumable_and_rebinds_uploader` is now `begin_is_resumable_by_its_uploader`, because another caller can no longer rebind an active session.
  - web `wire::tests::check_options_parse`: `{}` now parses as anchor "missing".
- **Added**
  - common: case count bound, limit constants, `CaseInfo` round trip.
  - registry: `media_shard` reported by submit, dup and get.
  - media unit tests: upgrade decode of old `Meta`, `held_for`, replay takeover, activity and staleness, stale restart with new content, anonymous callers, free case caps, 256-chunk Plus cases, grant quota/expiry/forgery/no-signer, Plus case expiry, `case_info`.
  - media PocketIC: replay with a real second principal and the anonymous principal, takeover after the stale timeout, `case_info` and a Plus case past the free limit.
  - web: opener bitset beyond 64 chunks, caps match canister common, `mediaShard` in JSON with old-payload tolerance, `mediaShard` not in the proof, the three anchor modes, the unpinned anchor matches Android's copy.
  - android: the four anchor modes, `case_limits`, `case_info`.
  - smoke: `ca_root` parsing.

### Commands verified
- `cargo test` for each touched crate:

  | Crate | Result |
  |---|---|
  | witness-canister-common | 102 unit + did |
  | witness-registry | 120 + 7 PocketIC (incl. `dev-attestation`) |
  | witness-media | 122 + 14 PocketIC (incl. `local-dev`) |
  | witness-pages (`local-dev`) | 88 + 6 |
  | witness-web | 106 + golden + pocketic + 6 compat |
  | witness-android | 109 |
  | witness-smoke | 29 |

- `cargo test --target wasm32-unknown-unknown -p witness-web`: passes.
- Clippy `-D warnings`, native and with feature combos: `dev-attestation`, `local-dev`, `testkit`, `trustmark`, plus wasm32 `--all-features` for common, registry, media, pages, index and web: clean. `cargo fmt --all --check`: clean.
- `scripts/check-canister-deps.sh`, `scripts/check-core-deps.sh`, and the wasm32 release builds: clean.
- `./gradlew :native:witness:testDebugUnitTest :androidApp:testDebugUnitTest`: green. I did not run the full `test allTests`.
- `scripts/smoke-local.sh` (redeploy included): SMOKE OK, 32 steps. `ids.json` now holds `ca_root`, and the web config carries the pinned `anchor`.
- `adb -s emulator-5554 reverse tcp:8090 tcp:8090`: run.

### Deviations
- `case_info` is an uncertified Candid query, not certified HTTP.
- The Android side of task 6 can't be finished in my lane. `client/androidApp/build.gradle.kts` needs to read `ca_root` from `ids.json` into a BuildConfig field (empty outside local), and `RealNative.kt` should pass that value instead of `null` as `WitnessConfig.caRoot`. Until then the app runs in local DEV-query mode.
- `client/webApp/bridges/witness.mjs` currently throws when there is no anchor outside local mode. It should instead pass `{anchor: ''}` and treat `anchor: "missing"` / `kind: "unverified"`. When it falls back to the `ca_root` query in local mode, it should pass `devAnchor: true` and show the DEV flag. `credentialStage` should treat `unverified` as a credentials state. These are in the client lane.
- I did not rebuild `crates/witness-web/pkg` or the bridges' wasm, because the core crates were mid-edit by another agent. The client agent should run `client/webApp/bridges/build-wasm.sh`.

### Open issues
- An attacker who replays a begin and then sends chunks of the right length can keep an upload held until its pending TTL (1 day). A commit with the wrong bytes still fails the hash check.
- `media_shard` is still always the last media shard (registry selection is unchanged), and Android's `http_get_any` does not use it yet.
- Docs (STATUS, ARCHITECTURE §7.5 gaps) should be updated by the docs owner. Upload caller binding, case expiry exposure, the Plus case size decision and the pinned anchor are now done.
- Another agent's `witness-verdict` WIP (`zz_probe_false_altered`, plus a debug `eprintln!` in `confirm.rs`) makes `cargo test --workspace` hang.
- `scripts/android-local.sh` is not mine; someone else added it this session.
### rust proofs
#### Report: rust-proofs lane

Android checks now verify every registry read before using it. A `get_record`, `get_key` or by-index read whose proof fails is treated as not found, and the verdict says so with a new `proofs_rejected` flag. The proof verifiers moved out of witness-web into a new shared crate, `crates/witness-proof`, which witness-web, witness-android and witness-smoke now all use. Every check is green, including `scripts/smoke-local.sh` with a redeploy: **SMOKE OK, 33 steps** (one new step).

### Changes (files)

**New crate `crates/witness-proof`**
- The library code (`src/lib.rs`, `error.rs`, `certificate.rs`, `proof.rs`) is `#![no_std]` plus `alloc`.
- Its dependencies `ic-certification` and `serde_cbor` still need std, so it is no_std-friendly rather than strictly no_std.
- `certificate.rs` was moved from witness-web. It holds:
  - the IC certificate check: BLS signature, a single level of delegation, the canister-range check against the root key, LEB128 time, and the freshness check;
  - new: `IC_ROOT_KEY_DER`, the mainnet root key, checked byte for byte against ic-agent's copy.
- `proof.rs` holds:
  - `leaf_data` (the MMR leaf format), and `RecordParts::{record_id, root}`, which recompute `certified_data = SHA-256(mmr_root‖leaf_count‖smt_root)`;
  - `key_root` (the SMT proof for KeyInfo), `smt_bundle_root`, `shard_map_hash` over `ShardEntry` iterators, and `split_shards_bundle` / `shards_root`;
  - `Trust::{certify, record, key, shards}`.
- `registry.rs` (feature `registry`): `record_parts`, `key_info`, and `Trust::{certified_record, certified_key}` for the Candid `CertifiedRecord` / `CertifiedKey` types.
- `testkit.rs` (feature `testkit`): the test-subnet and certificate builders moved from web, plus a new `TestLedger` (MMR + SMT + signed certificate).
- `tests/live.rs` and the golden fixture `tests/data/local_registry.json`, captured from the live local replica. The record certificate includes a real delegation.
- Root `Cargo.toml`: added to the workspace members and `[workspace.dependencies]`.

**crates/witness-web**
- `certificate.rs` was deleted; `lib.rs` now re-exports `witness_proof::certificate`.
- `proof.rs` is now only a thin layer that maps the JSON wire types onto witness-proof. The public API and the JSON output are unchanged.
- `error.rs`: `From<witness_proof::Error>` keeps the same variant and the same text for each failure.
- `testkit.rs`: `ledger()` is built on `TestLedger`, and the certificate helpers are re-exported.
- `Cargo.toml`: dropped `ic-verify-bls-signature`, `ic_bls12_381`, `sha2` and `serde_bytes`; `ic-certification`, `serde_cbor` and `serde_bytes` are now native-only dev-dependencies.

**crates/witness-android**
- `transport.rs`: `Transport::root_key()`.
  - Local builds fetch the replica's root key through `fetch_root_key`.
  - Production builds use the pinned `IC_ROOT_KEY_DER`.
  - The test mock gained `with_root_key`.
- `client.rs`: `registry()` and `root_key()`.
- `witness.rs`:
  - The root key is fetched once and cached.
  - The clock is injected (`wall_ns` by default, `with_clock` in tests).
  - New `WitnessConfig.max_cert_age_s` (default 300 s; `Some(0)` is clamped to 1 ns rather than switching the freshness check off).
  - `check` builds a `Trust` against the registry principal.
- `oracle.rs`: `NetOracle` holds a `Trust` and an `AtomicBool` that records rejections.
  - `get_record`, `get_key` and every by-index read (exact, watermark, PDQ, derivatives) are verified first.
  - A failure, a key-id mismatch, a record-id mismatch, or a record sitting at an index that wasn't requested is treated as not found and raises the flag. It is not recorded as a network failure.
- `verdict.rs`: `VerdictView.proofs_rejected`.

**tools/witness-smoke**
- `net.rs` and `main.rs` now verify with witness-proof, and the `witness-web` dependency is gone.
- New step: "shards + verify certificate/SMT proof".
- The two judge steps now also require `!proofs_rejected`, so they cover Android's proof checks end to end against the live replica.
- `WITNESS_SMOKE_PROOF_FIXTURE=<path>` writes the golden fixture (an extra step that only runs when the variable is set).

**Regenerated** with `scripts/build-android-native.sh`: `witness_android.kt` and both jniLibs.

### New contract fields for the client agent
- **Kotlin `VerdictView.proofsRejected: Boolean = false`** (last field). When it is true, the UI should say that the registry gave answers that failed verification, rather than presenting a plain "No Witness record".
- **Kotlin `WitnessConfig.maxCertAgeS: ULong? = null`** (last field, so existing constructors still compile).
- The JS / wasm API is unchanged.

### Tests
- **witness-proof: 39 tests natively, 7 on wasm32.**
  - 15 certificate tests and the `leaf_data` test moved from web.
  - New: the IC root key, error display, record, key and shard proofs over ledgers of 1 to 9 leaves, and every tampered field.
  - New: malformed and hostile proofs, the `u16` and principal-length bounds on the shard map, wrong root key, wrong canister, and stale certificates.
  - New: 3 tests for the `registry` feature.
- **Live golden** (`tests/live.rs`, run both natively and on wasm32 with wasm-bindgen-test):
  - The live record, key and shard map all verify.
  - A tampered certificate, proof, leaf or claim is each rejected.
  - Freshness is enforced at ±5 min exactly, and the mainnet key and a different canister are both rejected.
  - The determinism digest `42ad079a…713f` is identical on native and wasm32.
- **witness-android: 109 → 120 tests.**
  - Honest reads leave the flag down; tampered records, tampered keys, wrong-index and wrong-id answers are absent and raise it.
  - Stale and foreign certificates are rejected, the root key is fetched once and a missing one is reported, and `check` sets `proofs_rejected`.
  - The config clamps as described, and the production root key needs no network.
  - The live fixture verifies through `NetOracle`, and a forged copy of it raises the flag.
- **witness-web: 106 → 91 unit tests.** 16 moved to witness-proof and 1 was added (error mapping). The golden, PocketIC and compat tests are unchanged and pass.
- **Intended changes to existing android tests:**
  - The test `World` now serves certified records and keys.
  - `get_key_rejects_a_different_key` now also asserts the flag is raised.
- **Removed one test:** smoke's `record_wire_copies_every_field`, because `record_wire` no longer exists. `record_parts_borrow_every_proof_field` in witness-proof covers the same ground.

### Commands verified
- `cargo test --workspace --no-fail-fast`: **2200 passed, 0 failed**.
- `cargo test --target wasm32-unknown-unknown -p witness-proof -p witness-web`: passes.
- Clippy `-D warnings` is clean:
  - all targets of proof, web, android and smoke;
  - witness-proof with `registry,testkit`;
  - android with `trustmark`;
  - wasm32 `--all-features` for proof and web.
- `cargo fmt --all --check`, `check-core-deps.sh` and `check-canister-deps.sh` are clean, and the wasm32 release build succeeds.
- `./gradlew :native:witness:testDebugUnitTest :androidApp:testDebugUnitTest`: green. I did not run the full `test allTests`.
- `scripts/smoke-local.sh` with a redeploy: **SMOKE OK, 33 steps**. `adb -s emulator-5554 reverse tcp:8090 tcp:8090` has been re-run.

**witness-web wasm size** (`--profile wasm`):

| Stage | Before | After | Change |
|---|---|---|---|
| wasm-bindgen | 1,451,965 B | 1,452,393 B | +428 B (+0.03%) |
| wasm-opt -Oz | 1,328,906 B | 1,329,127 B | +221 B (+0.02%) |

### Deviations
- Shards are verified in smoke but not on Android. Android routes probes using the shard map without checking it. Every record that comes back is still verified against the registry, so a forged map can only hide matches, not inject them.
- `Witness::get` and `Witness::get_key` still return raw, unverified replies to Kotlin. Only the check path verifies.
- I did not rebuild `crates/witness-web/pkg` or the bridges' wasm, because the JS API did not change.

### Open issues
- `canisters/registry/src/proof.rs` and `record.rs` still have their own copies of `leaf_data` and `shard_map_hash`; that is the producer side, in another lane. The `registry_compat` tests pin the two copies to each other. The registry owner could switch to witness-proof.
- The client lane needs to:
  - show `proofsRejected` in the UI;
  - optionally pass `maxCertAgeS`;
  - pass the pinned `ca_root` in `RealNative.kt`, which is still outstanding from the last report.
- The docs owner should remove the line in `docs/dev/STATUS.md` that says Android reads don't verify the MMR/SMT proofs, and update next step 4.
### case SRI
### Report: case-shell SRI (pages, stage-web, deploy-local, boot.mjs)

The case shell now pins the app bundle as ARCHITECTURE §8.1 requires. Pages takes an integrity manifest as an init/upgrade argument (no setter) and embeds two things in the case shell: an import map whose `integrity` entries list every code file, and a `boot.mjs` tag with `integrity` and `crossorigin`. The nonce plus `'strict-dynamic'` CSP is unchanged. In the browser through the local gateway, a tampered `boot.mjs` and a tampered wasm were each blocked. After restoring them the case decrypted again, and smoke passes: **SMOKE OK, 33 steps**.

### Changes (files)
- **`canisters/pages/src/pins.rs`** (new):
  - `Pin { path, sri }` and `Pins`.
  - Validation, done at install/upgrade and trapping on failure:
    - paths are relative, use only `[A-Za-z0-9._/-]`, are at most 128 bytes, and contain no `.`, `..` or empty segments;
    - integrity values must be exactly `sha384-` plus 64 base64 characters;
    - no duplicates, at most 256 entries, and `boot.mjs` must be present.
  - Entries are sorted, so input order doesn't matter.
  - `import_map(app)` writes `{"integrity":{"<app>/<path>":"sha384-…"}}`. Its JSON escaping turns `<`, `>`, `&`, control characters, U+2028 and U+2029 into `\uXXXX`, so nothing can close the `<script>`.
- **`canisters/pages/src/lib.rs`**:
  - `init` and `post_upgrade` now take `(PeerInit, opt vec Pin)`. An absent second argument decodes as unpinned, so existing installs still work.
  - `status` reports a new `case-unpinned` flag when no manifest was passed.
- **`canisters/pages/src/html.rs`**: `case_shell(o, pins)` puts the import map in `<head>` with the case nonce. The boot tag gets `integrity="…" crossorigin="anonymous"` next to the nonce.
- **`canisters/pages/src/site.rs`, `api.rs`**: `Site::new(origins, &pins)`. Every upgrade rebuilds the case response and re-certifies the root. The 404 and tombstone pages are re-certified too; the manifest doesn't change their bytes.
- **`canisters/pages/pages.did`**: adds `type Pin` and `service : (PeerInit, opt vec Pin)`. The did-drift test passes.
- **`scripts/stage-web.sh`**:
  - Computes sha384 SRI for every `*.js`, `*.mjs` and `*.wasm` in the staged dist, including `boot.mjs` and `witness-config.mjs`.
  - Writes `target/web/integrity.json` (override with `WITNESS_WEB_INTEGRITY`). `target/` is already git-ignored.
  - Fails on unsafe file names or more than 256 code files.
- **`scripts/deploy-local.sh`**:
  - New order: registry, index, media, then `ids.json`, then `stage-web.sh`, then pages with the manifest, then web.
  - Fails if the web build changed between pinning and deploying.
  - After deploying, checks that the served `boot.mjs` hashes to the pinned value and that the served `/v/*` shell carries that integrity.
- **`client/webApp/src/webMain/resources/boot.mjs`**: when the page has an import map, which only the case shell does:
  - A `fetch` wrapper adds `integrity` to every pinned URL. The browser therefore checks SHA-384 before the response reaches `instantiateStreaming`.
  - Any unpinned `.js`, `.mjs` or `.wasm` fetch is rejected.
  - The bundle script is loaded with `integrity` and `crossOrigin`.
  - The `witness.js` fallback is dropped, because it would load an unpinned script. On pages without an import map, behaviour is unchanged.
- Rebuilt with `./gradlew :webApp:composeCompatibilityBrowserDistribution`. This also re-emitted the Kotlin wasm from the current sources, as `2a4af7b3….wasm`.

### Tests added (17)
- **`pins.rs`, 10 tests**: sorting and lookup, input order, an absent argument, `boot.mjs` required, 17 unsafe paths, 10 malformed integrity values, duplicates and the 256 limit, the exact JSON, script breakout, and a Candid round trip.
- **`html.rs`, 4 tests**:
  - a pinned shell has the import map in `<head>` before the module tag, nonces on both, and the exact boot tag;
  - an unpinned shell has no import map and no integrity;
  - a hostile app origin is escaped inside the import map;
  - the shell is deterministic and ignores input order.
- **`site.rs`, 1 test**: pinning changes only the case hash and the certified root; the 404, tombstone and ic-domains hashes are unchanged.
- **`lib.rs`, 1 test**: init arguments decode both with and without pins.
- **`tests/pocket.rs`, 1 PocketIC test** (`upgrade_pins_the_case_shell_and_recertifies_it`):
  - the `case-unpinned` flag;
  - after an upgrade with pins, the import map and boot integrity are served on a verified certified response;
  - the CSP keeps the nonce and `'strict-dynamic'`;
  - the 404 and tombstone bodies stay byte-identical and verified;
  - re-pinning re-certifies the shell;
  - 4 bad manifests are rejected and leave the shell unchanged;
  - an upgrade without pins removes them.

### Commands verified
- `cargo test --workspace --no-fail-fast`: **2217 passed, 0 failed** (was 2200).
- `cargo test -p witness-pages --features local-dev`: 104 unit + 7 PocketIC, all passing.
- `cargo fmt --all --check` is clean. Clippy with `-D warnings` is clean for pages (all targets, with and without `local-dev`) and for wasm32 `--all-features`. `check-canister-deps.sh` is clean.
- `scripts/deploy-local.sh` printed "case shell pins boot.mjs and 11 more code files".
- `scripts/smoke-local.sh` with a redeploy: **SMOKE OK (33 steps)**.
- **Built-in browser, pages `/v/{id}#k=…&h=…`:**
  - It decrypted "3 of 3 verified". All 6 code requests (boot, config, bundle, 3 wasm) were in the pin set.
  - **Tampered `boot.mjs`** on web: Chrome logged "SRI's integrity checks failed… The resource has been blocked". Nothing beyond `boot.mjs` loaded and Compose never mounted.
  - **Tampered `witness_web` wasm**, still a valid wasm file: the fetch was blocked by SRI and the page showed "Couldn't reach Witness", with no decryption.
  - Both were restored and the web canister redeployed. The served hashes match the manifest again, the case decrypts, and `/p/{id}` (unpinned) still mounts.

### Deviations
- **Integrity map keys:** the import map pins by absolute URL in `integrity`, with no bare `imports` names. `boot.mjs` is shared with `/p/` shells and the web `index.html`, which have no map, so bare names would break them. On the case page, every module, script and wasm `boot.mjs` loads, directly or through the bundle, must be in the map or it is refused.
- **File names are not content-hashed.** `boot.mjs`, `originWasmWitness.js`, `skiko.*` and `witness-config.mjs` keep their names. The pin is the hash, not the name, so security doesn't depend on renaming, which would mean rewriting the webpack bundles.
- **Proof shells (`/p/`) are not pinned.** Their bodies are stored per record, so every web release would need a re-render pass over all stored shells, the same multi-tick problem as an origin change.
- **No smoke step was added** because `tools/witness-smoke` is outside my lane. `deploy-local.sh` does the end-to-end consistency check instead.
- **No JS unit test for `boot.mjs`.** The test harness lives in bridges, which is also outside my lane; the browser checks cover it.

### Open issues
- **Release coupling:** any web deploy must be followed by a pages upgrade with the new `target/web/integrity.json`, or the case page stops loading. It fails closed. `deploy-local.sh` handles this; a production pipeline must too.
- **Older browsers:** without import-map integrity support (before Chrome 127, Firefox 138, Safari 18), the static import of `witness-config.mjs` isn't verified. Everything else is still checked by the `integrity` attribute and the `fetch` wrapper.
- **Workers:** a worker started from a URL, for example a future onnxruntime-web worker for TrustMark, isn't covered by the `fetch` wrapper. It needs its own pin when added.
- **For the docs owner:**
  - In ARCHITECTURE §8.1 "As built", replace "The case shell has no integrity import map yet" with this mechanism, and add `(PeerInit, opt vec Pin)` to §7.4.
  - In `docs/dev/STATUS.md`, drop the "case shell has no integrity import map" limitation and remove it from next step 4.
- **For the smoke owner:** a step could assert that the `case-unpinned` flag is absent from pages `status`, and that `/v/*` carries `integrity=` matching the served `boot.mjs`.
### client fixes
Client fixes report

All five items are done and the client gate is green. The one gap: the web case page can't show provenance yet. The case bundle format is defined in Rust, and the Rust decoder rejects unknown keys, so I couldn't add a provenance field without editing Rust. For now the web page never claims "AI-drafted" unless provenance is present (details under item 2 and open issue 1).

**Changes** (paths are under `/Users/andrejones/Desktop/workspace/projects/witness`)

1. **Cleartext.** The allowances were already out of `src/main`: they sit in `client/androidApp/src/local/res/xml`, which is added only to the debug and benchmark variants. `src/main` only has `base-config cleartextTrafficPermitted="false"`. What was missing was a check on release, because only the debug unit tests run. I added a Gradle task to `client/androidApp/build.gradle.kts`:
   - It registers `verify<Variant>Cleartext` for every non-local variant (release and nonMinifiedRelease).
   - It reads the merged manifest and all of the variant's `network_security_config.xml` files, and fails on any `cleartextTrafficPermitted="true"` or `usesCleartextTraffic="true"`.
   - It is wired into `check` and `test`.
   - I proved it both ways: with a temporary `src/release` config allowing cleartext, the build fails and names the file; after removing it, the check passes.

2. **Summary provenance.**
   - **Model:** new `SummarySource` (GENERATED / EDITED / WRITTEN) on `Case`, in `core/model/.../Case.kt`. `afterUserEdit` decides the transitions.
   - **Domain:** `ManageCases.updateSummary` now records the user's edit as written or edited. New `draftSummary` records a generated summary; nothing calls it yet, because no generation path exists.
   - **Storage:** Room goes from version 2 to 3 (`WitnessMigrations.V2_TO_V3`). It adds a `summarySource` column and marks summaries that already exist as WRITTEN. `schemas/3.json` was generated.
   - **Android:** the case screen (`CaseReportScreen.kt`, `CaseReportViewModel.kt`) and the A2UI summary card (`core/a2ui` `SummaryDraftCard` with `LocalSummaryHeader`, provided by `androidApp/ui/CaseReport.kt`) use the AI frame, the "Draft summary" title and the flag button only for GENERATED. Written and edited summaries get a plain card titled "Summary · written by you" or "Summary · edited by you".
   - **Share and export:** the share toggle says "Include AI summary" only for generated summaries. The ZIP's `case.json` now includes `summary_source`.
   - **Web:** `feature/proof` (`OpenedCase.summarySource`, `SenderSummaryCopy`, `WebPages.kt`) labels by provenance. Since no provenance reaches the web today, it shows a neutral "Sender's summary" in a plain card instead of "AI-drafted".

3. **Local links.**
   - New `Links.at(origin)` in `core/domain/.../ServicePorts.kt`.
   - The build adds a `PAGES_ORIGIN` BuildConfig field, read from `.icp-local/ids.json` (`canisters.pages.url`) the same way the registry id is. It is set only for local variants; production keeps `witness.app`.
   - `AppGraph.links()` uses it, and `Features.links`, `AppNavigator` and `DeepLinks` (via `MainActivity`) are now passed it.
   - The composer chip already switches to the real link once the post is published. On the emulator it showed `4qggx-…localhost:8090/p/CJZ5MNBXR22E6T1C`, and that URL returned 200 from the host.

4. **Local run script and unreachable replica.**
   - New `scripts/android-local.sh [serial]` (default `emulator-5554`). It reads the port from `ids.json`, warns if no replica answers, runs `adb reverse`, builds the native library only if it's missing, builds `assembleBenchmarkRelease`, installs only if the APK's SHA-256 differs from what's installed, then launches. It passes `bash -n` and works with macOS bash 3.2.
   - Root cause of the hang: ic-agent's HTTP client waits up to 360 s per request, and update polling can run up to 5 minutes.
   - Fix: a new `remote {}` wrapper in `native/witness/.../NativeErrors.kt` gives every network call a 30 s limit (90 s for check and uploads) and then raises an offline error.
   - Verified on the emulator against a listener that accepts connections but never answers: the proof page went from "Loading proof…" to "Couldn't reach the registry · Try again" within 30 s, and after restoring the reverse it loaded "Original".
   - The device setup screen's offline state now says "Can't reach the Witness network right now…".

5. **Per-item erase.**
   - New `feature/settings/.../PublicItems.kt` lists each original that still has erasable public copies, newest first, with its grouped record id and how many public copies it has.
   - `PrivacyDataViewModel` and `PrivacyDataScreen` add a "Public items" section of rows styled like the "Erase all public copies" row: red "Erase public copies" title, the item and its copy count underneath. Each needs confirmation and calls the existing `EraseMine.publicCopiesOf`.
   - Wired in `androidApp/di/Features.kt`.
   - On the emulator: "2 public copies erased. Proofs stay.", the list emptied, and the page answered 410.

**Tests added:** about 88 new results, all passing.
- New test files: `LinksTest`, `SummarySourceTest`, `NativeTimeoutsTest`, `LocalLinksTest`, `PublicItemsTest`, `PrivacyItemEraseTest`.
- Extended:
  - case and domain: `ManageCasesTest`, `MigrationsTest` (2 → 3), `RoomStoresTest`, `CaseReportViewModelTest`, `CaseScreensRenderTest`
  - share: `ZipCaseExporterTest`, `ShareTextTest`, `ShareViewModelsTest`
  - links and web: `CaseLinkViewModelTest`, `ComposerViewModelTest`, `CaseReportRouteTest`
  - onboarding and settings: `DeviceSetupViewModelTest`, `SettingsSnapshotTest`
- I added a `hang` switch to the test fake `FakeWitness`. No tests were deleted and no existing expectations were changed.

**Commands verified**
- `./gradlew test allTests assembleDebug :androidApp:assembleBenchmarkRelease :androidApp:assembleRelease`: BUILD SUCCESSFUL. 3,754 test results, 0 failures (up from 3,666).
- `verifyReleaseCleartext` passes, and fails on a planted cleartext config.
- `bash -n scripts/android-local.sh`.
- The script run three times on `emulator-5554`: one fresh install, then "already installed" on each repeat.

**Deviations**
- I kept the existing `src/local` folder rather than creating separate `src/debug` and `src/benchmarkRelease` folders. It is the same idea with one file instead of two.
- A summary saved before this change counts as WRITTEN, because the app has never generated summaries.

**Open issues**
1. **Web provenance needs a Rust change.** Add an optional `summary_source` key 4 to the bundle in `crates/witness-web/src/bundle.rs` (encode, decode and `to_json`), and mirror it in `core/domain/.../CaseBundleCbor.kt`. Then pass it through `BridgeCases` into `OpenedCase.summarySource`; the web page already labels from that field.
2. **The emulator is short on space** (about 400 MB free). One reinstall failed with INSUFFICIENT_STORAGE; the script now skips reinstalling an unchanged APK, which avoids it.
3. **Another agent regenerated the uniffi bindings while my gate was running**, so one intermediate build failed. The final run against the settled bindings was clean, and I left no changes from that episode.
4. **Not wired:** pasting a local link into the web `/check` page, and the "check a photo at …/check" line in the case export when built locally. Both are local-only cosmetic gaps.
5. **Out of my lane:** the STATUS.md items that say "not scripted" and "labels a hand-written summary AI-drafted" need updating by its owner.
### matcher
### Report: matcher crop margin and crop retrieval

An uncropped copy now reports 100% coverage and no crop. Real crops report their crop to within ±2%. Large one-axis crops that hold a stored PDQ shape at the same anchor are now found. The ≈40%-kept crops of posts remain unreachable without the watermark. Adding crop regression tests exposed false "Altered" verdicts on unedited crops (about 0.5% of cases). I fixed those as well.

### Changes
**Coverage accounting**
- `crates/witness-match/src/diff/warp.rs`: the warp now counts the overlap: reference pixels whose centre maps inside the copy (within half a pixel). The edge margin, partial windows and overlay masks don't reduce it. Which pixels get compared is unchanged.
- `crates/witness-match/src/diff/mod.rs`: `PixelDiff` has a new field, `overlap_fraction`.
- `crates/witness-verdict/src/confirm.rs`:
  - `Comparison` has a new field, `covered_q16`. On the pixel path it is the overlap; on the grid path it equals the existing grid coverage.
  - `Reference.compared_q16`, which the client shows as "Cropped about N%" and "% of reference frame", now reports `covered_q16`.
  - The measured compared fraction is still what gates the `MIN_COMPARED_Q16` minimum and ranks placements.
  - `COULDNT_COMPARE` is now raised when coverage is below 75%, or when less than 75% of the overlap was actually compared (`Comparison::partly_compared`). Overlays and low resolution still raise it; a plain full-frame copy no longer does.

**False "Altered" on unedited crops**
- The cause was alignment drift on crops: about 0.2% scale or 0.1° rotation turned into small "changed" regions far from the centre.
- Added `upright()` in `confirm.rs`: the aligned placement with rotation removed and the scale snapped to 1 when within 1% (`UNIT_SCALE_SNAP`), pinned at the frame centre, and offered only within the existing `snap_px` limit.
- It is tried only when no placement came out clean, and it is kept only if it comes out clean (`Consistent`). It can only remove accusations, never add them.
- Measured on 780 crops (5 sizes × 4 seeds, 2–60% removed, all anchors, release build): false "Altered" went from 4 to 0.
- On 108 crops with a real pasted edit, detection went from 101 to 100. The one lost case was caught only by luck: under the correct placement that edit splits into three pieces below the size floor, and the misaligned placement merged them.

**Crop retrieval** (`crates/witness-verdict/src/query.rs`)
- New `recrop_hashes` fills the unused part of the 16-hash budget, so the cap holds.
- It hashes the copy's own crops at the stored shapes (`stored_shape_tags` on the copy's frame, skipping the full frame and low-quality hashes). Centred shapes come first, largest kept area first.
- The existing base, mirror and inset/outset hashes come first and are unchanged. For screenshots that already fill 16 hashes, nothing changes.
- I did this in `witness-verdict` with public `witness-fingerprint` functions; `witness-fingerprint` itself is not in my lane and is untouched.

**Golden digests** (verdict debug output changed because `compared_q16` changed; all 16 judge outcomes are unchanged)
- `crates/witness-core/tests/golden.rs`: `0aaed78d…` → `3d51aec49a25160ac291f24825a03c722effdcc7a444aa56155be74296e58667`.
- `crates/witness-web/tests/golden.rs`: `32f0cc5e…` → `82424ad6e1ca630b094bd94ae55f161b515b1b2fe1fdc867f520e9ea7b25d67c`.
- Both are identical on native and wasm32.

**Changed test expectations** (intended behaviour change): `assert_full_compare`, `assert_full_grid_compare` and `assert_unchanged_full_compare` in `crates/witness-verdict/src/tests.rs` now require `compared_q16 == Q16_ONE`, up from "at least 75%".

### Tests added
- **witness-match, `warp.rs`**: overlap counts margin and overlay pixels; overlap under translation equals the shared area (with or without margin, and zero when there is no overlap); plain resizes cover the full frame (including 640×480 → 213×160); a scaled crop's overlap is its footprint; a broken (NaN) transform gives zero overlap.
- **witness-match, `diff/mod.rs`**: a full-frame copy has overlap 1.0 while under 95% is compared; a 25% crop has overlap exactly 0.75; no overlap gives 0.
- **witness-verdict, `confirm.rs`**:
  - `partly_compared` rules.
  - `upright`: snaps drift to a pure translation through the centre; keeps a real rescale and drops only rotation; declines exact, mirrored, broken, shifted and large-rotation placements; is deterministic.
- **witness-verdict, `query.rs`**:
  - The budget fills with no duplicates and never goes over.
  - The order is centred shapes, largest area first.
  - A full budget is left alone; flat or degenerate images add nothing.
  - Anchored one-axis crops reach a stored hash: 4 seeds × 5–50% removed × start/centre/end, plus vertical 10–30%.
  - The base hashes alone miss a crop between stored aspects, and the new query finds it.
- **witness-verdict, `tests.rs`**:
  - `uncropped_copy_reports_no_crop`.
  - `uncropped_resizes_report_full_coverage_across_sizes_qualities_and_seeds`: 4 sizes including portrait × 4 scales × 2 qualities.
  - `real_crops_report_their_removed_share_across_sizes_and_seeds`: 5–60% removed, horizontal and vertical, several anchors, checked to ±2%; `COULDNT_COMPARE` above 25% removed.
  - `two_axis_crops_report_the_removed_area`: 19%, 36% and 51% removed.
  - `anchored_one_axis_crops_are_found_without_a_watermark`.
  - `crops_of_unrelated_photos_never_match_a_record`.
  - `unedited_crops_that_misregistered_are_not_accused`: the exact failing cases, reproduced with record-index padding. I checked that it fails with the new placement turned off.

### Commands verified
- `cargo test -p witness-verdict -p witness-match`, also in release: all pass, including the 118 `witness-verdict` lib tests.
- `cargo test -p witness-core -p witness-web -p witness-android -p witness-smoke -p witness-registry`: all pass.
- wasm32: `cargo test --target wasm32-unknown-unknown -p witness-core -p witness-web` passes, including both goldens. The new `query::` and `confirm::` tests also pass on wasm32 (13 and 22).
- `cargo clippy -D warnings` is clean: native for match, verdict and core, with the `testkit` feature for web and verdict, and wasm32 for core and web.
- `cargo fmt --check` is clean for my files.
- `crates/witness-web/build.sh` and `client/webApp/bridges/build-wasm.sh` rebuilt the web package; `node --test test/` gives 60 pass, 1 skipped.
- `scripts/build-android-native.sh` ran after I checked with `pgrep` that no one else was running it. Both ABIs built.
- Not run: the full `cargo test --workspace` (another agent was running it), Gradle and `smoke-local.sh`. No Kotlin or API contract changed.

### Deviations
- I fixed the false-accusation alignment issue, which goes beyond "crop margin". It lives in `confirm.rs` and blocks the extra crops that the new query hashes now send to confirmation.
- The new matrix tests are native-only and trimmed for debug runtime: about 35 s and 48 s. 1024 and 2048 px crops were measured only in the release probe.
- Docs to update, not edited because they are outside my lane:
  - `docs/dev/STATUS.md`: remove the "cropped about 12%" limitation.
  - ARCHITECTURE §6.1: query-side shape crops.
  - ARCHITECTURE §6.4: `compared` is now geometric coverage; the upright rescue placement.

### Investigation: large crops of posts showing "No Witness record"
Per ARCHITECTURE §6.1, PDQ is for near-whole-frame copies and arbitrary crops rely on the watermark. The grid can't help either: it only confirms candidates, and its placements are the stored shapes ±10% scale. A crop can be found by PDQ only if it contains a stored shape at that shape's own anchor (start, centre or end). The stored shapes always span the full length of one axis.

Measured on 6 sizes × 3 seeds = 18 cases per row, one-axis crops (release build):

| Crop | Found before | Found after | Confirmed after |
|---|---|---|---|
| 5–30% removed, anchored start/centre/end | 0–6 | 18 | 12–18 |
| 40% (matches the stored 4:5) | 16–18 | 16–18 | 16–18 |
| 50% | 0 | 18 | 12–13 |
| 60% (kept aspect 0.53, narrower than 9:16) | 0–5 | 0–5 | 0–5 |
| Off-anchor (1/3 offset): 10% / 40–60% | 0 | 0 | 0 |
| Off-anchor: 5% / 20% / 30% | | 9 / 7 / 12 | 8 / 5 / 12 |
| Vertical 40–60% (kept aspect wider than 1.91:1) | 0 | 0 | 0 |

The confirmed column drops to 12 of 18 because one seed in three has no public reference. On that grid-only path a crop between stored shapes can't be placed.

For a crop keeping about 40% of the frame:
- One axis of a 4:3 post: kept aspect 0.53, below 9:16 (0.5625), so no stored shape fits.
- Two axes (about 63% × 63%): no stored shape spans a full axis, so none fits.
- A 4:5 post cut vertically to 0.4 of its height: the 1.91:1 shape needs 0.419, so it is just out of reach.

There is no principled PDQ fix within 16 hashes: the stored shapes can't be rebuilt from inside a smaller crop. These crops need TrustMark.

### Open issues
- Grid-path crop percentages are coarse: −20% vertical shows as 32–36%, and −5% at 1024 px shows as 0%. The grid can't confirm crops between stored shapes. Both sit in `witness-fingerprint` grid alignment.
- At 128–256 px, pixel alignment can be off by about 3%, so a crop may be reported up to 4 points high (for example 24% for a 20% crop).
- Edits that break into pieces each under half a 40 px cell are missed even when they differ visibly (mean difference 31 levels). This was already the case.
- Another agent's in-progress `witness-proof` / `witness-web` proof files currently fail `cargo fmt --check` (none of my files are involved).
### consume
Everything in the client-consume task is done and green, with two exceptions noted under open issues. The Gradle gate passes with 3,779 test results and 0 failures (was 3,754). Bridges are 66 pass, 1 skipped, and the live test passes against the replica. `scripts/smoke-local.sh` redeployed everything and reports SMOKE OK, 33 steps.

Checking in the browser exposed a real bug: earlier web deploys shipped stale bridge JS and an old `witness_web` wasm. The Gradle npm install and webpack tasks never noticed changes under `client/webApp/bridges`. That means earlier browser checks by other lanes ran old bridge code. I fixed this in the build (details under Changes).

### Changes (files under `client/` unless noted)
- **Bindings:** ran `scripts/build-android-native.sh`. Both ABIs rebuilt, and the regenerated `witness_android.kt` was identical to the committed one.
- **Preview uploads use `media_shard`:** `native/witness/.../NativeNetwork.kt`
  - `NativeMedia.publish` reads the shard from `witness.get(recordId).mediaShard` instead of `shards().last()`.
  - If the record isn't in the registry yet, it raises `OfflineException` rather than guessing a shard.
- **Case expiry from `case_info`:**
  - `NativeMedia.caseExpiry` calls the top-level `caseInfo` through an injected lambda, so `FakeWitness` still compiles. It returns null when offline or refused, so a share never fails after commit.
  - `MediaGateway.caseExpiry(caseId)` defaults to null (`core/domain/.../NetworkPorts.kt`).
  - `ShareCase` stores the canister expiry on the `CaseShare` after commit. The share sheet and case page now show the date the canister will actually delete the case.
  - `FakeMedia.caseExpiryMs` added for tests.
- **Plus case sizing:** `Entitlement.PLUS_CASE_BYTES` = 256 MiB. `ShareCase.limitFor` returns `min(quotaBytes, 256 MiB)` for Plus and 16 MiB for free. `NativeMedia.MAX_CASE_CHUNKS` went from 16 to 256.
- **Failed proof checks are a distinct UI state:**
  - New `CheckResult.proofsRejected` and `registryUnverified`. `NativeMapping.verdict` maps `proofsRejected`.
  - When a check finds no record and some proofs were rejected, `CheckOutcome` shows "Couldn't verify the registry", a body saying nothing was trusted, and a failed "Registry" fact.
  - "Saw this online?" is hidden in both `CheckScreen` and `WebCheckPage`, and the check is not saved to recent checks.
  - When there is a match but some proofs were rejected, a note is added instead.
- **Pinned anchor:**
  - `androidApp/build.gradle.kts` adds a `WITNESS_CA_ROOT` BuildConfig field. Local variants read `ca_root` from `.icp-local/ids.json`. Other variants read `-Pwitness.caRoot` or `WITNESS_CA_ROOT` and are validated as hex.
  - `IcpEndpoint` gains `caRoot`.
  - `NativeAvailability.endpointOf` returns null in production without a well-formed anchor.
  - `RealNative.config` passes the anchor bytes as `WitnessConfig.caRoot`.
  - `NativeTrust` uses the pinned root for sealing without querying the registry.
- **Web bridges:**
  - `candid.mjs`: `media_shard` added to the record type, plus a new `mediaIdl` with `case_info`.
  - `engine.mjs`:
    - `recordWire.mediaShard`.
    - Three anchor modes: pinned, local DEV query with `devAnchor: true`, and production without an anchor, which passes `anchor: ''`.
    - `unverified` counts as a credentials result.
    - Previews try the record's media shard first.
    - `openCase` uses `case_info` for the chunk count (up to 256) and returns `expiresMs`.
  - `witness.mjs`: `networkIo.caseInfo`, and `open` passes `config.local`.
  - `BridgeCases` and `WebCaseLinks` carry `expiresAtMs`. On the deployed web case page (`/v/`) I confirmed in the browser that it now reads "expires Dec 19" and "3 of 3 verified" instead of "No expiry set".
- **Build fix for stale bridges:** `client/build.gradle.kts` makes `kotlinNpmInstall` and `kotlinWasmNpmInstall` treat the bridge sources as inputs and reinstall the copied package. `webApp/build.gradle.kts` adds the same inputs to the webpack tasks.

### Tests added
- **Domain and check UI:**
  - `ShareCaseTest`: case limits match the canister; the share records the canister's expiry.
  - `CheckOutcomeTest`: rejected proofs with no record give the new state; rejected proofs beside a match only add a note.
  - `ShareViewModelsTest`: the share sheet shows the canister's expiry.
- **Native module:**
  - `NativeNetworkTest`: uploads follow each record's shard; an unanchored record waits instead of guessing; case expiry comes from `case_info`; 256-chunk Plus ceiling.
  - `NativeMappingTest`: `proofsRejected` is mapped.
  - `NativeVerifyAndKeysTest`: a pinned anchor is used without asking the registry.
- **Android app:** `NativeWiringTest` (production refuses to run without an anchor, local builds pin when they can, `WitnessConfig` carries the anchor bytes, BuildConfig anchor is hex DER or empty).
- **Web Kotlin:** `BridgeCasesTest` and `WebGraphTest` (expiry travels with the opened case).
- **Bridges:**
  - `engine.test.mjs`: the three anchor modes, including retry; `unverified` credentials stage; preview shard order; `case_info` chunk count and expiry; `case_info` down.
  - `candid.test.mjs`: media IDL against `media.did`; `case_info` round trip.
  - `live.test.mjs`: `expiresMs` is present.

**Expectations I changed (each is an intended behaviour change):**
- `limitsDependOnEntitlement`: the Plus limit goes from 2^34 to 256 MiB.
- The preview test expects the record's shard (0) instead of the last shard (1).
- `begin` now rejects 257 chunks instead of 17.
- `endpointOf` for production without an anchor now returns null.
- The engine anchor test was rewritten for the three modes; the `openCase` cap is 256; `recordWire` and the opened-case JSON include `mediaShard` and `expiresMs`.
- The candid test fixture includes `media_shard`.

### Commands verified
- `./gradlew test allTests assembleDebug wasmJsBrowserDistribution :webApp:composeCompatibilityBrowserDistribution :androidApp:assembleBenchmarkRelease`: BUILD SUCCESSFUL, 3,779 results, 0 failures.
- `node --test test/`: 66 pass, 1 skipped. `WITNESS_LIVE=1 node --test test/`: 67 pass.
- `scripts/stage-web.sh`: anchor pinned, 12 code files pinned.
- `scripts/smoke-local.sh` (redeploys web and pages with the manifest): SMOKE OK, 33 steps.
- `adb -s emulator-5554 reverse tcp:8090 tcp:8090`: re-run.

### Deviations
- I didn't add a build-time failure for release variants without an anchor, because `test` compiles the release variant and would break. Production instead refuses to start the native client without a pinned anchor.
- The verdict JSON doesn't carry `devAnchor` or `anchor`, and the Kotlin UI doesn't show `credentialsJudged`.
- I didn't install the APK on the emulator. The benchmark release APK builds.

### Open issues
- **Browser cache breaks the SRI-pinned case page after a redeploy (pages/web lane).** The browser kept the old cached `/v/` shell, so the new bundle failed the integrity check and the page didn't load until I forced a refetch. The `/v/` shell and the un-hashed code files need `Cache-Control: no-cache`.
- **Plus cases over 16 chunks will fail.** `NativeMedia` passes `grant = null`, so the canister refuses them. There's no grant plumbing in Kotlin yet.
- **The share-sheet expiry picker isn't enforced.** The canister keeps cases for a fixed 30 days, so the picker choice is ignored. A share set to "24 h" now honestly shows the 30-day canister date.
- **Smaller gaps:**
  - The release registry id and host are still empty in BuildConfig.
  - `NativeMedia.reference` still tries every media shard.
- **For the STATUS docs owner:** remove these lines:
  - "Android reads don't verify proofs"
  - "media_shard is always the last shard" (on the client side)
  - "No expiry set"
  - the Plus case size gap
# End-to-end walk

I ran the whole e2e flow on emulator-5554 against the local replica and fixed three bugs along the way. Everything is green afterwards. The ~40% crop result is mixed: cutting 40% off the height is found, cutting 40% off the width is not.

Bugs fixed, each with a regression test:
1. **`scripts/android-local.sh` aborted on a fresh install.** When the app isn't installed, `adb shell pm path` exits 1. With `set -euo pipefail` that killed the script silently right after `adb reverse`, so nothing was built or installed. Fix: wrap the `pm path` and `sha256sum` probes in `{ …|| true; }`. New `scripts/test-android-local.sh` stubs adb, curl and gradlew and covers both fresh install and upgrade; it fails with the old line and passes with the fix.
2. **The property-damage case screen showed three identical "SUMMARY / Nothing written yet." cards.** The `LocalSummaryHeader` added by the client-fixes lane applied to every `SummaryDraft` component, so "Affected areas" and "Repair notes" (and "Room notes" in the rental template) were relabelled. Fix: added `SummaryHeader.forTarget()` and `WitnessEventSink.targetOf()`, and `SummaryDraftComponent` now uses the provenance header only for the `summary` target. Tests: new `client/core/a2ui/src/commonTest/kotlin/app/witness/core/a2ui/SummaryHeaderTest.kt` (5 tests) and `onlyTheSummarySlotTakesTheSummaryHeader` in `client/androidApp/src/test/kotlin/app/witness/ui/CaseReportRouteTest.kt`.
3. **The SRI-pinned `/v/` case shell was cached for 5 minutes** (`public, max-age=300`), the stale-cache problem the consume lane reported. Fix: `canisters/pages/src/http.rs` now sends `no-cache` for the case shell. Test: `the_pinned_case_shell_is_revalidated_on_every_load`. After redeploying, the gateway serves `cache-control: no-cache` and the case link still decrypts.

**Commands verified:**
- `scripts/deploy-local.sh`: OK.
- `scripts/smoke-local.sh`: SMOKE OK, 33 steps, both before and after the pages fix (the second run included the redeploy).
- `scripts/android-local.sh`: fresh install and launch, then reinstall after the a2ui fix.
- `cargo test -p witness-pages`, with and without `local-dev`: 105 + 7 passed. Clippy `-D warnings` and `cargo fmt --all --check` are clean.
- `cd client && ./gradlew test allTests --continue`: exit 0, 3,790 test results, 0 failures.
- `adb reverse tcp:8090 tcp:8090` re-run.

**Deviations:**
- The pages fix is in another lane; the task allowed minimal fixes anywhere.
- To pick files on web `/check` I stubbed `HTMLInputElement.click` in the page with JS, because the built-in browser has no file-upload tool. The copies were the media canister's preview of the post, and the crops were made in the browser.
- For the in-app check I pushed that same preview to `/sdcard/Pictures/witness-post-copy.jpg` on the emulator and picked it with the system picker. It is still there.
- I erased only the post. Erasing the case items would have broken the case link.
- No commits and no deployment beyond the local replica.

**Open issues:**
- A 40% width crop of a portrait post (kept aspect 0.45) is "No Witness record". This needs TrustMark.
- The case summary editor sheet is titled "Draft summary" even when you write the summary yourself.
- Photo times on the case page show in UTC (3:42), while the phone shows local time (10:42).
- The case was shared with the default expiry option, but the page shows the canister's 30-day expiry (Dec 19). This matches the known picker gap.
- `docs/dev/STATUS.md` still has `__PLACEHOLDER__` tokens and outdated limitation lines; that's for the docs owner.

Screenshots are in `/Users/andrejones/Desktop/workspace/projects/witness/client/screenshots/e2e-final/` (01 to 31).

| Item | Status | Evidence |
|---|---|---|
| scripts/deploy-local.sh | verified | Exit 0. Registry, index_0, pages, media_0 and web deployed; ids.json written; 'case shell pins boot.mjs and 11 more code files'. |
| scripts/smoke-local.sh | verified | SMOKE OK (33 steps) with WITNESS_SMOKE_SKIP_DEPLOY=1, and SMOKE OK (33 steps) again with a full redeploy after the pages Cache-Control fix. |
| scripts/android-local.sh (fresh install) | fixed-now | After 'adb uninstall app.witness' the script exited silently after adb reverse: with set -euo pipefail, 'pm path' returning 1 killed it. Wrapped the pm path and sha256sum probes in '{ ... \|\| true; }'. New scripts/test-android-local.sh (stubbed adb, curl and gradlew) covers fresh install and upgrade; it fails with the old line and passes with the fix. Real run: 'installed androidApp-benchmarkRelease.apk', 'launched app.witness'. |
| Fresh install -> enroll (DEV) | verified | Welcome -> permissions (camera and location granted) -> 'Hardware key created · Dev key', 'Development device · proofs show a DEV badge', 'Witness certificate issued', 'Ready to capture'. Screenshots 01-03. |
| Capture -> anchored | verified | Camera shows 'Dev key seal'. After capture the composer showed 'Sealed 10:31:29 AM · anchored' within about 5 s. Screenshots 04-05. |
| Post with 2 blurs | verified | Edit -> Blur -> two taps -> Done. Composer shows '2 declared blurs' with two 'Area' chips. Share post opened the system share sheet with the image and http://4qggx-...localhost:8090/p/K8RXRVG2J5TWXHJV. Screenshots 06-08 and 14. |
| Share link opens LOCAL proof page in built-in browser | verified | Pages canister /p/K8RXRVG2J5TWXHJV, title '[DEV] Matches an original, with 2 declared blurs · Witness'. The page reads 'Matches an original · with 2 declared blurs · the photographer's own edits', with a DEV notice and a '2 declared blurs' chip. Screenshot 09. |
| Web /check full-frame copy | verified | post-copy.jpg (810x1080 preview): 'Matches an original with changes', 'Aligned 100% of reference frame', What changed: 'Recompressed 100% of the compared area matches' and '2 blurs declared'. No 'Cropped' line. Screenshots 10-11. |
| Web /check ~40% crop | verified | Centred crop removing 40% of the width (486x1080, kept aspect 0.45, below 9:16): 'No Witness record'. Fingerprints: no match; watermark reader unavailable. This matches the matcher lane's analysis that it needs TrustMark. Centred crop removing 40% of the height (810x648): 'Matches an original with changes', 'Aligned 60% of reference frame', with a 'Cropped' row and 'Some regions couldn't be compared'. Screenshots 12-13. |
| In-app check of the post | verified | The post copy was picked via the system picker. Result: 'Matches an original · with changes', sealed reference 'With 2 declared blurs', 'Recompressed 100% of the compared area matches', '2 blurs declared · 2 areas'. Not Original. Screenshot 17. |
| Case report screen showed duplicate SUMMARY cards | fixed-now | A new Property damage case showed three 'SUMMARY / Nothing written yet.' cards: LocalSummaryHeader relabelled the Affected areas and Repair notes SummaryDraft components too. Added SummaryHeader.forTarget and WitnessEventSink.targetOf; SummaryDraftComponent now applies the provenance header only to the 'summary' target. Tests: SummaryHeaderTest (5) and CaseReportRouteTest.onlyTheSummarySlotTakesTheSummaryHeader. After reinstall the screen shows 'AFFECTED AREAS', 'REPAIR NOTES' and 'SUMMARY'. Screenshot 19. |
| Case with 2 items | verified | Case 'Hallway water leak' (Property damage); 2 captures from the case camera with the evidence coach; 'EVIDENCE · 2 SEALED · 2 anchored'. Hand-written summary labelled 'SUMMARY · WRITTEN BY YOU'. Screenshots 18 and 20-23. |
| Share case | verified | Share sheet: '2 of 2 selected', toggle reads 'Include summary' (not AI), '2 of 2 verified'. Share link produced http://4qggx-...localhost:8090/v/006RV99SSX4J97JBRP1JY0RXK8#k=…&h=…. Screenshots 24-25. |
| /v/ link decrypts, shows expiry and the correct summary label | verified | Built-in browser: 'Hallway water leak · Shared Nov 19 · 2 items · expires Dec 19', '2 of 2 verified · 2 original', 'SENDER'S SUMMARY: Water came through the hallway ceiling overnight. From the sender · not part of the proof' (neutral, not AI-drafted), 'Expires Dec 19, 2026'. Screenshot 26. |
| SRI-protected case shell (HTML inspected) | verified | The /v/ HTML has an importmap with 12 integrity entries and a boot tag <script type=module nonce=… integrity=sha384-sE0Q+3pt… crossorigin=anonymous src=…/boot.mjs>. The CSP has the nonce plus 'strict-dynamic' and the response carries an ic-certificate. The served boot.mjs is byte-identical to the source and its sha384 matches the pin. All 6 code resources loaded with 200. |
| Case shell browser caching (stale SRI shell after redeploy) | fixed-now | The /v/ shell was 'public, max-age=300', so a cached shell could pin an old bundle. canisters/pages/src/http.rs now sends 'no-cache' for Resp::Case. Test the_pinned_case_shell_is_revalidated_on_every_load. Pages: 105 unit + 7 PocketIC tests pass; clippy and fmt clean. After redeploy curl shows 'cache-control: no-cache' and the case link decrypts again (screenshot 31). |
| Per-item erase from Privacy | verified | Settings -> Privacy & data -> PUBLIC ITEMS listed 3 items. Erased 'Photo · 2BPJ-S9FP-4ZMK-GHVV · 2 public copies' (the post) after the confirm dialog, and it left the list. Screenshots 27-29. |
| Tombstone | verified | Pages /p/K8RXRVG2J5TWXHJV returns 410 'Page removed by its owner' in the browser; the media preview returns 404; registry get still returns the certified record. Still 410 after the pages redeploy. Screenshot 30. |
| Client Gradle gate after changes | verified | './gradlew test allTests --continue' exit 0; 3,790 test results, 0 failures, 0 errors. |
| Case summary editor sheet title | still-broken | The editor sheet says 'Draft summary' even for a hand-written summary. It is cosmetic and doesn't claim AI, so I left it. |
| Case page photo times | still-broken | The web case page shows 3:42 (browser UTC) for photos the phone shows as 10:42. The same time is presented in different timezones. |

# Security verification

I tried to break all seven closeout fixes and found three real holes. All three are fixed, each with a regression test, and everything is green.

**Holes found and fixed:**
1. **Android sent every upload as the anonymous principal.** `RealNative.kt` passed `identitySeed = null`, so the media canister's new caller binding protected nothing for the real app: any anonymous replayer counted as the same uploader. Each install now gets a random 32-byte identity, kept in `files/native/identity.seed`.
2. **`Witness::get` and `get_key` handed unverified registry answers to Kotlin.** The Android proof page, enrollment and preview upload all use them. Both now check the proof and fail with `Rejected` when a proof, the certificate age or the record/key id is wrong.
3. **Release builds could still send cleartext.** The native networking ignores Android's network security config and accepted `http://` outside local builds; it now requires https. The Gradle cleartext check also missed config files with other names or single-quoted values; it now catches both.

**Changes (files):**
- `crates/witness-android/src/transport.rs`, `crates/witness-android/src/witness.rs`
- `client/androidApp/src/main/kotlin/app/witness/di/RealNative.kt`, `client/androidApp/build.gradle.kts`
- I rebuilt the native libraries (jniLibs) with `scripts/build-android-native.sh`; `witness_android.kt` came out byte-identical.

**Tests added:**
- `crates/witness-proof/src/proof/tests.rs`: a record can't be moved to another index or ledger size, or verified with another record's proof or a padded/short one.
- `crates/witness-android/src/witness/tests.rs`: `get_and_get_key_refuse_answers_that_fail_their_proofs`, plus a production-over-http case in `new_rejects_bad_config`.
- `crates/witness-android/src/transport.rs`: `production_refuses_cleartext_replica_urls`.
- `canisters/media/src/store/tests.rs`: `free_chunks_cannot_ride_on_a_plus_case_and_grants_do_not_transfer`.
- `client/androidApp/src/test/kotlin/app/witness/di/NativeWiringTest.kt`: the per-install identity test.
- One expectation changed on purpose: `production_root_key_is_the_pinned_ic_key_without_network` now uses an https URL, because production http is refused. No tests deleted.

**Commands verified:**

| Command | Result |
|---|---|
| `cargo test -p witness-android --lib` | 122 passed |
| `cargo test -p witness-media --lib` | 123 passed |
| `cargo test -p witness-proof --lib` | 33 passed |
| `cargo test --target wasm32-unknown-unknown -p witness-proof` | passes |
| `cargo test -p witness-smoke` | 29 passed |
| `cargo test -p witness-pages http_origins_need_a_local_dev_build` (no `local-dev`) | passes |
| clippy `-D warnings` on android, proof and media (all targets) | clean |
| `./gradlew test allTests` | BUILD SUCCESSFUL |
| `verifyReleaseCleartext` | fails on a planted config, passes once it's removed |
| `WITNESS_SMOKE_SKIP_DEPLOY=1 scripts/smoke-local.sh` | SMOKE OK, 33 steps; its live `get`/`get_key` steps now go through the verified path |
| Live case page | the pinned hash on the `/v/` page matches the `boot.mjs` the web canister serves |

I also re-ran `adb -s emulator-5554 reverse tcp:8090 tcp:8090`.

**Not a problem:** `cargo fmt --all --check` reports one difference, in `crates/witness-verdict/src/tests.rs`. That is another agent's in-progress work, not mine.

**Deviations:**
- I edited files in other lanes (`RealNative.kt`, `build.gradle.kts`, android, media), as this task allowed for real fixes.
- The media canister still accepts anonymous uploads; the Android identity fixes the real client, and refusing anonymous wouldn't stop an attacker using any other principal.

**Open issues (still broken by design):**
- **Upload front-running.** The upload signature doesn't cover the caller or a nonce. Someone who sees the signed begin can replay it first and hold the upload for up to a day by sending junk chunks. Fixing this means adding the caller or a nonce to the signed upload bytes, across the common, media, Android and smoke lanes.
- **Plus grants aren't tied to one media shard.** Usage is counted per media canister, so with N shards a Plus user gets N times their quota. Today there is only one shard and production has no grant signer.
- **Smaller gaps:**
  - `benchmarkRelease` counts as a local build (cleartext allowed, root key fetched from the replica), so it must never ship.
  - The case-page gaps the earlier lanes already reported remain: older browsers without import-map integrity, future workers, and the browser-cache problem after a redeploy.

| Item | Status | Evidence |
|---|---|---|
| (a) Android check path (NetOracle): tampered record, key or certificate, or a stale certificate | verified | crates/witness-android/src/oracle.rs verifies every get_record, get_key and by-index read through witness-proof Trust. Existing tests reject tampered fields and stale or foreign certificates. I added a new attack test in crates/witness-proof/src/proof/tests.rs, no_record_verifies_at_another_index_size_or_with_a_borrowed_or_padded_proof. For ledgers of 1 to 9 records it tries every other index, every wrong leaf_count, another record's proof (borrowed and relabelled), and proofs with an extra or missing hash; all are rejected. witness-proof lib now has 33 tests, all passing, and passes on wasm32 too. |
| (a) Android Witness::get / Witness::get_key returned unverified registry data to Kotlin (the proof page, enrollment and NativeMedia.publish all use them) | fixed-now | This was real: ProofPageViewModel shows records from registry.get, which calls Witness::get, and that never checked the MMR, SMT or certificate. crates/witness-android/src/witness.rs now has a `verified` helper (cached root key, injected clock, max_cert_age). get and get_key return Err(Rejected "registry answer failed proof verification") when a proof fails, the certificate is stale, or the answer is for a different record or key id; a missing answer is still Ok(None). Regression test: get_and_get_key_refuse_answers_that_fail_their_proofs in src/witness/tests.rs (5 kinds of record tampering, wrong id, tampered key, wrong key id, stale clock, malformed id). witness-android: 122 tests pass. Smoke's live get and get_key steps now go through the verified path: SMOKE OK, 33 steps. |
| (b) Production web or Android judging Content Credentials without a pinned anchor | verified | Android: NativeAvailability.endpointOf returns null in production without a well-formed anchor, so StandInNative is used and nothing goes on the network. In Rust, local=false with no ca_root uses UNPINNED_ANCHOR_DER, sets credentials_judged=false and never queries ca_root. Web: stage-web.sh fails when not local and WITNESS_CA_ROOT is missing. In witness.mjs, caRoot() throws unless config.local. engine.mjs passes anchor '' outside local, which makes check.rs AnchorTrust::Missing and gives credentials kind 'unverified'. Nothing found that breaks this. |
| (c) Case shell running a modified boot.mjs, module or wasm | verified | Live check: the /v/ shell on pages carries the import map and integrity=sha384-sE0Q…DJXq on boot.mjs, and the boot.mjs served by web hashes to the same value. CSP is nonce + 'strict-dynamic', base-uri 'none'. In boot.mjs, every pinned URL gets its integrity forced and any unpinned .js/.mjs/.wasm fetch is refused. I checked the skiko and origin bundles: their XMLHttpRequest paths are only for file:// URIs or workers, and the wasm fallback goes back through fetch, so there is no unpinned way to load code on the main thread. Remaining gaps were already known: browsers without import-map integrity, future workers, and a stale-cache availability problem (fails closed). |
| (d) Media upload takeover through a replayed BeginUpload: the real Android client uploaded as the anonymous principal | fixed-now | This was real. RealNative passed identitySeed=null, so every Android upload came from the anonymous principal. The canister treats anonymous as a single caller, so any anonymous replayer counted as the same uploader and could take over an active upload. Binding uploads to the caller protected nothing for the shipped client. Fix in client/androidApp/src/main/kotlin/app/witness/di/RealNative.kt: a persistent random 32-byte seed per install (files/native/identity.seed, written atomically, from SecureRandomSource) is passed as WitnessConfig.identitySeed. Regression test: NativeAvailabilityTest.uploadsComeFromAPersistentPerInstallIdentityNotTheAnonymousPrincipal (seed persists, differs per install, a corrupt file is replaced, it is carried into the config). Canister logic is unchanged and correct: held_for checks a non-anonymous uploader correctly, touched_ns never moves backwards, and the stale boundary at exactly 15 minutes is covered by existing tests. |
| (d) Front-running before the first begin, or holding an upload by sending garbage chunks | still-broken | Known design limit, not fixed. BeginUpload's signature does not cover the caller or a nonce. An observer who replays the signed begin before the real device does becomes the uploader, and can keep the upload held for up to PENDING_TTL (1 day) by sending chunks of the right length. Hash checking stops content forgery, but this is a denial of service. Fixing it needs a contract change: put the caller principal (or a nonce or expiry) into UPLOAD_DOMAIN signing_bytes. That spans the common, media, android and smoke lanes. |
| (e) pages or media accepting http origins in non-local builds | verified | checked_origins in both canisters (pages/src/lib.rs, media/src/lib.rs) traps at init and post_upgrade unless every origin starts with https:// when local-dev is off. host_ok rejects uppercase hosts, paths and userinfo. Default build (no local-dev): cargo test -p witness-pages http_origins_need_a_local_dev_build passes, and so do the media local-origin and grant-signer tests. local-dev is enabled only in icp.yaml (local replica), and GRANT_SIGNER is None without local-dev. |
| (f) Release APK cleartext: the native Rust transport ignored Android's network security config | fixed-now | This was real. ic-agent in libwitness_android does its own networking, which Android's network security config does not govern, and AgentTransport::new accepted http:// when local=false. crates/witness-android/src/transport.rs now rejects non-https replica URLs outside local builds. Tests: production_refuses_cleartext_replica_urls, and a production http config added to new_rejects_bad_config. I changed one existing expectation: production_root_key_is_the_pinned_ic_key_without_network now uses https:// because production http is intentionally refused. |
| (f) Gradle release cleartext check could be bypassed | fixed-now | In client/androidApp/build.gradle.kts, CleartextCheck only scanned files named network_security_config.xml and only matched double-quoted "true". Now it scans every res/xml*/*.xml and accepts either quote style. Shown both ways: a planted src/release/res/xml/release_net.xml with cleartextTrafficPermitted='true' made verifyReleaseCleartext fail and name the file; after deleting it, BUILD SUCCESSFUL. verifyReleaseCleartext and verifyNonMinifiedReleaseCleartext both pass. |
| (g) Plus 256 MiB path: quota bypass without a grant, grant replay by another principal, overflow in the size math | verified | Every attempt failed: free chunks into a case already over 16 MiB (QuotaExceeded), regrowing a 16-chunk case to 256 with a grant (Unauthorized), the grant replayed by another principal or anonymous (Unauthorized), an inflated quota_bytes (GrantInvalid), size u64::MAX or over 16 MiB per chunk (TooLarge), and usage stays at 16 MiB. New regression test free_chunks_cannot_ride_on_a_plus_case_and_grants_do_not_transfer in canisters/media/src/store/tests.rs. Size math uses saturating_add and a per-chunk cap through check_shape. witness-media: 123 lib tests pass. |
| (g) A Plus grant is not bound to one media shard | still-broken | Not fixed, because it is architectural. EntitlementGrant signing_bytes has no audience or shard, and usage is tracked per media canister, so a Plus user can store up to quota_bytes on each media shard (N shards means N times the quota). Fixing it needs a shard or audience field in the grant format, or accounting across shards. Today only one media shard exists, and production has no GRANT_SIGNER. |

# Correctness verification

I checked eight of the closeout fixes against adversarial cases and found one real bug, which I fixed.

**Bug fixed (Plus case sizing):** Android lets a Plus user build a case up to 256 MiB, but it never sends an entitlement grant, so the media canister refuses anything over 16 chunks with `too_large`. The share failed only after encryption.
- I added `MediaGateway.maxCaseBytes`. NativeMedia sets it to the free 16 MiB limit, and ShareCase now caps its limit at that value, so the user gets TooLarge before anything uploads.
- Once grants are wired up, raising NativeMedia's value restores 256 MiB.
- Tests added: `plusCasesNeverExceedWhatTheGatewayCanUpload` and `casesWithoutAGrantAreCappedAtTheFreeCanisterLimit`. No existing expectations changed.

**Tests added elsewhere:**
- **Older clients still decode new replies:** a Candid test confirms clients built before `media_shard` can still read submit results and certified records. Nothing tested this before.
- **Matcher:** three new property tests on crops:
  - edits inside crops are never reported clean;
  - slight rotation neither accuses an unedited crop nor clears an edited one;
  - crops resized or padded back to full frame don't hide the crop.
  - In a wider 108-case probe, every edited crop came back Altered or flagged "couldn't compare"; none came back clean.

**Gaps you should know about (not fixed):**
- **Padded crops:** a crop padded back to full size with flat colour is reported as "cropped 0%". It always carries the couldn't-compare flag, so it is never a clean match.
- **Release unit tests never run:** AGP disables unit tests for release, so the release half of `LocalLinksTest` never executes. I checked the generated release and nonMinifiedRelease BuildConfig by hand: all local fields are empty.
- **witness-proof on wasm32:** only its 7 live-fixture tests run there; the 33 unit tests are native only. The live golden digest matches on both.
- **Docs and the next deploy order:**
  - `docs/dev/STATUS.md` line 28 still quotes the old web golden digest (docs owner).
  - A new client can't decode replies from a registry that predates `media_shard`, so canisters must be upgraded before clients.

**Also fixed:** wasm32 clippy with `-D warnings` was already failing on two unused native-only test helpers in witness-verdict. I gated them so it passes.

Everything else checked out: the stored-data decoding, no proof logic duplicated outside witness-proof, both golden digests on native and wasm32, summary labels on Android and web, and the per-item erase path down to the registry's `erase_mine`.

**Commands run, all passing:**
- Goldens, native and wasm32: `cargo test -p witness-core --test golden` and `-p witness-web --test golden`.
- witness-proof: natively with `registry,testkit`, and the live tests on wasm32 (7/7).
- witness-verdict: 121 tests native, 107 on wasm32.
- witness-canister-common: 103 unit tests plus the Candid file tests.
- `cargo fmt --all --check` and clippy `-D warnings` for verdict (native and wasm32) and common.
- `./gradlew test allTests`: 3,790 results, 0 failures.

I didn't redeploy or rerun smoke because no canister code changed. Nothing was committed.

**Files changed** (all under `/Users/andrejones/Desktop/workspace/projects/witness`):
- `canisters/common/src/registry/tests.rs`
- `crates/witness-verdict/src/tests.rs`
- `client/core/domain/src/commonMain/kotlin/app/witness/core/domain/port/NetworkPorts.kt`
- `client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/ShareCase.kt`
- `client/core/domain/src/commonMain/kotlin/app/witness/core/domain/fake/FakeNetwork.kt`
- `client/core/domain/src/commonTest/kotlin/app/witness/core/domain/usecase/ShareCaseTest.kt`
- `client/native/witness/src/main/kotlin/app/witness/nativelib/NativeNetwork.kt`
- `client/native/witness/src/test/kotlin/app/witness/nativelib/NativeNetworkTest.kt`

| Item | Status | Evidence |
|---|---|---|
| media_shard upgrade safety: old stored values decode | verified | canisters/media/src/records.rs Meta::from_bytes reads touched_ns only when 8 bytes remain and otherwise falls back to created_ns. The existing test meta_written_before_touched_ns_decodes_with_created_time covers this. CaseMeta, Usage and State layouts are fixed-size and unchanged. The registry RecordEntry already stored media_shard (record.rs head[17], round-trip test). The submit, read and erase paths route by the stored entry's media_shard. |
| media_shard: older clients that ignore the field | fixed-now | No test covered this, so I added clients_built_before_media_shard_still_decode_replies in canisters/common/src/registry/tests.rs. Structs shaped like the old clients (Anchored without media_shard, SubmitResult ok/dup, and opt CertifiedRecord without media_shard) decode new replies through Candid subtyping. Result: 103 common unit tests pass. In the client layers the field also has a default: web RecordWire uses #[serde(default)], the UniFFI records use #[uniffi(default = 0)], and the JS engine uses r.media_shard ?? 0. One limit: a new client cannot decode replies from a registry that predates the field (the Candid field is a plain nat8, not optional), so canisters must be upgraded before clients. |
| witness-proof is the only copy of the verifier logic in web, android and smoke | verified | I grepped for leaf_data, shard_map_hash, key_root, InclusionProof, SmtProof, certified_data, Sha256 and BLS. witness-web/src/proof.rs only maps wire types onto witness_proof (RecordParts, ShardEntry, Trust). Android oracle.rs uses Trust::certified_record and certified_key. Smoke net.rs uses witness_proof::shard_map_hash and Trust. Nothing re-implements them; smoke's sha2 is only used for identity seeds. The producer-side copies in canisters/registry (proof.rs shard_map_hash, record.rs leaf_data) remain, as already reported, and registry_compat pins them to witness-proof. Android to_record re-derives the record id, which repeats a check RecordParts::record_id already does. It is harmless and I left it. |
| witness-proof gives the same results natively and on wasm32 | verified | tests/live.rs passes natively and on wasm32 (cargo test --target wasm32-unknown-unknown -p witness-proof --test live: 7/7), including live_verification_is_deterministic_and_matches_golden (42ad079a…713f). The 33 lib unit tests run only natively; on wasm32 the lib reports 'no tests to run'. The earlier report's '7 on wasm32' therefore means the live tests, not the unit tests. |
| Matcher coverage fix does not hide real local edits inside crops | verified | New property test local_edits_inside_crops_are_never_reported_clean: 2 seeds × {10, 30}% removed × both axes × anchors {0, 2}, with a 64×64 foreign paste. Every case matches the right record and is either Altered or flagged COULDNT_COMPARE; none is reported clean. At least 80% are Altered (measured 14/16). In a wider release probe of 54 edited crops, 52 were Altered and the other 2 were COULDNT_COMPARE; none was clean. |
| upright rescue placement cannot clear real edits | verified | New test rotation_drift_neither_accuses_clean_crops_nor_clears_edited_ones: 20% crops rotated 0.2° and 0.5° (bilinear). Unedited copies give MatchesWithChanges, no regions and 20% reported. With a pasted edit added, the result is never clean. In the probe, the one rotated edit that wasn't Altered ended Inconclusive(NotRinged) with COULDNT_COMPARE, not Consistent, so upright never accepted it. upright() is only taken when its own result is Consistent. |
| Coverage fix does not hide crops resized back to full frame | verified | New test crops_resized_or_padded_back_to_full_frame_do_not_hide_the_crop. A centred 90% crop and a corner 80% crop, each resized back to 640×480, report 19% and 36% removed to within ±2 (assert_crop_reported). For 80% kept, COULDNT_COMPARE is set. |
| Crop padded back to full frame with flat fill | verified | Instrumented run: compared about 92%, covered 100%, changed 4–12%, classified Inconclusive(InBorderBand). The verdict says 'cropped 0%' (geometrically true, since the copy spans the frame) but always raises COULDNT_COMPARE, so it is never a clean match. The same test above asserts this (never clean) for 2 seeds. One thing for the product owner: the UI reads '% of reference frame 100%' together with the couldn't-compare flag in this case. |
| Golden digests updated consistently on native and wasm32 | verified | witness-core golden 3d51aec4… and witness-web golden 82424ad6… each pass natively and on wasm32 (cargo test -p … --test golden, both targets). No other code pins the old digests. docs/dev/STATUS.md:28 still cites the old web digest 32f0cc5e…; that belongs to the docs owner. |
| Summary provenance labels on Android | verified | SummarySource.afterUserEdit transitions:<br>- null → WRITTEN<br>- unchanged text keeps the previous source<br>- GENERATED or EDITED → EDITED<br>- cleared → null<br>The migration maps old summaries to WRITTEN. The AI frame, flag button and 'Draft summary' title appear only when isGenerated (CaseReportScreen SummaryCard and the A2UI SummaryDraftCard through LocalSummaryHeader, which the only consumer, androidApp/ui/CaseReport.kt, provides). Share toggle copy and the ZIP summary_source follow the same rule. One weak spot: LocalSummaryHeader defaults to generated=true, which is safe today because its only consumer provides it. |
| Summary provenance labels on the web | verified | OpenedCase.summarySource is always null because the Rust case bundle has no field for it. SenderSummaryCopy.label(null) gives the neutral 'Sender's summary' in a plain card, and isGenerated(null) is false, so the web never claims 'AI-drafted'. Carrying the real provenance needs a bundle key 4 in crates/witness-web/src/bundle.rs, which is still open. |
| Local links never reach release builds | verified | I regenerated BuildConfig. release and nonMinifiedRelease both have PAGES_ORIGIN, ICP_REGISTRY_ID, ICP_REPLICA_URL and WITNESS_CA_ROOT empty and ICP_FETCH_ROOT_KEY=false, and Links.at('') resolves to the witness.app defaults. Only debug and benchmarkRelease are local. One gap: AGP disables unit tests for release, so the release branch of LocalLinksTest never runs. Only testDebugUnitTest results exist, and the guarantee rests on the `local` predicate in build.gradle.kts. A stale build/intermediates/merged_manifests/staging folder exists, but no staging build type does. |
| Plus case sizing consistent across canister, web and Android | fixed-now | Canister: 16 chunks / 16 MiB free and 256 / 256 MiB with a grant, at 1 MiB per chunk. The web opener and NativeMedia both allow 256 chunks.<br><br>Bug: Android never presents a grant (NativeMedia.putChunk passes null) and no grant issuer exists. ShareCase still let a Plus user build a 17–256 MiB case, and every chunk's begin_upload would then be refused with too_large after encryption.<br><br>Fix:<br>- MediaGateway.maxCaseBytes (defaults to Long.MAX_VALUE).<br>- NativeMedia overrides it as Entitlement.FREE_CASE_BYTES.<br>- ShareCase limit = minOf(limitFor(entitlement), media.maxCaseBytes), so such a share returns TooLarge before anything is uploaded.<br>- FakeMedia exposes a settable maxCaseBytes.<br><br>Once grant plumbing exists, raising NativeMedia's value restores 256 MiB. Regression tests: ShareCaseTest.plusCasesNeverExceedWhatTheGatewayCanUpload and NativeNetworkTest.casesWithoutAGrantAreCappedAtTheFreeCanisterLimit. No existing expectations changed. |
| Per-item erase reaches registry erase_mine and updates the UI | verified | PrivacyDataViewModel.confirmEraseItem → eraseItem (wired in Features.kt:140 as eraseMine::publicCopiesOf) → EraseMine.erase → RegistryGateway.eraseMine → NativeRegistry.eraseMine → witness.eraseMine (the registry's erase_mine), which routes the media delete by entry.media_shard. On success EraseMine sets publicErased=true. The captures flow then rebuilds PublicItems.of, so the row drops off. derivativesOf is transitive (recursive CTE), and BuildPost.sourceOf always posts from the original, so grouping by `parent ?: recordId` matches what gets erased. |
| Pre-existing wasm32 clippy failure in witness-verdict tests | fixed-now | `cargo clippy -p witness-verdict --features testkit --tests --target wasm32-unknown-unknown -D warnings` failed on unused native-only helpers assert_unchanged_full_compare and recompress_matrix. I gated those two, plus my new rotated and assert_never_clean helpers, with #[cfg(not(target_arch = "wasm32"))]. Clippy is now clean on wasm32 and natively, and the wasm32 verdict lib passes 107/107. |

# Final gate

I ran the whole gate in the order given and every step passed, so I made no code changes. Then I updated the four docs. Nothing was committed, nothing was deployed beyond the local replica, and no tests were deleted or changed.

### Gate results (exact counts)

| Step | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| Clippy with feature sets | clean with registry+attest `dev-attestation`, pages+media `local-dev`, android `trustmark`, android `bindgen`, verdict+web `testkit`, index `bench`, proof `registry,testkit` |
| wasm32 clippy (`--all-features`) | core, web, proof, verdict and common clean with all targets. Registry, index, pages and media clean on the lib only (see deviations). |
| `cargo test --workspace --no-fail-fast` | **2,226 passed, 0 failed** |
| `dev-attestation` tests (registry + attest) | **293 passed** |
| `local-dev` tests (pages + media) | **249 passed** |
| `cargo test --target wasm32-unknown-unknown -p witness-core -p witness-web -p witness-proof` | **39 passed** (core 10 unit + 14 e2e + 1 golden; web 6 api + 1 golden; proof 7 live) |
| wasm32 release builds | OK for core, web, proof, registry, index, pages and media |
| `scripts/check-core-deps.sh`, `scripts/check-canister-deps.sh` | clean |
| `./gradlew test allTests assembleDebug wasmJsBrowserDistribution :webApp:composeCompatibilityBrowserDistribution --continue` | BUILD SUCCESSFUL, **3,790 test results, 0 failures** (counted from the result XMLs) |
| `node --test test/` (bridges) | 67 tests: 66 passed, 1 skipped |
| `scripts/test-android-local.sh` | OK |
| `scripts/smoke-local.sh` (full redeploy) | **SMOKE OK (33 steps)**; the case shell pins `boot.mjs` and 11 more code files |

I re-ran `adb -s emulator-5554 reverse tcp:8090 tcp:8090` after the redeploy.

**Rust tests per crate:** match 233, image 186, c2pa 176, attest 166, format 141, media 137, registry 127, fingerprint 122, android 122, verdict 121, pages 112, watermark 110, canister-common 107, web 99, merkle 92, index 81, proof 40, smoke 29, core 25.

**Kotlin results per module:** core/domain 588, core/icp 415, designsystem 244, model 243, feature/proof 237, check 223, core/ui 220, settings 172, a2ui 172, verify 164, androidApp 150, home 149, post 133, platform 126, case 121, capture 100, share 89, native/witness 78, core/data 73, webApp 60, telemetry 27, build-logic 6.

### Changes (docs only)
- **`docs/dev/STATUS.md`**:
  - Replaced every `__PLACEHOLDER__` with the counts above, and the per-crate and per-module tables.
  - Updated the golden digests: core `3d51aec4…`, web `82424ad6…`, proof `42ad079a…`.
  - Rewrote "What works" and "Known limitations" from the lane reports: removed the fixed items and added the open ones, such as upload front-running, grants without a shard, the anchor modes, release coupling and `benchmarkRelease` being treated as a local build.
  - Updated next step 4.
- **`README.md`**:
  - Added `witness-proof` to the status table and the repository layout, and to the wasm32 test command.
  - Added `test-android-local.sh` and `composeCompatibilityBrowserDistribution` to the test commands.
  - New deploy order, `ca_root` written to `ids.json`, the SRI manifest, and the 33 smoke steps.
  - The website section now redeploys through `deploy-local.sh` instead of `icp deploy web` alone, and says how anchors are pinned outside local. I removed the stale yarn workaround, since the consume lane's Gradle fix makes it unnecessary.
  - The Android section now uses `android-local.sh`, and covers `PAGES_ORIGIN`, `WITNESS_CA_ROOT`, the per-install identity, the 30 s and 90 s network timeouts, the release cleartext guards, and Plus case sizing.
- **`docs/ARCHITECTURE.md`**:
  - §16 rev 4 table: 7 new rows (`media_shard`, the `witness-proof` crate, the SRI import map, upload session binding, anchor pinning, Plus sizing and `case_info`, the coverage fix), and an updated "Open, recorded" row.
  - I also brought the "as built" text in line with the code: §4.3 anchor modes, §6.5 coverage, the upright placement and query-side crops, §7.2 `witness-proof` and `media_shard`, §7.4 `(PeerInit, opt vec Pin)` and `no-cache`, §7.5 caller binding, case sizes, `case_info` and gaps, §8.1 the SRI mechanism, §9 bundle limits, §14.1 and §14.2 gate, deploy order and `android-local.sh`.
- **`docs/SPEC.md`**:
  - §4.8: case links are 16 MB free and 256 MB on Plus, and they expire after a fixed 30 days with the date shown.
  - §11.2: the Plus feature list mentions the 256 MB case links.

### Deviations
- For registry, index, pages and media, wasm32 clippy with `--all-targets` fails inside `mio`, which the PocketIC dev-dependencies pull in. This is a toolchain limit, not a code problem, so I checked the lib target only, as the earlier lanes did.
- Most Gradle tasks were up to date, because the inputs hadn't changed since the last lanes' runs. The 3,790 count comes from those current result files.
- I did not run the `witness-verdict` wasm32 tests (107 per the correctness report); they aren't part of the gate.

### Open issues
All are recorded in STATUS:
- **Upload front-running:** the upload signature covers neither the caller nor a nonce.
- **Plus grants:** grants aren't tied to one media shard, and Android sends no grant yet, so it caps cases at 16 MiB.
- **Shard assignment:** the registry still gives every record the last media shard.
- **Release coupling:** every web deploy needs a pages upgrade with the new manifest, or case links stop loading.
- **Web case page:** summaries are always labelled neutrally, because the case bundle has no field for who wrote them. Photo times show in UTC.
- **Crops:** crops narrower than 9:16 or wider than 1.91:1 need TrustMark.

Files are in /Users/andrejones/Desktop/workspace/projects/witness:
- docs/dev/STATUS.md
- README.md
- docs/ARCHITECTURE.md
- docs/SPEC.md