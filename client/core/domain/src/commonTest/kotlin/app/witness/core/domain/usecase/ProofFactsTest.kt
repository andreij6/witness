package app.witness.core.domain.usecase

import app.witness.core.model.Anchor
import app.witness.core.model.BootState
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.CheckStrength
import app.witness.core.model.Checks
import app.witness.core.model.ClaimKind
import app.witness.core.model.Dims
import app.witness.core.model.EnumBits
import app.witness.core.model.EvidenceKind
import app.witness.core.model.Freshness
import app.witness.core.model.KeyHistory
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.KeyRef
import app.witness.core.model.KeyStatus
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationReason
import app.witness.core.model.Provisioning
import app.witness.core.model.SecurityLevel
import app.witness.core.model.StatusChange
import app.witness.core.model.TimeEvidence
import app.witness.core.model.VerdictFlag
import app.witness.core.model.WitnessClaim
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ProofFactsTest {
    private val hour = 3_600_000L
    private val keyId = KeyId.fromBytes(ByteArray(16))
    private val claim = WitnessClaim(
        CaptureId.of(1, 0)!!, ClaimKind.Original, ByteString.EMPTY, ByteString.EMPTY, ByteString.EMPTY, Dims(1, 1),
        TimeEvidence(1_000, 1_000, 0, 0),
        LocationEvidence(null, LocationConfidence.HIGH, EnumBits.of(LocationReason.RAW_GNSS_PASSED)),
        Checks(SecurityLevel.STRONG_BOX, BootState.VERIFIED, true, false, EnumBits.none(), 10),
        KeyRef(keyId, 0), "1",
    )
    private val key = KeyInfo(keyId, SecurityLevel.STRONG_BOX, Provisioning.RKP, Freshness.FRESH, 0, 100 * hour, KeyHistory.enrolled(0))

    private fun strengths(c: WitnessClaim = claim, a: Anchor? = Anchor(1, 2_000), k: KeyInfo? = key) =
        ProofFacts.evidence(c, a, k).associate { it.kind to it.strength }

    @Test
    fun bestCaseIsAllStrong() {
        val s = strengths()
        assertEquals(EvidenceKind.entries.toSet(), s.keys)
        assertTrue(s.values.all { it == CheckStrength.STRONG })
        assertTrue(ProofFacts.flags(claim, Anchor(1, 2_000), key).isEmpty)
    }

    @Test
    fun weakerSignals() {
        assertEquals(CheckStrength.MODERATE, strengths(k = key.copy(security = SecurityLevel.TEE))[EvidenceKind.HARDWARE_KEY])
        assertEquals(CheckStrength.WEAK, strengths(k = key.copy(security = SecurityLevel.DEV))[EvidenceKind.HARDWARE_KEY])
        assertEquals(CheckStrength.MODERATE, strengths(k = key.copy(provisioning = Provisioning.FACTORY))[EvidenceKind.DEVICE_INTEGRITY])
        assertEquals(CheckStrength.WEAK, strengths(k = key.copy(provisioning = Provisioning.DEV))[EvidenceKind.DEVICE_INTEGRITY])
        assertEquals(CheckStrength.MODERATE, strengths(k = null)[EvidenceKind.DEVICE_INTEGRITY])
        assertEquals(CheckStrength.WEAK, strengths(claim.copy(checks = claim.checks.copy(deviceLocked = false)))[EvidenceKind.DEVICE_INTEGRITY])
        assertEquals(CheckStrength.MODERATE, strengths(claim.copy(time = claim.time.copy(gnssMs = null)))[EvidenceKind.TIME])
        assertEquals(CheckStrength.WEAK, strengths(claim.copy(time = claim.time.copy(gnssMs = 1_000 + 5 * 60_000)))[EvidenceKind.TIME])
        assertEquals(CheckStrength.MODERATE, strengths(claim.copy(location = claim.location.copy(confidence = LocationConfidence.MEDIUM)))[EvidenceKind.LOCATION])
        assertEquals(CheckStrength.WEAK, strengths(claim.copy(location = claim.location.copy(confidence = LocationConfidence.LOW)))[EvidenceKind.LOCATION])
        assertEquals(CheckStrength.UNAVAILABLE, strengths(claim.copy(location = LocationEvidence.NONE))[EvidenceKind.LOCATION])
        assertEquals(CheckStrength.WEAK, strengths(claim.copy(checks = claim.checks.copy(mockLocation = true)))[EvidenceKind.LOCATION])
        assertEquals(CheckStrength.MODERATE, strengths(claim.copy(checks = claim.checks.copy(recapture = 64)))[EvidenceKind.RECAPTURE])
        assertEquals(CheckStrength.WEAK, strengths(claim.copy(checks = claim.checks.copy(recapture = 160)))[EvidenceKind.RECAPTURE])
        assertEquals(CheckStrength.UNAVAILABLE, strengths(a = null)[EvidenceKind.ANCHOR])
        assertEquals(CheckStrength.MODERATE, strengths(a = Anchor(1, 1_000 + 25 * hour))[EvidenceKind.ANCHOR])
    }

    @Test
    fun flagsCoverClockAnchorDevAndRevocation() {
        val late = ProofFacts.flags(claim, Anchor(1, 1_000 + 24 * hour + 1), key)
        assertEquals(EnumBits.of(VerdictFlag.LATE_ANCHOR), late)
        assertTrue(ProofFacts.flags(claim, Anchor(1, 1_000 + 24 * hour), key).isEmpty)
        val skew = ProofFacts.flags(claim.copy(time = claim.time.copy(gnssMs = 1_000_000)), null, null)
        assertEquals(EnumBits.of(VerdictFlag.CLOCK_MISMATCH), skew)
        val revoked = key.copy(history = KeyHistory.enrolled(0).plus(StatusChange(KeyStatus.REVOKED, 5_000))!!)
        assertTrue(VerdictFlag.KEY_REVOKED_AFTER_ANCHOR in ProofFacts.flags(claim, Anchor(1, 2_000), revoked))
        assertTrue(VerdictFlag.KEY_REVOKED_AFTER_ANCHOR !in ProofFacts.flags(claim, Anchor(1, 6_000), revoked))
        assertTrue(VerdictFlag.DEV_ATTESTATION in ProofFacts.flags(claim, null, key.copy(provisioning = Provisioning.DEV)))
    }
}
