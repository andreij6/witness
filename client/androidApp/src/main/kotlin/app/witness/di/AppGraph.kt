package app.witness.di

import android.app.Application
import android.content.Context
import android.location.LocationManager
import android.os.Handler
import android.os.HandlerThread
import android.telephony.TelephonyManager
import app.witness.BuildConfig
import app.witness.core.data.CheckHistoryStore
import app.witness.core.data.DataGraph
import app.witness.core.data.ImageFiles
import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnchorScheduler
import app.witness.core.domain.port.Assistant
import app.witness.core.domain.port.BlurDetector
import app.witness.core.domain.port.CaptureSource
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.CounterStore
import app.witness.core.domain.port.Crypto
import app.witness.core.domain.port.EntitlementSource
import app.witness.core.domain.port.IndexGateway
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.Links
import app.witness.core.domain.port.LocationEvidenceSource
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.OutboxStore
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.SealEngine
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.port.Telemetry
import app.witness.core.domain.port.TemplateCatalog
import app.witness.core.domain.port.VerifyEngine
import app.witness.core.domain.usecase.AnchorOutbox
import app.witness.core.domain.usecase.PublishPreviews
import app.witness.core.domain.usecase.BuildPost
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.domain.usecase.Enrollment
import app.witness.core.domain.usecase.EraseMine
import app.witness.core.domain.usecase.KeyRotation
import app.witness.core.domain.usecase.ManageCases
import app.witness.core.domain.usecase.SealCapture
import app.witness.core.domain.usecase.SealSigner
import app.witness.core.domain.usecase.ShareCase
import app.witness.core.model.Platform
import app.witness.core.platform.assist.FallbackAssistant
import app.witness.core.platform.assist.GeminiNanoAssistant
import app.witness.core.platform.blur.MlKitBlurDetector
import app.witness.core.platform.capture.CameraXCaptureSource
import app.witness.core.platform.gnss.AndroidLocationEvidenceSource
import app.witness.core.platform.keys.KeystoreKeyVault
import app.witness.core.telemetry.AndroidTelemetry
import app.witness.feature.cases.AiFeedback
import app.witness.feature.cases.InMemoryAiFeedback
import app.witness.feature.home.CaptureThumbnails
import app.witness.feature.settings.PlusBilling
import app.witness.feature.settings.RevenueCatPlus
import app.witness.system.SharedImages
import dev.zacsweers.metro.AppScope
import dev.zacsweers.metro.BindingContainer
import dev.zacsweers.metro.DependencyGraph
import dev.zacsweers.metro.Provides
import dev.zacsweers.metro.SingleIn
import java.util.concurrent.Executor
import java.util.concurrent.Executors

@DependencyGraph(AppScope::class, bindingContainers = [PlatformBindings::class, NativeBindings::class, UseCaseBindings::class])
interface AppGraph {
    val telemetry: Telemetry
    val settings: SettingsStore
    val vault: KeyVault
    val anchorOutbox: AnchorOutbox
    val native: NativePorts
    val captureSource: CameraXCaptureSource
    val features: Features

    @DependencyGraph.Factory
    fun interface Factory {
        fun create(@Provides application: Application): AppGraph
    }
}

@BindingContainer
object PlatformBindings {
    @Provides
    fun context(application: Application): Context = application

    @Provides
    fun clock(): Clock = SystemClock

    @Provides
    fun crypto(): Crypto = JcaCrypto

    @Provides
    fun links(): Links = Links.at(BuildConfig.PAGES_ORIGIN)

    @Provides
    @SingleIn(AppScope::class)
    fun random(): RandomSource = SecureRandomSource()

    @Provides
    @SingleIn(AppScope::class)
    fun data(context: Context): DataGraph = DataGraph(context)

    @Provides
    fun captures(data: DataGraph): CaptureStore = data.captures

    @Provides
    fun outbox(data: DataGraph): OutboxStore = data.outbox

    @Provides
    fun cases(data: DataGraph): CaseStore = data.cases

    @Provides
    fun counters(data: DataGraph): CounterStore = data.counters

    @Provides
    fun settings(data: DataGraph): SettingsStore = data.settings

