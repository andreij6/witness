package app.witness.core.domain.usecase

import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.AnchorScheduler
import app.witness.core.domain.port.CaptureFrame
import app.witness.core.domain.port.CaptureSource
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.LocationEvidenceSource
import app.witness.core.domain.port.LocationReading
import app.witness.core.domain.port.PrepareRequest
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.SealEngine
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.port.Telemetry
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureId
import app.witness.core.model.CaptureMode
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Checks
import app.witness.core.model.DeviceKey
import app.witness.core.model.EnumBits
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationReason
import app.witness.core.model.OutboxEntry
import app.witness.core.model.OutboxState
import app.witness.core.model.TimeEvidence
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.first

sealed interface SealOutcome {
    data class Sealed(val capture: Capture) : SealOutcome

    data object NoKey : SealOutcome
}

class SealCapture(
    private val source: CaptureSource,
    private val location: LocationEvidenceSource,
    private val engine: SealEngine,
    private val signer: SealSigner,
    private val vault: KeyVault,
    private val captures: CaptureStore,
    private val settings: SettingsStore,
    private val scheduler: AnchorScheduler,
    private val clock: Clock,
    private val random: RandomSource,
    private val telemetry: Telemetry = Telemetry.None,
) {
    suspend operator fun invoke(frame: CaptureFrame? = null): SealOutcome {
        val key = vault.activeKey()?.takeUnless { it.retired } ?: return SealOutcome.NoKey
        val shot = frame ?: source.capture()
        val reading = readLocation()
        val evidence = reading?.evidence ?: LocationEvidence.NONE
        val time = TimeEvidence(clock.nowEpochMillis(), reading?.gnssTimeMs, shot.elapsedNs, shot.bootCount)
        val prepared = engine.prepare(PrepareRequest(shot, newCaptureId(), time, evidence, checksFor(key, reading)))
        val sealed = signer.seal(prepared, key)
        val prefs = settings.settings.first()
        val claim = sealed.claim
        val capture = Capture(
            recordId = sealed.recordId,
            captureId = claim.captureId,
            origin = CaptureOrigin.ORIGINAL,
            status = if (key.isEnrolled) AnchoringStatus.QUEUED else AnchoringStatus.SEALED,
            createdAtMs = time.wallMs,
            keyRef = claim.keyRef,
            locationConfidence = claim.location.confidence,
            dims = claim.dims,
            caseId = prefs.activeCase.takeIf { prefs.captureMode == CaptureMode.CASE },
        )
        val entry = OutboxEntry(sealed.submission, state = if (key.isEnrolled) OutboxState.PENDING else OutboxState.AWAITING_ENROLLMENT)
        captures.insert(capture, entry, reading?.opening, sealed.jpeg)
        if (key.isEnrolled) scheduler.requestAnchoring()
        telemetry.event(AnalyticsEvent.CAPTURE_SEALED, mapOf("location_confidence" to claim.location.confidence.ordinal.toLong()))
        return SealOutcome.Sealed(capture)
    }

    private suspend fun readLocation(): LocationReading? =
        try {
            location.current()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            null
        }

    private fun newCaptureId(): CaptureId =
        CaptureId.fromRandomBytes(random.nextBytes(CAPTURE_RANDOM_BYTES), CAPTURE_SCHEMA) ?: error("capture id")

    private fun checksFor(key: DeviceKey, reading: LocationReading?) = Checks(
        keySecurity = key.security,
        boot = key.boot,
        deviceLocked = key.deviceLocked,
        mockLocation = reading?.evidence?.reasons?.contains(LocationReason.MOCK_LOCATION) == true,
        gnss = reading?.gnss ?: EnumBits.none(),
        recapture = 0,
    )

    companion object {
        const val CAPTURE_SCHEMA: Int = 0
        const val CAPTURE_RANDOM_BYTES: Int = 7
    }
}
