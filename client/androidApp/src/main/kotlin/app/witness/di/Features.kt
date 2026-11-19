package app.witness.di

import android.content.Context
import app.witness.core.data.CheckHistoryStore
import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnchorScheduler
import app.witness.core.domain.port.BlurDetector
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.EntitlementSource
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.Links
import app.witness.core.domain.port.LocationEvidenceSource
import app.witness.core.domain.port.OutboxStore
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.port.Telemetry
import app.witness.core.domain.usecase.BuildPost
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.domain.usecase.Enrollment
import app.witness.core.domain.usecase.EraseMine
import app.witness.core.domain.usecase.KeyRotation
import app.witness.core.domain.usecase.ManageCases
import app.witness.core.domain.usecase.SealCapture
import app.witness.core.domain.usecase.ShareCase
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import app.witness.core.platform.assist.GeminiNanoAssistant
import app.witness.core.platform.blur.MlKitBlurDetector
import app.witness.core.platform.capture.CameraXCaptureSource
import app.witness.feature.capture.CameraViewModel
import app.witness.feature.capture.ClockText
import app.witness.feature.capture.LiveBlurAnalysis
import app.witness.feature.cases.AiFeedback
import app.witness.feature.cases.CaseDeps
import app.witness.feature.cases.RecordImage
import app.witness.feature.check.CheckDependencies
import app.witness.feature.home.CaptureThumbnails
import app.witness.feature.home.HomeDependencies
import app.witness.feature.post.ComposerViewModel
import app.witness.feature.post.PostSharedViewModel
import app.witness.feature.proof.ItemDependencies
import app.witness.feature.proof.TimeText
import app.witness.feature.proof.WebProofDependencies
import app.witness.feature.settings.IdentityLinkFlow
import app.witness.feature.settings.PlusBilling
import app.witness.feature.settings.SettingsDependencies
import app.witness.feature.settings.SettingsTelemetryConsent
import app.witness.feature.share.ShareDeps
import app.witness.feature.share.ZipCaseExporter
import app.witness.system.CapturesExporter
import app.witness.system.DownloadsSink
import app.witness.system.SharedImages
import app.witness.ui.CaseReport
import app.witness.ui.RecordThumbnail
import dev.zacsweers.metro.AppScope
import dev.zacsweers.metro.Inject
import dev.zacsweers.metro.SingleIn
import app.witness.feature.check.TimeText as CheckTime
import app.witness.feature.share.UtcOffset as ShareOffset
import app.witness.feature.cases.UtcOffset as CaseOffset

@Inject
@SingleIn(AppScope::class)
class Features(
    private val context: Context,
    private val clock: Clock,
    val links: Links,
    private val vault: KeyVault,
    private val captures: CaptureStore,
    private val cases: CaseStore,
    private val outbox: OutboxStore,
    private val settings: SettingsStore,
    private val scheduler: AnchorScheduler,
    private val enrollment: Enrollment,
    private val rotation: KeyRotation,
    private val sealCapture: SealCapture,
    private val buildPost: BuildPost,
    private val manageCases: ManageCases,
    private val shareCase: ShareCase,
    private val eraseMine: EraseMine,
    private val checkImage: CheckImage,
    private val location: LocationEvidenceSource,
    private val detector: BlurDetector,
    private val mlKit: MlKitBlurDetector,
    private val nano: GeminiNanoAssistant,
    private val catalog: BundledTemplates,
    private val feedback: AiFeedback,
    private val entitlements: EntitlementSource,
    private val billing: PlusBilling,
    private val random: RandomSource,
    private val history: CheckHistoryStore,
    private val telemetry: Telemetry,
    private val native: NativePorts,
    val thumbnails: CaptureThumbnails,
    private val cameraSource: Lazy<CameraXCaptureSource>,
    val sharedImages: SharedImages,
    val gnss: GnssSession,
) {
    val captureSource: CameraXCaptureSource get() = cameraSource.value

    private val sink by lazy { DownloadsSink(context) }
    private val offset: (Long) -> Long = LocalTime::offsetMs
    val recordImage: RecordImage = { id -> RecordThumbnail(thumbnails, id) }

    val homeDeps: HomeDependencies by lazy {
        HomeDependencies(vault, captures, cases, outbox, enrollment, rotation, scheduler, thumbnails, clock)
    }

    val online by lazy { Connectivity.online(context) }

    val liveBlur: LiveBlurAnalysis by lazy { LiveBlurAnalysis(mlKit::detectFrame, onError = telemetry::nonFatal) }

    fun camera(): CameraViewModel = CameraViewModel(
        sealCapture, captures, settings, vault, location, online, liveBlur.detections, ClockText.everySecond(clock), offset, telemetry,
    )

    suspend fun postImage(record: RecordId): ByteArray? = captures.readImage(record)?.toByteArray()

    fun composer(parent: RecordId): ComposerViewModel = ComposerViewModel(parent, buildPost, captures, links, offset, telemetry)

    fun shared(recordId: RecordId, precision: LocationPrecision): PostSharedViewModel =
        PostSharedViewModel(recordId, precision, captures, PostSharedViewModel.ticks(clock), links, offset)

    val caseDeps: CaseDeps by lazy {
        CaseDeps(cases, captures, settings, manageCases, catalog, feedback, clock, CaseOffset(offset), recordImage, CaseReport.slot(catalog))
    }

    val manage: ManageCases get() = manageCases

    val settingsStore: SettingsStore get() = settings

    val shareDeps: ShareDeps by lazy {
        ShareDeps(cases, captures, shareCase, entitlements, ZipCaseExporter(cases, captures, sink, clock, links), clock, detector, null, ShareOffset(offset), recordImage)
    }

    val settingsDeps: SettingsDependencies by lazy {
        SettingsDependencies(
            settings, entitlements, native.identity, IdentityLinkFlow(native.identity, random), SettingsTelemetryConsent(settings),
            nano, billing, CapturesExporter(captures, sink, clock), eraseMine::everything, clock, captures::observeAll, eraseMine::publicCopiesOf,
        )
    }

    val checkDeps: CheckDependencies by lazy {
        CheckDependencies(checkImage, native.media, clock, RoomRecentChecks(history), links, CheckTime(offset), sharedImages::get)
    }

    val itemDeps: ItemDependencies by lazy { ItemDependencies(captures, cases, native.registry, eraseMine, links, TimeText(offset)) }

    val proofDeps: WebProofDependencies by lazy {
        WebProofDependencies(native.registry, native.media, checkImage, native.caseLinks, native.reports, TimeText(offset), links)
    }
}