    @Provides
    fun scheduler(data: DataGraph): AnchorScheduler = data.anchorScheduler

    @Provides
    fun vault(keystore: KeystoreKeyVault): KeyVault = keystore

    @Provides
    fun images(data: DataGraph): ImageFiles = data.images

    @Provides
    fun history(data: DataGraph): CheckHistoryStore = data.checkHistory

    @Provides
    @SingleIn(AppScope::class)
    fun telemetry(context: Context): Telemetry = LocalLogTelemetry.wrapIf(BuildConfig.ICP_FETCH_ROOT_KEY, AndroidTelemetry.create(context))

    @Provides
    @SingleIn(AppScope::class)
    fun keystore(context: Context, clock: Clock): KeystoreKeyVault = KeystoreKeyVault(
        context.getSharedPreferences(KEYS_PREFS, Context.MODE_PRIVATE),
        clock,
        KeystoreKeyVault.hasStrongBox(context.packageManager),
    )

    @Provides
    @SingleIn(AppScope::class)
    fun captureExecutor(): Executor = Executors.newSingleThreadExecutor { r -> Thread(r, "witness-capture") }

    @Provides
    @SingleIn(AppScope::class)
    fun captureSource(context: Context, executor: Executor): CameraXCaptureSource =
        CameraXCaptureSource(executor, CameraXCaptureSource.systemBootCount(context.contentResolver))

    @Provides
    @SingleIn(AppScope::class)
    fun locationSource(context: Context, executor: Executor): AndroidLocationEvidenceSource = AndroidLocationEvidenceSource(
        checkNotNull(context.getSystemService(LocationManager::class.java)),
        context.getSystemService(TelephonyManager::class.java),
        executor,
        Handler(HandlerThread("witness-gnss").apply { start() }.looper),
    )

    @Provides
    fun capture(source: Lazy<CameraXCaptureSource>): CaptureSource = CaptureSource { source.value.capture() }

    @Provides
    fun location(source: AndroidLocationEvidenceSource): LocationEvidenceSource = source

    @Provides
    @SingleIn(AppScope::class)
    fun gnss(source: AndroidLocationEvidenceSource): GnssSession =
        GnssSession({ source.start() }, source::stop, Executors.newSingleThreadExecutor { r -> Thread(r, "witness-gnss-control") })

    @Provides
    @SingleIn(AppScope::class)
    fun mlKit(captures: CaptureStore): MlKitBlurDetector = MlKitBlurDetector(captures::readImage)

    @Provides
    fun blur(mlKit: MlKitBlurDetector): BlurDetector = mlKit

    @Provides
    @SingleIn(AppScope::class)
    fun nano(): GeminiNanoAssistant = GeminiNanoAssistant()

    @Provides
    @SingleIn(AppScope::class)
    fun assistant(nano: GeminiNanoAssistant): Assistant = FallbackAssistant(nano, AppForeground::isForeground)

    @Provides
    @SingleIn(AppScope::class)
    fun bundled(): BundledTemplates = BundledTemplates()

    @Provides
    fun templates(bundled: BundledTemplates): TemplateCatalog = bundled

    @Provides
    @SingleIn(AppScope::class)
    fun plus(): RevenueCatPlus = RevenueCatPlus()

    @Provides
    fun billing(plus: RevenueCatPlus): PlusBilling = plus

    @Provides
    fun entitlements(plus: RevenueCatPlus): EntitlementSource = plus

    @Provides
    @SingleIn(AppScope::class)
    fun feedback(): AiFeedback = InMemoryAiFeedback()

    @Provides
    @SingleIn(AppScope::class)
    fun thumbnails(captures: CaptureStore): CaptureThumbnails = CaptureThumbnails(captures)

    @Provides
    @SingleIn(AppScope::class)
    fun sharedImages(): SharedImages = SharedImages()

    const val KEYS_PREFS: String = "witness.keys"
}

@BindingContainer
object NativeBindings {
    @Provides
    @SingleIn(AppScope::class)
    fun ports(context: Context, clock: Clock, images: ImageFiles, keystore: KeystoreKeyVault, random: RandomSource): NativePorts =
        NativeAvailability.factory.create(NativeEnvironment(clock, images, keystore, context.filesDir, random, keystore::c2paCertificate))

