package app.witness.core.platform.gnss

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class GnssHeuristicsTest {
    private val h = GnssHeuristics()

    @Test
    fun emptyIsNotRaw() {
        assertEquals(GnssAssessment.EMPTY, h.assess(emptyList()))
    }

    @Test
    fun genuineSignalsPassAll() {
        val a = h.assess(goodEpochs())
        assertTrue(a.raw)
        assertEquals(Outcome.PASS, a.cn0)
        assertEquals(Outcome.PASS, a.pseudorange)
        assertEquals(Outcome.PASS, a.clock)
        assertEquals(Outcome.UNKNOWN, a.agc)
        assertFalse(a.anyFailed)
        assertEquals(5, a.satellites)
        assertEquals(35.0, a.meanCn0)
    }

    @Test
    fun tooFewEpochsOrSatellitesIsNotRaw() {
        assertFalse(h.assess(goodEpochs(2)).raw)
        val few = List(5) { epoch(it, sats(30.0, 31.0, 32.0)) }
        val a = h.assess(few)
        assertFalse(a.raw)
        assertEquals(3, a.satellites)
    }

    @Test
    fun uniformCn0IsSpoofLike() {
        val flat = List(5) { epoch(it, sats(42.0, 42.2, 41.9, 42.1, 42.0)) }
        assertEquals(Outcome.FAIL, h.cn0(flat))
    }

    @Test
    fun implausiblyStrongCn0Fails() {
        val strong = List(5) { epoch(it, sats(55.0, 60.0, 65.0, 58.0)) }
        assertEquals(Outcome.FAIL, h.cn0(strong))
    }

    @Test
    fun cn0UnknownWithoutEnoughSatellites() {
        assertEquals(Outcome.UNKNOWN, h.cn0(listOf(epoch(0, sats(30.0)))))
    }

    @Test
    fun agcJumpFails() {
        val e = listOf(epoch(0, sats(30.0, 35.0, 40.0, 45.0, agc = 1.0)), epoch(1, sats(30.0, 35.0, 40.0, 45.0, agc = 9.0)))
        assertEquals(Outcome.FAIL, h.agc(e))
    }

    @Test
    fun agcSteadyPasses() {
        val e = List(4) { epoch(it, sats(30.0, 35.0, 40.0, 45.0, agc = 1.0 + it)) }
        assertEquals(Outcome.PASS, h.agc(e))
    }

    @Test
    fun rangeRateJumpFails() {
        val e = listOf(epoch(0), epoch(1, sats(30.0, 35.0, 40.0, 25.0, 45.0) { 100.0 * it + 50 }))
        assertEquals(Outcome.FAIL, h.pseudorange(e))
    }

    @Test
    fun impossibleRangeRateFails() {
        assertEquals(Outcome.FAIL, h.pseudorange(listOf(epoch(0, sats(30.0, 35.0, 40.0, 25.0) { 5_000.0 }))))
    }

    @Test
    fun pseudorangeUnknownWithSingleEpoch() {
        assertEquals(Outcome.UNKNOWN, h.pseudorange(listOf(epoch(0))))
    }

    @Test
    fun clockDiscontinuityFails() {
        assertEquals(Outcome.FAIL, h.clock(listOf(epoch(0), epoch(1, discontinuity = 1))))
    }

    @Test
    fun clockBiasJumpFails() {
        assertEquals(Outcome.FAIL, h.clock(listOf(epoch(0), epoch(1, bias = 1_000_000.0))))
    }

    @Test
    fun driftJumpFails() {
        assertEquals(Outcome.FAIL, h.clock(listOf(epoch(0, drift = 10.0), epoch(1, drift = 900.0))))
    }

    @Test
    fun clockUnknownWithoutBias() {
        assertEquals(Outcome.UNKNOWN, h.clock(listOf(epoch(0, fullBias = null), epoch(1, fullBias = null))))
    }

    @Test
    fun gnssTimeConvertsToUnix() {
        val gpsNanos = (1_700_000_000_000L - GnssHeuristics.GPS_EPOCH_UNIX_MS + 18_000) * 1_000_000
        assertEquals(1_700_000_000_000L, GnssHeuristics.gpsNanosToUnixMs(gpsNanos, 18))
    }

    @Test
    fun gnssTimeAbsentWithoutBias() {
        assertNull(h.assess(List(5) { epoch(it, fullBias = null) }).gnssTimeMs)
    }

    @Test
    fun assessmentIsDeterministic() {
        val e = goodEpochs()
        assertEquals(h.assess(e), h.assess(e))
    }
}
