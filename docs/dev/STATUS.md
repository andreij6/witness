# Witness: build status

As of 2026-11-19 (ARCHITECTURE revision 4, after the closeout lanes). Everything below runs on a developer machine against a **local** Internet Computer replica and the Android emulator `emulator-5554`; nothing has been deployed to mainnet. Commands are in the root `README.md`; the build reports these numbers come from are the other files in this folder.

## Gate

| Step | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean; also clean with `dev-attestation` (attest, registry), `local-dev` (pages, media), `testkit` (verdict, web), `registry,testkit` (proof), `bench` (index), `trustmark` and `bindgen` (android); wasm32 clippy clean for core, web, proof, verdict and common (all targets, all features) and for registry, index, pages and media (lib, all features) |
| `cargo test --workspace` | 2,226 passed, 0 failed |
| Feature test runs | `dev-attestation` (attest + registry): 293 passed; `local-dev` (pages + media): 249 passed |
| wasm32 tests | `cargo test --target wasm32-unknown-unknown -p witness-core -p witness-web -p witness-proof`: 39 passed (core 10 unit + 14 e2e + golden; web 6 api + golden; proof 7 live) |
| wasm32 release builds | core, web, proof, registry, index, pages, media: OK |
| `scripts/check-core-deps.sh`, `scripts/check-canister-deps.sh` | clean |
| `cd client && ./gradlew test allTests assembleDebug wasmJsBrowserDistribution :webApp:composeCompatibilityBrowserDistribution` | BUILD SUCCESSFUL; 3,790 test results (Android host + JS + wasmJs runs), 0 failures; includes `verifyReleaseCleartext` |
| Bridges `node --test test/` | 66 passed, 1 skipped (the live test; `WITNESS_LIVE=1` runs it against the replica) |
| `scripts/test-android-local.sh` | fresh install and upgrade OK |
| `scripts/smoke-local.sh` (deploy + end to end) | SMOKE OK, 33 steps; case shell pins `boot.mjs` and 11 more code files |

## Test totals

### Rust (`cargo test --workspace`, unit + integration + PocketIC + golden)

| Crate | Tests |
|---|---|
| witness-match | 233 |
| witness-image | 186 |
| witness-c2pa | 176 |
| witness-attest | 166 |
| witness-format | 141 |
| witness-media | 137 |
| witness-registry | 127 |
| witness-fingerprint | 122 |
| witness-android | 122 |
| witness-verdict | 121 |
| witness-pages | 112 |
| witness-watermark | 110 |
| witness-canister-common | 107 |
| witness-web | 99 |
| witness-merkle | 92 |
| witness-index | 81 |
| witness-proof | 40 |
| witness-smoke | 29 |
| witness-core | 25 |
| **Total** | **2,226** |

Golden digests, identical on aarch64 and wasm32: witness-core `3d51aec4…` (50 synthetic images, 16 judge outcomes), witness-web `82424ad6…`, witness-proof live fixture `42ad079a…`.

### Kotlin (Gradle test results; a multiplatform module counts each of its Android host, JS and wasmJs runs)

| Module | Results |
|---|---|
| core/domain | 588 |
| core/icp | 415 |
| core/designsystem | 244 |
| core/model | 243 |
| feature/proof | 237 |
| feature/check | 223 |
| core/ui | 220 |
| feature/settings | 172 |
| core/a2ui | 172 |
| core/verify | 164 |
| androidApp | 150 |
| feature/home | 149 |
| feature/post | 133 |
| core/platform | 126 |
| feature/case | 121 |
| feature/capture | 100 |
| feature/share | 89 |
| native/witness | 78 |
| core/data | 73 |
| webApp | 60 |
| core/telemetry | 27 |
| build-logic/convention | 6 |
| **Total** | **3,790** |

### JavaScript
`client/webApp/bridges`: 67 tests (66 pass, 1 live test skipped by default).

## What works end to end

Verified on emulator-5554 and in a browser against the local canisters:

