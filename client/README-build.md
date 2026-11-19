# Witness client build

Gradle 9.9.1 wrapper. The Gradle daemon runs on JDK 21 (Metro's Gradle plugin needs it, see `gradle/gradle-daemon-jvm.properties`), and every module compiles to JVM 17 bytecode. `local.properties` needs `sdk.dir`.

| What | Command |
|---|---|
| Everything (tests, debug + release APKs, web bundles) | `./gradlew build -x lint` |
| All unit tests (android host, js, wasmJs) | `./gradlew allTests testDebugUnitTest` |
| One module's tests | `./gradlew :core:model:allTests` |
| Android debug APK | `./gradlew :androidApp:assembleDebug` |
| Install on the emulator | `./gradlew :androidApp:installDebug` |
| Minified, debug-signed APK (57 MB, for emulators low on storage) | `./gradlew :androidApp:assembleBenchmarkRelease` |
| Serve the web bundle locally | serve `webApp/build/dist/composeWebCompatibility/productionExecutable` with an SPA fallback to `index.html` |
| Web (WasmGC) | `./gradlew :webApp:wasmJsBrowserDistribution` |
| Web (JS fallback) | `./gradlew :webApp:jsBrowserDistribution` |
| Web (wasm + JS compatibility bundle) | `./gradlew :webApp:composeCompatibilityBrowserDistribution` |
| Web dev server | `./gradlew :webApp:wasmJsBrowserDevelopmentRun` |
| Baseline profile (connected device) | `./gradlew :androidApp:generateBaselineProfile` |
| Startup macrobenchmark (connected device) | `./gradlew :androidBenchmark:connectedBenchmarkReleaseAndroidTest` |

Browser tests (js, wasmJs) run in headless Chrome through Karma.

Firebase Analytics and Crashlytics plugins apply only when `androidApp/google-services.json` exists (git-ignored).

Convention plugins live in `build-logic/convention`: `witness.kmp.library` (android + js + wasmJs), `witness.kmp.android`, `witness.cmp`, `witness.android.app`, `witness.web.app`, `witness.metro`, `witness.room`. Module rules (a feature only depends on `core/{model,designsystem,ui,domain}`, `core/domain` only on `core/model`) fail configuration when broken.
