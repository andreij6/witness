package app.witness.feature.capture

import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.port.LocationReading
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.CaptureMode
import app.witness.core.model.Dims
import app.witness.core.model.EnumBits
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationPrecision
import app.witness.core.model.LocationReason
import app.witness.core.model.Rect
import app.witness.core.model.SecurityLevel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CameraCopyTest {
    private fun reading(confidence: LocationConfidence, vararg reasons: LocationReason) =
        LocationReading(LocationEvidence(null, confidence, EnumBits.of(*reasons)), null, null, EnumBits.none())

    @Test
    fun sealPillNamesHardware() {
        assertEquals(SealPill("StrongBox seal", true), CameraCopy.sealPill(SecurityLevel.STRONG_BOX))
        assertEquals(SealPill("TEE seal", true), CameraCopy.sealPill(SecurityLevel.TEE))
        assertEquals(SealPill("Dev key seal", true), CameraCopy.sealPill(SecurityLevel.DEV))
    }

    @Test
    fun sealPillWithoutKeyIsNotReady() = assertFalse(CameraCopy.sealPill(null).ready)

    @Test
    fun locationPillHigh() {
        val pill = CameraCopy.locationPill(reading(LocationConfidence.HIGH, LocationReason.RAW_GNSS_PASSED))
        assertEquals("Location · High", pill.text)
        assertEquals(WitnessIcon.SatelliteAlt, pill.icon)
        assertEquals(Tone.Ok, pill.tone)
    }

    @Test
    fun locationPillMediumAndLow() {
        assertEquals("Location · Medium", CameraCopy.locationPill(reading(LocationConfidence.MEDIUM, LocationReason.FUSED_FIX)).text)
        assertEquals(Tone.Crit, CameraCopy.locationPill(reading(LocationConfidence.LOW, LocationReason.WEAK_SIGNAL)).tone)
    }

    @Test
    fun locationPillOffWhenNoReadingOrNoFix() {
        assertEquals("Location off", CameraCopy.locationPill(null).text)
        assertNull(CameraCopy.locationPill(null).tone)
        assertEquals("Location off", CameraCopy.locationPill(LocationReading(LocationEvidence.NONE, null, null, EnumBits.none())).text)
    }

    @Test
    fun locationDetailPrefersMockWarning() =
        assertEquals("mock location detected", CameraCopy.locationDetail(reading(LocationConfidence.HIGH, LocationReason.MOCK_LOCATION, LocationReason.RAW_GNSS_PASSED)))

    @Test
    fun locationDetailByConfidence() {
        assertEquals("spoof check passed", CameraCopy.locationDetail(reading(LocationConfidence.HIGH)))
        assertEquals("fused fix · no satellite check", CameraCopy.locationDetail(reading(LocationConfidence.MEDIUM)))
        assertEquals("weak satellite signal", CameraCopy.locationDetail(reading(LocationConfidence.LOW)))
        assertEquals("no location fix", CameraCopy.locationDetail(null))
        assertEquals("no location fix", CameraCopy.locationDetail(reading(LocationConfidence.LOW, LocationReason.NO_FIX)))
    }

    @Test
    fun metaOnlineAndOffline() {
        val r = reading(LocationConfidence.HIGH)
        assertEquals("4:12:09 PM · spoof check passed", CameraCopy.meta("4:12:09 PM", true, r))
        assertEquals("4:31:52 PM · no network · sealed on-device", CameraCopy.meta("4:31:52 PM", false, r))
    }

    @Test
    fun blurNamesCoverEveryClass() {
        assertEquals(listOf("Face", "Plate", "House number", "Area"), BlurClass.entries.map(CameraCopy::blurName))
    }

    private val frame = Dims(1000, 2000)

    @Test
    fun detectionsAreNormalizedAndLabelled() {
        val d = CameraCopy.detections(FrameDetections(frame, listOf(BlurProposal(Rect(100, 200, 300, 400), BlurClass.PLATE))), CaptureMode.POST)
        assertEquals(1, d.size)
        assertEquals("Plate · auto-blur", d[0].label)
        assertEquals(0.1f, d[0].box.left)
        assertEquals(0.1f, d[0].box.top)
        assertEquals(0.4f, d[0].box.right)
        assertEquals(0.3f, d[0].box.bottom)
    }

    @Test
    fun detectionsHiddenInCaseMode() =
        assertTrue(CameraCopy.detections(FrameDetections(frame, listOf(BlurProposal(Rect(0, 0, 10, 10), BlurClass.FACE))), CaptureMode.CASE).isEmpty())

    @Test
    fun disabledDetectionsAreDropped() =
        assertTrue(CameraCopy.detections(FrameDetections(frame, listOf(BlurProposal(Rect(0, 0, 10, 10), BlurClass.FACE, enabled = false))), CaptureMode.POST).isEmpty())

    @Test
    fun detectionsAreClippedToFrame() {
        val d = CameraCopy.detections(FrameDetections(frame, listOf(BlurProposal(Rect(900, 1900, 500, 500), BlurClass.FACE))), CaptureMode.POST)
        assertEquals(1f, d.single().box.right)
        assertEquals(1f, d.single().box.bottom)
    }

    @Test
    fun detectionsOutsideFrameAreDropped() =
        assertTrue(CameraCopy.detections(FrameDetections(frame, listOf(BlurProposal(Rect(5000, 5000, 10, 10), BlurClass.FACE))), CaptureMode.POST).isEmpty())

    @Test
    fun invalidFrameYieldsNothing() =
        assertTrue(CameraCopy.detections(FrameDetections(Dims(0, 0), listOf(BlurProposal(Rect(0, 0, 1, 1), BlurClass.FACE))), CaptureMode.POST).isEmpty())

    @Test
    fun detectionsAreCapped() {
        val many = List(100) { BlurProposal(Rect(it, it, 5, 5), BlurClass.OTHER) }
        assertEquals(CameraCopy.MAX_DETECTIONS, CameraCopy.detections(FrameDetections(frame, many), CaptureMode.POST).size)
    }

    @Test
    fun detectionsKeepInputOrder() {
        val ps = listOf(BlurProposal(Rect(10, 10, 5, 5), BlurClass.HOUSE_NO), BlurProposal(Rect(0, 0, 5, 5), BlurClass.PLATE))
        assertEquals(listOf("House number · auto-blur", "Plate · auto-blur"), CameraCopy.detections(FrameDetections(frame, ps), CaptureMode.POST).map { it.label })
    }

    @Test
    fun hintCountsBlurs() {
        assertEquals("2 things will be blurred when you post", CameraCopy.hint(2, CaptureMode.POST, LocationPrecision.AREA)?.title)
        assertEquals("1 thing will be blurred when you post", CameraCopy.hint(1, CaptureMode.POST, LocationPrecision.AREA)?.title)
    }

    @Test
    fun hintSubtitleFollowsPrecision() {
        assertEquals("Location will show as an area · precise stays sealed", CameraCopy.hint(2, CaptureMode.POST, LocationPrecision.AREA)?.subtitle)
        assertEquals("No location on the post · precise stays sealed", CameraCopy.hint(1, CaptureMode.POST, LocationPrecision.HIDDEN)?.subtitle)
    }

    @Test
    fun hintAbsentWithoutBlursForSafePrecision() {
        assertNull(CameraCopy.hint(0, CaptureMode.POST, LocationPrecision.AREA))
        assertNull(CameraCopy.hint(0, CaptureMode.POST, LocationPrecision.HIDDEN))
    }

    @Test
    fun hintWarnsAboutPreciseLocationEvenWithoutBlurs() {
        val h = CameraCopy.hint(0, CaptureMode.POST, LocationPrecision.EXACT)
        assertEquals("Your post will narrow down where you were", h?.title)
        assertEquals("Exact spot will show · shows where you were standing", h?.subtitle)
    }

    @Test
    fun hintNeverInCaseMode() = assertNull(CameraCopy.hint(3, CaptureMode.CASE, LocationPrecision.EXACT))

    @Test
    fun negativeCountTreatedAsNone() = assertNull(CameraCopy.hint(-1, CaptureMode.POST, LocationPrecision.AREA))

    @Test
    fun badgeMapsEveryStatus() {
        assertEquals(
            listOf(WitnessState.Sealed, WitnessState.Queued, WitnessState.Anchored, WitnessState.Unverified, WitnessState.Unanchored),
            AnchoringStatus.entries.map(CameraCopy::badge),
        )
    }

    @Test
    fun toastTitleNamesSecurity() {
        assertEquals("Sealed · StrongBox", CameraCopy.toastTitle(SecurityLevel.STRONG_BOX))
        assertEquals("Sealed", CameraCopy.toastTitle(null))
    }

    @Test
    fun toastStatusQueuedDependsOnNetwork() {
        assertEquals("Queued for anchoring", CameraCopy.toastStatus(AnchoringStatus.QUEUED, online = false).text)
        assertEquals("Anchoring…", CameraCopy.toastStatus(AnchoringStatus.QUEUED, online = true).text)
    }

    @Test
    fun toastStatusCoversEveryStatus() {
        for (s in AnchoringStatus.entries) assertTrue(CameraCopy.toastStatus(s, true).text.isNotBlank())
        assertEquals(Tone.Ok, CameraCopy.toastStatus(AnchoringStatus.ANCHORED, false).tone)
        assertEquals("Anchors after device setup", CameraCopy.toastStatus(AnchoringStatus.SEALED, true).text)
        assertEquals(Tone.Warn, CameraCopy.toastStatus(AnchoringStatus.FAILED, true).tone)
    }

    @Test
    fun flashAndLensToggle() {
        assertEquals(Flash.ON, Flash.OFF.next())
        assertEquals(Flash.OFF, Flash.ON.next())
        assertEquals(Lens.FRONT, Lens.BACK.flipped())
        assertEquals(Lens.BACK, Lens.FRONT.flipped())
    }

    @Test
    fun modesMatchCaptureModeOrder() =
        assertEquals(CaptureMode.entries.map { it.name.lowercase().replaceFirstChar(Char::uppercase) }, CameraCopy.MODES)

    @Test
    fun defaultStateIsPostAndOnline() {
        val s = CameraUiState()
        assertEquals(0, s.modeIndex)
        assertTrue(s.online)
        assertNull(s.toast)
    }
}