    @Provides
    fun seal(ports: NativePorts): SealEngine = ports.seal

    @Provides
    fun verify(ports: NativePorts): VerifyEngine = ports.verify

    @Provides
    fun registry(ports: NativePorts): RegistryGateway = ports.registry

    @Provides
    fun index(ports: NativePorts): IndexGateway = ports.index

    @Provides
    fun media(ports: NativePorts): MediaGateway = ports.media
}

@BindingContainer
object UseCaseBindings {
    @Provides
    @SingleIn(AppScope::class)
    fun signer(engine: SealEngine, counters: CounterStore): SealSigner = SealSigner(engine, counters)

    @Provides
    @SingleIn(AppScope::class)
    fun enrollment(
        vault: KeyVault, registry: RegistryGateway, outbox: OutboxStore, crypto: Crypto, settings: SettingsStore,
        random: RandomSource, scheduler: AnchorScheduler, clock: Clock, telemetry: Telemetry,
    ): Enrollment = Enrollment(vault, registry, outbox, crypto, settings, random, scheduler, clock, telemetry)

    @Provides
    fun rotation(vault: KeyVault, enrollment: Enrollment, clock: Clock, telemetry: Telemetry): KeyRotation =
        KeyRotation(vault, enrollment, clock, telemetry)

    @Provides
    @SingleIn(AppScope::class)
    fun sealCapture(
        source: CaptureSource, location: LocationEvidenceSource, engine: SealEngine, signer: SealSigner, vault: KeyVault,
        captures: CaptureStore, settings: SettingsStore, scheduler: AnchorScheduler, clock: Clock, random: RandomSource, telemetry: Telemetry,
    ): SealCapture = SealCapture(source, location, engine, signer, vault, captures, settings, scheduler, clock, random, telemetry)

    @Provides
    @SingleIn(AppScope::class)
    fun buildPost(
        captures: CaptureStore, detector: BlurDetector, assistant: Assistant, settings: SettingsStore, engine: SealEngine, signer: SealSigner,
        vault: KeyVault, scheduler: AnchorScheduler, clock: Clock, links: Links, telemetry: Telemetry,
    ): BuildPost = BuildPost(captures, detector, assistant, settings, engine, signer, vault, scheduler, clock, links, telemetry)

    @Provides
    @SingleIn(AppScope::class)
    fun anchorOutbox(
        registry: RegistryGateway, outbox: OutboxStore, captures: CaptureStore, vault: KeyVault, engine: SealEngine, signer: SealSigner,
        clock: Clock, telemetry: Telemetry, ports: NativePorts,
    ): AnchorOutbox = AnchorOutbox(registry, outbox, captures, vault, engine, signer, clock, telemetry, PublishPreviews(captures, ports.previews, clock, telemetry))

    @Provides
    fun manageCases(
        cases: CaseStore, captures: CaptureStore, settings: SettingsStore, catalog: TemplateCatalog, assistant: Assistant, clock: Clock, telemetry: Telemetry,
    ): ManageCases = ManageCases(cases, captures, settings, catalog, assistant, clock, telemetry)

    @Provides
    fun shareCase(
        cases: CaseStore, captures: CaptureStore, ports: NativePorts, crypto: Crypto, media: MediaGateway, registry: RegistryGateway,
        entitlements: EntitlementSource, random: RandomSource, clock: Clock, links: Links, telemetry: Telemetry,
    ): ShareCase = ShareCase(cases, captures, ports.bundles, crypto, media, registry, entitlements, random, clock, links, telemetry)

    @Provides
    fun eraseMine(captures: CaptureStore, registry: RegistryGateway, vault: KeyVault, random: RandomSource, telemetry: Telemetry): EraseMine =
        EraseMine(captures, registry, vault, random, telemetry)

    @Provides
    @SingleIn(AppScope::class)
    fun checkImage(engine: VerifyEngine, registry: RegistryGateway, index: IndexGateway, media: MediaGateway, telemetry: Telemetry): CheckImage =
        CheckImage(engine, registry, index, media, Platform.ANDROID, telemetry)
}
