

# ---- report ----
I've finished all five items. The local replica is redeployed in place and `scripts/smoke-local.sh` passes all 27 steps, run twice including the deploy. Nothing was committed, and nothing ran anywhere but the local replica.

**1. Shared contract types**
- **Moved into `canisters/common/common.did`:** `Shell` (all 7 fields: `record_id`, `record_ix`, `anchored_ns`, `claim`, `media_shard`, `media`, `dev`), `Removal`, `PageErr`, `PageResult` and `Allow` (key, record, revoke). The Rust twins are in the new `canisters/common/src/peer.rs`.
- **Peer method types:** three service types, `IndexPeer`, `PagesPeer` and `MediaPeer`, list the methods the registry calls. Their Rust twins come from `define_service!`. The method-name constants (`PUT`, `ERASE`, `PUT_SHELL`, `TOMBSTONE`, `SYNC`) are used by the registry outbox.
- **`Fingerprints`** now has `quality : nat8; portrait : bool`, and index fills them from the stored row.
- **No duplicates left:** registry and pages now re-export the common types, and media uses `Allow` from common.
- **Guards:**
  - The did-drift test covers every new type and now also checks services, with a negative test.
  - Index, pages and media each test that the interface they export can serve their peer service type.
  - Registry tests that every outbox call names a method of the right peer service.
- **Upgrade safety:** values queued or stored before the change still decode (tests cover `Allow` without revoke, the narrower `Removal`, and a `PeerInit` stored before `origins` existed).

**2. Pages origins**
- **Init args:** I extended `PeerInit` rather than adding a `PagesInit`, so media can use it too. It gained `origins : opt Origins`, with fields `site`, `case`, `app`, `api` and `canister`.
  - Leaving it out gives the production defaults: witness.app, case.witness.app, app.witness.app, icp-api.io and `https://*.icp0.io`.
  - Every value must be a bare origin, checked at install and upgrade; a bad value makes the install fail.
- **Shell HTML:** the canonical URL, `og:image`, the report and contact links and `boot.mjs` all come from the configured origins. The `/v/*` case shell, the 404 and the tombstone pages do too.
- **CSP** lists exactly the configured origins.
- **Upgrades that change the origins** re-certify every stored page. A shell rendered before the change keeps its old body and stays certified, but still points at the old origin.
- **Bug fixed:** the `og:image` link now uses `/o/{id}`. Pages was linking to `/og/{id}`, which media doesn't serve.
- **New check:** `put_shell` now rejects a `record_id` that doesn't match the claim.

**3. Media**
- **CORS** allows the configured site, case and app origins. Plain-http origins are allowed only in `local-dev` builds; a non-local-dev build refuses them at install.
- **Certified 404:** erased or never-existing paths now return a certified 404 instead of 503. The cert tree has a 404 fallback entry and proves that each requested path is absent.
- **Proof that it works:**
  - The tree's root hash matches `ic-http-certification`'s own tree.
  - Absence proofs are checked for every path the verifier looks up.
  - `ic-response-verification` accepts the 404s in PocketIC.
  - The live gateway returns 404 for missing paths.

**4. Registry**
- **Stuck enrollments:** a pending enrollment older than 10 minutes (`PENDING_TTL_NS`) can be retried, with tests.
- **Outbox scan:** each tick now scans at most 1,024 entries in total (`OUTBOX_TOTAL`), plus the existing 256 per destination. A cursor rotates the starting destination between ticks.
- **Fairness tests:**
  - the total cap holds every tick;
  - stuck destinations are served round-robin, never more than one turn apart;
  - a small budget still drains every queue;
  - the cursor wraps around correctly.

**5. Deploy and smoke**
- `deploy-local.sh` now passes local origins to pages and media: pages' own `localhost:8090` URL for site and case, the web canister's URL for app, the gateway for api, and `http://*.localhost:8090` for canisters.
- **New smoke steps:**
  - The `/p/{id}` page, fetched with certification checked, loads its bundle from `http://46el7-ql777-77775-aaada-cai.localhost:8090/boot.mjs`, and that file returns 200.
  - The erased preview is a certified 404, both through the agent and with curl through the gateway.

**Tests (all pass)**

| Crate | Unit | PocketIC / other |
|---|---|---|
| common | 100 | 4 did |
| index | 75 (+2) | 4 |
| registry | 117 (+8), also with `dev-attestation` | 7 |
| pages | 87 (+10) | 6 (+1) |
| media | 111 (+10), also with `local-dev` | 11 (+1) |
| smoke | 26 (+1) | — |

Clippy with `-D warnings` is clean for every feature combination, native and wasm32. All wasm32 release builds pass, `check-canister-deps.sh` is clean, and fmt is clean on these crates. `cargo fmt --all --check` still flags `crates/witness-verdict/src/confirm.rs`, which isn't my lane and I didn't touch.

**Outside my lane**
- To keep the workspace compiling, I added `quality` and `portrait` to one test struct in `crates/witness-android/src/client/tests.rs`; that test passes.
- `oracle::to_bundle` in witness-android still takes `portrait` as a separate argument and uses quality 0. It could read the new fields instead; that's the android owner's call.

**Things to know**
- **One-pass re-certification:** the pass after an origin change goes through every page within a single upgrade. That's fine locally; production would need it spread over timer ticks.
- **Stuck enrollments:** a retried enrollment uses up its quota again. A late-finishing first attempt and its retry could both commit; both write the same key, so no harm.
- **Outbox cursor:** it lives in memory, so an upgrade resets it to the first destination.
- **Contract change:** `pages.put_shell` now takes `vec Shell`, which the registry was already sending.

**Files**, under `/Users/andrejones/Desktop/workspace/projects/witness`:
- **common:** `canisters/common/{common.did,src/lib.rs,src/peer.rs,src/init.rs,src/index.rs,src/index/tests.rs,tests/did.rs}`
- **index:** `canisters/index/{Cargo.toml,index.did,src/lib.rs,src/store.rs,src/guard.rs,tests/pocket.rs}`
- **registry:** `canisters/registry/{Cargo.toml,src/types.rs,src/outbox.rs,src/ic.rs,src/config.rs,src/enroll.rs,src/enroll/tests.rs,src/erase.rs}`
- **pages:** `canisters/pages/{Cargo.toml,pages.did,src/lib.rs,src/api.rs,src/html.rs,src/http.rs,src/site.rs,src/store.rs,tests/pocket.rs}`
- **media:** `canisters/media/{media.did,src/lib.rs,src/http.rs,src/http/tests.rs,src/certtree.rs,src/certtree/tests.rs,src/store.rs,src/store/tests.rs,tests/pocketic.rs}`
- **scripts and smoke:** `scripts/deploy-local.sh` and `tools/witness-smoke/src/{main.rs,net.rs,steps.rs}`
- **other lanes:** `crates/witness-android/src/client/tests.rs`

# ---- report ----
# Report: StandInNative replaced by real UniFFI adapters

Android now runs on the real `witness_android` library against the local canisters. All tests pass (3,323 Kotlin tests, 0 failures) and `./gradlew test assembleDebug assembleBenchmarkRelease` builds. On emulator-5554, live Check and live registry reads work. **Enrollment on the emulator is rejected by the registry**, because the emulator's attestation root isn't on the dev-attestation allow list. So I could not show the DEV badge end to end, and no on-device seal or anchor ran. That fix belongs in the attest/registry lane (details under Open issues).

## What changed

**Contracts (minimal)**
- `core/model/.../Capture.kt`: `Submission.fingerprints: ByteString? = null`. This file is **outside my lane**, but without it the fingerprint bundle is lost and the index gets no PDQ entries.
- `core/domain/port`:
  - `SealedCapture.jpeg: ByteString? = null`
  - `CaptureStore.insert(..., image)` and `replace(..., image)`, both defaulting to null.
- `SealCapture`, `BuildPost` and `AnchorOutbox` now persist `sealed.jpeg`.
- `FakeSealEngine` returns the frame, and `InMemoryCaptureStore` stores images.
- New `core/domain/usecase/CaseBundleCbor.kt`: canonical CBOR in exactly the `witness-web` `bundle.rs` format, with the Rust decoder's limits enforced.

**core/data**
- `outbox.fingerprints` column added; DB version 2 with migration `V1_TO_V2`; `schemas/.../2.json` generated.
- `RoomCaptureStore` writes the sealed JPEG to app-private `captures/<recordId>.jpg`. The record id is the claim digest, so storage is content-addressed.
- On reseal, the new image is written and the old one deleted.

**native/witness**
- `build.gradle.kts`: `api(core.domain)`, coroutines, and test dependencies. Removed the stale `org.rustls.platformverifier` keep rule.
- New Kotlin wrappers in `app/witness/nativelib/`:
  - `NativeMapping` (+ `Hex`): converts every UniFFI type to and from the model.
  - `NativeErrors`: `Network` and media `RetryAfterNs` become `OfflineException`; every other variant becomes `NativeException(code)`.
  - `NativeKeys`:
    - `VaultSigner` implements UniFFI's `KeystoreSigner` over `KeyVault`, with cert chain `[c2paCert, caRoot]`.
    - `NativeTrust` caches the CA root on disk, so sealing works offline after the first fetch.
    - The cert serial is the X.509 serial in hex.
  - `NativeRegistry`:
    - Enrollment sends the attestation chain and takes the key info from `getKey`, so provisioning is whatever the registry says (Dev on an emulator).
    - Enroll errors starting `quota`, `not ready` or `enrollment pending` are treated as offline (the key is kept); other rejections are final.
    - Erase signs through the vault key.
  - `NativeIndex`: `probe`.
  - `NativeMedia`: each case chunk is a signed `beginUpload(CaseChunk)`, `putChunk`, `commit`. Reference images are fetched through the gateway with `?canisterId=`.
  - `NativeSealEngine`:
    - Native prepare/finish happens inside `seal()`, where the key and counter are known.
    - Camera rotation is passed as an EXIF orientation.
    - Claim CBOR is persisted content-addressed (`ClaimFiles`).
    - `derive` renders the declared edits onto the parent image; `reseal` uses the stored image.
  - `NativeVerifyEngine`: calls native `check`; the stage list is derived from the verdict.

**androidApp**
- `build.gradle.kts`:
  - BuildConfig is turned on, with `ICP_REPLICA_URL`, `ICP_REGISTRY_ID` and `ICP_FETCH_ROOT_KEY`.
  - Debug and benchmark builds point at the local replica; the registry id is read lazily from `.icp-local/ids.json` (configuration-cache safe).
  - If the file is missing, the id is `""` and the app falls back to stand-ins.
  - Override the URL with `-Pwitness.replicaUrl=...`.
  - Depends on `:native:witness`.
- `NativeAvailability.kt` is still the single switch: real adapters when an endpoint is configured and the `.so` loads, otherwise `StandInNative`. On a host JVM the library never loads, so tests and previews use stand-ins.
- New `RealNative.kt`: wiring. Reference images come from the phone first. Enrollment rejections are logged with tag `Witness`.
- New `BitmapEditRenderer.kt`: applies Crop, Resize and Blur (pixelate) to derivatives.
- `AppGraph.kt`: `KeystoreKeyVault` is bound concretely so the C2PA certificate is reachable.
- `StandInNative.kt` bug fix: in-process enrollment now records Dev provisioning and the vault key's security (previously StrongBox/RKP).
- `res/xml/network_security_config.xml` and the manifest: cleartext allowed only to loopback and 10.0.2.2.

**feature/check** (`CheckOutcome.kt`, outside my lane but asked for in the task): the "Captured …" line now says "development device" whenever the result is DEV, instead of "genuine Android phone".

## Tests added (all passing)

| Module | Added |
|---|---|
| native/witness | 60: every `WitnessException` and `MediaErr` variant, every `SubmitErr` variant, all 8 edit kinds, enum parity, the registry/media/seal/verify/trust adapters against a fake `WitnessInterface` |
| core/domain | 14: 11 `CaseBundleCborTest`, image persistence (store and seal) |
| core/data | 9: fingerprints round-trip, image write/replace/rename, v1→v2 migration on a real v1 table, 130-capture Room backlog submitted as batches of 64/64/2 in counter order, counters surviving a reopened store |
| androidApp | 13: `NativeAvailabilityTest`, `RealNativeWiringTest` (wiring, renderer under native Robolectric graphics, JCA case-chunk vector), in-process Dev enrollment |
| feature/check | 1: Dev copy |

