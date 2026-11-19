package app.witness.core.platform.gnss

import kotlin.math.abs
import kotlin.math.sqrt

enum class Outcome { PASS, FAIL, UNKNOWN }

data class GnssAssessment(
    val raw: Boolean,
    val cn0: Outcome,
    val agc: Outcome,
    val pseudorange: Outcome,
    val clock: Outcome,
    val gnssTimeMs: Long?,
    val satellites: Int,
    val meanCn0: Double,
) {
    val anyFailed: Boolean get() = cn0 == Outcome.FAIL || agc == Outcome.FAIL || pseudorange == Outcome.FAIL || clock == Outcome.FAIL

    companion object {
        val EMPTY = GnssAssessment(false, Outcome.UNKNOWN, Outcome.UNKNOWN, Outcome.UNKNOWN, Outcome.UNKNOWN, null, 0, 0.0)
    }
}

class GnssHeuristics(private val t: Thresholds = Thresholds()) {
    data class Thresholds(
        val minSatellites: Int = 4,
        val minEpochs: Int = 3,
        val minCn0Spread: Double = 1.5,
        val maxMeanCn0: Double = 55.0,
        val maxAgcJumpDb: Double = 6.0,
        val maxRangeRateMps: Double = 1_200.0,
        val maxRangeAccelMps2: Double = 15.0,
        val maxClockResidualNs: Double = 5_000.0,
        val maxDriftJumpNsPerS: Double = 200.0,
        val leapSeconds: Int = 18,
    )

    fun assess(epochs: List<GnssEpoch>): GnssAssessment {
        if (epochs.isEmpty()) return GnssAssessment.EMPTY
        val last = epochs.last()
        val raw = epochs.size >= t.minEpochs && last.satellites.size >= t.minSatellites
        val meanCn0 = last.satellites.meanCn0()
        if (!raw) return GnssAssessment.EMPTY.copy(satellites = last.satellites.size, meanCn0 = meanCn0)
        return GnssAssessment(
            raw = true,
            cn0 = cn0(epochs),
            agc = agc(epochs),
            pseudorange = pseudorange(epochs),
            clock = clock(epochs),
            gnssTimeMs = last.gpsTimeNanos()?.let { gpsNanosToUnixMs(it, t.leapSeconds) },
            satellites = last.satellites.size,
            meanCn0 = meanCn0,
        )
    }

    fun cn0(epochs: List<GnssEpoch>): Outcome {
        var spreadSum = 0.0
        var meanSum = 0.0
        var counted = 0
        for (e in epochs) {
            val n = e.satellites.size
            if (n < t.minSatellites) continue
            var sum = 0.0
            var sq = 0.0
            for (s in e.satellites) {
                sum += s.cn0DbHz
                sq += s.cn0DbHz * s.cn0DbHz
            }
            val mean = sum / n
            spreadSum += sqrt((sq / n - mean * mean).coerceAtLeast(0.0))
            meanSum += mean
            counted++
        }
        if (counted == 0) return Outcome.UNKNOWN
        val spread = spreadSum / counted
        val mean = meanSum / counted
        return if (spread < t.minCn0Spread || mean > t.maxMeanCn0) Outcome.FAIL else Outcome.PASS
    }

    fun agc(epochs: List<GnssEpoch>): Outcome {
        var previous = Double.NaN
        var seen = 0
        for (e in epochs) {
            var sum = 0.0
            var n = 0
            for (s in e.satellites) {
                val a = s.agcDb ?: continue
                sum += a
                n++
            }
            if (n == 0) continue
            val mean = sum / n
            if (!previous.isNaN() && abs(mean - previous) > t.maxAgcJumpDb) return Outcome.FAIL
            previous = mean
            seen++
        }
        return if (seen >= 2) Outcome.PASS else Outcome.UNKNOWN
    }

    fun pseudorange(epochs: List<GnssEpoch>): Outcome {
        val lastRate = HashMap<Int, Double>(64)
        var previousNs = Long.MIN_VALUE
        var compared = 0
        for (e in epochs) {
            val dtS = if (previousNs == Long.MIN_VALUE) 0.0 else (e.elapsedNs - previousNs) / 1e9
            for (s in e.satellites) {
                if (abs(s.pseudorangeRateMps) > t.maxRangeRateMps) return Outcome.FAIL
                val before = lastRate.put(s.key, s.pseudorangeRateMps)
                if (before != null && dtS > 0) {
                    if (abs(s.pseudorangeRateMps - before) / dtS > t.maxRangeAccelMps2) return Outcome.FAIL
                    compared++
                }
            }
            previousNs = e.elapsedNs
        }
        return if (compared > 0) Outcome.PASS else Outcome.UNKNOWN
    }

    fun clock(epochs: List<GnssEpoch>): Outcome {
        var compared = 0
        for (i in 1 until epochs.size) {
            val a = epochs[i - 1]
            val b = epochs[i]
            if (a.discontinuityCount != b.discontinuityCount) return Outcome.FAIL
            val ba = a.fullBiasNanos ?: continue
            val bb = b.fullBiasNanos ?: continue
            val da = a.driftNsPerS ?: continue
            val db = b.driftNsPerS ?: continue
            if (abs(db - da) > t.maxDriftJumpNsPerS) return Outcome.FAIL
            val dtS = (b.timeNanos - a.timeNanos) / 1e9
            val biasChange = (bb - ba).toDouble() + (b.biasNanos - a.biasNanos)
            val expected = ((da + db) / 2) * dtS
            if (abs(biasChange - expected) > t.maxClockResidualNs) return Outcome.FAIL
            compared++
        }
        return if (compared > 0) Outcome.PASS else Outcome.UNKNOWN
    }

    companion object {
        const val GPS_EPOCH_UNIX_MS = 315_964_800_000L

        fun gpsNanosToUnixMs(gpsNanos: Long, leapSeconds: Int): Long = gpsNanos / 1_000_000 + GPS_EPOCH_UNIX_MS - leapSeconds * 1_000L

        private fun List<SatelliteSample>.meanCn0(): Double {
            if (isEmpty()) return 0.0
            var sum = 0.0
            for (s in this) sum += s.cn0DbHz
            return sum / size
        }
    }
}