1. **Onboarding and enrollment.** A fresh install creates a Keystore key, the registry verifies the emulator's attestation chain under `dev-attestation`, issues a Witness C2PA certificate, and the app shows the **DEV** badge.
2. **Capture → seal → anchor.** A camera capture is sealed natively and anchored by WorkManager in about 2–10 s. `registry get` returns the certified record, now with its `media_shard`.
3. **Post with blurs.** Live detection, real blurs, a sealed derivative, the redacted preview uploaded to the record's `media_shard`, and the share sheet carrying the JPEG, caption and a link that points at the local pages canister in local builds.
4. **Proof page `/p/{id}`.** Certified HTML shell on `pages` with the Compose app mounting on top.
5. **Checking.** In-app Check, Share → Witness and web `/check`. A sealed file is *Original*; a recompressed copy *Matches an original, with changes* at 100% of the reference frame (no false "cropped"); a crop that removes 40% of the height is found with its crop reported; an unrelated image is *No Witness record*. **Both clients verify every registry read:** the web through `witness-web`, Android through `witness-proof` (IC certificate, MMR/SMT proofs, freshness) on `check`, `get` and `get_key`; a failed proof shows "Couldn't verify the registry", never a plain "No Witness record".
6. **Trust anchor.** `ids.json` carries the registry's `ca_root`; the web config and the Android BuildConfig pin it, and smoke checks the registry still matches the pin.
7. **Cases.** Create a case, capture into it, write the summary (labelled "written by you", AI framing only for generated drafts), and share an encrypted case link. The `/v/` page is SRI-pinned (import map with integrity, `boot.mjs` integrity, `no-cache`), decrypts every item ("2 of 2 verified · 2 original"), shows the canister's real expiry from `case_info`, and labels the summary neutrally. A tampered `boot.mjs` or wasm is blocked by the browser.
8. **Uploads.** Upload sessions are bound to the caller (smoke: a replayed begin from another principal is refused); Android uploads under a persistent per-install identity. Plus cases up to 256 chunks / 256 MiB work with a grant (canister and PocketIC).
9. **Erase.** Per-item "Erase public copies" under Settings → Privacy & data, and "Erase all public copies": pages answers 410, media a certified 404, the index forgets the fingerprints, and the permanent record stays readable and verifiable.
10. **Local runs.** `scripts/android-local.sh` does the `adb reverse`, build, install-if-changed and launch. Release variants fail the build if any cleartext allowance reaches them, and the native transport refuses `http://` outside local builds.
11. **Smoke test.** `scripts/smoke-local.sh` exercises all of the canister paths above at the API level (33 steps).

## Known limitations

**Matching and verdicts**
- No TrustMark models, so there is no watermark. Crops that keep an aspect narrower than 9:16 or wider than 1.91:1 (for example 40% cut off the width of a portrait post), two-axis crops near 40% kept, and off-anchor crops come back *No Witness record*; the "watermark from a different photo" warning can't fire.
- Grid-path crop percentages are coarse (−20% vertical shows as 32–36%), and the grid can't confirm crops between stored shapes when there is no public reference. At 128–256 px a crop can read up to 4 points high.
- A crop padded back to full frame with flat colour reads "cropped 0%" with the couldn't-compare flag (never a clean match).
- Edits that break into pieces each under half a 40 px cell are missed. Below about 256 px wide, small grid-only edits show as *Matches with changes*.
- The benchmark corpus (§12) isn't built, so none of the recall or false-match targets are measured yet.

