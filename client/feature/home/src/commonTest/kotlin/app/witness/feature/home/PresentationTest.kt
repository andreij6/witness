package app.witness.feature.home

import app.witness.core.designsystem.WitnessState
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.SecurityLevel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class PresentationTest {
    @Test
    fun statusMapsToWitnessState() {
        assertEquals(WitnessState.Anchored, capture(1, status = AnchoringStatus.ANCHORED).witnessState())
        assertEquals(WitnessState.Queued, capture(1, status = AnchoringStatus.QUEUED).witnessState())
        assertEquals(WitnessState.Sealed, capture(1, status = AnchoringStatus.SEALED).witnessState())
        assertEquals(WitnessState.Sealed, capture(1, status = AnchoringStatus.FAILED).witnessState())
        assertEquals(WitnessState.Unverified, capture(1, status = AnchoringStatus.UNVERIFIED).witnessState())
    }

    @Test
    fun importsAreAlwaysUnverified() = assertEquals(WitnessState.Unverified, capture(1, origin = CaptureOrigin.IMPORT).witnessState())

    @Test
    fun headlinePrefersCaption() = assertEquals("Hail!", capture(1, caption = "Hail!", label = "Porch").headline())

    @Test
    fun shortTitlePrefersLabel() = assertEquals("Porch", capture(1, caption = "Hail!", label = "Porch").shortTitle())

    @Test
    fun blankTextFallsBack() {
        assertEquals(SEALED_PHOTO, capture(1, caption = "  ", label = "").headline())
        assertEquals(IMPORTED_PHOTO, capture(1, origin = CaptureOrigin.IMPORT).shortTitle())
    }

    @Test
    fun titlesAreTrimmed() = assertEquals("Porch", capture(1, label = "  Porch ").shortTitle())

    @Test
    fun photoLabels() {
        assertEquals("Photo 3", photoLabel(3))
        assertNull(photoLabel(null))
    }

    @Test
    fun securityLabels() {
        assertEquals("StrongBox", SecurityLevel.STRONG_BOX.label())
        assertEquals("TEE", SecurityLevel.TEE.label())
        assertEquals("Dev key", SecurityLevel.DEV.label())
    }
}
