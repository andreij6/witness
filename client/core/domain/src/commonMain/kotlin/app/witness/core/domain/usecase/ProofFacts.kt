package app.witness.core.domain.usecase

import app.witness.core.model.Anchor
import app.witness.core.model.CheckStrength
import app.witness.core.model.EnumBits
import app.witness.core.model.EvidenceKind
import app.witness.core.model.EvidenceState
import app.witness.core.model.KeyInfo
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationReason
import app.witness.core.model.Provisioning
import app.witness.core.model.SecurityLevel
import app.witness.core.model.VerdictFlag
import app.witness.core.model.VerdictFlags
import app.witness.core.model.WitnessClaim

object ProofFacts {
    const val LATE_ANCHOR_MS: Long = 24L * 3_600_000
    const val RECAPTURE_STRONG_BELOW: Int = 64
    const val RECAPTURE_WEAK_FROM: Int = 160

    fun flags(claim: WitnessClaim, anchor: Anchor?, key: KeyInfo?): VerdictFlags {
        var f = EnumBits.none<VerdictFlag>()
        if (anchor != null && anchor.anchoredAtMs - claim.time.wallMs > LATE_ANCHOR_MS) f += VerdictFlag.LATE_ANCHOR
        if (claim.time.clockMismatch()) f += VerdictFlag.CLOCK_MISMATCH
        if (key != null) {
            if (key.isDev) f += VerdictFlag.DEV_ATTESTATION
            if (anchor != null && key.history.revokedAfter(anchor.anchoredAtMs)) f += VerdictFlag.KEY_REVOKED_AFTER_ANCHOR
        }
        return f
    }

    fun evidence(claim: WitnessClaim, anchor: Anchor?, key: KeyInfo?): List<EvidenceState> {
        val checks = claim.checks
        val hardware = when (key?.security ?: checks.keySecurity) {
            SecurityLevel.STRONG_BOX -> CheckStrength.STRONG
            SecurityLevel.TEE -> CheckStrength.MODERATE
            SecurityLevel.DEV -> CheckStrength.WEAK
        }
        val integrity = when {
            !checks.bootTrusted -> CheckStrength.WEAK
            key?.provisioning == Provisioning.DEV -> CheckStrength.WEAK
            key?.provisioning == Provisioning.FACTORY -> CheckStrength.MODERATE
            key == null -> CheckStrength.MODERATE
            else -> CheckStrength.STRONG
        }
        val time = when {
            claim.time.clockMismatch() -> CheckStrength.WEAK
            claim.time.gnssMs != null -> CheckStrength.STRONG
            else -> CheckStrength.MODERATE
        }
        val reasons = claim.location.reasons
        val location = when {
            LocationReason.NO_FIX in reasons -> CheckStrength.UNAVAILABLE
            checks.mockLocation || LocationReason.MOCK_LOCATION in reasons -> CheckStrength.WEAK
            claim.location.confidence == LocationConfidence.HIGH -> CheckStrength.STRONG
            claim.location.confidence == LocationConfidence.MEDIUM -> CheckStrength.MODERATE
            else -> CheckStrength.WEAK
        }
        val recapture = when {
            checks.recapture < RECAPTURE_STRONG_BELOW -> CheckStrength.STRONG
            checks.recapture < RECAPTURE_WEAK_FROM -> CheckStrength.MODERATE
            else -> CheckStrength.WEAK
        }
        val anchored = when {
            anchor == null -> CheckStrength.UNAVAILABLE
            anchor.anchoredAtMs - claim.time.wallMs > LATE_ANCHOR_MS -> CheckStrength.MODERATE
            else -> CheckStrength.STRONG
        }
        return listOf(
            EvidenceState(EvidenceKind.HARDWARE_KEY, hardware),
            EvidenceState(EvidenceKind.DEVICE_INTEGRITY, integrity),
            EvidenceState(EvidenceKind.TIME, time),
            EvidenceState(EvidenceKind.LOCATION, location),
            EvidenceState(EvidenceKind.RECAPTURE, recapture),
            EvidenceState(EvidenceKind.ANCHOR, anchored),
        )
    }
}
