# Witness — engineering brief (for every contributor and agent)

Source of truth: `docs/SPEC.md` (product), `docs/ARCHITECTURE.md` (technical), `design/renders/*.png` (UI). This file overrides them where noted under "Build decisions".

## Rules
1. **Modular.** Respect the module and crate boundaries in ARCHITECTURE §2. One responsibility per module or file. No cyclic dependencies. Interfaces (ports) at every IO edge.
2. **Hyper-efficient.** Choose the best time and space complexity. Avoid needless allocation in hot paths: reuse buffers, use slices instead of `Vec` clones, and stream instead of materializing. No O(n²) where O(n log n) or O(n) exists. Document complexity in test names or doc-free function names only when it's non-obvious. Benchmarks are welcome as tests.
3. **Lots of unit tests.**
   - Every public function gets tests: happy path, edge cases, adversarial and malformed input, and determinism.
   - **Rust:** `#[cfg(test)] mod tests` beside the code, plus `tests/` for integration.
   - **Kotlin:** `commonTest`, `androidUnitTest` and `webTest` where applicable.
   - Tests must pass. Never delete a failing test to get green; fix the code.
4. **No code comments** (repository convention). Express intent through names, types and small functions. Doc comments are also off, except where `#[doc(hidden)]` attributes are required.
5. **Determinism** (ARCHITECTURE D1): `witness-core` must give bit-identical results on aarch64 and wasm32.
   - No floating-point `mul_add`.
   - No `HashMap` iteration order in outputs.
   - No SIMD that reorders reductions.
   - No time or randomness inside pure functions; inject them.
6. **No deployment beyond local.** ICP canisters deploy only to a local replica. Nothing is published, pushed or sent to external services, apart from fetching dependencies with cargo, Gradle and npm.
7. **No secrets in code.** Firebase config files (`google-services.json`) are git-ignored and optional.
8. **Stay in your lane.** Edit only the files and modules you were assigned. If you need a change in a shared contract, write it into your report instead of editing someone else's files.

## Build decisions (override the docs where they differ)
- **D15′ — the Content Credentials writer lives in `witness-core::c2pa_lite`** (minimal C2PA 2.x profile: claim v2, assertions `c2pa.actions.v2`, `c2pa.hash.data`, `app.witness.claim`, COSE_Sign1 ES256 with x5chain, JUMBF in JPEG APP11). The `c2pa` crate is a **host-only dev-dependency**: integration tests validate our manifests with c2pa-rs `Reader`. The c2pa crate is never linked into Android or canisters.
- **TrustMark:**
  - In `witness-core::watermark`: the BCH_5 codec, 256×256 preprocessing, and residual upsample-and-add (pure and tested).
  - In `witness-android` behind cargo feature `trustmark` (off by default): the ONNX session (ort).
  - Models are fetched only by `scripts/fetch-models.sh` with pinned SHA-256 values. Without models, the app and site report "watermark unavailable" and use exact-hash and PDQ lookup.
- **Dev attestation:** the `registry` cargo feature `dev-attestation` (**local builds only**) additionally accepts the Android emulator's software attestation root and unverified boot. Keys enrolled that way get `provisioning = Dev`, and every proof from them shows a **DEV** badge. Production builds never enable it.
- **Telemetry:**
  - Port `Telemetry` (analytics events, crash breadcrumbs, non-fatal errors) in `client/core/telemetry`, with a no-op default.
  - The Android actual uses **Firebase Analytics and Crashlytics**. The Gradle plugins apply only when `client/androidApp/google-services.json` exists.
  - Collection is **off until the user opts in** (Settings), and analytics events never contain record IDs, locations, hashes or images.
  - The web has no telemetry.

## Toolchain (pinned)
- **Rust:** 1.95 (stable); targets `aarch64-apple-darwin`, `wasm32-unknown-unknown`, `aarch64-linux-android`, `x86_64-linux-android`; `cargo-ndk`, `wasm-bindgen-cli`.
- **ICP:** `icp` CLI 1.7 / `dfx` 0.29; `ic-cdk` 0.20 family; PocketIC for canister tests.
- **Kotlin and Android:** Kotlin 2.4.21, Compose Multiplatform 1.12.2, AGP 9.x, Gradle 9.x, Android SDK 37, NDK 28.2, JDK 17.
- **Local emulator:** `emulator-5554` (API 36.1).

## Commands
Full list, prerequisites and the Android/web local runs: `README.md`.
- Rust: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` (from repo root); feature sets: `--features witness-registry/dev-attestation,witness-attest/dev-attestation` and `--features witness-pages/local-dev,witness-media/local-dev`
- wasm32: `cargo test --target wasm32-unknown-unknown -p witness-core -p witness-web`; `cargo build -p witness-web --target wasm32-unknown-unknown --release`; web package: `crates/witness-web/build.sh && client/webApp/bridges/build-wasm.sh`
- Dependency bans: `scripts/check-core-deps.sh`; `scripts/check-canister-deps.sh`
- Android native library (before the first Gradle build): `scripts/build-android-native.sh`
- Canisters: `scripts/deploy-local.sh`; end to end: `scripts/smoke-local.sh` (`WITNESS_SMOKE_SKIP_DEPLOY=1` to skip the deploy)
- Client: `cd client && ./gradlew test allTests assembleDebug wasmJsBrowserDistribution`; bridges: `cd client/webApp/bridges && node --test test/`
- Web canister: `scripts/stage-web.sh && icp deploy web --yes`
- Emulator: `./gradlew :androidApp:assembleBenchmarkRelease`, then `adb -s emulator-5554 reverse tcp:8090 tcp:8090`
