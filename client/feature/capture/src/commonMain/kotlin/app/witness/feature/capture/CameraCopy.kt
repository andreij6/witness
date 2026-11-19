package app.witness.feature.capture

import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.port.LocationReading
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.CaptureMode
import app.witness.core.model.LocationPrecision
import app.witness.core.model.LocationReason
import app.witness.core.model.SecurityLevel
import app.witness.core.ui.LocationConfidence
import app.witness.core.ui.NormRect
import app.witness.core.ui.pillText
import app.witness.core.model.LocationConfidence as Confidence

object CameraCopy {
    const val FOOTNOTE: String = "Photo · video and audio coming later"
    const val OFFLINE: String = "Offline · will anchor later"
    const val NO_KEY: String = "This phone's seal isn't set up yet"
    const val SEAL_FAILED: String = "Couldn't seal that photo. Try again."
    const val MAX_DETECTIONS: Int = 16
    val MODES: List<String> = listOf("Post", "Case")

    fun securityName(level: SecurityLevel): String = when (level) {
        SecurityLevel.STRONG_BOX -> "StrongBox"
        SecurityLevel.TEE -> "TEE"
        SecurityLevel.DEV -> "Dev key"
    }

    fun sealPill(level: SecurityLevel?): SealPill =
        if (level == null) SealPill("Seal not set up", ready = false) else SealPill("${securityName(level)} seal", ready = true)

    fun confidence(c: Confidence): LocationConfidence = when (c) {
        Confidence.HIGH -> LocationConfidence.High
        Confidence.MEDIUM -> LocationConfidence.Medium
        Confidence.LOW -> LocationConfidence.Low
    }

    fun locationPill(reading: LocationReading?): LocationPillUi {
        val evidence = reading?.evidence
        if (evidence == null || LocationReason.NO_FIX in evidence.reasons) return LocationPillUi("Location off", WitnessIcon.LocationOn, null)
        val c = confidence(evidence.confidence)
        return LocationPillUi(c.pillText(), c.icon, c.tone)
    }

    fun locationDetail(reading: LocationReading?): String {
        val evidence = reading?.evidence ?: return "no location fix"
        val reasons = evidence.reasons
        return when {
            LocationReason.MOCK_LOCATION in reasons -> "mock location detected"
            LocationReason.NO_FIX in reasons -> "no location fix"
            evidence.confidence == Confidence.HIGH -> "spoof check passed"
            evidence.confidence == Confidence.MEDIUM -> "fused fix · no satellite check"
            else -> "weak satellite signal"
        }
    }

    fun meta(clock: String, online: Boolean, reading: LocationReading?): String =
        if (online) "$clock · ${locationDetail(reading)}" else "$clock · no network · sealed on-device"

    fun blurName(c: BlurClass): String = when (c) {
        BlurClass.FACE -> "Face"
        BlurClass.PLATE -> "Plate"
        BlurClass.HOUSE_NO -> "House number"
        BlurClass.OTHER -> "Area"
    }

    fun detections(frame: FrameDetections, mode: CaptureMode): List<Detection> {
        if (mode != CaptureMode.POST || !frame.dims.isValid || frame.proposals.isEmpty()) return emptyList()
        val w = frame.dims.width.toFloat()
        val h = frame.dims.height.toFloat()
        val out = ArrayList<Detection>(minOf(frame.proposals.size, MAX_DETECTIONS))
        for (p in frame.proposals) {
            if (out.size == MAX_DETECTIONS) break
            detection(p, frame, w, h)?.let(out::add)
        }
        return out
    }

    private fun detection(p: BlurProposal, frame: FrameDetections, w: Float, h: Float): Detection? {
        if (!p.enabled) return null
        val r = p.rect.clippedTo(frame.dims.width, frame.dims.height) ?: return null
        return Detection("${blurName(p.blurClass)} · auto-blur", NormRect(r.x / w, r.y / h, r.right / w, r.bottom / h))
    }

    fun locationPhrase(precision: LocationPrecision): String = when (precision) {
        LocationPrecision.HIDDEN -> "No location on the post · precise stays sealed"
        LocationPrecision.AREA -> "Location will show as an area · precise stays sealed"
        LocationPrecision.NEIGHBORHOOD -> "Location will show as a neighborhood · easier to pin down"
        LocationPrecision.EXACT -> "Exact spot will show · shows where you were standing"
    }

    fun hint(count: Int, mode: CaptureMode, precision: LocationPrecision): CameraHint? {
        if (mode != CaptureMode.POST || (count <= 0 && !precision.revealsPrecise)) return null
        val title = when {
            count == 1 -> "1 thing will be blurred when you post"
            count > 1 -> "$count things will be blurred when you post"
            else -> "Your post will narrow down where you were"
        }
        return CameraHint(title, locationPhrase(precision))
    }

    fun badge(status: AnchoringStatus): WitnessState = when (status) {
        AnchoringStatus.ANCHORED -> WitnessState.Anchored
        AnchoringStatus.QUEUED -> WitnessState.Queued
        AnchoringStatus.SEALED -> WitnessState.Sealed
        AnchoringStatus.UNVERIFIED -> WitnessState.Unverified
        AnchoringStatus.FAILED -> WitnessState.Unanchored
    }

    fun toastTitle(level: SecurityLevel?): String = if (level == null) "Sealed" else "Sealed · ${securityName(level)}"

    data class ToastStatus(val text: String, val tone: Tone, val icon: WitnessIcon)

    fun toastStatus(status: AnchoringStatus, online: Boolean): ToastStatus = when (status) {
        AnchoringStatus.ANCHORED -> ToastStatus("Anchored", Tone.Ok, WitnessIcon.Verified)
        AnchoringStatus.QUEUED ->
            if (online) ToastStatus("Anchoring…", Tone.Warn, WitnessIcon.HourglassTop)
            else ToastStatus("Queued for anchoring", Tone.Warn, WitnessIcon.HourglassTop)
        AnchoringStatus.SEALED -> ToastStatus("Anchors after device setup", Tone.Warn, WitnessIcon.HourglassTop)
        AnchoringStatus.FAILED -> ToastStatus("Anchoring will retry", Tone.Warn, WitnessIcon.HourglassTop)
        AnchoringStatus.UNVERIFIED -> ToastStatus("Unverified import", Tone.Muted, WitnessIcon.Help)
    }
}