**Cross-check vector shared with Rust:** I generated it with a scratch program that links `witness-web` and decrypts with its own `CaseOpener`. Kotlin `CaseBundleCbor` output matches byte for byte, and a `ShareCase` run with `JcaCrypto` produces exactly the Rust single-item chunk. The values are in `core/domain/src/commonTest/.../usecase/CaseVector.kt`; the generator is at `/private/tmp/claude-501/-Users-andrejones-Desktop-workspace-projects-wingman-c2/0d16e041-94d5-4236-8582-2263280ea996/scratchpad/casevec/src/main.rs`. **The Rust side does not have the vector yet:** `crates/witness-web/src/case/tests.rs` should add the same constants.

## Commands verified
- `cd client && ./gradlew test assembleDebug assembleBenchmarkRelease`: build successful.
- Minified `benchmarkRelease` APK (71 MB, both ABIs) installed on emulator-5554 after uninstalling the old `app.witness`. Storage is unchanged at 487 MB free; my temp files in `/data/local/tmp` are removed.
- On device:
  - `libwitness_android.so` and JNA load.
  - A live registry `get` decodes the claim (proof link for the smoke run's original shows "removed", which is correct because the smoke run erased it).
  - A live native Check of a shared JPEG returns No record with no crash.
  - Enrollment reaches the registry and is rejected with `attestation: UntrustedRoot`.

## Deviations
- **Replica URL is `http://127.0.0.1:8090`, not `http://10.0.2.2:8090`.** The local gateway rejects the Host `10.0.2.2` with 400 `client_unknown_domain`. The emulator therefore needs `adb -s emulator-5554 reverse tcp:8090 tcp:8090`; I set this on the emulator but it is not scripted anywhere. To use 10.0.2.2, the gateway in `icp.yaml` would have to accept that domain.
- **Native `prepare` runs inside `seal()`** because `CaptureMeta` needs the key id and cert serial. As a result, `PreparedCapture.draft` carries placeholder hashes and dimensions; the real claim comes back in `SealedCapture.claim`.
- **Stand-ins remain** for Internet Identity, the case-link opener and the report sender, because there are no native APIs for them.

## Open issues and feature gaps for the next agent
1. **Blocker (attest/registry lane):** the API 36.1 emulator attests through RKP test certificates. Its chain root is "CN=Droid Unregistered Device CA, O=Google Test LLC", SPKI SHA-256 `ff7a4c045501b8b8db811ee7d49867e8751356b644534e17658740d3a1a14e20`. `witness-attest` has to add it as a Dev root under `dev-attestation`, then the registry must be redeployed locally. The DER files are in the scratchpad: `/private/tmp/claude-501/-Users-andrejones-Desktop-workspace-projects-wingman-c2/0d16e041-94d5-4236-8582-2263280ea996/scratchpad/droid_unregistered_device_ca.der` and `/private/tmp/claude-501/-Users-andrejones-Desktop-workspace-projects-wingman-c2/0d16e041-94d5-4236-8582-2263280ea996/scratchpad/droid_unregistered_device_ca.spki.der`. After that, re-run on device: enroll (DEV badge), seal, anchor, Check = Original with the DEV copy.
2. **Late (offline) enrollment isn't possible.** The domain `EnrollmentRequest` doesn't carry the install nonce and wall time, so the adapter treats a late request as offline. A key made offline stays pending until the 7-day window passes.
3. **Captures can't be sealed before enrollment** with native, because the claim needs the certificate serial; the seal fails with `not_enrolled`. The "awaiting enrollment" outbox path needs a reseal-after-enrollment flow.
4. Not exposed by native: `getByIndex`, index `derivatives`, media downloads, case metadata and upload expiry. The in-app case-link opener has no way to learn the chunk count or bundle hash (ARCHITECTURE §9).
5. **Derivative rendering is partial:** Rotate, Tone, Filter and Orient are declared in the claim but not applied to the pixels.
6. No TrustMark models, so watermark checks are skipped.
7. The emulator's native library loads lazily on the first port use. It currently happens on the app's background startup coroutine, but could land on the main thread.
8. **Lane note:** `core/model` and `feature/check` were edited outside my lane, both minimally.

# ---- report ----
# Report: post blurs, item move/delete, live blur detection and the sealed-toast overlap

All four feature gaps are fixed, and `cd client && ./gradlew test assembleDebug --continue` builds successfully with 3,411 tests and 0 failures. Nothing was run on emulator-5554; the checks are host tests plus Robolectric renders.

To finish the work I had to edit modules outside my lane: one new port method in `core/domain`, and small changes in `core/data`, `core/platform` and `androidApp`. They are listed under "Outside my lane" below.

## 1. Posts show and seal real blurs
There is no native blur API, so I wrote a three-pass separable box blur in Kotlin. It approximates a Gaussian at the declared `sigmaE2`, uses integer arithmetic, takes linear time per pass and reuses one scratch buffer.
- **`core/ui/src/commonMain/.../RegionBlur.kt`** (new): `boxRadii(sigmaE2)`, `blur(pixels, w, h, sigmaE2, scratch?)`, `scaledSigma`, `Rect.scaled`.
- **`core/ui/src/androidMain/.../BitmapBlur.kt`** (new): `Bitmap.blurRegion(rect, sigmaE2)` and `Bitmap.applyBlurs(blurs, scale)`.
- **Sealed derivative** (`androidApp/.../BitmapEditRenderer.kt`): `Edit.Blur` now applies this blur instead of pixelating, so the pixels match the declared sigma.
  - Pixelation leaves hard block edges, which would probably fail the verifier's high-frequency "blur holds" check.
  - The stand-in seal engine (`StandInNative.kt`) now renders a derivative's edits too, through a new `render` parameter.
- **Composer (B3) and editor (B4)**: the grey mosaic overlay is gone. `PhotoImage` takes `blurs` and `sourceWidth` and blurs a copy of the decoded preview off the main thread, scaling the rect and sigma to the preview size.
- **Same edits on screen and in the seal**: `blurEdits(blurs, dims)` in `Redaction.kt` builds the preview's blurs, and a test checks it gives exactly what `BuildPost.derivativeEdits` seals.

## 2. Delete and move-to-case from item actions
- **Port:** `CaptureStore.delete(recordId)`.
  - The in-memory store and `RoomCaptureStore` implement it. Room deletes the capture row, and the existing cascades remove the outbox entry, location opening and case membership; the JPEG file is deleted too.
  - Derivatives of a deleted capture are kept.
- **`ItemViewModel`** has two new state machines:
  - Move: `MoveState` goes Idle → Picking (every case plus "Not in a case", the current one marked) → Confirm → Working → Idle or Failed.
  - Delete: `DeleteState` goes Idle → Confirm(anchored) → Working → Deleted or Failed.
- **Dialogs:** the delete warning is stronger for a photo that isn't anchored yet ("it will never get a public proof").
- **UI and navigation:** `ItemScreens.kt` has the picker sheet and the confirm/failure dialogs. `ProofNavigation.kt` wires them to the view model and goes back once the item is deleted. I removed the unused `moveToCase` and `delete` callbacks from `ItemNavigation`.

## 3. Live blur detection on the camera
- **`LiveFrames.kt`** (commonMain): `FrameThrottle` (at most 5 frames per second), upright frame size from the rotation, mirroring for the front lens, clipping and a cap on the number of boxes.
- **`LiveBlurAnalysis.kt`** (androidMain):
  - CameraX `ImageAnalysis` keeping only the latest frame, at about 640×480, on its own background thread.
  - It always closes the frame and publishes a `StateFlow<FrameDetections>`.
  - It runs only in Post mode and clears when inactive.
- **Detector:** the ML Kit face and text code stays in `core/platform`. `MlKitBlurDetector` gained `detectFrame(ImageProxy)`, which reuses the existing text classifier and box geometry.
- **Wiring:** `CameraRoute(analysis = …)` binds the analysis alongside the preview and still-capture use cases. `Features.camera()` now feeds `liveBlur.detections` into the existing camera chips and hint, instead of an empty stream.

## 4. Sealed toast no longer shows through the coach card
- `CameraScreen` and `CameraRoute` have two new slots, `topAccessory` and `bottomAccessory`. The toast, error or hint card now sits in a column above the bottom slot, so nothing overlaps.
- `feature/case` replaces `CaseCameraLayer` with `CaseCameraPillsLayer` (top slot) and `CaseCoachLayer` (bottom slot). Both share the same view model.
- The app shell passes these into the slots; the old full-screen overlay and the public `CameraPanelHeight` are gone.
- I checked the stacking on a Robolectric render (`feature/capture/build/snapshots/c1-toast.png`): the toast sits above a stand-in coach card. The faint frame-guide corner still shows through the toast, which is 90% opaque.

## Tests
| Module | Added |
|---|---|
| core/ui | 19 `RegionBlurTest` |
| feature/post | 4 blur-edit and preview-scale tests (replacing the 2 mosaic tests); 3 Robolectric `BlurPhotoTest` |
| feature/proof | 17 `ItemMoveDeleteTest` (every move and delete path, including failures and dismissals) |
| feature/capture | 11 `LiveFramesTest`, 7 Robolectric `LiveBlurAnalysisTest` (fake `ImageProxy`), 1 `c1-toast` snapshot |
| core/domain | 1 in-memory delete test |
| core/data | 3 Room delete tests (cascade, file removal, derivatives kept, unknown id) |
| androidApp | Stand-in renders derivative edits; "stronger sigma blurs more" replaces the old `blockSize` test |

Two existing tests were already failing because sealing now stores the photo:
- `ComposerViewModelTest.missingPhotoIsNull` now uses a store that returns no image.
- `CameraViewModelTest.photoIsLoadedOnceAndCached` now expects the photo right after sealing; it still checks that the photo is loaded once and cached.

I also re-pointed one existing stand-in test's trailing lambda to the named `upright` parameter after the new constructor parameter.

## Commands run
- `./gradlew test assembleDebug --continue`: build successful.
- `./gradlew :core:ui:allTests :feature:proof:allTests :core:domain:allTests -x kotlinStoreYarnLock -x kotlinWasmStoreYarnLock`: exit 0.

## Outside my lane (all small)
- `core/domain`: `CaptureStore.delete` plus the in-memory implementation.
- `core/data`: `CaptureDao.delete` and `RoomCaptureStore.delete`.
- `core/platform`: `MlKitBlurDetector.detectFrame`; the photo detection path was refactored onto a shared private method.
- `androidApp`:
  - `BitmapEditRenderer` (blur), `StandInNative` (render), `AppGraph` (a singleton `MlKitBlurDetector`), `Features` (`liveBlur`), `WitnessApp` (camera slots).
  - `build.gradle.kts`: added a dependency on `:core:ui`.

## Open issues
1. **Blur strength:** the sigma comes from `BuildPost.sigmaFor` (a domain rule: the smaller side ÷ 8, clamped to 4–64 px). For a plate about 50 px tall that is about 6 px, which may still leave text partly readable. That rule belongs to the domain owner, and Gaussian blur alone can sometimes be partly reversed.
2. **Partial derivative rendering:** rotate, tone, filter and orient are still declared but not applied to the pixels (adapter gap #5, in `androidApp`'s renderer). Straighten and tone edits made in B4 are therefore not in the shared image.
3. **Not run on a device:** live detection hasn't been tried on the emulator. Detection boxes assume the preview and analysis frames have the same 4:3 aspect ratio.
4. **B4 back press** is still not handled. The repo has no back-handler pattern yet.
5. **Not feature-level, so left alone:** the remaining adapter gaps (enrollment root, late enrollment, sealing before enrollment, missing native media/index APIs, TrustMark models, lazy native-library load).

# ---- report ----
The root cause is how coverage was measured on the grid path, not codec noise or alignment. A q80 copy matched its sealed grid perfectly (every cell distance 0, agreement 1.0), but the copy was flagged because of how `compared_q16` was computed.

**Root cause.** The grid comparison skips any cell where neither grid has at least 4 stable bits (flat sky, walls). In these photos 137 to 157 of the 192 cells have that much texture, so the "compared" share was 0.71 to 0.82. The full-compare threshold is 0.75, so about half the seeds landed below it and got CouldntCompare. Instrumented over 640×480 q80 seeds without a reference: 6 of 12 flagged before the fix, 0 after. Over all 288 sizes × qualities × seeds probed, CouldntCompare tracked low coverage exactly at 128 px and up. The measure was counting texture, not how much of the frame the copy covers.

**What I changed, all in `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-verdict/src/confirm.rs`:**
1. **Coverage fix (`grid_coverage_q16`).** Coverage on the grid path is now compared cells ÷ cells that could be compared (textured in the reference, or compared anyway), capped at 1.0. A full-frame copy now scores exactly 1.0. Crops still drop below 0.75 because textured cells outside the copy count against them.
2. **Change-size floor (`grid_min_change_cells`, `CODEC_MCU_PX = 16`).** The measurements found a worse bug: grid-only copies were being called **Altered** by mistake. That was 10 of 288 copies, all at 96×72 and 72×96 (16.7% of those two sizes), plus isolated single noisy cells up to 176 px wide. I measured noise patch sizes over 3,000 copies across 15 sizes. On the grid map, a changed region now has to cover more cells than one 16 px JPEG block (4:2:0) spans: (ceil(16 / cell side))² cells, using the smaller cell of the original and the copy. All measured noise falls below that. Cells of 16 px and larger keep the old one-cell rule, so detection on normal-size photos is unchanged.
3. **Alignment fallback (`hypotheses`).** The new tests also caught a with-reference failure at 96×72 q70. The feature-based alignment came out about 3° and 3 px off, so the plain-resize placement was never tried. The plain resize is now tried last whenever the aligned placement doesn't settle. Of all tried placements, the one showing the fewest changes still wins.

**Tests (no tests deleted):**
- **Unit tests in `confirm.rs`:** 7 new tests for coverage and the change floor. I renamed and changed one existing test, `hypotheses_skip_resize_for_far_or_reshaped_alignments` → `hypotheses_try_resize_last_for_far_alignments_and_never_for_reshaped_views`, because its expected behaviour deliberately changed (item 3).
- **Integration tests in `src/tests.rs`:**
  - `grid_only_640x480_q80_recompresses_are_full_compares`: the reported case, 12 seeds.
  - `recompress_is_a_full_compare_with_and_without_reference`: runs on native and wasm.
  - `seeded_recompress_matrix_{small_frames,medium_frames,640x480,1024x768}`: native only. 14 sizes, q50 to q95 in steps of 5, a fresh seed for each quality; every quality runs without a reference, and q50/60/70/80/90 also run with one.
  - `seeded_grid_only_downscales_are_never_accused`: native only.
  - `grid_only_local_edits_on_full_frames_are_still_caught` and `grid_only_crop_is_not_a_full_compare`.
- **witness-core:** `tests/e2e.rs` has a new `recompressed_full_frames_are_full_compares_with_or_without_reference` test, run through the real `seal()` path.
- **Fix checks:** with the coverage change undone, the 640×480 and e2e tests fail. With the change floor undone, the small-frame matrix fails.

**Golden digests:**
- The witness-core golden is unchanged on native and wasm32.
- I updated the witness-web golden in `crates/witness-web/tests/golden.rs` from `9a2aad9f…` to `32f0cc5e18c4c62a71d609fa2479c983999c0a01236fd5e5bf7fff5266dce8a7`. That file is outside my lane, changed only because the task allowed golden updates with a reason. I dumped every rendered verdict before and after. The only difference is the `no_reference` case's `comparedQ16`, which goes from 51200 to 65536 — the intended coverage fix.

**Commands run, all passing:**
- `cargo test -p witness-verdict -p witness-core -p witness-match -p witness-web -p witness-android`: verdict 99, core 10 + 14 e2e + 1 golden, web 102 + golden.
- `cargo test --target wasm32-unknown-unknown` for `-p witness-verdict` (93), `-p witness-core` (10 + 14 + 1 golden) and `-p witness-web` (6 + golden).
- `cargo test -p witness-fingerprint`.
- `cargo clippy -p witness-verdict -p witness-core -p witness-match --all-targets -D warnings` is clean, and rustfmt is clean.

**Builds, done last:** I rebuilt the web package with `crates/witness-web/build.sh` (1,320,145 bytes after wasm-opt), and the node smoke test passes (sealed → ORIGINAL, recompressed → MATCHES_WITH_CHANGES). Then, at the very end, I ran `scripts/build-android-native.sh`, which rebuilt both Android `.so` files and the UniFFI Kotlin bindings. Nothing was deployed or installed on the emulator.

**Open issues:**
- **Small frames:** below 256 px wide, a grid-only check only reports changes covering more cells than one JPEG block spans (4 cells at 160×120, 9 at 96×72). Small grid-only edits there come back as MatchesWithChanges, which is never Original.
- **Grid-only edits on large frames:** about half of the 3×3-cell pastes at 640×480 come back CouldntCompare (border-band or poorly-surrounded patches) instead of Altered. That behaviour was the same before this fix.
- **Thumbnails with a reference:** at 48×36 the pixel alignment can't run, so the grid is used and the verdict says NO_PUBLIC_REFERENCE even though a reference image exists. That logic in `decide` is unchanged.
- **Test speed:** the debug test suite went from about 25 s to about 45 s. Setting a dev opt-level for the workspace crates would help, but that is in the root `Cargo.toml`, outside my lane.
- **Doc to update:** the note in `docs/dev/canisters-bindings.md:770` about recompressed copies getting CouldntCompare is now fixed.

# ---- report ----
I've made the web app work against the local canisters. `/check`, `/p/{id}` and `/v/{id}#k=` all ran in a real browser through the local gateway with real certificate and proof checks, and every test passes. One contract gap remains: a case link only opens on the web if its fragment also carries the bundle hash (`&h=`). Nothing was committed, and only the `web` canister was redeployed (`icp deploy web --yes`, local network).

## What I verified in the browser
I checked in the built-in browser pane, then took the saved screenshots with headless Chrome over DevTools at 1280 and 412 px wide. The test records came from a small seed tool; it uses the dev device the smoke test enrolls and does not erase anything.

- **/check:**
  - A sealed copy showed "Matches an original, with changes · 1 blur declared · Face".
  - A JPEG re-encoded in the page showed "Matches a sealed photo, with no public reference". It matched the original record, which has no preview.
  - An unrelated image showed "No Witness record".
  - The file is never uploaded: it goes from the page into witness-web.
- **/p/{id}** on the web canister and on the pages shell: the X1 Original page, the W4 blurred page with the DEV warning, W9 Removed (an erased smoke record) and not-found.
- **/v/{id}#k=…&h=…** on the web canister and on the pages shell (its CSP nonce works): "3 of 3 verified · 2 original · 1 with a declared edit (face blurred)", plus the tampered and missing-key states.
- **Console:** only a Kotlin/Wasm deprecation warning; no errors.

## Changes (under `/Users/andrejones/Desktop/workspace/projects/witness`)
- **witness-web** (one missing binding): `decodeClaim(bytes)` returns the full claim; it's in `src/render.rs` and `src/api.rs`. The existing `render::claim` is unchanged, so the golden digest still matches.
- **Bridges package** (`client/webApp/bridges`):
  - `candid.mjs` is new Candid matching `registry.did` and `index.did`.
  - `config.mjs` reads the `witness-config` meta tag and falls back to production defaults. It only fetches the root key for a localhost host.
  - `engine.mjs` runs the plan/judge loop, querying all index shards in parallel, and checks the certificate and proof on every record, key and shard map. It also fetches media and opens cases.
  - `witness.mjs` connects through `@icp-sdk/core` and loads the wasm.
  - `build-wasm.sh` copies the witness-web package into `wasm/`, which is git-ignored.
- **core/icp:** a bridge port, JSON-to-domain mapping, and read-only registry, media and index gateways plus a case opener, all in `src/commonMain/.../witness/`. The JS-backed version is in `src/webMain/.../witness/`.
- **core/verify:** a verdict parser and a web verify engine that implements the domain `VerifyEngine`, in `src/commonMain/.../witness/`.
- **webApp:**
  - `WebGraph.kt` now uses the real bridge and adds a case-link adapter; `Main.kt` passes it in.
  - `boot.mjs` adds the config meta tag from `witness-config.mjs` and hides the pages shell's static summary once Compose mounts. Without that, the summary showed above the app.
  - `build.gradle.kts` gained the new module dependencies.
- **`scripts/stage-web.sh`:** stages the compatibility bundle and writes `witness-config.mjs` from `.icp-local/ids.json`.
- **Seed tool:** `client/webApp/seed/` (its own cargo workspace, reusing the smoke modules). It writes `.icp-local/web-seed.json` and creates:
  - an original;
  - a resized post and a face-blurred copy, both with previews;
  - an encrypted case.

## Tests (all passing)
| Suite | Result |
|---|---|
| Bridges (`npm test`) | 58 pass (26 new). The live test is skipped unless `WITNESS_LIVE=1`; run that way against the local replica it passes all proof, check and case outcomes. |
| core/icp | 138 js and wasm, 136 android (+14) |
| core/verify | 56 js and wasm, 52 android (+8) |
| webApp | 30 per target (+4) |
| witness-web | 96 unit including golden; wasm32 api 6 (+1). Clippy is clean native and wasm32, fmt is clean. |

The seed tool also passes clippy with `-D warnings` and fmt.

**Commands I ran:**
```
crates/witness-web/build.sh && client/webApp/bridges/build-wasm.sh
./gradlew :core:icp:allTests :core:verify:allTests :webApp:allTests
./gradlew :webApp:wasmJsBrowserDistribution :webApp:composeCompatibilityBrowserDistribution
scripts/stage-web.sh && icp deploy web --yes
CARGO_TARGET_DIR=target cargo run --manifest-path client/webApp/seed/Cargo.toml
```

**Gotcha:** yarn copies `witness-bridges`, so the build doesn't pick up changes to it on its own. After editing the bridges, delete `client/build/{js,wasm}/node_modules/.yarn-integrity`, then run:
```
./gradlew :kotlinNpmInstall --rerun :kotlinWasmNpmInstall --rerun :webApp:jsBrowserProductionWebpack --rerun :webApp:wasmJsBrowserProductionWebpack --rerun
```

## Deviations
- `@dfinity/agent` is deprecated in favour of `@icp-sdk/core`, so I kept `@icp-sdk/core` 6.1.1, which is the current release.
- The web engine ignores the domain oracle, because witness-web fetches its own data in the plan/judge loop.
- The web gateways are read-only; write methods throw.
- The trust anchor comes from `config.anchor` if set, otherwise from the uncertified `ca_root` query. That's fine locally, but production must set the anchor.
- A reference claim that only came from the photo's own credentials (an unanchored match) uses neutral defaults for fields the lean rendering leaves out.

## Open issues for other owners
1. **Case-link contract:** media exposes neither the bundle hash nor the chunk count. The web needs `#k=…&h=<b64url sha256(bundle)>` and finds the count by fetching chunks until one is short. The Android CaseLink/ShareCase code and ARCHITECTURE §9 need to adopt `h=`. Expiry isn't exposed either, so the case page shows "No expiry set".
2. **Media shard:** a certified record doesn't say which media shard holds it, so the web tries each media shard in turn. That's fine with one shard.
3. **Production config:** the production registry id isn't known yet, so a production build shows the offline states until it is configured.
4. **Old bridge code:** the old `icp.mjs`, `idl.mjs`, `verify.mjs` and `wire.mjs`, plus core/icp `BridgeIcp` and core/verify `BridgeVerifyEngine`, target a Candid the canisters don't implement. I left them untouched because they have their own tests; deleting them is the owner's call.
5. **UI notes for the feature lanes:**
   - The check preview squares non-square images.
   - "Reference" wraps at mid widths.
   - The Removed page says "taken down after a report" even when the owner erased it.
6. **Staging on a fresh network:** `stage-web.sh` reads `ids.json`, which a first-ever deploy hasn't written yet, so the first deploy stages production config. Running the deploy again fixes it.
7. **Missing screenshot:** the checking (D6/W3) state finishes too fast locally to capture.

Screenshots are in `client/screenshots/web-local/` (17 PNGs), and the seed output is in `.icp-local/web-seed.json`.

# ---- report ----
I ran all three parts of the end-to-end flow on emulator-5554 against the local canisters, and it now works through to the end. To get there I fixed eight bugs that broke a flow step and several copy and display bugs. Everything passes at the end: `cargo test --workspace` (2093 passed), Gradle `test allTests` (3626 results, 0 failures) and `scripts/smoke-local.sh` (27/27). Nothing was committed and nothing ran outside the local replica.

## Step 1: build and test
| Command | Result | Time |
|---|---|---|
| `cargo test --workspace` | 2091 passed; after my fixes 2093 passed | 364 s |
| `cargo test -p witness-attest --features dev-attestation` | pass | – |
| `scripts/build-android-native.sh` | both `.so` files and the UniFFI bindings | 45 s |
| `scripts/smoke-local.sh` (deploys too) | SMOKE OK, 27 steps; rerun at the end OK | 47 s / 65 s |
| `./gradlew test allTests` | 0 failures at the start and at the end | 122 s |
| APK (`assembleBenchmarkRelease`) plus both web distributions | 71 MB minified APK | 57 s |
| `scripts/stage-web.sh` then `icp deploy web --yes` | ok | 3 s |

## Step 2: emulator flow
1. **Fresh install, onboarding (A2, A3) and permissions:** ok.
2. **Enrollment:** first rejected with `UntrustedRoot`, then `ExtensionPlacement` (bugs 1 and 2). After the fix it enrolls in under a second with the Dev key and the DEV badge.
3. **Capture with the virtual scene camera:** sealed and anchored in about 2–10 s. `icp canister call registry get '("YQNYMTJHXYP1536C")' --query` returns `record_ix 21` with an `anchored_ns` value.
4. **Post with a blur:** "Couldn't seal this post" (bug 3). After the fix, post `DSTBCMFSEB12G44B` anchored (ix 22). A later post, `GK8GHYWN4RMGPDA6`, has 2 blurs; the share sheet takes about 5 s to appear and now shares the image.
5. **Proof page `/p/{id}` in the built-in browser:** first showed "This proof page was removed" because the app never uploaded the post preview (bug 5). After the fix it shows "Matches an original · with 2 declared blurs" with the DEV badge, on desktop and at 412 px wide.
6. **Web `/check`:**
   - The emulator screencap of the post is a crop of about 40% of the frame. It returns **No Witness record**, because the fingerprint doesn't survive a crop that large.
   - A full-frame copy (the media preview) returns "Matches an original, with changes · 2 blurs declared · development device".
7. **In-app Check** (post image shared into Witness): "Original · Matches the sealed capture exactly · development device". That verdict was itself a bug: it was a post of a post (bug 6).
8. **Case flow:**
   - Created "Living room TV damage" (Property damage) and captured 2 items, both anchored.
   - Nothing was AI-drafted, because the emulator has no on-device model. The summary Edit button did nothing (bug 7); after the fix I typed a summary by hand and saved it.
   - The first share link wouldn't open on the web (bug 4). The new link opens: both items decrypt and show "2 of 2 verified · 2 original", with the summary shown.
   - Creating the share link took about 3.4 s.
9. **Erase:** Settings → Privacy only offers "Erase all public copies", so to erase a single item I used the item menu's "Erase public copies" on `DSTB…`. Pages answers 410 "Page removed by its owner" within 4.1 s, media gives 404, and `registry get` still returns the permanent record.
10. **Final check:** a clean onboarding on the final build shows the corrected DEV copy. The emulator has 481 MB free, and my temp file in `/data/local/tmp` is removed.

## Bugs fixed (each with regression tests)
1. **Emulator attestation root not trusted.**
   - `crates/witness-attest/src/roots.rs` adds the "Droid Unregistered Device CA" as a Dev root, only under `dev-attestation`. The new files are `roots/droid_unregistered_ec.spki.der` plus `tests/data/{droid_unregistered_root,emulator_rkp_leaf,emulator_rkp_intermediate}.der`.
   - `scripts/build-canister.sh` now takes `WITNESS_DEBUG_DIGEST` from `~/.android/debug.keystore` for dev-attestation builds. Without it the next rejection would have been `SignerDigest`.
2. **`ExtensionPlacement`:** the emulator's test root itself carries the provisioning-info extension. In `policy.rs`, `attestation_index(certs, root)` now ignores that extension on a Dev root only; Google chains are still strict. Tests run against the real emulator chain.
3. **Posts never sealed** ("seal claim: Invalid"): a derivative claim carried the parent's location commitment. `NativeSealEngine.derive` now drops it.
4. **Case links couldn't open on the web:** `ShareCase.link` now appends `&h=<b64url sha256(bundle)>`, and `decodeKey` parses `&`-separated fragments. This closes the web lane's open issue #1.
5. **Post previews never reached media,** so the web showed "Removed". New pieces:
   - port `PreviewPublisher` and use case `PublishPreviews`, which the anchor worker runs and retries;
   - `NativeMedia` uploads a signed `UploadTarget.Preview` to the registry's media shard;
   - `PreviewJpeg` sizes the image to at most 1080 px and 150 KB.
6. **Posting from a post** produced a derivative with no declared edits, which checked as "Original". `BuildPost.draft` now starts from the root original.
7. **Report "Edit" did nothing:** A2UI edit events were dropped. They are now routed, and the saved summary is bound into the report's summary slot.
8. **"Share post" sent only the link text:** it now shares the JPEG with the caption and link.
9. **Copy and display fixes:**
   - DEV records no longer claim "genuine phone" or "verified device" on the device setup screen, the web proof page, the case link page, or the pages shell.
   - Geohash codes such as "Area dr5rs" now read "near 40.7° N, 74.0° W" on the proof page and the check result.
   - Case item details are now cut off with an ellipsis instead of being clipped.
   - The Removed page no longer says "taken down after a report" when the owner erased it.
10. **Logging (diagnostics):** local builds now log non-fatal errors to logcat under the tag `Witness`, and failed post publishes are reported instead of swallowed. This is how I found bug 3.

## Files changed (under `/Users/andrejones/Desktop/workspace/projects/witness`)
- **Rust:**
  - `crates/witness-attest/src/{roots.rs,policy.rs,policy/tests.rs}`, `tests/real_roots.rs`
  - `canisters/pages/src/html.rs`
- **Scripts:** `scripts/build-canister.sh`
- **Kotlin, core:**
  - `core/domain/.../port/NetworkPorts.kt`
  - `core/domain/.../usecase/{PublishPreviews.kt (new),AnchorOutbox.kt,BuildPost.kt,ShareCase.kt,Geohash.kt}`
  - `core/ui/.../ProvesCard.kt`
- **Kotlin, features:**
  - `feature/proof/.../{ProofPageViewModel.kt,CaseLinkViewModel.kt,WebPages.kt}`
  - `feature/check/.../CheckOutcome.kt`
  - `feature/post/.../ComposerViewModel.kt`
  - `feature/home/.../DeviceSetup.kt`
  - `feature/case/.../CaseReportScreen.kt`
- **Kotlin, native and app:**
  - `native/witness/.../{NativeSealEngine.kt,NativeNetwork.kt}`
  - `androidApp/.../{MainActivity.kt,WitnessApp.kt,WitnessApplication.kt,nav/AppNavigator.kt,system/SystemActions.kt,ui/CaseReport.kt}`
  - `androidApp/.../di/{AppGraph.kt,Features.kt,NativeAvailability.kt,RealNative.kt,LocalLogTelemetry.kt (new),PreviewJpeg.kt (new)}`
- **Tests:**
  - New: `GeohashTest`, `PublishPreviewsTest`, `LocalLogTelemetryTest`, `ShareIntentTest`, `CaseReportRouteTest`.
  - Extended: `ShareCaseTest`, `BuildPostTest`, `NativeSealEngineTest`, `NativeNetworkTest` and its Fakes, `NativeWiringTest`, `AppGraphTest`, `ProofPageViewModelTest`, `CaseLinkViewModelTest`, `CheckOutcomeTest`, `OnboardingViewModelTest`, `CaseViewModelsTest`.
  - Changed expectations, on purpose:
    - the proof-page area string;
    - the case-link vector URL, which now carries `&h=`;
    - `AppGraphTest`, which now looks inside the logging wrapper.

## Screenshots
`client/screenshots/e2e/`, 32 files numbered 01–43:
- Emulator: A2, A3, A4, B1, B3, B4, B6, D1, D3, A7, D7, C1, C3, C5, C9, E5, plus the erase screens.
- Web: proof page desktop (X1-style) and mobile (W1-style), `/check` results, case link desktop (X3-style) and mobile, and the erased tombstone.

All of the listed screens matched their renders apart from the deviations under "What still doesn't work". I compared X2 (desktop check) only against the web lane's notes, not against its render.

## What still doesn't work
- **"Cropped about 12%" on an uncropped copy:** the pixel path counts its 1.5% edge margin and partial windows as "not compared" (`crates/witness-verdict/src/confirm.rs`). This is the matcher lane's area and would change the golden digests, so I left it.
- **Large crops** of a post aren't found on `/check` ("No Witness record").
- **Recommended shots never tick off** in the case report and no AI drafts appear, because the emulator has no on-device model. The "Edit" buttons on affected areas and repair notes still do nothing; only the summary can be edited.
- **Case page details:**
  - shows "No expiry set" because media doesn't expose expiry;
  - labels a hand-written summary "AI-drafted".
- **Privacy screen** has only "erase all"; erasing a single item is done from the item's menu.
- **Composer differences from the B3 render:**
  - the link chip shows `witness.app/p/…` until the post is published;
  - location shows "city or area" because there's no reverse geocoding;
  - links use production domains (witness.app), so on local you open the proof page via the pages canister URL.
- **Emulator networking:** the emulator still needs `adb -s emulator-5554 reverse tcp:8090 tcp:8090`, because the gateway rejects the `10.0.2.2` Host header. This isn't scripted.
- **Leftover record:** post `3HVEN…` was made before fix 6 and still checks as "Original".

# ---- report ----
{"summary": "I reviewed the whole repo for test quality and found four real gaps, all at untrusted-input boundaries. All four are fixed, each with a regression test that fails without the fix. Coverage is otherwise strong: every pub fn in crates/, canisters/ and tools/ is referenced from tests, every Kotlin ViewModel has tests, and the canister ingress paths (BlobPath, page shells, origins, outbox, enroll/submit/erase) already have hostile-input tests. Assertion-free tests are deliberate \"never throws\" robustness checks, and the snapshot tests are render generators. Neither is a defect.\n\nTwo bugs were found and fixed:\n1. A case link whose bundle has an undecodable item claim showed the Offline state on the web instead of Tampered. Fixed in client/webApp/bridges/engine.mjs.\n2. Links shared to Android from any host containing /p/<id> (e.g. https://example.com/p/<id>) opened the proof page, bypassing the host check that deep links already enforce. They also ignored the configured Links (proofBase), unlike deep links. Fixed in client/androidApp/src/main/kotlin/app/witness/nav/DeepLinks.kt.\n\nTwo parser inconsistencies were fixed:\n- ShareCase.decodeKey rejected padded keys (k=...=), while Rust key_from_fragment and core/icp CaseLink.keyFrom accept them.\n- Hex.parse accepted non-ASCII Unicode digits.\n\nA concurrent reviewer had already restricted shared image streams to content:// URIs (with a test), so I removed my duplicate test.\n\nTests verified passing: bridges `npm test` (59 pass, 1 live test skipped); `:core:domain:testAndroidHostTest`; `:core:platform:testAndroidHostTest`; `:native:witness:testDebugUnitTest`; `:feature:check:testAndroidHostTest`; `:feature:proof:testAndroidHostTest`; `:androidApp:testDebugUnitTest`. Each new test was run red before its fix and green after, except the Hex.parse one: there the fix and test were applied together, so it wasn't seen failing. Not run: the full cargo workspace, since no Rust changed; the wasm and js Gradle targets; the webApp distribution builds; nothing on the emulator. Nothing was committed or deployed.\n\nFiles changed (under /Users/andrejones/Desktop/workspace/projects/witness):\n- client/webApp/bridges/engine.mjs and test/engine.test.mjs\n- client/androidApp/src/main/kotlin/app/witness/nav/DeepLinks.kt and src/test/kotlin/app/witness/nav/DeepLinksTest.kt\n- client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/ShareCase.kt and src/commonTest/kotlin/app/witness/core/domain/usecase/ShareCaseTest.kt\n- client/native/witness/src/main/kotlin/app/witness/nativelib/NativeMapping.kt and src/test/kotlin/app/witness/nativelib/NativeMappingTest.kt\n\nOpen, low-severity:\n- CaseBundleCborTest.acceptsTheLimits only encodes at the limits. Its decoder-side acceptance is covered on the Rust side.\n- BridgeCases.decode lets a WireException surface as Offline. The JSON it parses comes from our own engine, so this is not an attacker-reachable path.", "findings": [{"title": "Web case opener throws (shows Offline) on a bundle item whose submission claim is undecodable", "file": "client/webApp/bridges/engine.mjs", "severity": "medium", "detail": "openCase only wrapped decryption and bundle parsing in try/catch. The per-item core.decodeClaim(...) for capture times ran outside the try. witness-web's Submission decode never validates claim_cbor; the record id is just its hash, so any bytes pass (the shared Kotlin/Rust CaseVector itself uses the claim bytes 'claim'). A link sender who builds such a bundle therefore made openCase reject. JsWitnessBridge turns that rejection into an IcpException, and CaseLinkViewModel shows Offline ('check your connection') instead of Tampered. Fix: the item mapping moved inside the try, so a malformed claim returns {status:'tampered'}.", "test": "client/webApp/bridges/test/engine.test.mjs: 'openCase treats an undecodable item claim as tampered instead of throwing'", "fixed": true}, {"title": "Links shared from foreign hosts open the proof page, bypassing the deep-link host check", "file": "client/androidApp/src/main/kotlin/app/witness/nav/DeepLinks.kt", "severity": "medium", "detail": "ShareParser.proofRecord recognises '/p/<id>' on any host, so text shared via ACTION_SEND or ACTION_VIEW containing https://example.com/p/<id>, https://witness.app.evil.com/p/<id> or https://evil.com/witness.app/p/<id> became SharedItem.ProofLink. DeepLinks.keyFor(item) then opened ProofPageKey directly, although DeepLinksTest.foreignAndMalformedLinksOpenNothing asserts that the same URL opens nothing as a link. It also ignored the configured Links (proofBase). Fix: ProofLink now goes through keyFor(item.url, links), the same host-checked PastedLink path that every other link uses.", "test": "client/androidApp/src/test/kotlin/app/witness/nav/DeepLinksTest.kt: sharedProofLinksFromForeignHostsOpenNothing", "fixed": true}, {"title": "Case-link key parsers disagree on padded base64url keys", "file": "client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/ShareCase.kt", "severity": "low", "detail": "Three parsers read the same #k= fragment. Rust key_from_fragment (URL_SAFE_ANY_PADDING) and core/icp CaseLink.keyFrom (ABSENT_OPTIONAL) accept 'k=<43 chars>='. ShareCase.decodeKey used PaddingOption.ABSENT and rejected it. As a result, a padded link opened on the web but PastedLink and DeepLinks reported MissingKey in the app. Fix: ABSENT_OPTIONAL; encoding is unchanged and still unpadded. New assertions cover padded keys, padded keys among other params, double padding (rejected), an empty k= and an empty fragment.", "test": "client/core/domain/src/commonTest/kotlin/app/witness/core/domain/usecase/ShareCaseTest.kt: linkCarriesKeyInFragmentOnly (extended)", "fixed": true}, {"title": "Hex.parse accepts non-ASCII Unicode digits", "file": "client/native/witness/src/main/kotlin/app/witness/nativelib/NativeMapping.kt", "severity": "low", "detail": "digit() used Character.digit(c, 16), which accepts fullwidth (U+FF11) and Arabic-Indic (U+0661) digits. So Hex.parse(\"\uff11\uff12\") returned 0x12 instead of failing. Hex.parse is used for the certificate serial in NativeSealEngine. Fix: characters at or above 128 are rejected.", "test": "client/native/witness/src/test/kotlin/app/witness/nativelib/NativeMappingTest.kt (hex test extended with fullwidth and Arabic-Indic inputs)", "fixed": true}, {"title": "Share intents could make the app read its own private files via file:// streams", "file": "client/core/platform/src/commonMain/kotlin/app/witness/core/platform/share/SharedItem.kt", "severity": "medium", "detail": "ShareParser accepted any EXTRA_STREAM scheme. readImage then calls ContentResolver.openInputStream, which also opens file:// URIs, so another app could point Witness at its own private files. A concurrent reviewer fixed this during my pass (isContentUri, with test onlyContentUrisAreAcceptedAsImages). I removed my duplicate test, and the platform suite passes with their change.", "test": "client/core/platform/src/commonTest/kotlin/app/witness/core/platform/share/ShareParserTest.kt: onlyContentUrisAreAcceptedAsImages (by another reviewer)", "fixed": true}]}

# ---- report ----
All five fixes hold up. I changed no code. Every affected suite passes, and each regression test would fail if its fix were reverted.

1. **Web `openCase` and undecodable claims: verified.** In `/Users/andrejones/Desktop/workspace/projects/witness/client/webApp/bridges/engine.mjs` (lines 277–290), the item mapping that calls `core.decodeClaim` is now inside the try, so a bad claim returns `{status:'tampered'}`. The test's fake bundle has an item with `submission: {claim:'ab'}` and a `decodeClaim` that throws. With the old code the promise would reject and the test would fail. `decodeClaim` has one other caller (`fullClaim`, line 224), which reads proof records, not case bundles, so it is outside this finding.

2. **Shared proof links from foreign hosts: verified.** The only consumer of `SharedItem.ProofLink` in main code is `DeepLinks.keyFor(item, …)`, which now routes through `keyFor(item.url, links)` and so `PastedLink.parse` and its host check. `item.url` is the URL alone, extracted by `ShareParser.firstUrl`, so the tidy-up in `PastedLink` behaves the same. The new test first asserts that `ShareParser` produces a `ProofLink` for all three foreign URLs, so the old direct `ProofPageKey` route would have opened them and failed the test. On the "configured Links" point: `MainActivity` still passes the default `Links()`, but `AppGraph.links()` also returns `Links()`, so nothing differs today.

3. **Padded case-link keys: verified.** `ShareCase.B64URL` is now `ABSENT_OPTIONAL`, which matches `CaseLink.keyFrom` and Rust's `URL_SAFE_ANY_PADDING`. Encoding stays unpadded; line 76 of the test asserts there is no `=`. The new `"$fragment="` and `"x=1&$fragment=&h=2"` cases would throw under `ABSENT`, decode to null, and fail. Double padding is still rejected, as Rust does. `PastedLink` is the only caller in main code, so `DeepLinks` gets the fix too.

4. **`Hex.parse` and non-ASCII digits: verified.** `digit()` now returns -1 for any character at code 128 or above. The fullwidth (U+FF11) and Arabic-Indic (U+0661) inputs would have decoded under plain `Character.digit`, so the test would fail without the fix. The only other hex decoder in the app is a private one in `EcdsaSignature.kt`, used only on constant strings.

5. **`file://` streams in share intents: verified.** `ShareParser.parse` keeps only `content://` URIs that have something after the scheme, and both `ACTION_SEND` and `ACTION_SEND_MULTIPLE` go through it. `onlyContentUrisAreAcceptedAsImages` covers `file://`, `FILE://`, bare paths, `android.resource://`, `http` and a bare `content://`, so the old accept-any-scheme code would fail it. One small gap remains, not fixed: another app can still pass a `content://` URI for Witness's own FileProvider (`${applicationId}.files`). That provider only exposes `cache/shared/`, which holds files Witness shares outward anyway, so I consider it acceptable.

Test runs:
- **Web bridges:** `node --test test/` in `client/webApp/bridges` gave 60 tests, 59 passed, 0 failed, 1 skipped. The skipped one is likely the live test.
- **Kotlin:** `./gradlew :native:witness:test :core:domain:testAndroidHostTest :core:platform:testAndroidHostTest :feature:check:testAndroidHostTest :androidApp:testDebugUnitTest` exited 0, with no failures in any result file.

| Test class | Tests | Failures |
|---|---|---|
| `DeepLinksTest` | 12 | 0 |
| `PastedLinkTest` | 11 | 0 |
| `ShareParserTest` | 8 | 0 |
| `ShareIntentsTest` | 6 | 0 |
| `ShareCaseTest` (Android host, JS and wasm builds) | 12 each | 0 |
| `NativeMappingTest` | 16 | 0 |

The changed lines add no code comments. Nothing was deployed or committed.

# ---- report ----
{"summary": "I reviewed the whole repo through the security and privacy lens and fixed 3 real issues, each with a regression test. 5 more are reported but not fixed: they need design, config or release decisions. Each of these held up on inspection:\n- **Canisters:** registry-only guards on index put/erase, pages put_shell/tombstone and media sync/erase; inspect_message size limits; certified 404s.\n- **Attestation:** HMAC-timestamped challenge with a TTL; the Dev root and debug signing digest exist only under `dev-attestation`.\n- **Android keys:** generated in AndroidKeyStore and never exported.\n- **E2EE case links:** a fresh key and random nonce per chunk; the AAD binds case id, index, count and bundle hash, and the bundle hash is checked on open; the key moves to sessionStorage and is removed from the URL.\n- **Telemetry:** off by default (Firebase manifest flags off, and the gate is off until opt-in); metric keys are allowlisted.\n- **PII:** EXIF is stripped from sealed images, the area is only a geohash-5 cell, and the claim has no device model.\n- **Secrets:** none in tracked or untracked files.\n\nCommands verified:\n- `cargo test -p witness-smoke`: 28 passed.\n- `cargo clippy -p witness-smoke --all-targets -D warnings`: clean.\n- `./gradlew :core:domain:testAndroidHostTest :core:domain:jsBrowserTest :core:domain:wasmJsBrowserTest :core:platform:allTests`: pass.\n- `scripts/stage-web.sh && icp deploy web --yes` (local only).\n- `WITNESS_SMOKE_SKIP_DEPLOY=1 scripts/smoke-local.sh`: SMOKE OK, 28 steps, including the new CSP step.\n- In the browser pane, `/check`, `/p/{id}` and `/v/{id}#k=&h=` on the web canister still load, query the registry, decrypt and show images under the new CSP, with no CSP violations in the console.\n\nNothing was committed or deployed beyond the local replica.", "findings": [{"title": "Web canister (app origin) sent no CSP, frame-ancestors, nosniff or referrer policy", "file": "scripts/stage-web.sh", "severity": "medium", "detail": "The static-site (certified-assets) canister adds no security headers by default. The web canister serves /check, which decodes untrusted images in wasm, and its SPA fallback also serves /v/* routes, which put case keys in sessionStorage. Both had no CSP at all, so the pages could be framed (clickjacking) and loaded scripts had no restriction. Fix: stage-web.sh now writes dist/_headers. It sets a strict CSP (default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src and img-src limited to self, the API host and the canister wildcard, with local values taken from ids.json and production values otherwise; base-uri 'none'; frame-ancestors 'none'; form-action 'none'), plus nosniff and Referrer-Policy: no-referrer. Redeployed web locally and confirmed in a real browser that /check, /p and /v still work with no CSP violations. Files: scripts/stage-web.sh, tools/witness-smoke/src/net.rs, tools/witness-smoke/src/main.rs.", "fixed": true, "test": "tools/witness-smoke/src/net.rs: strict_page_csp_requires_lockdown_directives and header_value_is_case_insensitive_and_trimmed. New smoke step 'web app pages send a strict CSP' (passes, 28/28)."}, {"title": "Android share target read any stream URI, including file:// paths to app-private data (confused deputy)", "file": "client/core/platform/src/commonMain/kotlin/app/witness/core/platform/share/SharedItem.kt", "severity": "medium", "detail": "MainActivity is exported with SEND and SEND_MULTIPLE image/* filters. ShareParser accepted any EXTRA_STREAM string, and readImage() opens it with Witness's own permissions. Any app could therefore send file:///data/data/app.witness/files/captures/<id>.jpg (unredacted originals) or other private files and make Witness load and process them. Fix: only content:// URIs are accepted. Other schemes are dropped before the image branch, so a share with only bad streams falls back to text or null.", "fixed": true, "test": "client/core/platform/src/commonTest/kotlin/app/witness/core/platform/share/ShareParserTest.kt: onlyContentUrisAreAcceptedAsImages"}, {"title": "Telemetry redactor let through record IDs that contain no digits", "file": "client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/TelemetryGate.kt", "severity": "low", "detail": "Redactor's 16-plus-character token rule needs at least one digit. About 0.25% of 16-character Crockford record IDs, (22/32)^16, are all letters, and those passed into Crashlytics breadcrumbs and non-fatal messages unredacted. That breaks the rule that telemetry never contains record IDs. Fix: an extra pattern redacts standalone tokens of 16 or more upper-case alphanumeric characters. Lower-case prose such as 'internationalization' is untouched.", "fixed": true, "test": "client/core/domain/src/commonTest/kotlin/app/witness/core/domain/usecase/TelemetryGateTest.kt: recordIdsWithoutDigitsAreRedacted (androidHost, js, wasmJs)"}, {"title": "Web trust anchor falls back to the uncertified ca_root query, and production has no anchor", "file": "client/webApp/bridges/config.mjs", "severity": "medium", "detail": "engine.mjs trustAnchor() uses config.anchor if set, otherwise registry.ca_root() as a plain query. That root is not certified-state-verified; it relies only on the query response signature. PRODUCTION.anchor is '', so production /check would trust the Witness CA root on an uncertified query when judging Content Credentials. The fix needs either a pinned anchor in the production config or ca_root added to the registry's certified tree. Both are config or contract decisions for the web and registry owners, so not fixed.", "fixed": false}, {"title": "Case shell loads {app}/boot.mjs without SRI or an integrity import map", "file": "canisters/pages/src/html.rs", "severity": "medium", "detail": "ARCHITECTURE section 8.1 asks the case.witness.app shell to pin the bundle through an import map with integrity entries. The shell only has a nonce plus strict-dynamic on the boot.mjs tag, so whoever controls app-origin delivery can run code on the case origin and read case keys from sessionStorage. Fixing it needs the build to emit hashes and pages to embed them, which is a cross-lane build contract. Not fixed.", "fixed": false}, {"title": "Pages accepts http:// origins in non-local builds", "file": "canisters/pages/src/lib.rs", "severity": "low", "detail": "Media rejects non-https origins unless built with local-dev (checked_origins). Pages only runs Origins::validate(), so a production pages install would accept http origins in the CSP, canonical URL and boot.mjs src. A matching local-dev gate in pages (plus the icp.yaml feature) would close it. Not fixed: it touches the pages build config.", "fixed": false}, {"title": "Release builds also permit cleartext traffic to localhost, 127.0.0.1 and 10.0.2.2", "file": "client/androidApp/src/main/res/xml/network_security_config.xml", "severity": "low", "detail": "The config sits in src/main, so production APKs also allow cleartext to these loopback and emulator hosts. Exploitability is minimal, but it belongs in a debug or benchmark source set. Not fixed.", "fixed": false}, {"title": "Media BeginUpload signature is not bound to the caller", "file": "canisters/media/src/store.rs", "severity": "low", "detail": "For an uncommitted upload with the same sha256, size and key, begin() sets the uploader to the current caller. Anyone who replays a signed BeginUpload can take over the upload and block the real uploader's put_chunk until they call begin again. Content cannot be substituted, because commit checks the hash, so the impact is a temporary denial of service only. Including the caller principal in the signed bytes would fix it, but that changes the signing contract. Not fixed.", "fixed": false}]}

# ---- report ----
{"summary": "I reviewed the repo for efficiency and modularity, made four fixes with regression tests, and left four smaller issues open. All touched suites pass:\n- `cargo test -p witness-index`: 77 unit + 4 PocketIC, plus clippy `-D warnings` and rustfmt clean.\n- Full client `./gradlew test --continue` exits 0; the 1,971 freshly rerun results have 0 failures.\n- The changed client modules' `allTests` (`core:ui`, `feature:check`, `feature:proof`) pass on js and wasm.\n- The build-logic convention tests pass, and webApp compiles for js and wasm.\n\nNothing was deployed or committed, and no code comments were added.\n\nFiles changed, under /Users/andrejones/Desktop/workspace/projects/witness:\n- `canisters/index/src/buckets.rs`\n- `client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/PublishPreviews.kt` and its `PublishPreviewsTest.kt`\n- New in `core/ui`: `client/core/ui/src/commonMain/kotlin/app/witness/core/ui/Bitmaps.kt`, `client/core/ui/src/androidMain/kotlin/app/witness/core/ui/Bitmaps.android.kt`, `client/core/ui/src/webMain/kotlin/app/witness/core/ui/Bitmaps.web.kt` and `client/core/ui/src/commonTest/kotlin/app/witness/core/ui/BitmapsTest.kt`\n- `client/feature/check/src/commonMain/kotlin/app/witness/feature/check/{CheckSections.kt,CheckScreen.kt,WebCheckPage.kt}`\n- `client/feature/proof/src/commonMain/kotlin/app/witness/feature/proof/ProofImages.kt`\n- `client/feature/capture/src/commonMain/kotlin/app/witness/feature/capture/Photo.kt`. `client/feature/capture/src/androidMain/kotlin/app/witness/feature/capture/Photo.android.kt` was deleted.\n- `client/feature/post/src/{commonMain,androidMain}/kotlin/app/witness/feature/post/Photo*.kt`\n- Test imports only: `client/feature/capture/src/commonTest/kotlin/app/witness/feature/capture/PhotoTest.kt` and `client/feature/post/src/androidHostTest/kotlin/app/witness/feature/post/BlurPhotoTest.kt`\n- `client/androidApp/src/main/kotlin/app/witness/di/PreviewJpeg.kt`\n- `client/core/{icp,verify,platform}/build.gradle.kts`\n- `client/build-logic/convention/src/main/kotlin/Witness.kt` and `client/build-logic/convention/src/test/kotlin/WitnessConventionsTest.kt`\n\nDeviations:\n- I tried a HEAD request for the published-preview check, but the local gateway answers HEAD with 503, so I did not change the network call. The cost is cut on the domain side instead.\n- On the web, image decoding still decodes at full size: the browser decoder offers no subsampling, and the web picker supplies no dimensions, so the size shown on the web still comes from the decoded image.", "findings": [{"title": "Every index put read the whole posting chain of each bucket it touched", "file": "canisters/index/src/buckets.rs", "severity": "high", "detail": "`Buckets::append` found a bucket's last block with `self.walk(key).last()`, which reads every 1 KiB block of the bucket from stable memory. Each indexed record appends to about shapes x 16 bucket keys, and a put batch carries up to 64 postings. Stable reads therefore grew linearly with bucket length, the dominant write-path cost as the index grows.\n\nFix: a new `tail()` does a single reverse range lookup, `range(..=stable_key(u16::MAX)).next_back()`, and checks the key belongs to the same bucket. The existing dense-block layout guarantees the highest block is the tail.\n\nMeasured stable bytes read per append, old then new:\n- old: 80 KB into a 40-block bucket, 242 KB into a 160-block bucket\n- new: about the same for both sizes", "test": "buckets::tests::append_reads_only_the_tail_block_not_the_whole_bucket (counting Memory wrapper; fails on the old code with 79938 vs 241998 bytes), buckets::tests::tail_is_the_highest_block_of_its_bucket_only", "fixed": true}, {"title": "Preview publishing re-downloaded every recent post on every anchor run and kept going after going offline", "file": "client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/PublishPreviews.kt", "severity": "high", "detail": "The anchor worker runs after every capture. Each run, `PublishPreviews` called `publisher.isPublished` for every anchored post from the last 7 days. `NativeMedia.isPublished` makes an uncached `shards()` registry call and downloads the whole preview (up to 150 KB) for each media shard. On `OfflineException` it carried on to the next post, so every remaining post waited out its own 10 s timeout.\n\nFix:\n- Record ids confirmed or just published are remembered in a set for the life of the process, so later runs skip them.\n- The first `OfflineException` ends the run and counts every remaining post as pending, so the worker still retries.", "test": "PublishPreviewsTest.publishedPostsAreNotAskedAboutAgainOnLaterRuns, PublishPreviewsTest.offlineStopsTheRunAndCountsEveryRemainingPostAsPending (8/8 pass)", "fixed": true}, {"title": "Check and proof screens decoded photos at full resolution, with the same helper copied into two feature modules", "file": "client/feature/check/src/commonMain/kotlin/app/witness/feature/check/CheckSections.kt", "severity": "medium", "detail": "`feature/check` and `feature/proof` each had an identical `rememberBitmap` that decodes at full size. The Check picker accepts files up to 64 MB, so a 12 MP photo became a 48 MB bitmap and a 48 MP photo about 192 MB. Each call site decodes separately, and the same image is decoded again on Check state changes, which risks running out of memory on Android.\n\n`decodePhoto` and `sampleSize` were also duplicated in `feature/capture` and `feature/post`, and `PreviewJpeg` had another copy of the sampling formula.\n\nFix:\n- One shared `rememberBitmap(bytes, maxDimension = 1280)` in `core/ui` decodes through `decodeBitmap`. On Android it subsamples with `inSampleSize`; on the web it decodes at full size.\n- The duplicates in check, proof, capture and post are removed, and `PreviewJpeg.sampleSize` delegates to the shared function.\n- `dimsText` now prefers the picked image's real size over the decoded bitmap's, so the size shown stays correct after subsampling.", "test": "core/ui BitmapsTest (5 cases, android+js+wasm); existing capture PhotoTest and post BlurPhotoTest now run against the shared helpers", "fixed": true}, {"title": "Three core adapters declared an unused dependency on the native library module", "file": "client/core/icp/build.gradle.kts", "severity": "low", "detail": "`core/icp`, `core/verify` and `core/platform` declared `implementation(projects.native.witness)` on Android, but none of their sources import `app.witness.nativelib` or `uniffi`. That coupled core adapters to the UniFFI module, which belongs only in `androidApp`, and added it to their compile graph.\n\nFix: removed the three dependencies. The build-logic `enforceModuleRules` now limits these modules to `core:model` and `core:domain` (plus `core:icp` for `core:verify`), so the build fails if the dependency comes back.", "test": "WitnessConventionsTest.coreAdaptersStayOffTheNativeLibrary; androidApp compileDebugKotlin + full ./gradlew test pass", "fixed": true}, {"title": "Registry outbox drain sends one batch at a time", "file": "canisters/registry/src/ic.rs", "severity": "low", "detail": "`drain()` waits for each destination's `bounded_wait` call before starting the next. A tick with index, pages and media batches therefore takes about the sum of their round trips instead of the longest one. Sending them in parallel (for example with `join_all`) would shorten delivery. I did not fix this: it changes when calls interleave with outbox `complete`, and only PocketIC can test that.", "fixed": false}, {"title": "Outbox planning still scans entries for a destination whose batch is already full", "file": "canisters/registry/src/outbox.rs", "severity": "low", "detail": "`plan()` decodes and counts up to 256 entries per lane even after that destination's batch of 64 is full. Up to 192 decodes per backed-up lane are wasted, and they count against the 1,024 per-tick limit. Stopping the lane early is not safe while unlaned entries from before the lane change still share lane 0 with other destinations (`legacy_unlaned_entries_still_drain_in_order`). The waste is bounded, so I left it.", "fixed": false}, {"title": "Anchoring view model re-reads every outbox blob on each change", "file": "client/feature/home/src/commonMain/kotlin/app/witness/feature/home/AnchoringViewModel.kt", "severity": "low", "detail": "`OutboxDao.observe()` uses `SELECT *`, so each emission loads `claimCbor`, `c2paClaimCbor`, `coseSign1` and `fingerprints` for every queued row, only to show status. A projection without the blob columns behind a separate port method would avoid this. I did not change it because it alters the `OutboxStore` port.", "fixed": false}, {"title": "NativeMedia checks whether a preview exists by downloading it and refetching shards", "file": "client/native/witness/src/main/kotlin/app/witness/nativelib/NativeNetwork.kt", "severity": "low", "detail": "`reference()` and `publish()` call `witness.shards()` every time, which is a certified registry query; the Rust side caches shards only for its own internal use. `isPublished` downloads the full image. A HEAD request would be cheaper, but the local gateway answers HEAD with 503. The `PublishPreviews` cache now limits this to once per post per process. Caching shards on the Kotlin side, or an exists query on media, would remove it entirely.", "fixed": false}]}

# ---- report ----
Verification report (security lens)

**Fixed findings: all three verified**

1. **Web canister CSP and headers: verified.** `scripts/stage-web.sh` writes `dist/_headers`. After my redeploy, `curl` shows the strict CSP, `nosniff` and `no-referrer` on `/check`, `/v/*`, `/p/*` and `/boot.mjs`, and the static-site canister certifies all three headers. `strict_page_csp` rejects a missing `default-src`, `frame-ancestors` or `base-uri`, and any `unsafe-eval`, `unsafe-inline` or `*` in `script-src`, so the smoke step would fail if the headers were removed. `cargo test -p witness-smoke` 28/28, `scripts/smoke-local.sh` SMOKE OK (28 steps).
2. **Share target accepts only `content://`: verified.** All callers go through `ShareParser.parse`, including `ShareIntents` for SEND and SEND_MULTIPLE; VIEW only takes the text path. Without the fix, the `file://` list would come back as `Images`, so the test would fail. One residual case: the app's own non-exported FileProvider (`content://app.witness.files/…`) can still be reached by an attacker this way. It exposes only `cache/shared/`, which holds images Witness already shared out, so I'm accepting it. Platform tests: 126 passing.
3. **Redactor and digit-free IDs: verified.** The new pattern is an upper-case `[0-9A-Z]{16,}` token with alphanumeric boundaries. The test builds an all-letter `RecordId` from bytes `0x55`, which the old rule would have passed through. TelemetryGateTest 7/7 on androidHost, js and wasmJs; domain tests: 186 passing.

**Unfixed findings**

4. **Web trust anchor uses an uncertified `ca_root` query: real, fixed now.** ARCHITECTURE §227 says the CA root is pinned on the website. In `client/webApp/bridges/witness.mjs`, `networkIo.caRoot` now refuses unless `config.local`. A production build without a pinned `config.anchor` makes `check()` fail closed; `/p` proofs, keys and previews still work. Local behaviour is unchanged. New test: `client/webApp/bridges/test/witness.test.mjs` ("production never trusts the CA root from an uncertified query"). `node --test test/`: 60 pass, 1 skipped (the live test). Still open: production needs a real pinned anchor in its `witness-config`.
5. **Case shell loads `boot.mjs` without SRI or an integrity import map: real, not fixed.** `html.rs:189` has only a nonce plus `strict-dynamic`, which differs from ARCHITECTURE §8.1. A fix needs the web build to emit hashes and pages to take them as an init argument, which is a cross-lane contract change.
6. **Pages accepts `http://` origins outside local builds: real, fixed now.**
   - `canisters/pages` gets a `local-dev` feature, and `checked_origins` in `canisters/pages/src/lib.rs` traps on non-https origins unless built with it, the same gate media uses.
   - `icp.yaml` builds pages with `local-dev`, and `canisters/pages/tests/pocket.rs` builds its wasm with `--features local-dev`.
   - New test: `http_origins_need_a_local_dev_build`. Pages: 88 unit tests and 6 pocket tests pass; clippy `-D warnings` is clean with and without the feature.
   - Redeployed locally: the pages metadata reads `witness-pages +local-dev` and the smoke run passes.
7. **Release builds allow cleartext to local hosts: real, fixed now.**
   - `src/main/res/xml/network_security_config.xml` is now `base-config cleartextTrafficPermitted="false"`.
   - The old permissive file moved to `client/androidApp/src/local/res/xml/network_security_config.xml`. `androidApp/build.gradle.kts` adds that folder only for local variants (debug and benchmark), the same rule that already decides the local replica URL.
   - Checked in the merged resources: release and nonMinifiedRelease are strict; debug and benchmarkRelease still allow 10.0.2.2, so the emulator path is unaffected.
   - New test: `client/androidApp/src/test/kotlin/app/witness/system/NetworkSecurityConfigTest.kt` (2 tests). androidApp `testDebugUnitTest`: 137 tests, 0 failures.
8. **Media BeginUpload not bound to the caller: real, not fixed.** `store.rs:124` hands an uncommitted upload to whoever replays the signed request. The impact is a temporary denial of service only, because commit still checks the hash. A fix means changing the signed bytes, a signing contract shared by Android and media.

**Commands run:** `cargo test -p witness-pages` (with and without `local-dev`), `cargo clippy -p witness-pages` (both), `cargo test -p witness-smoke`, `scripts/deploy-local.sh`, `scripts/smoke-local.sh`, `node --test test/` in `client/webApp/bridges`, `./gradlew :core:platform:allTests :core:domain:allTests :androidApp:testDebugUnitTest`, and `:androidApp:merge{Release,BenchmarkRelease,NonMinifiedRelease}Resources`.

**Deviations and open issues**
- `docs/dev/canisters-bindings.md` doesn't yet say that pages now uses `local-dev`.
- The deployed web bundle wasn't rebuilt, so it doesn't include the `witness.mjs` change yet. Local behaviour is the same either way.
- Nothing was installed on the emulator, and no commits were made.

# ---- report ----
{"summary": "I reviewed verdict semantics, ID formats, submission ordering, key windows, erase and upload authorization, and the Rust/Kotlin/JS formats. I fixed 3 real bugs, each with a regression test, and added the missing cross-language test vectors. All touched suites pass: cargo tests for witness-verdict (100), witness-web (98 lib plus golden), witness-core (10 + 14 e2e + golden, unchanged), witness-android, witness-registry (118, also with dev-attestation), witness-format and witness-attest; and Gradle host tests for :core:domain (186) and :core:platform (126). Clippy with -D warnings is clean on the touched crates and rustfmt was applied to the touched files. I did not redeploy the local replica or rebuild the .so/wasm artifacts. The registry rule and the clock flags only take effect after scripts/deploy-local.sh, scripts/build-android-native.sh and crates/witness-web/build.sh are run. I didn't run the wasm32 test target or the PocketIC suites. Nothing was committed.\n\nFiles changed:\n- Fix 1: canisters/registry/src/submit.rs, canisters/registry/src/submit/tests.rs\n- Fix 2: client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/AnchorOutbox.kt, client/core/domain/src/commonTest/kotlin/app/witness/core/domain/usecase/AnchorOutboxTest.kt\n- Fix 3: crates/witness-verdict/src/{oracle.rs,lib.rs,testkit.rs}, crates/witness-web/src/{wire.rs,wire/tests.rs}, crates/witness-android/src/{oracle.rs,oracle/tests.rs}\n- Vectors: crates/witness-web/src/case/tests.rs, crates/witness-format/src/evidence/tests.rs, crates/witness-attest/src/challenge.rs, client/core/platform/src/commonTest/kotlin/app/witness/core/platform/crypto/EcdsaSignatureTest.kt\n\nChecked and consistent, no change needed:\n- Erase signing bytes in Rust and Kotlin, and erase semantics (tombstone, fan-out, queued items dropped).\n- Upload signing bytes (Rust only); media only accepts derivative records signed by the record's key.\n- Crockford base32 encoding of record, key and case IDs, and the capture_id layout (56 random bits + 5-bit schema).\n- Outcome, flag, reason and edit-class codes across Rust, Kotlin and JS.\n- The perceptual path never returns ORIGINAL, and the \"no record\" copy never says \"fake\".\n- The 60-day key window and Fresh/Late challenges.\n- Counter reservation is atomic, and batches are sent per key in counter order.", "findings": [{"title": "Re-derived content-edited post verifies as ORIGINAL", "file": "canisters/registry/src/submit.rs", "severity": "high", "detail": "The registry accepted a derivative whose parent was itself a content-edited derivative. Edits are declared relative to the parent, and the verdict only looks at the copy's own edits (claim.kind.presentation_only()). A resize of a blurred or cropped post was therefore reported as ORIGINAL, with the blur missing from the declared edits. This contradicts ARCHITECTURE \u00a76.5: \"A content-edited derivative is therefore never reported as Original\". The e2e lane only fixed the Kotlin client; the registry still accepted such chains, from any client. Fix: parent_of now rejects a parent that is not presentation-only, with Malformed(\"parent has content edits\"). Chains of presentation-only edits are still allowed. Smoke, the seed tool and the PocketIC tests all derive from originals, so they are unaffected.", "fixed": true, "test": "canisters/registry/src/submit/tests.rs::content_edited_parents_cannot_be_derived_again"}, {"title": "Posts permanently FAILED when their parent was deferred", "file": "client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/AnchorOutbox.kt", "severity": "medium", "detail": "BuildPost allows posting an original that is not anchored yet. If the original gets RetryAfter (quota) in a batch, or is waiting on a notBefore backoff, the derivative is still submitted and the registry answers ParentUnknown. The adapter maps that to Rejected(\"parent_unknown\"), which AnchorOutbox marked FAILED for good, so the post would never anchor. Fix: a parent_unknown rejection is now deferred, PENDING with the default backoff, while the capture's parent is still queued in the outbox. It still fails when the parent is not queued.", "fixed": true, "test": "AnchorOutboxTest.derivativeWaitsWhileItsParentIsStillQueued, AnchorOutboxTest.parentUnknownFailsWhenTheParentIsNotQueued"}, {"title": "LATE_ANCHOR and CLOCK_MISMATCH never appeared on Check results", "file": "crates/witness-verdict/src/oracle.rs", "severity": "medium", "detail": "ARCHITECTURE \u00a77.2 says the clock flags are derived when records are read. SPEC \u00a74.2 says the proof must say so when capture time and anchor are more than 24 hours apart, or when the clock disagrees with GNSS. Both the Android (witness-android oracle::to_record) and web (witness-web RecordWire::to_record) readers hard-coded VerdictFlags::EMPTY, so the copy for these cases in Check and on the proof page never showed. Fix: a new pure clock_flags(claim, anchored_ns) uses the same rules as Kotlin ProofFacts: LATE_ANCHOR when the anchor is more than 24 hours after wall time, CLOCK_MISMATCH when wall and GNSS time differ by more than 60 s. Both readers and the testkit MemoryOracle now use it. The witness-core and witness-web golden digests are unchanged.", "fixed": true, "test": "witness-verdict oracle::tests::clock_flags_follow_the_24_hour_and_gnss_rules; witness-web wire::tests::record_wire_derives_clock_flags_when_read; witness-android oracle::tests::to_record_accepts_consistent_records"}, {"title": "Case bundle Kotlin vector was missing on the Rust side", "file": "crates/witness-web/src/case/tests.rs", "severity": "low", "detail": "The Kotlin CaseVector (bundle CBOR, AES-GCM chunk, case id, fragment, hash) had no Rust counterpart. The new test decrypts the Kotlin chunk with CaseOpener, checks the sha256, and parses the bundle with CaseBundle::parse (title, summary, items, submission record id). It re-encodes the bundle byte for byte and re-seals the chunk with the same nonce to get the identical ciphertext.", "fixed": true, "test": "witness-web case::tests::kotlin_share_case_vector_opens_and_parses"}, {"title": "Location commitment and offline challenge had no cross-language vectors", "file": "client/core/platform/src/commonTest/kotlin/app/witness/core/platform/crypto/EcdsaSignatureTest.kt", "severity": "low", "detail": "Kotlin builds these preimages itself (Preimages.locationCommitment, Preimages.offlineChallenge) while Rust checks them (location_commit, late_challenge), and no shared vector tied the two together. I added the same preimage hex on both sides, and the Rust tests also pin the resulting SHA-256 values (8ac44dbe\u2026 and 5b68e971\u2026).", "fixed": true, "test": "PreimagesTest.preimagesMatchTheRustVectors; witness-format evidence::tests::location_commit_matches_the_kotlin_preimage_vector; witness-attest challenge::tests::late_matches_the_kotlin_preimage_vector"}, {"title": "Android checks trust registry reads without verifying their proofs", "file": "crates/witness-android/src/oracle.rs", "severity": "medium", "detail": "NetOracle.get_record and get_key take query responses as they come; only the record_id is checked against the claim. Nothing verifies the certificate, MMR or SMT proof, although ARCHITECTURE \u00a76.5 says \"certified SMT read\" and \u00a77.3 says \"confirm with a certified registry.get\". The trust anchor also comes from the uncertified ca_root query unless WitnessConfig.ca_root is pinned. Whoever controls the replica or boundary path could supply a key and record and make a forged copy check as ORIGINAL. witness-web already has proof::verify_record and verify_key for this. Not fixed: the change is larger than a surgical fix and belongs to the android lane. Production must at least pin ca_root.", "fixed": false}, {"title": "Preview upload targets the last media shard, not the record's shard", "file": "client/native/witness/src/main/kotlin/app/witness/nativelib/NativeNetwork.kt", "severity": "low", "detail": "NativeMedia.publish uploads to shards().media.last(). The registry fixes media_shard when the record is anchored and sends the Allow only to that shard. Once a new media shard is added, previews for older records would get RetryAfter forever. CertifiedRecord doesn't expose media_shard (the web lane hit the same gap), so this needs a contract change. Not fixed.", "fixed": false}, {"title": "Plus case limit is larger than the canister and opener allow", "file": "client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/ShareCase.kt", "severity": "low", "detail": "ShareCase.limitFor allows entitlement.quotaBytes for Plus users. The media canister caps each case at MAX_CASE_BYTES (16 MiB), and the native adapter and the web CaseOpener cap it at 16 chunks. A Plus case over 16 MiB passes the client check and then fails at upload. Not fixed: Plus case sizing is a product decision.", "fixed": false}]}

# ---- report ----
I checked all eight findings. The four marked fixed hold up, and their tests fail on the old code. Of the four left open, I fixed #6 and agree with leaving the other three. No commits, and nothing deployed beyond PocketIC.

**Fixed findings**

1. **Index `Buckets::append` reading the whole bucket: verified.** `tail()` looks up the highest key of the bucket and checks it still belongs to that bucket, which is correct for the key layout. Every operation keeps a bucket's blocks contiguous, so the highest block is always the last one. I put the old `walk(key).last()` back temporarily and `append_reads_only_the_tail_block_not_the_whole_bucket` failed with `short=79938 long=241998`.

2. **PublishPreviews: verified.** `pending + due.size - i` counts the failing post and every one after it. The remembered set does last the whole process, because `anchorOutbox` is created once per app (`@SingleIn(AppScope)` in `AppGraph.kt`). Runs never overlap, because the worker is unique work with `APPEND_OR_REPLACE`, so the plain `HashSet` is safe. With the set and the early return removed, both new tests fail; restored, 8 of 8 pass.

3. **Shared `rememberBitmap` / `decodeBitmap`: verified, plus one gap fixed.**
   - The duplicates are gone from check, proof, capture and post, and `PreviewJpeg.sampleSize` now uses the shared function.
   - `ThumbnailSource.sampleSize` and `BlurGeometry.sampleSize` remain on purpose: they compute different things.
   - `BitmapsTest` passes on Android host, JS and wasm.
   - `dimsText` had no test, so I added `client/feature/check/src/wasmJsTest/kotlin/app/witness/feature/check/DimsTextTest.kt` (4 cases: picked size beats a subsampled bitmap, bitmap fallback, picked size before decode, nothing known). It lives in `wasmJsTest` because creating an `ImageBitmap` fails under the JS test runner (the Skia graphics library isn't loaded there). It passes 4 of 4 on wasm.

4. **Unused native-library dependency: verified.** None of the three build files declares `:native:witness`, and none of their sources import it. The build-logic `enforceModuleRules` check covers all three, and `coreAdaptersStayOffTheNativeLibrary` asserts it. The androidApp compile and the affected modules' host tests pass.

**Open findings**

5. **Registry drain is sequential: real, not fixed.** The timer runs one tick at a time (`set_timer_interval_serial`), so a slow call delays the next tick. I wrote a temporary PocketIC test to measure it, then removed it. With all canisters on one subnet, as in the local deployment, every call finished within one tick. So the parallel version would show no gain here and can't be tested. It would also mean adding the `futures` crate to the registry canister. Worth doing only if the index or media canisters move to another subnet.

6. **Outbox `plan()` keeps scanning a full lane: fixed now** in `canisters/registry/src/outbox.rs`.
   - Once a destination's batch is full, the scan stops for that lane, but only when the lane belongs to one destination.
   - Lane 0 (`Index(0)`) is shared with older entries saved before lanes existed, so it keeps the old full scan. That means the hottest lane, `Index(0)`, gets no benefit; Pages and Media lanes do.
   - Side effect: tombstoned entries after the stopping point are dropped on a later tick instead of this one.
   - New tests: `a_full_dedicated_lane_stops_scanning_and_leaves_budget_for_the_next` fails without the early stop, and `a_full_shared_lane_keeps_scanning_for_legacy_entries` fails if the scan always stops early.

7. **AnchoringViewModel loads blob columns: real, but not worth fixing.** The outbox table holds only pending rows, which are deleted once anchored. That is a few rows of a few KB each, read only when the table changes. Fixing it would change the shared `OutboxStore` port and its implementations for a negligible saving.

8. **NativeMedia `shards()` and full download: real, low, not fixed.** With finding 2's set, `isPublished` runs at most once per post per app session. For a new post the lookups return 404, so nothing is downloaded; the full download happens only for a post already published, after the app restarts. What remains is one certified query per check and per publish. Caching shards on the Kotlin side would risk a stale shard list for little gain.

**Commands run (all passing)**
- `cargo test -p witness-index`: 77 unit tests and 4 PocketIC tests.
- `cargo test -p witness-registry`: 120 unit tests and 7 PocketIC tests.
- `cargo clippy -p witness-index -p witness-registry --all-targets` (no warnings) and `cargo fmt --check`.
- Gradle:
  - `:core:domain:testAndroidHostTest`
  - `:feature:check:allTests`
  - `:core:ui:allTests`
  - `:build-logic:convention:test`
  - `:androidApp:compileDebugKotlin`
  - host tests for `:core:icp`, `:core:verify`, `:core:platform`, `:feature:capture`, `:feature:post`, `:feature:proof`
  - `:androidApp:testDebugUnitTest`

Most Gradle tasks were up to date because their inputs hadn't changed since they last passed. Each temporary change made to prove a test fails was reverted before the final runs.

# ---- report ----
Five of the reviewer's fixes hold up, and the three unfixed findings are all real. I changed no files: the three unfixed ones need either a shared contract change or a product decision, so they belong to other lanes.

**Fixed findings**

1. **Re-derived content-edited post verifies as ORIGINAL (registry) → verified.** `parent_of` in `canisters/registry/src/submit.rs` now rejects any parent that is not `presentation_only()`. Every submit, single or batched, goes through `parent_of`, so all callers are covered. Crop counts as a content edit (`Global`), so the new test's crop-then-derive case is rejected, and a chain of empty-edit derivatives is still accepted. Without the guard, `err(...)` would panic on the Ok result, so the test does fail without the fix. Smoke, PocketIC and the pages canister only derive from originals. `derivatives_link_to_root_and_get_shells` still passes, because its chain has no edits.

2. **Posts permanently FAILED when their parent was deferred (AnchorOutbox) → verified.** The adapter's `parent_unknown` reason matches `AnchorOutbox.PARENT_UNKNOWN`. The `queued` set is all outbox entries that are not FAILED. The registry requires a parent to be signed by the same key, so parent and child land in the same batch group, sorted by counter. Both tests would fail without the fix: the old code marked the post FAILED.
   - One remaining edge, which existed before: a derivative whose parent gets resealed under a new key loses the parent's old record id and then fails. That is correct, since the registry would reject a parent signed by another key anyway.

3. **LATE_ANCHOR and CLOCK_MISMATCH never appeared (`clock_flags`) → verified.** The rules match Kotlin `ProofFacts` and `TimeEvidence.clockMismatch`: strictly more than 24 hours, and an absolute skew strictly more than 60 s. A grep for every place a verdict `Record` is built finds only three: the Android reader, the web reader and the testkit `MemoryOracle`. All three now call `clock_flags`. The boundary tests cover exactly 24 hours, the next millisecond, an anchor earlier than wall time, and the u64 max. The reader tests now assert LATE_ANCHOR, so they would fail if the flags went back to `EMPTY`. The witness-core golden tests pass unchanged.

4. **Case bundle Kotlin vector missing on the Rust side → verified.** I diffed `KOTLIN_BUNDLE` and `KOTLIN_CHUNK` against `CaseVector.kt` by hand: they are identical, and so are the case id, fragment and hash. The test opens the chunk, parses the bundle, re-encodes it byte for byte and re-seals it to the same ciphertext.

5. **Location and offline-challenge vectors → verified.** The preimage hex strings are the same on both sides. I recomputed the offline timestamp by hand: 1_700_000_000_000 as 8 big-endian bytes is `00 00 01 8b cf e5 68 00`. The Rust tests also pin the resulting SHA-256 values.

**Unfixed findings**

6. **Android trusts registry reads without certificate or proof checks → real, not fixed here.** `NetOracle::get_record` and `get_key`, and the `ca_root` fallback when nothing is pinned, never check `certificate`, `mmr_proof` or `smt_proof`. The risk is somewhat smaller than reported: ic-agent 0.45 checks query-response signatures by default, which covers a hostile boundary node on mainnet, but a malicious replica is not covered. ARCHITECTURE §6.5 and §7.3 call for certified reads.
   - A fix means moving `leaf_data`, `record_root` and `key_root` out of `crates/witness-web/src/proof.rs` into a shared crate such as `witness-core::merkle`. The certificate check itself is `Agent::verify` and `lookup_value` in ic-agent.
   - Tests would need a BLS test root key. That is a cross-lane contract change, so it is deferred to the android lane under BUILD rule 8.

7. **Previews upload to the last media shard, not the record's → real, latent.** The registry sets `media_shard = shards.media.last()` when it anchors a record (`submit.rs` `apply`). `NativeMedia.publish` uploads to `media.lastOrNull()`. These diverge only once a second media shard is added between anchoring and publishing, which can't happen with today's single shard.
   - Fixing it means adding `media_shard` to `Anchored` and/or `CertifiedRecord`, the `.did` files, the UniFFI layer and Kotlin. That is a contract change, so it is deferred.

8. **Plus case limit exceeds the canister cap → real, deferred.** `limitFor` returns `PLUS_QUOTA_BYTES`, which is 25 GiB. The media canister's `MAX_CASE_BYTES` and the native and web 16-chunk caps both stop at 16 MiB, so a Plus case over 16 MiB fails at upload instead of returning `TooLarge`.
   - The existing tests `ShareCaseTest.freeTierRejectsOversizedCases` (Plus shares an 18 MB case) and `limitFor(plus) == 1L shl 34` assert larger Plus cases on purpose.
   - Capping the limit would break those tests, which encode the intended behaviour. Raising the canister cap for Plus is a product and canister decision. The decision should be raised with whoever owns Plus sizing.

**Commands verified**
- `cargo test -p witness-verdict -p witness-format -p witness-attest -p witness-web -p witness-android`: all passed.
- `cargo test -p witness-core`: all passed, golden tests included.
- `cargo test -p witness-registry`: 120 unit and 7 PocketIC tests passed. I ran it after another agent's mutation run on `outbox.rs` had finished and the file was back to normal.
- `cargo clippy` with `--all-targets` on verdict, registry, android and web: no warnings.
- `client/ ./gradlew :core:domain:testAndroidHostTest :core:platform:testAndroidHostTest`: domain 186 tests and platform 126 tests, 0 failures. That includes `AnchorOutboxTest.derivativeWaitsWhileItsParentIsStillQueued`, `parentUnknownFailsWhenTheParentIsNotQueued` and `PreimagesTest.preimagesMatchTheRustVectors`.

I did not mutation-test the fixes. The claims that each test fails without its fix come from reading the code.

Relevant files:
- `/Users/andrejones/Desktop/workspace/projects/witness/canisters/registry/src/submit.rs`
- `/Users/andrejones/Desktop/workspace/projects/witness/client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/AnchorOutbox.kt`
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-verdict/src/oracle.rs`
- `/Users/andrejones/Desktop/workspace/projects/witness/crates/witness-android/src/oracle.rs`
- `/Users/andrejones/Desktop/workspace/projects/witness/client/native/witness/src/main/kotlin/app/witness/nativelib/NativeNetwork.kt`
- `/Users/andrejones/Desktop/workspace/projects/witness/client/core/domain/src/commonMain/kotlin/app/witness/core/domain/usecase/ShareCase.kt`

# ---- report ----
I'll wait for the wasm gate to finish.