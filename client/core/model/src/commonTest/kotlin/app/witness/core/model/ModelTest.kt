package app.witness.core.model

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ModelTest {
    private val hash = ByteString.of(ByteArray(32))
    private val checks = Checks(SecurityLevel.STRONG_BOX, BootState.VERIFIED, true, false, EnumBits.none(), 0)

    private fun draft(kind: ClaimKind = ClaimKind.Original) = ClaimDraft(
        captureId = CaptureId.of(5, 1)!!,
        kind = kind,
        assetHash = hash,
        pixelHash = hash,
        fpCommit = hash,
        dims = Dims(4032, 3024),
        time = TimeEvidence(1_000, 1_500, 10, 2),
        location = LocationEvidence.NONE,
        checks = checks,
    )

    @Test
    fun timeEvidenceSkew() {
        assertEquals(500, TimeEvidence(1_000, 1_500, 0, 0).clockSkewMs)
        assertEquals(500, TimeEvidence(1_500, 1_000, 0, 0).clockSkewMs)
        assertNull(TimeEvidence(1_000, null, 0, 0).clockSkewMs)
        assertFalse(TimeEvidence(0, 60_000, 0, 0).clockMismatch())
        assertTrue(TimeEvidence(0, 60_001, 0, 0).clockMismatch())
        assertFalse(TimeEvidence(0, null, 0, 0).clockMismatch())
    }

    @Test
    fun locationOpeningRangeAndRedaction() {
        val o = LocationOpening(hash, 900_000_000, -1_800_000_000, 100)
        assertTrue(o.inRange)
        assertFalse(o.copy(latE7 = 900_000_001).inRange)
        assertFalse(o.copy(lonE7 = 1_800_000_001).inRange)
        assertFalse(o.copy(accuracyCm = -1).inRange)
        assertFalse(o.toString().contains("900"))
    }

    @Test
    fun checksBootTrust() {
        assertTrue(checks.bootTrusted)
        assertFalse(checks.copy(deviceLocked = false).bootTrusted)
        assertFalse(checks.copy(boot = BootState.SELF_SIGNED).bootTrusted)
    }

    @Test
    fun claimDraftFinalizesWithKeyRef() {
        val key = KeyRef(KeyId.fromBytes(ByteArray(16)), 7)
        val claim = draft().finalize(key, "1a")
        assertEquals(key, claim.keyRef)
        assertEquals("1a", claim.certSerial)
        assertEquals(2, claim.version)
        assertTrue(claim.isOriginal)
        assertTrue(claim.kind.isPresentationOnly)
    }

    @Test
    fun derivativeKindCarriesEdits() {
        val parent = RecordId.fromBytes(ByteArray(10))
        val d = ClaimKind.Derivative(parent, CaptureId.of(1, 0)!!, listOf(Edit.Resize(10, 10)))
        assertTrue(d.isPresentationOnly)
        assertFalse(d.copy(edits = listOf(Edit.Crop(Rect(0, 0, 1, 1)))).isPresentationOnly)
        assertFalse(draft(d).finalize(KeyRef(KeyId.fromBytes(ByteArray(16)), 1), null).isOriginal)
        assertEquals(emptyList(), ClaimKind.Original.edits)
    }

    @Test
    fun dims() {
        assertTrue(Dims(1, 2).isPortrait)
        assertFalse(Dims(2, 1).isPortrait)
        assertFalse(Dims(0, 1).isValid)
    }

    @Test
    fun regionNormalizationMatchesRust() {
        assertEquals(Region(0, 0, 65536, 65536), Region.normalized(0, 0, 100, 50, 100, 50))
        assertEquals(Region(32768, 0, 32768, 65536), Region.normalized(50, 0, 50, 50, 100, 50))
        assertEquals(Region(32768, 0, 32768, 65536), Region.normalized(50, 0, 500, 500, 100, 50))
        assertNull(Region.normalized(100, 0, 10, 10, 100, 50))
        assertNull(Region.normalized(0, 0, 0, 10, 100, 50))
        assertNull(Region.normalized(0, 0, 10, 10, 0, 50))
        assertNull(Region.normalized(-1, 0, 10, 10, 100, 50))
        val third = Region.normalized(0, 0, 1, 1, 3, 3)!!
        assertEquals(21846, third.w)
        assertEquals(0.5f, Region(32768, 0, 1, 1).leftFraction)
        assertEquals(1f, Region(0, 0, 65536, 65536).heightFraction)
    }

    @Test
    fun shareExpiry() {
        assertEquals(86_400_000L, ShareExpiry.DAY.expiresAt(0))
        assertEquals(7 * 86_400_000L + 5, ShareExpiry.WEEK.expiresAt(5))
        assertEquals(30 * 86_400_000L, ShareExpiry.MONTH.expiresAt(0))
        assertNull(ShareExpiry.NEVER.expiresAt(0))
        val share = CaseShare(CaseId.of(0, ByteArray(15)), "u", 0, 10, 1)
        assertFalse(share.isExpired(9))
        assertTrue(share.isExpired(10))
        assertFalse(share.copy(expiresAtMs = null).isExpired(Long.MAX_VALUE))
    }

    @Test
    fun entitlement() {
        assertTrue(Entitlement.FREE.isActive(Long.MAX_VALUE))
        val plus = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, 100)
        assertEquals(Tier.PLUS, plus.effectiveTier(99))
        assertEquals(Tier.FREE, plus.effectiveTier(100))
        assertEquals(Tier.PLUS, plus.copy(expiresAtMs = null).effectiveTier(Long.MAX_VALUE))
    }

    @Test
    fun locationPrecisionLevels() {
        assertEquals(listOf(null, 5, 6, 9), LocationPrecision.entries.map { it.geohashChars })
        assertEquals(listOf(false, false, true, true), LocationPrecision.entries.map { it.revealsPrecise })
    }

    @Test
    fun platformTopK() {
        assertEquals(8, Platform.ANDROID.topK)
        assertEquals(3, Platform.WEB.topK)
    }

    @Test
    fun settingsDefaultsArePrivate() {
        val s = Settings()
        assertFalse(s.telemetryOptIn)
        assertEquals(LocationPrecision.AREA, s.defaultPrecision)
        assertEquals(CaptureMode.POST, s.captureMode)
    }

    @Test
    fun outboxEntryAccessors() {
        val key = KeyRef(KeyId.fromBytes(ByteArray(16)), 3)
        val rid = RecordId.fromBytes(ByteArray(10))
        val e = OutboxEntry(Submission(rid, key, true, 0, ByteString.EMPTY, ByteString.EMPTY, ByteString.EMPTY))
        assertEquals(rid, e.recordId)
        assertEquals(key.keyId, e.keyId)
        assertEquals(3, e.counter)
        assertEquals(OutboxState.PENDING, e.state)
    }

    @Test
    fun importsAreUnverified() {
        val c = Capture(RecordId.fromBytes(ByteArray(10)), null, CaptureOrigin.IMPORT, AnchoringStatus.UNVERIFIED, 0, null, LocationConfidence.LOW, Dims(1, 1))
        assertFalse(c.isVerified)
        assertTrue(c.copy(origin = CaptureOrigin.ORIGINAL).isVerified)
    }

    @Test
    fun templateIds() {
        assertEquals("car_accident", TemplateId.CAR_ACCIDENT.toString())
        assertEquals(TemplateId("general"), TemplateId.GENERAL)
    }
}
