package app.witness.core.model

data class TimeEvidence(
    val wallMs: Long,
    val gnssMs: Long? = null,
    val elapsedNs: Long,
    val bootCount: Int,
) {
    val clockSkewMs: Long? get() = gnssMs?.let { kotlin.math.abs(wallMs - it) }

    fun clockMismatch(toleranceMs: Long = CLOCK_TOLERANCE_MS): Boolean = (clockSkewMs ?: 0) > toleranceMs

    companion object {
        const val CLOCK_TOLERANCE_MS: Long = 60_000
    }
}

enum class LocationConfidence { HIGH, MEDIUM, LOW }

enum class LocationReason {
    RAW_GNSS_PASSED,
    FUSED_FIX,
    NO_FIX,
    WEAK_SIGNAL,
    CN0_ANOMALY,
    AGC_JUMP,
    PSEUDORANGE_RESIDUAL,
    CLOCK_DRIFT,
    MOCK_LOCATION,
    MCC_MISMATCH,
    INDOOR,
}

typealias LocationReasons = EnumBits<LocationReason>

data class LocationEvidence(
    val commit: ByteString?,
    val confidence: LocationConfidence,
    val reasons: LocationReasons,
) {
    companion object {
        val NONE: LocationEvidence = LocationEvidence(null, LocationConfidence.LOW, EnumBits.of(LocationReason.NO_FIX))
    }
}

data class LocationOpening(
    val salt: ByteString,
    val latE7: Int,
    val lonE7: Int,
    val accuracyCm: Int,
) {
    val inRange: Boolean get() = latE7 in -LAT_E7_MAX..LAT_E7_MAX && lonE7 in -LON_E7_MAX..LON_E7_MAX && accuracyCm >= 0

    override fun toString(): String = "LocationOpening(redacted)"

    companion object {
        const val LAT_E7_MAX: Int = 900_000_000
        const val LON_E7_MAX: Int = 1_800_000_000
        const val SALT_SIZE: Int = 32
    }
}

enum class SecurityLevel { STRONG_BOX, TEE, DEV }

enum class BootState { VERIFIED, SELF_SIGNED, UNVERIFIED, FAILED }

enum class GnssCheck {
    RAW_AVAILABLE,
    CN0_CONSISTENT,
    AGC_STABLE,
    PSEUDORANGE_CONSISTENT,
    CLOCK_CONTINUOUS,
    MCC_MATCH,
    GNSS_TIME,
}

typealias GnssChecks = EnumBits<GnssCheck>

data class Checks(
    val keySecurity: SecurityLevel,
    val boot: BootState,
    val deviceLocked: Boolean,
    val mockLocation: Boolean,
    val gnss: GnssChecks,
    val recapture: Int,
) {
    val bootTrusted: Boolean get() = boot == BootState.VERIFIED && deviceLocked
}

enum class CheckStrength { STRONG, MODERATE, WEAK, UNAVAILABLE }

enum class EvidenceKind { HARDWARE_KEY, DEVICE_INTEGRITY, TIME, LOCATION, RECAPTURE, ANCHOR }

data class EvidenceState(val kind: EvidenceKind, val strength: CheckStrength)
