

# ---- module report ----
Done. Every module from ARCHITECTURE §2 (rev 3) builds and passes its tests: `./gradlew build -x lint` succeeds. The web bundles build too: `:webApp:wasmJsBrowserDistribution`, `:webApp:jsBrowserDistribution` and `:webApp:composeCompatibilityBrowserDistribution`. I could not run the app on `emulator-5554`, because the install failed with "not enough space" on the emulator.

**Modules touched** (everything is under `/Users/andrejones/Desktop/workspace/projects/witness/client`):
- **Root:** `settings.gradle.kts`, `build.gradle.kts`, `gradle.properties`, `gradle/libs.versions.toml`, `gradle/gradle-daemon-jvm.properties`, the wrapper (Gradle 9.9.1, copied from wingman-c2), `local.properties` (git-ignored) and `README-build.md`.
- **Convention plugins** in `build-logic/convention`: `witness.kmp.library`, `witness.kmp.android`, `witness.cmp`, `witness.android.app`, `witness.web.app`, `witness.metro`, `witness.room`. Shared helpers are in `Witness.kt`.
- **Modules:**
  - `native/witness`: plain Android library, with keep rules in `consumer-rules.pro`.
  - `core/{model,designsystem,ui,domain,verify,icp,telemetry}`: Android + js + wasmJs.
  - `core/{data,platform,a2ui}`: Android only.
  - `feature/{home,capture,post,case,share,settings}`: Android only.
  - `feature/{check,proof}`: Android + web.
  - `androidApp`, `androidBenchmark`, `webApp`.

**Versions, checked against Maven Central and Google Maven:** AGP 9.5.1, Kotlin 2.4.21, KSP 2.3.14, Compose Multiplatform 1.12.2 (material3 1.12.0-alpha03), JetBrains Navigation 3 1.2.0 and lifecycle 2.11.0, Metro 1.6.0, Room 3 3.0.3 with sqlite-bundled 2.7.1, DataStore 1.3.0, coroutines 1.11.0, serialization 1.13.0, CameraX 1.7.0, WorkManager 2.12.0, ML Kit (face 16.1.7, text 16.0.1, genai-prompt 1.0.0-beta5), A2UI 1.0.0-alpha02, RevenueCat purchases-kmp 4.2.0, Firebase BOM 35.2.0 (google-services 4.6.0, Crashlytics plugin 3.0.8), baselineprofile and benchmark 1.5.0, turbine 1.2.1, robolectric 4.18.1.

**Public APIs (placeholders for the module owners to replace):**
- `Verdict` (5 outcomes, `isTransient`)
- `Clock` and `FixedClock`
- `WitnessColors` (colour tokens from `wlib.py`)
- `Verdict.title()` (wording from the spec)
- `VerifyEngine` and `FakeVerifyEngine`
- `IcpEndpoint.LocalReplica` and `IcpEndpoint.Mainnet`
- `Telemetry` (with `Telemetry.None`) and `ConsentGatedTelemetry`, which drops everything until the user opts in
- Room 3 `WitnessDatabase` with a `key_counter` entity and DAO
- `CapturePermissions`
- `WitnessCatalog.isMappedEvent` (only `capture_for`, `edit`, `remove`, `flag`)
- `<Feature>Screen()` and `<Feature>Feature.ROUTE` for each feature
- `NativeLibrary`
- `AppGraph`: a Metro graph that provides `Telemetry.None`
- `WitnessApp()`: a Navigation 3 `NavDisplay` showing `HomeScreen`
- `WebRoute.parse()`: `/p/{id}`, `/v/{id}`, `/check`, otherwise unknown

**Tests:** 88, all passing.
- 83 run in the main build: Android host tests, plus js and wasmJs tests in headless Chrome for every web module.
- 5 cover the build logic itself (namespaces and module rules); run them with `./gradlew -p build-logic test`.
- The module rules are enforced: a `:feature:check` → `:feature:proof` dependency fails configuration. I tried it and reverted it.

**Deviations from the brief:**
1. **Gradle runs on JDK 21.** Metro's 1.6.0 Gradle plugin needs Java 21 to run. The daemon (set in `gradle-daemon-jvm.properties`, with the foojay resolver in settings) and `build-logic` use JDK 21. All app code still compiles to JVM 17.
2. **compileSdk is 37.1, not 37.** The A2UI compose AARs require 37.1, so the catalog has `android-compileSdkMinor = "1"`. AGP downloaded the android-37.1 platform automatically.
3. **No `FAIL_ON_PROJECT_REPOS` in settings.** Kotlin/JS and Wasm add their own download repositories for Node, Yarn and Binaryen, and that mode blocks them.
4. **One Compose Multiplatform check is turned off.** `checkComposeUiTestConfigurationFor*` demands a web executable in every Compose library; the browser tests run fine without one.
5. **The Kotlin compiler daemon has an 8 GB heap.** At 3 GB the first cold build spent about an hour in garbage collection and then ran out of memory.
6. **Two module namespaces avoid Java keywords:** `:feature:case` uses `app.witness.feature.cases`, and `:native:witness` uses `app.witness.nativelib`.
7. **Other small changes:**
   - Release builds of `androidApp` are minified, with `-dontwarn com.google.j2objc.annotations.**` in `androidApp/proguard-rules.pro` for Guava pulled in by ML Kit/CameraX.
   - `-Xexpect-actual-classes` is on for KMP modules, for Room's `@ConstructedBy` pattern.
   - Both web targets output a bundle named `witness.js`, which `index.html` loads.
   - RevenueCat, Robolectric and JNA are in the catalog but no module uses them yet.

**Open issues:**
- The emulator needs free space before `:androidApp:installDebug` will work. I didn't uninstall anything to make room.
- `androidBenchmark` (startup benchmark and baseline-profile generator) compiles, but its tests only run on a connected device, so it has none in the main build.
- Firebase: the `core/telemetry` module already depends on Analytics and Crashlytics, and `androidApp` applies the Firebase plugins only when `androidApp/google-services.json` exists. I did not create a Firebase project; the Firebase-backed `Telemetry` for Android is for the telemetry module's owner to write.
- The first cold build is slow because Kotlin/JS and Wasm link a full test bundle for each web module. Later builds take about 35–50 seconds.

# ---- module report ----
`:core:a2ui` is built: it compiles for Android and all 167 of its unit tests pass. The catalog components have not been rendered on screen yet, because the shared test setup can't run Compose UI tests (see open issues).

**Module touched:** only `client/core/a2ui`. I also put a throwaway template generator script in the scratchpad.

**Public API**

*Pure logic (`commonMain`)*
- `WitnessCatalog`: holds the catalog id `https://witness.app/a2ui/catalog/v1.json`, the component names, and the event allowlist (`capture_for`, `edit`, `remove`, `flag`).
- `TemplateCodec.decode(json): Template`, which throws `TemplateException`. It rejects a template unless all of these hold:
  - every message is A2UI 0.9.1 with exactly one envelope;
  - the first message is `createSurface` on the Witness catalog, and all messages use the same `surfaceId`;
  - component types are in the catalog, ids are unique, a `root` exists and every child reference resolves;
  - every event is on the allowlist and names a target, and there are no `functionCall` actions or `deleteSurface` messages;
  - every `/slots/x` binding points at a declared slot, and slot ids and choices are valid identifiers.
- `Template`: id, title, subtitle, keywords, surfaceId, slots, messages, a slot lookup by id, and `schema` (the domain `TemplateSchema`).
- `SlotSchema` / `SlotKind`: TEXT, PLATE, CHOICE, FLAG, plus a hint field and `toSpec()` to convert to the domain `SlotSpec`.
- `SlotBinder.bind(template, suggestion, fallback?) → Binding(values, dropped, missing, message)`:
  - It validates one pass over the suggestion against an index array, so cost grows linearly with the number of slots.
  - Output follows the template's slot order, so the same input always gives the same message.
  - Dropped slots carry one of five reasons: `WRONG_TEMPLATE`, `UNKNOWN_ID`, `DUPLICATE`, `MISSING_VALUE`, `INVALID_VALUE`.
  - The fallback (rules path) only fills slots that are still empty.
  - `message` is a single `updateDataModel` at `/slots`.
  - Helpers: `cleanText`, `normalizePlate`, `normalizeChoice`, `sanitizePlateInput`.
- `TemplateStore`: implements the domain `TemplateCatalog` port. It has `get`, `bind`, `decode`, and `bundled(read = Res.readBytes)`.
- `WitnessEventSink(onEvent)` / `WitnessEvent`: four event types (`CaptureFor`, `Edit`, `Remove`, `Flag`). An event is dropped if its name is not on the allowlist or its target is not a valid identifier.
- `A2uiMessages`: builders for `updateDataModel`, `deleteSurface` and `slotPath`.
- `ui/*`: plain Compose visuals that follow the C5 layout, plus `ImpactZones` for the impact diagram.