**Canisters**
- No revocation sync, no Internet Identity erase or linking, no moderation takedown, no off-chain root publisher.
- Upload front-running: the upload signature covers neither the caller nor a nonce, so whoever replays a signed begin *before* the device becomes the uploader and can hold the path for up to a day with junk chunks (commit still checks the hash). Fixing it is a contract change across common, media, android and smoke.
- Plus grants have no audience or shard, so with N media shards a Plus user could store N × quota. Plus is off without a grant-signer key, and the Android client presents no grant, so it caps cases at the free 16 MiB.
- `BeginUpload` carries no chosen expiry (fixed 30 days; the share-sheet expiry picker isn't enforced, and the case page honestly shows the 30-day date) and no content type; no early case deletion. `case_info` is an uncertified query.
- The registry still assigns every record the last media shard, and `NativeMedia.reference` tries every shard. A new client can't decode a registry that predates `media_shard`, so canisters must be upgraded before clients.
- Android verifies records and keys but not the shard map it routes probes with (a forged map can only hide matches). The registry keeps its own producer-side copies of `leaf_data` and `shard_map_hash`, pinned to `witness-proof` by tests.
- Release coupling: every web deploy must be followed by a pages upgrade with the new integrity manifest, or the case page fails closed (`deploy-local.sh` does this). `/p/` shells aren't pinned; browsers without import-map integrity don't verify `witness-config.mjs`; a future worker needs its own pin.
- An origin change re-certifies every page in one upgrade; production needs it spread over timer ticks.
- No canbench capacity runs; index capacity is estimated from a 10k-capture measurement.

**Android app**
- Runs only with DEV keys on the emulator so far; no physical phone, no real Google attestation chain, no StrongBox timing.
- `benchmarkRelease` counts as a local build (cleartext allowed, root key fetched from the replica), so it must never ship. Release BuildConfig still has no registry id or host, and production refuses to start the native client without a pinned anchor (`-Pwitness.caRoot`).
- Captures can't be sealed before enrollment and late (offline) enrollment isn't wired.
- Rotate, Tone, Filter and Orient edits are declared but not rendered into derivative pixels.
- No on-device model on the emulator, so no AI drafts or recommended-shot ticks. The summary editor sheet is titled "Draft summary" even for a hand-written summary.
- The verdict UI doesn't show `credentialsJudged` or the DEV anchor mode.
- Internet Identity, the in-app case-link opener and the report sender are stand-ins. Plus billing is wired to RevenueCat but `Purchases` is never configured, so purchases return "billing not configured".
- No reverse geocoding ("near 40.7° N, 74.0° W"). The minified APK is about 71 MB with both ABIs, over the 40 MB budget. The native library loads lazily on first use and could land on the main thread.

**Website**
- Production staging needs `WITNESS_CA_ROOT` and `WITNESS_REGISTRY_ID` (`stage-web.sh` refuses without them); without an anchor, Content Credentials read *unverified* and the check continues on hashes and PDQ.
- The case bundle has no summary-provenance key, so the case page always says "Sender's summary" (needs key 4 in `witness-web::bundle` and `CaseBundleCbor`).
- Case page photo times show in the browser's timezone (UTC on the test machine) while the phone shows local time.
- Pasting a local link into `/check` and the "check a photo at …/check" line in local case exports aren't wired. No desktop drag and drop on `/check`.
- Old bridge modules (`icp.mjs`, `idl.mjs`, `verify.mjs`, `wire.mjs`, `BridgeIcp`, `BridgeVerifyEngine`) are dead code kept for their tests.

## Next steps

**Before any mainnet deployment**
1. Settle the brand and **custom domains** (`witness.app`, `case.witness.app`, `app.witness.app` are placeholders) and register them (ARCHITECTURE §7.7); pass the real origins in `PeerInit.origins`.
2. Build production canisters **without** `dev-attestation` and `local-dev`; set `WITNESS_RELEASE_DIGEST` and `WITNESS_SCHNORR_KEY` (`key_1`); pin the resulting Witness CA root with `WITNESS_CA_ROOT` (web) and `-Pwitness.caRoot` (Android), and set the release registry id and host.
3. Revocation sync outcall (§4.4), the off-chain root publisher, controllers on hardware keys, reproducible builds with published module hashes, cycles top-up and freezing thresholds (§7.1). A release pipeline that upgrades pages with every web deploy's integrity manifest.
4. Remaining contract items: bind the upload signature to the caller or a nonce; give grants an audience/shard; assign `media_shard` across shards; a chosen case expiry and early deletion in media; multi-tick re-certification; summary provenance in the case bundle.
5. canbench: `post_upgrade` at 10 M records and the worst-case probe; set the shard-close threshold.
6. A physical Pixel run: real RKP chain enrollment, StrongBox vs TEE timing, seal latency budgets (M0).

**Product pieces**
- **TrustMark models:** pin the digests in `scripts/fetch-models.sh`, fetch, build with `trustmark`, ship `libonnxruntime.so`, and deliver the models through Play AI packs (§5.1); add the web decoder (`trustmark.mjs`) with its own SRI pin.
- **RevenueCat and Plus:** create the project, Play products and the entitlement, supply the public Android SDK key at build time and call `Purchases.configure`; stand up the off-chain entitlement signer whose public key is compiled into `media`; plumb grants through `NativeMedia` and raise its case cap to 256 MiB.
- **Firebase:** create the project, add the Android app `app.witness`, drop `google-services.json` into `client/androidApp/`, enable Crashlytics. No code changes.
- **Play listing:** release signing key, App Bundle (also fixes the APK size), Data safety form (SPEC §12.1), store copy with the approved claims, privacy policy and contact pages.
- **Internet Identity:** linking, erase from the web, sync.
- **Benchmark harness** (§12) and the published confusion matrix.
