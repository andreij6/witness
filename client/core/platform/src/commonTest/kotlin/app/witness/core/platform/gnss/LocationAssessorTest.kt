package app.witness.core.platform.gnss

import app.witness.core.model.GnssCheck
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationReason
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class LocationAssessorTest {
    private val paris = Fix(488_583_700, 22_944_810, 500, false)
    private val good = GnssHeuristics().assess(goodEpochs())

    @Test
    fun noFixIsLow() {
        val v = LocationAssessor.assess(null, good, "fr")
        assertEquals(LocationConfidence.LOW, v.confidence)
        assertTrue(LocationReason.NO_FIX in v.reasons)
    }

    @Test
    fun mockIsLowEvenWithGoodSignals() {
        val v = LocationAssessor.assess(paris.copy(isMock = true), good, "fr")
        assertEquals(LocationConfidence.LOW, v.confidence)
        assertTrue(LocationReason.MOCK_LOCATION in v.reasons)
    }

    @Test
    fun mccMismatchIsLow() {
        val v = LocationAssessor.assess(paris, good, "us")
        assertEquals(LocationConfidence.LOW, v.confidence)
        assertTrue(LocationReason.MCC_MISMATCH in v.reasons)
        assertFalse(GnssCheck.MCC_MATCH in v.checks)
    }

    @Test
    fun rawPassingIsHighWithChecks() {
        val v = LocationAssessor.assess(paris, good, "FR")
        assertEquals(LocationConfidence.HIGH, v.confidence)
        assertTrue(LocationReason.RAW_GNSS_PASSED in v.reasons)
        listOf(GnssCheck.RAW_AVAILABLE, GnssCheck.CN0_CONSISTENT, GnssCheck.PSEUDORANGE_CONSISTENT, GnssCheck.CLOCK_CONTINUOUS, GnssCheck.MCC_MATCH, GnssCheck.GNSS_TIME)
            .forEach { assertTrue(it in v.checks, it.name) }
        assertFalse(GnssCheck.AGC_STABLE in v.checks)
    }

    @Test
    fun unknownCountryNeitherPassesNorFails() {
        val v = LocationAssessor.assess(paris, good, "zz")
        assertEquals(LocationConfidence.HIGH, v.confidence)
        assertFalse(GnssCheck.MCC_MATCH in v.checks)
        assertEquals(LocationConfidence.HIGH, LocationAssessor.assess(paris, good, null).confidence)
        assertEquals(LocationConfidence.HIGH, LocationAssessor.assess(paris, good, "").confidence)
    }

    @Test
    fun failedChecksAreLowWithReasons() {
        val bad = good.copy(cn0 = Outcome.FAIL, clock = Outcome.FAIL, agc = Outcome.FAIL, pseudorange = Outcome.FAIL)
        val v = LocationAssessor.assess(paris, bad, "fr")
        assertEquals(LocationConfidence.LOW, v.confidence)
        listOf(LocationReason.CN0_ANOMALY, LocationReason.CLOCK_DRIFT, LocationReason.AGC_JUMP, LocationReason.PSEUDORANGE_RESIDUAL)
            .forEach { assertTrue(it in v.reasons) }
    }

    @Test
    fun poorAccuracyDemotesToMedium() {
        val v = LocationAssessor.assess(paris.copy(accuracyCm = 10_000), good, "fr")
        assertEquals(LocationConfidence.MEDIUM, v.confidence)
        assertTrue(LocationReason.WEAK_SIGNAL in v.reasons)
    }

    @Test
    fun fusedOnlyIsMedium() {
        val v = LocationAssessor.assess(paris, GnssAssessment.EMPTY, "fr")
        assertEquals(LocationConfidence.MEDIUM, v.confidence)
        assertTrue(LocationReason.FUSED_FIX in v.reasons)
        assertTrue(v.checks.contains(GnssCheck.MCC_MATCH))
        assertFalse(GnssCheck.RAW_AVAILABLE in v.checks)
    }

    @Test
    fun fewWeakSatellitesMarkIndoor() {
        val v = LocationAssessor.assess(paris, GnssAssessment.EMPTY.copy(satellites = 2, meanCn0 = 15.0), null)
        assertTrue(LocationReason.INDOOR in v.reasons)
        assertTrue(LocationReason.WEAK_SIGNAL in v.reasons)
    }

    @Test
    fun gnssChecksEmptyWithoutRaw() {
        assertTrue(LocationAssessor.gnssChecks(GnssAssessment.EMPTY).isEmpty)
    }
}

class CountryBoundsTest {
    @Test
    fun citiesResolve() {
        assertEquals(true, CountryBounds.contains("us", 40.7128, -74.0060))
        assertEquals(true, CountryBounds.contains("us", 61.2, -149.9))
        assertEquals(true, CountryBounds.contains("us", 21.3, -157.8))
        assertEquals(true, CountryBounds.contains("JP", 35.68, 139.69))
        assertEquals(true, CountryBounds.contains("au", -33.87, 151.21))
        assertEquals(true, CountryBounds.contains("es", 28.1, -15.4))
        assertEquals(true, CountryBounds.contains("br", -23.55, -46.63))
    }

    @Test
    fun foreignPointsAreOutside() {
        assertEquals(false, CountryBounds.contains("us", 48.85, 2.35))
        assertEquals(false, CountryBounds.contains("au", 35.68, 139.69))
        assertEquals(false, CountryBounds.contains("gb", -33.87, 151.21))
    }

    @Test
    fun marginCoversBorders() {
        assertEquals(true, CountryBounds.contains("de", 47.0, 7.6))
    }

    @Test
    fun unknownCountryIsNull() {
        assertNull(CountryBounds.contains("zz", 0.0, 0.0))
    }

    @Test
    fun tableIsWellFormed() {
        assertTrue(CountryBounds.known.size >= 25)
        for (iso in CountryBounds.known) assertEquals(2, iso.length)
    }
}