*Android (`androidMain`)*
- `WitnessCatalogV1.catalog`: built on the `androidx.a2ui` `A2uiCatalog`, with 8 components: `Column`, `GeneratedSurface`, `EvidenceChecklist`, `PlateField`, `ImpactDiagram`, `ChoiceRow`, `SummaryDraft`, `FlagButton`. The plate field and choice row write user edits back to the data model.
- `A2uiBridge.toLibrary(JsonObject)`: converts our JSON messages to the library's message types.
- `WitnessActionInterceptor`: sends event actions to the sink and lets nothing through to the engine.
- `WitnessReport(sink)`:
  - wraps the library's message processor;
  - `run()` processes messages and `errors` is a flow of engine errors;
  - `show(template, binding)` sends `deleteSurface` first, so switching templates works;
  - `send` passes a single message, and `slots(surfaceId)` reads back what the user edited.
- `@Composable WitnessReportSurface(report, surfaceId)`.

**Templates:** stored as JSON in `src/commonMain/composeResources/files/templates/`. There are four: `car_accident`, `property_damage` and `rental_condition` (wording copied from the C5 and C6 designs), plus `general`. `general` is the "Other" option in C6, and it is also what the domain's `TemplateMatcher` returns when nothing matches, so binding would fail without it.

**Tests (167, all passing):**

| Class | Tests |
|---|---|
| TemplateCodecTest | 57 |
| SlotBinderTest | 34 |
| A2uiBridgeTest | 14 |
| TemplateStoreTest | 12 |
| WitnessEventSinkTest | 12 |
| WitnessReportTest | 9 |
| BundledTemplatesTest | 7 |
| WitnessCatalogV1Test | 7 |
| ImpactZonesTest | 6 |
| A2uiMessagesTest | 5 |
| WitnessCatalogTest | 4 |

`WitnessReportTest` runs every bundled template through the real engine and checks it raises no errors. It also checks that slot values land in the surface's data model, that edits and removals apply, that rebinding and template switching work, and that events reach the sink.

**Deviations**
- **Extra `Column` component.** A surface needs a root container, and the library's basic catalog only ships interfaces with no implementations.
- **Plate slot type.** Templates have a `plate` slot type. It maps to TEXT with a maximum length of 10 in the domain schema.
- **Template keywords.** I added keywords to the template files so `TemplateMatcher` can pick a template.
- **Suggestion types.** I use `Suggestion`, `Slot` and `TemplateId` from `core/model` (another agent added them while I worked) and removed my own copies.
- **Message conversion.** Our JSON is converted straight into the library's message objects. The library's own parser relies on an Android JSON class that doesn't work in host tests.
- **Templates read resources through an injectable reader.** Tests read the JSON files from disk instead, because the Compose resource reader fails in host tests.
- **Local styling.** Icons are drawn on a canvas, and three colors (two surface shades and secondary text) are defined in the module because `core/designsystem` doesn't have them yet.

**Open issues**
1. Compose UI tests can't run because the convention plugin's host-test setup doesn't set `isReturnDefaultValues` or `isIncludeAndroidResources`, and the module can't override it. Asking the build-logic owner to set both would also let the bundled-resource test use `Res` directly.
2. `core/domain` and `core/platform` each have their own `SuggestionValidator`, and both overlap with `SlotBinder`. There should be one; I'd suggest `SlotBinder`, since it is the one that builds the data-model message.
3. Template packs fetched from `pages` and checked against a signature are not built.
4. Once `core/designsystem` lands, the module's colors, icons and fonts should switch to it.
5. Wiring into the screens is not done: the Case feature needs to run `WitnessReport.run()` from its ViewModel, map `WitnessEvent`s to its use cases, and send `errors` to Telemetry.

Templates are in `/Users/andrejones/Desktop/workspace/projects/witness/client/core/a2ui/src/commonMain/composeResources/files/templates/`; the template generator is `/private/tmp/claude-501/-Users-andrejones-Desktop-workspace-projects-wingman-c2/0d16e041-94d5-4236-8582-2263280ea996/scratchpad/gen_templates.py`.

# ---- module report ----
I built core/icp, core/verify and the `witness-bridges` npm package. All of them compile for android, js and wasmJs, and every test passes. Until the Rust bindings land, Android runs on in-memory fakes and the web check runs on a fallback stub that always returns No record.

**Tests (all passing)**

| Module | Android host | js (Chrome headless) | wasmJs |
|---|---|---|---|
| core/icp | 122 | 124 | 124 |
| core/verify | 44 | 48 | 48 |

The npm package has 32 more tests, run with `node --test`. To run the module tests without failing the yarn-lock check, I used `-x kotlinStoreYarnLock -x kotlinWasmStoreYarnLock` (open issue 2 explains why). When the whole project is configured, that check passes.

**Files touched**
- `client/core/icp/**` (build file and all sources)
- `client/core/verify/**` (build file and all sources; I replaced the old `VerifyEngine.kt` and its test)
- `client/webApp/bridges/**` (new package)
- Shared files: I added `kotlinx-browser = "0.5.0"`, the latest on Maven Central, to `gradle/libs.versions.toml`. Both `client/kotlin-js-store` yarn locks were regenerated.

**What core/icp provides**
- **IDs:** `CrockfordBase32`, `RecordId`, `CaseId` (with `shard`) and `CaptureId`. They use the same test vectors as `crates/witness-format` (for example `WERC8GMRZGE196QV`).
- **Registry:** a `RegistryGateway` port with `get`, `byCapture` and `shards`. `ShardMap` finds the right index shard by binary search.
- **Index search:** `ProbeQuery`, `IndexGateway` and `IndexSearch` ports.
  - `FanOutIndexSearch` queries shards in parallel with coroutines; a shard that fails is marked failed rather than failing the search.
  - `ProbeWire` packs probe results into one int array, so the JS-to-Kotlin hop is a single copy. Kotlin and the bridge tests share one vector.
  - `CandidateMerge` merges and dedupes hits in O(n log n) using sorts on packed longs. It follows the Rust crate's caps (8 / 32 / 32 / 64) and ordering, and is checked against a reference implementation.
- **Cases:**
  - `CaseAad` builds the AAD exactly as ARCHITECTURE §9 specifies.
  - `CaseChunks` keeps each chunk at most 1 MiB on the wire and each case at most 16 MiB.
  - `CaseLink` builds and parses the `#k=` link and never prints the key.
  - `CaseSealer` encrypts; `CaseOpener` fetches and decrypts chunks in parallel and checks the bundle hash. It returns `Opened`, `NotFound`, `Expired` or `Tampered`.
  - `CaseCrypto`, `CaseStore` and `CaseUploader` are the ports.
- **Factory:** `createIcpServices(endpoint)`.
  - Android: in-memory registry, index and case store, with real AES-GCM.
  - Web: calls into `icp.mjs` and `crypto.mjs`, with no uploader.
- AES-GCM gives byte-identical output to the standard test vector on Android, in WebCrypto and in Node.

**What core/verify provides**
- **Engine:** a `VerifyEngine` port. Its `check(bytes)` returns a stream of stage updates, then the final result; `judge(bytes)` returns just the result.
- **Check screen:** stages match the Checking screen. `CheckSteps` folds the updates into the five rows, including the "N shards · M answered" count.
- **Wire codes:** `Judgement` carries the verdict, record ID, flags and a fallback marker. Verdict codes and flag bits match the Rust crate.
- **Image info:** `ImageSniffer` reads format and size from JPEG (including EXIF rotation), PNG, GIF and WebP headers. Other formats fall back to the platform decoder: BitmapFactory on Android, `createImageBitmap` on the web.
- **Factory:** `createVerifyEngine(VerifyConfig(engineUrl))`. Android returns `FakeVerifyEngine`; the web calls into `verify.mjs`.

**The npm package (`witness-bridges`)**
- It pins `@icp-sdk/core` 6.1.1 and `onnxruntime-web` 1.31.1.
- **`icp.mjs`:** anonymous agent; checks the IC certificate on registry replies; queries all index shards in parallel with `Promise.all` and reports progress per shard; fetches case metadata and chunks.
- **`idl.mjs`:** Candid definitions for registry, index and media, written from the §7 sketches.
- **`crypto.mjs`:** AES-GCM via WebCrypto.
- **`image.mjs`:** `createImageBitmap` size fallback.
- **`verify.mjs`:** loads the witness-web module from a URL passed in. With no URL, or if the module fails to load or has no `judge`, it uses the stub, which returns No record and sets `fallback: true`.

