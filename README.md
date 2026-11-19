# Witness

Witness is an Android camera app, with a companion website, whose photos anyone can verify: sealed at the shutter by a hardware key, anchored on the Internet Computer, and traceable through reposts, crops and screenshots.
Capture happens on Android; checking works in any browser, where the image never leaves the visitor's device.
One Rust core gives bit-identical verdicts on the phone, in the browser and in the canisters.

- Product: [docs/SPEC.md](docs/SPEC.md) · Technical design: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · UI: `design/renders/*.png`
- Engineering rules for contributors and agents: [BUILD.md](BUILD.md)
- What works today, test totals, next steps: [docs/dev/STATUS.md](docs/dev/STATUS.md)

## Status at a glance

| Area | State |
|---|---|
| Rust core (format, image, PDQ, grid, matching, C2PA writer + reader, Merkle, attestation, watermark codec, verdict) and the shared proof verifier (`witness-proof`) | Real, tested, deterministic on aarch64 and wasm32 |
| Canisters (registry, index, pages, media, web) | Real, running on a **local replica only** |
| Android app | Real native seal, enrollment, anchoring, check, posts with blurs, cases, case links, erase, on the emulator with **DEV** keys |
| Website (`/check`, `/p/{id}`, `/v/{id}`) | Real, served by the local `web` canister with certified reads |
| TrustMark watermark | Codec real; models not fetched, so "watermark unavailable" |
| Internet Identity, revocation sync, custom domains, mainnet | Pending (stand-ins or not built) |
| Witness Plus billing | RevenueCat wired, not configured (no API key), so purchases report "billing not configured" |
| Firebase telemetry | Wired, off by default, inactive until a `google-services.json` is added |

## Repository layout

```
crates/            Rust workspace
  witness-core/      Facade + seal() over the pure crates below
  witness-format/ -image/ -fingerprint/ -match/ -c2pa/ -merkle/ -attest/ -watermark/ -verdict/
  witness-android/   UniFFI cdylib for Android (seal, ICP calls via ic-agent, optional TrustMark runner)
  witness-web/       wasm-bindgen cdylib for the website (check, proofs, case decryption)
  witness-proof/     IC certificate, MMR, SMT and shard-map verifiers shared by web, Android and smoke
canisters/         common (shared Candid + runtime) · registry · index · pages · media · web (static)
tools/witness-smoke/  End-to-end smoke test against the local replica
client/            Gradle root: Compose Multiplatform app (Android + web)
  build-logic/       Convention plugins and module rules
  core/              model · designsystem · ui · domain · verify · icp · data · platform · a2ui · telemetry
  feature/           home · capture · post · case · share · settings · check · proof
  native/witness/    The one module with the Rust .so, the UniFFI bindings and the native adapters
  androidApp/        Android shell (Metro graph, navigation, NativeAvailability switch)
  webApp/            Web entry, bridges/ (npm package witness-bridges), seed/ (local test-data tool)
scripts/           deploy, smoke, builds, staging, model fetch, dependency checks
design/  docs/  video/
```

## Prerequisites

| Tool | Version | Used for |
|---|---|---|
| Rust | 1.95 stable, targets `wasm32-unknown-unknown`, `aarch64-linux-android`, `x86_64-linux-android` | everything Rust |
| `wasm-bindgen-cli` | exactly 0.2.130 (matches the lockfile; also the wasm32 test runner) | witness-web, wasm32 tests |
| `wasm-opt` (binaryen) | any recent | optional, shrinks witness-web |
| `cargo-ndk` + Android NDK 28.2 | | Android `.so` |
| `icp` CLI | 1.7 | local network and deploys |
| `candid-extractor`, `ic-wasm` | | canister builds |
| `jq`, `curl`, `bash` | | scripts (bash 3.2 is fine) |
| JDK | 17 for app code, 21 for the Gradle daemon (picked automatically) | client |
| Android SDK | compileSdk 37.1 (`client/local.properties` needs `sdk.dir`) | client |
| Node | 20+ | bridges tests, Kotlin/JS tooling |
| Chrome | | headless browser tests |

## Test

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p witness-registry -p witness-attest --features witness-registry/dev-attestation,witness-attest/dev-attestation
cargo test -p witness-pages -p witness-media --features witness-pages/local-dev,witness-media/local-dev
cargo test --target wasm32-unknown-unknown -p witness-core -p witness-web -p witness-proof
scripts/check-core-deps.sh && scripts/check-canister-deps.sh

scripts/build-android-native.sh
cd client && ./gradlew test allTests assembleDebug wasmJsBrowserDistribution :webApp:composeCompatibilityBrowserDistribution
cd client/webApp/bridges && node --test test/
scripts/test-android-local.sh
```

`scripts/build-android-native.sh` must run once before the first Gradle build (and after any Rust change the app should see): it writes the git-ignored `.so` files and regenerates the UniFFI bindings. PocketIC tests download nothing; they build their own canister wasm into `target/`.

## Build

```sh
cargo build -p witness-web --target wasm32-unknown-unknown --release
crates/witness-web/build.sh && client/webApp/bridges/build-wasm.sh
cd client && ./gradlew :androidApp:assembleBenchmarkRelease
cd client && ./gradlew :webApp:composeCompatibilityBrowserDistribution
```

`assembleBenchmarkRelease` is the minified, debug-signed APK (about 71 MB, both ABIs); the debug APK is much larger. More client commands are in `client/README-build.md`.

## Deploy locally and run the smoke test

```sh
scripts/deploy-local.sh
scripts/smoke-local.sh
WITNESS_SMOKE_SKIP_DEPLOY=1 scripts/smoke-local.sh
scripts/stop-local.sh
```

`deploy-local.sh` starts the `icp` local network (gateway `http://localhost:8090`), deploys registry (with `dev-attestation`), `index_0` and `media_0` (with `local-dev`), writes the ids and the registry's Witness CA root (`ca_root`, hex DER) to `.icp-local/ids.json`, stages the web build with `stage-web.sh` (which pins that anchor in `witness-config` and writes the SRI manifest `target/web/integrity.json`), deploys `pages` (with `local-dev`) with that manifest, then `web`, and checks that the served `boot.mjs` matches its pin. `smoke-local.sh` deploys, then runs 33 end-to-end steps (anchor pin, enroll, seal, submit, certified and proof-verified reads, shard map, `media_shard` routing, probe, judge, derivative, certified page, CSP, media upload, upload takeover refused, case upload + `case_info`, erase, certified 404) and writes `.icp-local/smoke.json`. Only ever deploy to the local replica.

