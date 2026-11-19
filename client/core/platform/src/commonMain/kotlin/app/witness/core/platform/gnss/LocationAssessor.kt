package app.witness.core.platform.gnss

import app.witness.core.model.EnumBits
import app.witness.core.model.GnssCheck
import app.witness.core.model.GnssChecks
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationReason
import app.witness.core.model.LocationReasons

data class Fix(val latE7: Int, val lonE7: Int, val accuracyCm: Int, val isMock: Boolean) {
    val lat: Double get() = latE7 / 1e7
    val lon: Double get() = lonE7 / 1e7
}

data class LocationVerdict(val confidence: LocationConfidence, val reasons: LocationReasons, val checks: GnssChecks)

object LocationAssessor {
    const val MAX_HIGH_ACCURACY_CM = 5_000
    const val WEAK_CN0_DBHZ = 20.0

    fun assess(fix: Fix?, gnss: GnssAssessment, networkCountry: String?): LocationVerdict {
        var checks = gnssChecks(gnss)
        if (fix == null) return LocationVerdict(LocationConfidence.LOW, EnumBits.of(LocationReason.NO_FIX), checks)
        if (fix.isMock) return LocationVerdict(LocationConfidence.LOW, EnumBits.of(LocationReason.MOCK_LOCATION), checks)
        val country = networkCountry?.takeIf { it.isNotBlank() }?.let { CountryBounds.contains(it, fix.lat, fix.lon) }
        if (country == false) return LocationVerdict(LocationConfidence.LOW, EnumBits.of(LocationReason.MCC_MISMATCH), checks)
        if (country == true) checks += GnssCheck.MCC_MATCH
        if (gnss.raw) {
            var reasons = EnumBits.none<LocationReason>()
            if (gnss.cn0 == Outcome.FAIL) reasons += LocationReason.CN0_ANOMALY
            if (gnss.agc == Outcome.FAIL) reasons += LocationReason.AGC_JUMP
            if (gnss.pseudorange == Outcome.FAIL) reasons += LocationReason.PSEUDORANGE_RESIDUAL
            if (gnss.clock == Outcome.FAIL) reasons += LocationReason.CLOCK_DRIFT
            if (!reasons.isEmpty) return LocationVerdict(LocationConfidence.LOW, reasons, checks)
            if (fix.accuracyCm <= MAX_HIGH_ACCURACY_CM) {
                return LocationVerdict(LocationConfidence.HIGH, EnumBits.of(LocationReason.RAW_GNSS_PASSED), checks)
            }
            return LocationVerdict(LocationConfidence.MEDIUM, EnumBits.of(LocationReason.WEAK_SIGNAL), checks)
        }
        var reasons = EnumBits.of(LocationReason.FUSED_FIX)
        if (gnss.satellites in 1 until GnssHeuristics.Thresholds().minSatellites) reasons += LocationReason.INDOOR
        if (gnss.satellites > 0 && gnss.meanCn0 < WEAK_CN0_DBHZ) reasons += LocationReason.WEAK_SIGNAL
        return LocationVerdict(LocationConfidence.MEDIUM, reasons, checks)
    }

    fun gnssChecks(gnss: GnssAssessment): GnssChecks {
        var c = EnumBits.none<GnssCheck>()
        if (!gnss.raw) return c
        c += GnssCheck.RAW_AVAILABLE
        if (gnss.cn0 == Outcome.PASS) c += GnssCheck.CN0_CONSISTENT
        if (gnss.agc == Outcome.PASS) c += GnssCheck.AGC_STABLE
        if (gnss.pseudorange == Outcome.PASS) c += GnssCheck.PSEUDORANGE_CONSISTENT
        if (gnss.clock == Outcome.PASS) c += GnssCheck.CLOCK_CONTINUOUS
        if (gnss.gnssTimeMs != null) c += GnssCheck.GNSS_TIME
        return c
    }
}