**Deviations**
- **No shared ports yet.** core/domain only had `Clock`, so the ports live in my own modules. Feature modules aren't allowed to depend on these modules, so the domain owner should lift them into core/domain. The ID types probably belong in core/model.
- **core/verify depends on core/icp**, for `RecordId`.
- **ES modules on the js target.** My two modules call `js { useEsModules() }`, because external JS modules shared between js and wasmJs won't compile otherwise. I checked that `:webApp` still bundles for both js and wasmJs with these modules wired in, then removed that temporary wiring.
- **Candid types are my own.** `ShardInfo`, `ProbeResult`/`PdqHit` and `CaseMeta` don't exist in the canisters yet.
- **Chunk format:** each chunk is the nonce followed by ciphertext and tag. The case metadata has to carry `bundle_hash`, since it's part of the AAD.
- **Partial certificate check.** The JS side checks the IC certificate, but not the MMR proof or that the shard map matches the certified data. That needs witness-web.

**Open issues**
1. The canisters need to implement the Candid in `client/webApp/bridges/idl.mjs`. If they choose different types, update that file and the probe packing in `wire.mjs`.
2. The yarn-lock check fails whenever only some modules are built. Someone with access to the root build file should set the lock mismatch report to a warning, or always run the full build.
3. When `:native:witness` lands, swap the Android fakes for real implementations in `IcpServices.android.kt` and `Platform.android.kt`.
4. `verify.mjs` expects the witness-web module to export `judge(bytes, onProgress)` returning `{verdict, recordId, flags}`. Calling the canisters from that judge isn't wired yet.
5. To run the bridge tests: `cd client/webApp/bridges && npm install && npm test`.

Code is in `client/core/icp`, `client/core/verify` and `client/webApp/bridges`, all under `/Users/andrejones/Desktop/workspace/projects/witness`.

# ---- module report ----
Both modules are done. `core/model` and `core/domain` compile for android, js and wasmJs, and every test passes on all three targets. One caveat: two shared Gradle tasks were skipped in those test runs (open issue 1).

**Modules touched:** `client/core/model` and `client/core/domain`, plus their tests. No build files or other modules were edited. The existing `Verdict` enum is unchanged, because `core/ui` and `core/verify` use it; both still compile.

**Tests:** run as `./gradlew :core:model:allTests :core:domain:allTests -x kotlinWasmStoreYarnLock -x kotlinStoreYarnLock`.

| Module | Tests per target | Targets | Failures |
|---|---|---|---|
| `core/model` | 75 | android host, js browser, wasmJs browser | 0 |
| `core/domain` | 154 | android host, js browser, wasmJs browser | 0 |

**`core/model`** (`app.witness.core.model`):
- `Crockford`: base32 encode/decode that matches Rust, checked against the Rust golden vectors. It folds confusable letters, tolerates hyphens and rejects bad padding.
- IDs: `CaptureId` (61 bits), and `RecordId`, `KeyId`, `CaseId` stored as canonical base32 strings. `RecordId.grouped()` gives `WERC-8GMR-ZGE1-96QV`. `LocalCaseId` is the on-device case ID.
- `ByteString`: an immutable byte array that compares by content.
- `EnumBits<E>`: a typed bitset. `VerdictFlags`, `VerdictReasons`, `LocationReasons` and `GnssChecks` use the same bit positions as Rust.
- Edits: `Edit` (8 kinds with validation), `EditClass`, `Rect`, `isPresentationOnly()`, `hasLocal()`.
- Claims and evidence: `TimeEvidence`, `LocationEvidence`, `LocationOpening`, `Checks`, `EvidenceState`/`CheckStrength`, `ClaimDraft.finalize()`, `WitnessClaim`, `AnchoredRecord`, `Anchor`.
- Keys: `KeyHistory` (binary-search `statusAt`, `revokedAfter`), `KeyInfo`, `DeviceKey`.
- App records: `Capture`, `AnchoringStatus`, `Submission`, `OutboxEntry`/`OutboxState`, `Case`, `CaseShare`, `ShareExpiry`, `CaseSummary`, `PostDraft`, `Post`, `LocationPrecision`, `BlurProposal`.
- Checks: `Region` (Q16 coordinates, same as Rust), `Reference`, `CheckResult`, `CheckStage`/`StageStatus`, `Platform` (top-k is 8 on Android, 3 on web).
- A2UI and account: `TemplateId`, `SlotSpec`, `TemplateSchema`, `Slot`, `Suggestion`, `Entitlement`, `Settings`.

**`core/domain`:**
- **Ports** (`domain.port`): every port in the brief, plus `CounterStore`, `BlurDetector`, `TemplateCatalog`, `CaseBundleCodec`, `RandomSource`, `Links` and `OfflineException`.
  - `KeyVault` covers attestation through `attestation(keyId)`.
  - `VerifyEngine` takes a `VerifyOracle` and a `StageSink` for staged progress.
  - `Clock` stays in the root package.
- **Use cases** (`domain.usecase`):
  - `SealSigner`: one lock around counter reservation and signing, shared by every path that signs.
  - `SealCapture`, `AnchorOutbox`, and `GatewayOracle` (the bridge from the gateways to `VerifyEngine`).
  - `CheckImage`, which returns `Flow<CheckProgress>`.
  - `BuildPost` (with `Geohash` and `PostText`), `ManageCases` (with `TemplateMatcher`, `SuggestionValidator`, `Tokens`), `ShareCase`, `EraseMine`, `Enrollment`, `KeyRotation`.
  - `Search` / `SearchIndex`: a sorted prefix index intersected with bitsets.
  - `TelemetryGate` with `Redactor`, `ProofFacts`, and `ObserveAnchoring` for the anchoring-status screen.
- **Fakes** (`domain.fake`, in commonMain):
  - In-memory stores for captures, outbox, cases, settings and counters.
  - `FakeKeyVault`, `FakeSealEngine`, `FakeRegistry` (handles duplicates, `key_expired`, scripted results and offline), `FakeIndex`, `FakeMedia`, `FakeVerifyEngine`.
  - `FakeCrypto`, built on a pure-Kotlin `SoftwareSha256` that passes the standard test vectors. Its "AES-GCM" is a hash-based stand-in, so it only works with itself.
  - `FakeTelemetry`, `FakeAssistant`, `FakeTemplateCatalog` (4 bundled templates), plus clock, random, scheduler and codec fakes.

**Deviations:**
- **Telemetry port lives in `core/domain`.** Domain may only depend on `core/model`, so the existing interface in `core/telemetry` can't be used. The domain version takes typed `AnalyticsEvent`s with numeric metrics only. `core/telemetry` should implement it; that module has no dependency restriction.
- **Two `VerifyEngine` interfaces exist.** The one in `core/verify` (`judge(ByteArray): Verdict`) differs from the domain port, which takes an oracle and reports stages. `core/verify` should implement the domain port.
- **Fakes ship in commonMain.** A separate fakes module would have needed a `settings.gradle.kts` change, which isn't mine to make.
- **Client times are in milliseconds.** The registry uses nanoseconds, so the gateway implementations convert.
- **Errors sent to telemetry lose their stack traces.** `TelemetryGate.nonFatal` forwards a `RedactedError` with only the type and a scrubbed message. The Android implementation could copy the stack trace back on JVM.

**Open issues:**
1. **The shared yarn lock fails a check.** `client/kotlin-js-store/yarn.lock` was changed outside my modules, probably by someone adding npm dependencies. The lock-check tasks (`kotlinWasmStoreYarnLock`, `kotlinStoreYarnLock`) fail until someone runs `kotlinUpgradeYarnLock kotlinWasmUpgradeYarnLock`, which is why I skipped them.
2. **Case-link recipients need the bundle hash.** The encryption binding (AAD) includes SHA-256 of the whole bundle, which a recipient can't compute before decrypting. I pass it in `UploadRequest.bundleHash`, so the media service and the recipient must expose it. ARCHITECTURE §9 should say where it travels.
3. **The Rust oracle is synchronous; the domain one is not.** `VerifyOracle` uses suspend functions, so the native bridge has to prefetch or block.
4. **Recent checks aren't stored.** No check-history store exists yet for the recent-checks list on screen D5.
5. **Rejected keys can leave captures stuck.** A rejected late key is deleted, so any captures already sealed with it stay waiting for enrollment.

Firebase project creation wasn't in scope here. The domain telemetry port is off until the user opts in, so it is ready for the Firebase implementation.

Main files are in `/Users/andrejones/Desktop/workspace/projects/witness/client/core/domain/src/commonMain/kotlin/app/witness/core/domain/`:
- `port/`
- `usecase/`
- `fake/`

# ---- module report ----
Both modules build for Android, JS and wasmJs, and all their tests pass on every target. No code comments, and nothing outside my two modules changed except two lines in the version catalog (see the end).

**Test counts (all passing)**

| Module | Android host | JS browser | wasmJs browser |
|---|---|---|---|
| `:core:designsystem` | 80 | 74 | 90 |
| `:core:ui` | 44 | 44 | 60 |