## Run the website against local canisters

```sh
crates/witness-web/build.sh && client/webApp/bridges/build-wasm.sh
cd client && ./gradlew :webApp:composeCompatibilityBrowserDistribution && cd ..
scripts/deploy-local.sh
CARGO_TARGET_DIR=target cargo run --manifest-path client/webApp/seed/Cargo.toml
```

Open `http://<web-id>.localhost:8090/check` (the id is in `.icp-local/ids.json`). Proof pages are at `http://<pages-id>.localhost:8090/p/<RECORD_ID>` and on the web canister at `/p/<RECORD_ID>`. The seed tool creates an original, a resized and a face-blurred post, and an encrypted case, and writes their links to `.icp-local/web-seed.json`. Always redeploy the web build through `deploy-local.sh` rather than `icp deploy web` alone: the case shell `/v/` pins every code file by SHA-384, so a web deploy without the matching `pages` upgrade makes case links fail closed. Outside local, `stage-web.sh` refuses to stage unless `WITNESS_CA_ROOT` (hex DER of the pinned Witness CA root) and `WITNESS_REGISTRY_ID` are set; with no anchor, Content Credentials read *unverified* and `/check` continues on hashes and PDQ. Gradle reinstalls the bridges package whenever `client/webApp/bridges` changes.

## Run the Android app against local canisters

```sh
scripts/deploy-local.sh
scripts/android-local.sh [serial]
```

`android-local.sh` (default serial `emulator-5554`) reads the gateway port from `.icp-local/ids.json`, warns if no replica answers, runs `adb reverse` (needed because the local gateway rejects the `10.0.2.2` Host header), builds the native library only if it is missing, builds `assembleBenchmarkRelease` (`WITNESS_ANDROID_VARIANT` overrides), installs only when the APK's SHA-256 differs from the installed one, and launches the app. Run `scripts/build-android-native.sh` yourself after a Rust change. `scripts/test-android-local.sh` tests the script with stubbed tools.

Debug and benchmark builds read the registry id, the pages origin (`PAGES_ORIGIN`, so share links open the local pages canister) and the pinned Witness CA root (`WITNESS_CA_ROOT`, from `ca_root`) from `.icp-local/ids.json` at build time, and talk to `http://127.0.0.1:8090` (override with `-Pwitness.replicaUrl=...`). If the ids file is missing or the native library can't load, the app falls back to in-process stand-ins. Every network call gives up after 30 s (90 s for checks and uploads) with an offline error. The emulator enrolls as a **DEV** device; non-fatal errors are logged under the logcat tag `Witness`. Each install uploads under its own persistent random identity (`files/native/identity.seed`), not the anonymous principal.

Release builds allow no cleartext: `verifyReleaseCleartext` (part of `check` and `test`) fails on any cleartext allowance in the merged manifest or `res/xml*`, and the native transport refuses `http://` outside local builds. They have no endpoint configured, and the native client won't start without a pinned anchor: pass `-Pwitness.caRoot=<hex DER>` or `WITNESS_CA_ROOT`. `benchmarkRelease` counts as a local build and must never ship.

**Case sizes:** a free case link holds up to 16 chunks / 16 MiB; with a Witness Plus entitlement grant the media canister accepts up to 256 chunks / 256 MiB, charged to the grant's quota. The Android client doesn't present grants yet, so it caps every case at 16 MiB and reports *too large* before uploading anything. Case links expire after a fixed 30 days, read back with the media `case_info` query.

## TrustMark models (optional)

```sh
WITNESS_ENCODER_Q_SHA256=<sha256> WITNESS_DECODER_Q_SHA256=<sha256> scripts/fetch-models.sh --download
scripts/build-android-native.sh trustmark
```

The script downloads `encoder_Q.onnx` and `decoder_Q.onnx` into the git-ignored `models/` and refuses to run until both digests are pinned (verify them out of band against the official TrustMark release first). With the `trustmark` feature the app must also ship `libonnxruntime.so`. Without models everything works and reports "watermark unavailable".

## Firebase (later, optional)

Nothing in code needs to change:
1. Create a Firebase project.
2. Add an Android app with the application id `app.witness`.
3. Download `google-services.json` into `client/androidApp/` (it is git-ignored).
4. Enable Crashlytics (and Analytics) in the Firebase console.

The `witness.android.app` convention plugin applies the Google Services and Crashlytics Gradle plugins only when that file exists. Collection stays off until the user opts in under Settings → Diagnostics, and the website has no telemetry.

## Conventions

No code comments, lots of unit tests, deterministic core, modules and crates with one job each, no secrets in the repo, local-only deployment. See [BUILD.md](BUILD.md).