**Modules touched**
- `client/core/designsystem`: build file, 20 main sources, Inter and JetBrains Mono under `src/commonMain/composeResources/font/`, plus tests.
- `client/core/ui`: build file, 11 main sources, plus tests.

**Public API, `app.witness.core.designsystem`**
- **Tokens:**
  - `WitnessColors` has every `wlib.py` color, including the dark camera ones. The old names `Paper`, `Ink`, `Primary` and `Crit` are kept, so the feature screens still compile.
  - `WitnessPalette.Light` and `WitnessPalette.Camera`, with `Tone`, `tone()`, `toneText()` (green and amber get their darker text colors) and `onInk`.
  - `WitnessState`: 10 states, `unanchored` included, each with label, short label, tone and icon.
  - `WitnessType` (full type scale), `WitnessSpacing` (8-pt grid, 16 margins) and `WitnessShapes`.
  - `WitnessTheme` and `WitnessCameraTheme`, which also map the palette onto the Material 3 color scheme; `Witness.colors` and `Witness.type` read them.
- **Icons:** `WitnessIcon` has all 141 Material Symbols from `design/icons`, turned into Kotlin path data and built into vectors lazily (each built once). Draw one with `WIcon(...)`.
- **Components:**
  - Buttons: `WitnessButton` (filled, ink, tonal, outlined, glass and danger), `WitnessIconButton`, `GlassIconButton`, `TextAction`.
  - Chips: `WitnessChip`, `ToneChip`, `StateChip`, `FilterChip`, `GlassPill`.
  - Controls: `WitnessToggle`, `SegmentedControl`, `GlassSegmentedControl`, `LinearMeter`.
  - Surfaces: `WitnessCard`, `TonePanel`, `DropZone`, `Modifier.dashedBorder`, `Overline`, `SectionTitle`, `Hairline`, `IconDisc`, `ActionRow`, `StatusLine`.
  - Navigation: `WitnessAppBar`, and `WitnessBottomNav` with the raised Capture button.
  - Overlays: `WitnessSheet`, `SheetSurface`, `SheetHandle`, `SheetHeader`, `Scrim`, `WitnessDialog`, `DialogCard`, `BottomBar`.
  - Fields: `WitnessTextField`.
  - Brand: `WitnessMark` (drawn on a Canvas like `mark()`, via `MarkGeometry`), `Wordmark`, `AppIcon`, `SealBadge`.
  - Generated content: `GeneratedSurface`, `GeneratedTag`, `AiHeader`, `FlagButton`.
  - QR: `QrCode(data)`.
- **QR encoder (`.qr`):** `QrEncoder.encode()` and `encodeBytes()` support numeric, alphanumeric and UTF-8 byte modes, all four error-correction levels, versions 1–40 and automatic or forced masks. `QrMatrix` holds the result, and the renderer draws each row's dark runs as a single cached path.
  - **QR check:** the Android host tests compare it with the zxing library. Ten samples × 4 levels × 8 masks matched module for module. zxing also decodes every version, full-capacity symbols and UTF-8. Common tests cover published reference values for format bits, version bits, alignment positions, capacities and the error-correction codewords of the standard "HELLO WORLD" example.

**Public API, `app.witness.core.ui`**
- **Verdicts:** `Verdict.title()`, `headline()`, `qualifier()`, `state()`, `summary()` and `colors()`; `VerdictCard` covers all 5 outcomes, and `VerdictHeadline` is the proof-page title.
- **Proof:** `ProofFacts` (inline or stacked rows, pass/fail status, badges); `ChainOfCustody` with `CustodyStep` and `Strength` (Strong, Medium, Weak); `ProofHeader`; `ProvesCard` with `Proves.Default` and `Proves.CaseLink`.
- **Compare:** `ReferenceCompare` with `ComparePane` and `NormRect` (dashed crop rectangle with its "Crop" chip, red changed-region boxes); `mapToCropped` positions those overlays on a cropped image.
- **Check:** `CheckProgress` with `CheckStage`, `StageStatus` and `overallProgress()`.
- **Rows:** `CaseItemRow`, `PostCard`, `CaseRow`, `CaseCard`, `CaptureTile`, `CaseMosaic`, `stateCounts()`.
- **Location:** `LocationPill` and `LocationConfidencePill` (High, Medium, Low).
- **Web:** `WebHeader`, `WebFooter` with `WebLink`, and `BrowserSafeLayout` with `WebLayout.spec()`. Phones get one column; from 840 dp wide it switches to two columns, which gives the 64/600/48/504 dp split of the 1280 desktop design.
- **Formatters:** thousands separators, plurals, " · " joins, check counts, anchoring summary, verified summary, declared edits, byte sizes, dimensions, accuracy, shortened hashes.

**Deviations**
- `core/model` only has `Verdict`, so `core/ui` defines its own small input types (`ProofFact`, `CustodyStep`, `CheckStage`, `ComparePane` and others). Feature screens map their data onto these. Images are passed in as composable slots.
- Compose UI tests run on wasmJs only (34 of them, using the current `v2.runComposeUiTest`). In the plain-JS test runner the Skia graphics library isn't loaded, so they can't render there.
- The camera theme's surface and line colors come from the camera screens' hard-coded colors; `wlib.py` has no dark tokens.
- I checked the look of the main components against the renders by rendering them in a temporary test and viewing the images; that test has been removed. Nothing checks the visuals automatically.

**Open issues**
- I added two entries to `client/gradle/libs.versions.toml`, the version catalog every module shares: `zxing` 3.5.4 (version taken from Maven Central's metadata) and `compose-ui-test` (on the Compose Multiplatform version).
- At one point the shared root task `kotlinWasmStoreYarnLock` failed with "Lock file was changed". I didn't refresh the shared lock file. It passed again later, so another workstream probably refreshed it.
- `QrCode(data)` throws if the data is longer than a QR code can hold, up to 2953 bytes. Proof links are far below that.

# ---- module report ----
I built `core/data`, `core/platform` and `core/telemetry` against the ports that `core/domain` now has. All three modules compile for their targets, all their tests pass, and `:androidApp:assembleDebug` still builds. Nothing was run on the emulator or a real device, so CameraX, Keystore/StrongBox, raw GNSS, ML Kit and Gemini Nano are tested only through their pure logic, not end to end.

**Modules touched**: `client/core/telemetry`, `client/core/data`, `client/core/platform`. Outside those, I only added entries to `client/gradle/libs.versions.toml`: `androidx-test-core` 1.7.0, and `mlkit-genai-schema-compiler` 1.0.0-alpha2 (the latest on Google Maven). No other module was edited.

**Public APIs**
- **`core/telemetry`** (android, js, wasmJs):
  - `FirebaseTelemetry` implements the domain `Telemetry` port (Analytics plus Crashlytics, `setCollectionEnabled`).
  - `AndroidTelemetry.create(context)` returns a domain `TelemetryGate` around Firebase only when a Firebase app is configured, otherwise a no-op. The gate starts off and turns on only when the user opts in.
  - The module's Android manifest switches Analytics, Crashlytics and advertising-ID collection off by default.
  - `TelemetryRules` enforces Firebase name and parameter limits. The web uses the no-op.
  - I kept a small `app.witness.core.telemetry.Telemetry` interface with a `None` object, only because `AppGraph` still imports it.
- **`core/data`** (Android):
  - Room 3 database with all eight tables, DAOs, schema export and an empty migration list (`WitnessMigrations.ALL`).
  - Stores implementing the domain ports: captures (transaction-safe `replace` for resealing, recursive `derivativesOf`, encrypted `loc_opening`), outbox, cases, counter.
  - `DataStoreSettings` for the domain `SettingsStore`.
  - `AnchorWorker` and `WorkManagerAnchorScheduler`: unique work "anchor", APPEND_OR_REPLACE, expedited, network required, exponential backoff from 30 s.
  - Supporting pieces: `AesGcmBlobCipher` (AES-GCM key in Android Keystore), `ImageFiles`, `CheckHistoryStore`, a lazy `DataGraph` that wires everything, and binary codecs for edits, case shares and location openings.
- **`core/platform`** (Android):
  - `CameraXCaptureSource`: capped at 12 MP, `CAPTURE_MODE_MINIMIZE_LATENCY`, JPEG quality 100, in-memory `ImageProxy` bytes plus `rotationDegrees`.
  - `KeystoreKeyVault` (P-256):
    - tries StrongBox first and falls back to the TEE when StrongBox is missing or its signing P90 is over 120 ms;
    - keeps retired keys;
    - exports the attestation chain and challenge;
    - converts DER signatures to raw r‖s with low-S;
    - builds the offline challenge.
  - `AndroidLocationEvidenceSource`: GPS provider only, 10 s ring buffer of `GnssMeasurementsEvent`, mock flag, network country vs fix country, salted location commitment.
  - Blur suggestions: `MlKitBlurDetector` (face plus text, sorting text into plates and house numbers).
  - Assistants:
    - `GeminiNanoAssistant` uses typed output (`@Generable`) through KSP.
    - `RulesAssistant` always works without the model.
    - `FallbackAssistant` uses Nano only when it is available and the app is in the foreground, validates its answer and fills gaps from the rules.
  - `ShareIntents`, `ShareParser` and `ShareInbox` handle send, send-multiple and view intents, including proof links.

**Tests**: 210 test runs, 0 failures.

| Module | Test runs | What they cover |
|---|---|---|
| telemetry | 27 | common tests on android-host, js and wasmJs, plus Robolectric |
| data | 59 | codecs; Room stores; counter (including 64 concurrent reservations); settings; worker config; cipher; image files; an integration test that runs the domain seal and anchor use cases on Room stores against the fake registry |
| platform | 124 | DER conversion (including 200 real signatures cross-checked with the JDK); GNSS heuristics and confidence; assistants; blur; share; key records; pixel budget |

**Deviations**
- **Counter reservation** runs as INSERT OR IGNORE, then UPDATE, then SELECT inside one IMMEDIATE transaction instead of `UPDATE … RETURNING`. The framework SQLite driver (the one Robolectric runs) can't return rows from an UPDATE. The no-reuse guarantee is the same, and production still uses the bundled driver.
- **Telemetry gating** reuses the domain's `TelemetryGate` and `Redactor`. I deleted my own scrubbing and consent wrappers so the logic isn't duplicated.
- **Outbox writes** never use Room's upsert. Its conflict handling could silently drop a row that clashes on the unique (key, counter) index, so writes are update-or-insert instead, and a clash now fails loudly. A test covers this.
- **Case membership** is one case per capture, matching `Capture.caseId`. Case shares are stored as a blob column on the case row.

**Open issues for the integrator**
- `AppGraph` should provide `AndroidTelemetry.create(context)` and call `TelemetryGate.follow(settings, scope)`; once it does, the compatibility interface in `core.telemetry` can go.
- The Application class must implement `AnchorOutboxHost`, or `AnchorWorker` returns failure.
- Two contracts belong in `core/domain`: `CheckHistoryStore` and `ShareInbox`/`SharedItem`. Features can't reach them in `core/data` or `core/platform`.
- `DeviceKey.boot` and `deviceLocked` keep their defaults (verified, locked). The attestation RootOfTrust extension is not parsed on the device.
- The fix-country check covers 29 countries. For any other country it reports unknown rather than a mismatch.
- `MlKitBlurDetector` needs its image source wired to `RoomCaptureStore.readImage`. The `CaptureSource` needs binding to a Preview lifecycle in `feature/capture`.

The main files to look at are `core/data/src/commonMain/kotlin/app/witness/core/data/{WitnessDatabase.kt,Daos.kt,Entities.kt}`, `core/data/src/androidMain/kotlin/app/witness/core/data/{RoomStores.kt,AnchorWorker.kt,DataGraph.kt}`, `core/platform/src/androidMain/kotlin/app/witness/core/platform/keys/KeystoreKeyVault.kt` and `core/telemetry/src/androidMain/kotlin/app/witness/core/telemetry/FirebaseTelemetry.kt`, all under `/Users/andrejones/Desktop/workspace/projects/witness/client/`.

# ---- module report ----
I built all five settings screens (E1 Settings, E2 Witness Plus, E3 Internet Identity linking, E4 Delete account dialog, E5 Privacy & data) in `feature/settings`, each wired to a ViewModel. The module compiles, `:androidApp:assembleDebug` still builds, and all 158 tests pass.

To compare against the design I rendered each screen to an image in a Robolectric test. All five match the PNGs closely in layout, colours, sizes and copy. The output goes to `/Users/andrejones/Desktop/workspace/projects/witness/client/feature/settings/build/snapshots/e1.png` … `e5.png`.

**Modules touched:** only `:feature:settings`. Its build file now adds the kotlinx serialization plugin, `navigation3-ui` (as api), RevenueCat purchases-kmp on Android (4.2.0, checked as the latest on Maven Central), and Robolectric for host tests. I didn't edit the version catalog or any other module.

**Public API** (package `app.witness.feature.settings`)
- **Navigation:** keys `SettingsKey.Root`, `Plus`, `LinkIdentity`, `PrivacyData`. The app registers them with `EntryProviderScope<NavKey>.settingsEntries(deps, navigator)`. The app implements `SettingsNavigator` (`navigate`, `back`, `openExternal`). `SettingsDependencies` holds the ports and builds the ViewModels.
- **ViewModels:** `SettingsViewModel`, `PlusViewModel`, `IdentityLinkViewModel`, `PrivacyDataViewModel`. Each exposes an immutable UiState as a StateFlow; Plus and Identity also emit one-off events (open a URL, go back, link identity). The stateless screens can be called directly, e.g. `SettingsScreen(state, SettingsActions)`.
- **New ports in this module, each with a fake in `.fake`:** `IdentityAccount`, `PlusBilling`, `DataExporter`, `TelemetryConsentStore`. Two use cases: `DeleteAccount` and `IdentityLinkFlow`.
- **Internet Identity deep link:** the app opens `https://witness.app/ii/link?v=1&nonce=<32 hex>&redirect=witness%3A%2F%2Fii%2Fcallback` in the browser. The callback is `witness://ii/callback?v=1&nonce=…&principal=…` (or `&error=cancelled`).
  - Callbacks are strictly checked: duplicate or unknown keys, encoded characters and URIs over 512 chars are rejected.
  - The nonce is compared in constant time, and each nonce works only once.
  - The principal must be a valid, self-authenticating Internet Identity principal (checksum verified).
  - A forged callback can't cancel a real pending sign-in.
- **`RevenueCatPlus`** (Android only) covers both billing and the existing `EntitlementSource` port. It uses entitlement id `"plus"` and logs in with the user's principal. If the app hasn't configured RevenueCat, it reports billing as unavailable.

**Tests:** 158 in total. Unit tests cover the principal format (known vectors, every length, tampered or malformed input), the deep-link contract, the link flow, plan pricing, telemetry consent, copy, account deletion and all four ViewModels. Five more render each screen.

**Deviations**
1. **The ports live in this module, not in `core/domain`**, because I was told to stay in my lane. They should probably move to `core/domain`.
2. **The two telemetry toggles share one stored flag.** `Settings` only has a single `telemetryOptIn`, so data is collected only when both Analytics and Crash reports are on. If only one is on, nothing is collected and the partial choice resets to off after a restart. Fully separate toggles need two fields in the model, the DataStore code and the telemetry gate.
3. **Delete account can't remove hosted media yet.** It erases public copies and unlinks the identity, but there is no media-delete call in the gateway. If erasing is incomplete, the identity link is kept so the user can retry from the web.
4. **Subscribing requires a linked Internet Identity**, per the spec. Without one, the button sends the user to E3.
5. **I added a "Diagnostics" section to E1** for the two telemetry toggles; the design doesn't have it.

**For the app shell**
- Configure RevenueCat with its API key (no keys are in code).
- Keep one `IdentityLinkFlow` instance and pass any `witness://ii/callback` intent to `flow.complete(uri)`.
- Pass `eraseMine::everything` as `eraseAll`.
- Provide real `IdentityAccount` and `DataExporter`. The registry has no `link_ii` call yet, so linking is currently fake.

**Open issue in someone else's module:** in the design system, `WIcon` draws with the default `paint()` scaling, which never enlarges an icon. Anything set above 24dp still renders at 24dp. I worked around it locally with `LargeIconDisc`; the fix belongs in `WitnessIcons.kt`.

Key files, in `/Users/andrejones/Desktop/workspace/projects/witness/client/feature/settings/`:
- `build.gradle.kts`
- `src/commonMain/kotlin/app/witness/feature/settings/SettingsNavigation.kt`
- `src/commonMain/kotlin/app/witness/feature/settings/IdentityLink.kt`
- `src/commonMain/kotlin/app/witness/feature/settings/Principal.kt`
- `src/commonMain/kotlin/app/witness/feature/settings/TelemetryConsent.kt`
- `src/androidMain/kotlin/app/witness/feature/settings/RevenueCatPlus.kt`
- `src/androidHostTest/kotlin/app/witness/feature/settings/SettingsSnapshotTest.kt`

# ---- module report ----
`:feature:home` is built. All nine screens (A1–A9) are in place, it compiles for Android (its only target), and all 147 tests pass with 0 failures. `:androidApp` still compiles. I rendered each screen to PNG with Robolectric and checked A2, A4, A6, A7, A8 and A9 against the design renders; A1, A3 and A5 were rendered but I didn't look at them.

**Modules touched:** only `client/feature/home`. I added the serialization plugin, `navigation3-ui`, `kotlinx-serialization-json` and `activity-compose` to it, plus Robolectric for its host tests. The two placeholder files are gone, and the welcome photo is bundled as a 214 KB resource.

**Public API**
- **Navigation keys:** `HomeKey` (Splash, Welcome, Permissions, DeviceSetup, Unsupported, Home, Anchoring, Search). They are `@Serializable` and implement `NavKey`.
- **App shell hook:** the shell calls `EntryProviderScope<NavKey>.homeEntries(deps, navigator)`. `deps` is a `HomeDependencies` holding the domain pieces (vault, stores, outbox, enrollment, key rotation, scheduler, thumbnails, clock). `navigator` is a `HomeNavigator`, a set of callbacks for moving within home and opening other features.
- **Thumbnails:** `ThumbnailSource`, with the Android version `CaptureThumbnails(captures)`. It decodes images at reduced size and caches them up to 24 MB.
- **ViewModels:** `SplashViewModel`, `PermissionsViewModel`, `DeviceSetupViewModel`, `HomeViewModel`, `AnchoringViewModel`, `SearchViewModel`. Each exposes its screen state as an immutable `StateFlow`.
- **Screens and previews:** every screen is a stateless composable with a `@Preview`.
- **Permissions:** `PermissionsRoute` (Android only) requests camera plus fine and coarse location. It detects when the camera has been permanently denied and then offers to open Android settings.
- **Snapshots:** `HomeSnapshotTest` writes `build/snapshots/a1.png`–`a9.png`.

**Tests (147):** date and time formatting, the shared capture/case snapshot, display helpers, search highlighting and snippets, thumbnail sizing, nav key serialization, and the ViewModels for splash, permissions, device setup, home, anchoring and search. The ViewModel tests run against the in-memory fakes in `core/domain`.

**Deviations**
- **A4 progress:** `Enrollment` only reports its final result, not each step. While it runs, the screen shows "Hardware key created" as in progress if no key exists yet, otherwise "Verified by Google" as in progress. The design shows "certificate issued" in progress, but the app can't actually know that. Offline, the user can open the camera straight away, matching the design's offline banner. If enrollment throws an unexpected error, the screen offers "Try again".
- **Missing data in the model:**
  - "Checked 1,284×" and "posted with proof link" are omitted because there's no check count or posted flag.
  - The splash screen decides between Home and Welcome by whether a key exists, because `Settings` has no "onboarding done" flag.
- **What "Posts" means:** the Home feed treats any capture not in a case as a post, and shows only the newest version when a photo has edited copies.
- **Queue reasons on A8** come from the outbox state, for example "Waiting for network" or "Registry busy · retrying soon".
- **Search filters:** filters apply after the domain search's 50-result cap, so a filtered list can come back shorter than 50.
- **Temporary `HomeScreen()`:** I kept a deprecated no-argument `HomeScreen()` because `androidApp/WitnessApp.kt` still calls it. Remove it once the shell registers `homeEntries`.

**Open issues for other owners**
- **Shell:** use `rememberNavBackStack(HomeKey.Splash)`, call `homeEntries(...)`, and add the Navigation 3 ViewModel scoping decorator so ViewModels are cleared per screen. `CaptureThumbnails` should be one shared instance for the whole app.
- **Domain/model:** I'd suggest an `Enrollment` progress flow, a check count on `Capture`, and an `onboardingComplete` flag in `Settings`.
- **Not done:** relative times like "2 min ago" only update when the data changes, not on a timer. No Firebase telemetry events are sent from this module.

# ---- module report ----
I built B1–B6 in `feature/capture` and `feature/post`. Both modules compile for Android (their only target) and all 208 tests pass: 81 in capture and 127 in post. I also rendered every screen with Robolectric and compared it to the matching PNG in `design/renders`, and they line up closely.

One blocker you'll hit: right now the full Gradle build fails to configure. The cause is another agent's in-progress `feature/case/build.gradle.kts`, which enables host tests a second time. Until that's fixed, my modules only build with `./gradlew --configure-on-demand :feature:capture:testAndroidHostTest :feature:post:testAndroidHostTest`.

**Modules touched:**
- `feature/capture` and `feature/post`, each with a build file, source code and tests.
- One shared edit: I added `androidx-camera-view` to `gradle/libs.versions.toml` (CameraX 1.7.0, confirmed on Google Maven). It's needed for the `PreviewView` inside Compose that you asked for.

**Public APIs for the app shell:**
- **Camera (B1, B2):** nav key `CameraKey`. The app registers `EntryProviderScope<NavKey>.captureEntries(newCameraViewModel, cameraUseCases, onClose, onCompose, onOpenItem)`.
  - Pass the `ImageCapture` from `CameraXCaptureSource` as `cameraUseCases`, so the preview and the shutter share one camera binding.
  - `CameraViewModel(seal, captures, settings, vault, location, online, detections, ticks, utcOffset, telemetry)`; use `ClockText.everySecond(clock)` for `ticks`.
  - The screen pieces (`CameraScreen`, `CameraXViewfinder`, `CameraActions`) are also public if the shell wants them directly.
- **Post (B3–B6):** nav keys `PostComposerKey` and `PostSharedKey`. The app registers `postEntries(newComposerViewModel, newSharedViewModel, onClose, onShare, onShared, onDone)`.
  - "Copy link" puts the link on the clipboard inside the module; `onShare(text, post)` hands the shell what it needs to open the system share sheet.
  - `ComposerViewModel` drives B3, B4 and B5; B4 (edit photo) and B5 (location sheet) are overlays inside the composer, not separate nav entries.
  - `PostSharedViewModel` drives B6; use `PostSharedViewModel.ticks(clock)` for its clock.
  - The photo editor is plain logic in `PhotoEditor`: crop with Free/1:1/4:5/9:16 and drag handles, straighten up to ±10°, tone, and tap-to-blur. It writes its result back as declared `Edit`s that `BuildPost` seals.

**Tests:**
- **Capture (81):** clock formatting, image downsampling, all display text, and the ViewModel flows (seal pill, shutter going to the composer or the offline toast, no key, seal failure, double-tap guard, mode/flash/lens, detections).
- **Post (127):** text, the editor maths, redaction drawing, nav keys, the composer flows (blur toggles, location, alt text, caption, flagging AI text, editor, copy/share/republish, publish failure) and the shared screen.
- Each module also has a snapshot test that writes PNGs to `build/snapshots`. It checks nothing; it's there for visual comparison.

**Where the screens differ from the design, and why:**
- **Live blur boxes:** domain's `BlurDetector` only works on a photo after it's sealed, so the live boxes and the "2 things will be blurred" hint read from a `Flow<FrameDetections>` I defined in the feature. Until something real is wired in, it shows nothing.
- **Offline state:** there is no connectivity source in domain, so the camera takes an `online: Flow<Boolean>` input.
- **Camera footer line:** it can't show "14 satellites" because nothing exposes a satellite count; it shows "spoof check passed", "fused fix", "mock location detected" and so on from the location evidence instead.
- **Place name on the camera:** before capture there's no place name, so the camera hint says "an area". The composer shows the capture's place once there is one.
- **Proof link before posting:** the record ID only exists once the post is sealed, so B3 shows `witness.app/p/…` until the first copy or share. Changing blurs, location or crop after that re-seals as a new derivative; caption and alt text changes don't.
- **B6 "Checked" count:** it shows 0 at share time because nothing provides check counts.
- **Flash:** the icon set only has `flash_off`, so flash is a simple off/on toggle.
- **Flag button:** on "Before you post" it removes the AI caption and alt text and logs a breadcrumb.

**Open issues for other owners:**
- **`WIcon` (core/designsystem):** it can't draw icons larger than 24dp because `Modifier.paint` defaults to `ContentScale.Inside`. I worked around it in B6; the fix is `contentScale = ContentScale.Fit`.
- **Missing ports for core/domain:** a live-frame detector, connectivity, place lookup, check-count stats and satellite count.
- **Duplicated helpers:** the time formatter and photo decoding (which downsamples, so a 12 MP JPEG is never fully decoded) are copied in both modules. They belong in core/ui.
- **Unwired behaviour:**
  - Back press while B4 is open isn't handled.
  - Tap-to-blur ignores the straighten rotation, so taps are off by a few pixels at most.
  - Captures need their JPEG saved by the data layer for thumbnails to appear: `SealCapture` never writes the image.

Everything is in `/Users/andrejones/Desktop/workspace/projects/witness/client/feature/capture` and `/Users/andrejones/Desktop/workspace/projects/witness/client/feature/post`.

# ---- module report ----
I built both modules. Everything compiles for their only target (Android) and all 200 tests pass: `:feature:case` has 114, `:feature:share` has 86. Lint is clean, and there are no code comments.

I also checked the screens against the design PNGs. Robolectric renders each screen at 412dp and 3x density, and I compared those images by eye with `design/renders`. C1 to C9 match closely in layout, spacing, colours and copy. The PNGs land in `feature/{case,share}/build/snapshots/`.

**Modules touched:** `feature/case` and `feature/share`. I added the serialization plugin, `navigation3-ui` and `serialization-json`, plus Robolectric for host tests. Each module's own `build.gradle.kts` turns on `isIncludeAndroidResources` for host tests. I deleted the old stub screens and stub tests.

**`feature/case`** (package `app.witness.feature.cases`)
- **C1 Camera · Case:** `CaseCameraOverlay` shows the case pill, the "New case" pill and the evidence coach, driven by `CaseCameraViewModel`. It is a layer, not a full camera screen: `CaseCameraLayer(deps, nav)` is meant to sit inside the capture feature's viewfinder.
- **Evidence coach:** `EvidenceCoach` holds the steps for each template. Progress is the number of captures plus skips, so two captures shows "3 of 5" as in the design. Flagging the coach records the flag and skips the step.
- **C2 / C3 sheets:** `CaseSwitcherViewModel` and `NewCaseViewModel`. The situation is suggested from the case name by `TemplateMatcher` until the user picks one.
- **C4 Cases:** `CasesViewModel` + `CasesScreen` with the All / Open / Shared filters. "Open" means no active share link.
- **C5 Case report:** `CaseReportViewModel` + `CaseReportScreen`. C6 (report type), C7 (flag), a summary editor and delete-case run as sheets inside the screen. The a2ui report is hosted through `CaseReportSlot = @Composable (CaseReportHost) -> Unit`, which gets `onFlag` and `onCaptureFor`. With no slot, a native draft-summary card is shown instead.
- **Navigation:** keys `CasesKey`, `CaseKey(caseId)`, `CaseSwitcherKey`, `NewCaseKey`, registered with `EntryProviderScope<NavKey>.caseEntries(CaseDeps, CaseNavigator)`. C2 and C3 are dialog entries.
- **New port:** `AiFeedback` (flag, remove) with `InMemoryAiFeedback`, plus `UtcOffset` and `RecordImage`.

**`feature/share`** (package `app.witness.feature.share`)
- **C9 Share case:** `ShareCaseViewModel` + `ShareCaseScreen` cover item selection, privacy chips, expiry, AI summary toggle, QR code, a masked link (`#k=••••`) and the "recipients will see" line.
  - It wraps the existing `ShareCase` use case.
  - It reuses a link when nothing has changed, re-uploads when something has, and shows an existing active link on open.
  - It shows clear errors for "too large", "nothing to share" and upload failure.
- **C8 Export:** `ExportViewModel` + `ExportContent` as a dialog entry. ZIP is free; PDF is locked to Plus and sends an `Upgrade` event, and it falls back to ZIP if Plus lapses.
- **ZIP export:** `ZipCaseExporter` streams one photo at a time into a store-only ZIP. It contains photos, a proof JSON per item, `case.json` and `README.txt`, and rejects unsafe entry names. Location is rounded to a 6-character geohash unless "exact location" is on. The ZIP output is checked against `java.util.zip` in a test.
- **Navigation:** keys `ShareCaseKey`, `ExportCaseKey`, registered with `shareEntries(ShareDeps, ShareNavigator)`.
- **New ports:** `CaseExporter`, `ExportSink`/`ExportOutput`, and `ShareRedactor` with a pure `RedactionPlan`.

**Deviations:**
- **Ports live in the features, not `core/domain`.** Export, AI flags, the redactor and the export sink aren't in domain, and I stayed out of it. If you want them there, they should move.
- **Privacy chips depend on a redactor.** `ShareCase` can't redact, so the chips only appear when a `ShareRedactor` is wired. Blurring or hiding location replaces an item with a sealed derivative before upload, so the privacy promise is never shown without being enforced. Implementing the redactor (sealing a blurred derivative, as `BuildPost` does for posts) needs domain work.
- **Some code is duplicated.** Features can't depend on each other or edit core, so the date formatting and the small sheet and dialog helpers exist in both modules. They belong in `core/ui`. The coach's dark-mode colours (`#B794FF` and five others) are local constants; they should become designsystem tokens.

**What the app shell has to do:**
- Use a `NavKey` back stack with `DialogSceneStrategy` and the ViewModel-store entry decorator.
- Implement `CaseNavigator` and `ShareNavigator`, including clipboard, the system share sheet, and sharing the exported file.
- Provide `RecordImage`, the a2ui report slot, `UtcOffset` from `TimeZone.getDefault()`, and the `ExportSink`, `ShareRedactor` and `BlurDetector` implementations.
- Host `CaseCameraLayer` inside the capture feature's camera.

**Open issues:**
- PDF export returns `Unsupported`; it needs a platform renderer.
- Coach skips are kept in memory only and are lost if the process dies.
- Nothing has run on a device or emulator, because nothing is wired into the app shell yet.

Key files are in `/Users/andrejones/Desktop/workspace/projects/witness/client/feature/case/src/commonMain/kotlin/app/witness/feature/cases/` and `/Users/andrejones/Desktop/workspace/projects/witness/client/feature/share/src/commonMain/kotlin/app/witness/feature/share/`:
- `CaseEntries.kt`
- `CaseReportScreen.kt`
- `ShareEntries.kt`
- `ShareCaseViewModel.kt`
- `CaseExport.kt`
- `ZipWriter.kt`

# ---- module report ----
Both modules are built and pass their tests. `feature/check` has 70 tests and `feature/proof` has 56, and each runs green on android host, JS (Chrome headless) and wasmJs. `:webApp` (JS and wasmJs) and `:androidApp` compile against them.

I compared every screen with its PNG by rendering it in the browser at 412 px, and the web pages also at 1280 px. To do that I temporarily replaced `webApp/src/webMain/kotlin/app/witness/web/Main.kt` and `webApp/build.gradle.kts` with a screen gallery, then restored both files from backup. Layout, copy and colours match closely. Two buttons were cut off and are now fixed: "Technical record" on D1 and "Share result" on D7. The compare was layout-only: the gallery used blank image placeholders, so real photo rendering wasn't checked.

**Modules touched:** `feature/check`, `feature/proof` (sources and their `build.gradle.kts`). The build files add the Kotlin serialization plugin, Navigation 3 UI and `kotlinx-serialization-json`; check also adds activity-compose on Android and kotlinx-browser on web. I made no change to `libs.versions.toml`.

**Public API**
- **Check (D5–D8, W2–W6, X2):**
  - Navigation keys `CheckKey` and `CheckSharedKey(sharedId)`, registered with `EntryProviderScope<NavKey>.checkEntries(deps, CheckNavigation)`.
  - `CheckRoute` for the app and `WebCheckRoute` for the web.
  - `CheckViewModel`, which takes `CheckDependencies(checkImage, media, clock, recent, links, time, shared)` and exposes `StateFlow<CheckUiState>`.
  - Plain screen functions `CheckScreen(state, actions)` and `WebCheckPage`.
  - Pure helpers: `CheckOutcome.of` (turns a `CheckResult` into the result screen), `StageText`, `PastedLink.parse` (proof or case links), `PickedImage`, and an image picker (Android photo picker, browser file input).
  - A new `RecentChecks` port with an in-memory version.
- **Proof, app side (D1–D4):**
  - Keys `ItemProofKey`, `TechnicalRecordKey`, `ProveLocationKey`, registered with `itemProofEntries(ItemDependencies, ItemNavigation)`.
  - `ItemViewModel` handles loading, the chain of custody, the technical record JSON, the location proof file, and erasing public copies through `EraseMine`.
- **Proof, web side (W1, W7–W10, X1, X3):**
  - Keys `ProofPageKey`, `CaseLinkKey(caseId)` and `ReportKey`, registered with `webProofEntries(WebProofDependencies, WebProofNavigation)`.
  - View models `ProofPageViewModel`, `CaseLinkViewModel` and `ReportViewModel`.
  - The case link screen re-checks every decrypted photo with `CheckImage`, at most 4 at a time.
  - New ports `CaseLinkOpener` and `ReportSender`.
- **Desktop layouts:** these come from `BrowserSafeLayout` switching at 840 px, in shared code.
- **Temporary entry points:** `CheckScreen()` and `ProofScreen()` with no arguments still work so `webApp` keeps compiling. They run on in-memory fakes and are marked deprecated, the same way home does it.

**Deviations**
- **Case link decryption:** feature modules can't depend on `core/icp`, so `feature/proof` only defines the `CaseLinkOpener` port. The app shell needs to plug in `CaseOpener` plus the bundle decoder behind it.
- **Times:** `TimeText` takes the time-zone offset as a parameter and defaults to UTC, so the shell must pass the device offset or local times will show in UTC. The same file exists in both modules, because two features can't share code.
- **"Tell me when it's anchored" (D8):** the toggle only changes screen state. No domain use case does the background recheck.
- **Area name:** places show as the geohash ("area 9xj64") instead of a name like "Denver area", because there is no reverse geocoding.
- **Drag and drop on desktop (X2):** not built. Choosing a file and pasting a link work.
- **Crop outline:** the crop box on the reference image is estimated from the two aspect ratios and how much was compared, because `CheckResult` doesn't carry the alignment rectangle.
- **Case link key:** `CaseLinkKey.fragment` is not saved with navigation state, so the key never lands in saved state. After process death the page shows "missing key" instead.
- **Other things the design shows but no data source supports, left out:** "checked 1,284 times", the device model name, the total MMR leaf count, and recent-check thumbnails.

**Open issues / contract requests**
- Please move these into `core/domain`: `RecentChecks` (Room's `CheckHistoryDao` already exists), `CaseLinkOpener` as an "open case link" use case, `ReportSender`, plus a delete-capture use case. `CaptureStore` has no delete, so the "Delete from this phone" action just calls back to the app.
- Please move `TimeText` into `core/ui` so the two copies become one.
- App shell, Android: register `checkEntries`, `itemProofEntries` and `webProofEntries`, and supply share and share-file callbacks (the latter via `core/platform` ShareIntents).
- App shell, web: switch `Main.kt` to `WebCheckRoute`, `ProofPageRoute` and `CaseLinkRoute`, reading the `#k=` key from `sessionStorage`.
- Build quirk: a `ByteArray(n) { … }` initializer in a test `object` triggered a JVM `VerifyError` (looks like a compiler issue). The fixtures use `byteArrayOf` / `fill` instead.

Main sources are in `/Users/andrejones/Desktop/workspace/projects/witness/client/feature/check/src/` and `/Users/andrejones/Desktop/workspace/projects/witness/client/feature/proof/src/`.

# ---- module report ----
**Witness client integration report**

The Android app and the web app now run end to end on stand-in implementations (fakes). `./gradlew test allTests assembleDebug wasmJsBrowserDistribution composeCompatibilityBrowserDistribution` passes with 3226 tests and 0 failures. The app is installed on emulator-5554, and I walked these flows: onboarding → home → camera → post composer → shared → check, plus a case with its AI-built report, item proof, share case, export and deep links. Screenshots are in `/Users/andrejones/Desktop/workspace/projects/witness/client/screenshots/`. Nothing was committed.

**Modules touched**
- **`androidApp`**: rewritten.
- **`webApp`**: rewritten.
- **Small changes in other modules**, each fixing a cross-module break:
  - `gradle/libs.versions.toml`: added `core-splashscreen` 1.2.0, the latest on Google Maven. Metro 1.6.0 is confirmed as the latest.
  - `native/witness/build.gradle.kts`: added the JNA `@aar` dependency. The UniFFI bindings landed during this run, and without JNA the whole build failed.
  - `feature/check`: `readImage` is now public, so shared images reuse it. `CheckRoute` gained an `idleFooter` slot, because the tab bar should appear only on the idle Check screen (D5 has it, D6 and D7 don't).
  - `feature/capture`: the camera panel height is now a public `CameraPanelHeight`, so the case coach card sits above the shutter panel as in C1.
  - `README-build.md`: two rows added (minified APK, serving the web bundle locally).

**androidApp**
- **Metro graph** (`di/AppGraph.kt`, `di/Features.kt`): every domain port is bound to its Android implementation:
  - Room, DataStore and WorkManager (the data layer), with WorkManager starting on demand.
  - Keystore key vault, CameraX, GNSS location, ML Kit blur, Gemini Nano falling back to rules, RevenueCat, Firebase telemetry.
  - Real JCA crypto and `SecureRandom`.
- **Native switch** (`di/NativeAvailability.kt`): the only place fakes are used. `NativeAvailability.factory = StandInNative` supplies:
  - seal engine, verify engine, registry, index, media
  - case bundle format, Internet Identity account, case-link opener, report sender

  Swapping in `:native:witness` means changing that one line.
- **What the stand-in does**:
  - It stores the upright sealed JPEG and indexes exact hashes when a record is anchored, so a sealed photo checks as **Original** end to end.
  - On startup it re-seeds the enrolled key into the fake registry; such keys show the DEV badge.
- **Navigation 3 host** (`WitnessApp.kt`, `nav/`): every feature's entries are registered, with per-entry ViewModels and dialog sheets. The back stack survives process death without reflection.
- **Deep links and sharing**: `https://witness.app/p/…`, `https://case.witness.app/v/…#k=` and `/check` open the app. Sharing an image opens Check; shared text links are parsed too. A shared image that can't be read shows "no longer available" instead of crashing.
- **Splash, icon, case report**:
  - core-splashscreen on ink, plus an adaptive and monochrome icon converted from `icon.svg` to a vector drawable.
  - The case screen now renders the A2UI report (checklist and generated form, matching C5); before, that section was empty.
- **Export**: ZIP export writes to Downloads, and file sharing goes through a FileProvider.

**webApp**
- `WebRouter` handles `/p/{id}`, `/v/{id}`, `/check` and `/{p|v}/{id}/report` with `pushState` and the back button. Route segments are validated.
- Mounting follows §8.1:
  - The page's inline JSON summary is read and validated.
  - The case key in `#k=` moves to `sessionStorage` and is removed from the URL.
  - If WebGL 2 is missing, the HTML page stays and a note is added.
  - A `compose-ready` class is set once Compose is up.
- `configureWebResources` maps resources to a base given by a meta tag (set by `boot.mjs` when absent).
- `boot.mjs` picks the wasm or JS bundle relative to its own URL. Compose's own loader resolves against the page, which left `/p/…` pages blank.
- In the browser, `/check`, `/p/`, `/v/` and back navigation all work on the compatibility build.

**Tests**
- androidApp: 110 new tests covering the navigator, deep links, nav key serialization, the stand-in (sealed photo checks as Original), crypto against known vectors, export, shared images, GNSS session and the Metro graph (Robolectric).
- webApp: 52 tests (26 per target) covering routes, router, inline summary and resource paths.
- Bugs these tests and the device walk caught:
  - A crash on the `/report` route.
  - A startup hang: GNSS registration blocked the main thread on the emulator. It now runs on a background thread, only while the camera is open.
  - Images saved sideways.
  - The case coach card overlapping the shutter.
  - Settings saying "Gemini Nano available" on a device without it.

**Deviations**
- **Self-hosted fallback font isn't possible**: Compose 1.12.2 has no public setting for it; the Noto downloader is internal and hard-codes fonts.gstatic.com, which the §8.1 Content Security Policy would block anyway. Inter and JetBrains Mono are self-hosted through the resource mapping.
- **Emulator installs use the minified `benchmarkRelease` APK (57 MB)**: the debug APK (121 MB) doesn't fit the emulator's storage. I deleted a stale 139 MB copy of our own APK from `/data/local/tmp`.
- **Web uses the same in-memory stand-ins**: the `witness-bridges` and verify modules have their own contracts, not the domain ports.

**Awaiting the native bridge**
- Real sealing (watermark, C2PA, re-encode), real verify, and the ICP registry, index and media.
- Internet Identity linking, case-link opening and report sending.
- The native API shapes differ from the domain ports (e.g. `check(image)` is one monolithic call, and uploads need a signer), so adapters are needed.

**Contract gaps for other owners**
- `SealEngine` has no output for the sealed JPEG; the stand-in persists the frame itself.
- The fake seal engine and registry live in memory, so posting from a photo sealed before a restart fails.
- Posts don't render their blurs yet; a post shows the parent image.
- In-process enrollment reports StrongBox/RKP rather than DEV.
- Item actions "move to case" and "delete" aren't wired, because `CaptureStore` has no delete.
- Live blur detection on the camera preview isn't wired; the camera gets an empty detection stream.
- The camera's sealed toast shows through the case coach card.
- The magic comment in `bridges/verify.mjs` (`webpackIgnore`) is functional, so I left it; I also didn't strip comments from the generated UniFFI file.

**Firebase**: no project was created. The Gradle plugins apply only when `androidApp/google-services.json` exists; until then telemetry is a no-op, and it stays off until the user opts in.

**Files** (all under `/Users/andrejones/Desktop/workspace/projects/witness/client`)
- `androidApp/src/main/kotlin/app/witness/di/AppGraph.kt`
- `androidApp/src/main/kotlin/app/witness/di/NativeAvailability.kt`
- `androidApp/src/main/kotlin/app/witness/di/StandInNative.kt`
- `androidApp/src/main/kotlin/app/witness/WitnessApp.kt`
- `androidApp/src/main/kotlin/app/witness/nav/AppNavigator.kt`
- `webApp/src/commonMain/kotlin/app/witness/web/WebRouter.kt`
- `webApp/src/webMain/resources/boot.mjs`