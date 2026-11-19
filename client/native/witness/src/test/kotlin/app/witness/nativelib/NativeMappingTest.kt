package app.witness.nativelib

import app.witness.core.domain.port.SubmitResult
import app.witness.core.model.Anchor
import app.witness.core.model.BlurClass
import app.witness.core.model.BootState
import app.witness.core.model.ClaimKind
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.Freshness
import app.witness.core.model.GnssCheck
import app.witness.core.model.ImageFormat
import app.witness.core.model.KeyHistory
import app.witness.core.model.KeyId
import app.witness.core.model.KeyStatus
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationReason
import app.witness.core.model.MatchEvidence
import app.witness.core.model.Provisioning
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
import app.witness.core.model.SecurityLevel
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.model.VerdictReason
import uniffi.witness_android.Anchor as NAnchor
import uniffi.witness_android.Anchored
import uniffi.witness_android.Evidence
import uniffi.witness_android.Kind
import uniffi.witness_android.Outcome
import uniffi.witness_android.Prov
import uniffi.witness_android.Reason
import uniffi.witness_android.ReferenceView
import uniffi.witness_android.Region as NRegion
import uniffi.witness_android.Security
import uniffi.witness_android.StatusChange as NStatusChange
import uniffi.witness_android.SubmitErr
import uniffi.witness_android.SubmitResult as NSubmitResult
import uniffi.witness_android.VerdictFlag as NVerdictFlag
import uniffi.witness_android.VerdictView
import uniffi.witness_android.Submission as NSubmission
import uniffi.witness_android.KeyStatus as NKeyStatus
import uniffi.witness_android.BlurClass as NBlurClass
import uniffi.witness_android.BootState as NBootState
import uniffi.witness_android.Confidence
import uniffi.witness_android.SecurityLevel as NSecurityLevel
import uniffi.witness_android.ImageFormat as NImageFormat
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class NativeMappingTest {
    private val keyBytes = ByteArray(16) { 5 }
    private val anchored = Anchored(7uL, "WERC8GMRZGE196QV", 3_000_000_000uL)

    @Test
    fun claimViewMapsEveryField() {
        val c = NativeMapping.claim(Vectors.claimView(keyBytes))
        assertEquals(0x00AB_CDEFL, c.captureId.value)
        assertEquals(ClaimKind.Original, c.kind)
        assertEquals(Dims(640, 480), c.dims)
        assertEquals(1_700_000_000_000, c.time.wallMs)
        assertEquals(1_700_000_000_005, c.time.gnssMs)
        assertEquals(77, c.time.elapsedNs)
        assertEquals(3, c.time.bootCount)
        assertEquals(LocationConfidence.MEDIUM, c.location.confidence)
        assertTrue(LocationReason.RAW_GNSS_PASSED in c.location.reasons && LocationReason.NO_FIX in c.location.reasons)
        assertEquals(SecurityLevel.TEE, c.checks.keySecurity)
        assertTrue(GnssCheck.RAW_AVAILABLE in c.checks.gnss && GnssCheck.CN0_CONSISTENT in c.checks.gnss)
        assertEquals(KeyId.fromBytes(keyBytes), c.keyRef.keyId)
        assertEquals(4, c.keyRef.counter)
        assertEquals("0102", c.certSerial)
        assertEquals("9xj64", c.coarse)
        assertEquals(32, c.assetHash.size)
    }

    @Test
    fun claimInputsRoundTripToNative() {
        val c = NativeMapping.claim(Vectors.claimView(keyBytes))
        val v = Vectors.claimView(keyBytes)
        assertEquals(v.time, NativeMapping.time(c.time))
        assertEquals(v.checks, NativeMapping.checks(c.checks))
        val loc = NativeMapping.location(c.location)
        assertContentEquals(v.location.commit, loc.commit)
        assertEquals(v.location.reasons, loc.reasons)
        assertEquals(Confidence.MEDIUM, loc.confidence)
    }

    @Test
    fun missingGnssTimeAndCommitStayAbsent() {
        val v = Vectors.claimView(keyBytes).let { it.copy(time = it.time.copy(gnssMs = null), location = it.location.copy(commit = null), coarse = null) }
        val c = NativeMapping.claim(v)
        assertNull(c.time.gnssMs)
        assertNull(c.location.commit)
        assertNull(c.coarse)
        assertNull(NativeMapping.location(c.location).commit)
    }

    @Test
    fun everyEditKindRoundTrips() {
        val edits = listOf(
            Edit.Resize(100, 80, 2),
            Edit.Recompress(ImageFormat.WEBP, 80),
            Edit.Orient(6),
            Edit.Crop(Rect(1, 2, 3, 4)),
            Edit.Rotate(-250),
            Edit.Tone(-5, 10, 300),
            Edit.Filter(9, 65_000),
            Edit.Blur(Rect(5, 6, 7, 8), BlurClass.HOUSE_NO, 1200),
        )
        for (e in edits) assertEquals(e, NativeMapping.edit(NativeMapping.edit(e)))
        assertEquals(8, variantsOf(uniffi.witness_android.Edit::class.java))
    }

    @Test
    fun derivativeKindRoundTrips() {
        val k = ClaimKind.Derivative(RecordId.parse("WERC8GMRZGE196QV"), app.witness.core.model.CaptureId.fromLong(12)!!, listOf(Edit.Crop(Rect(0, 0, 9, 9))))
        val n = assertIs<Kind.Derivative>(NativeMapping.kind(k))
        assertEquals("WERC8GMRZGE196QV", n.parent)
        assertEquals(12uL, n.root)
        assertEquals(k, NativeMapping.kind(n))
        assertEquals(Kind.Original, NativeMapping.kind(ClaimKind.Original))
    }

    @Test
    fun captureIdOutOfRangeIsRejected() {
        assertFailsWith<IllegalArgumentException> { NativeMapping.captureId(ULong.MAX_VALUE) }
    }

    @Test
    fun okAndDupBecomeAnchorsInMilliseconds() {
        assertEquals(SubmitResult.Anchored(Anchor(7, 3_000)), NativeMapping.submitResult(NSubmitResult.Ok(anchored)))
        assertEquals(SubmitResult.Duplicate(Anchor(7, 3_000)), NativeMapping.submitResult(NSubmitResult.Dup(anchored)))
    }

    @Test
    fun everySubmitErrorVariantMaps() {
        fun err(e: SubmitErr) = NativeMapping.submitResult(NSubmitResult.Err(e))
        assertEquals(SubmitResult.RetryAfter(2_500), err(SubmitErr.RetryAfterNs(2_500_000_000uL)))
        assertEquals(SubmitResult.KeyExpired, err(SubmitErr.KeyExpired))
        assertEquals(SubmitResult.Rejected("malformed: bad cbor"), err(SubmitErr.Malformed("bad cbor")))
        val rejected = mapOf(
            SubmitErr.TooLarge to "too_large",
            SubmitErr.BadSignature to "bad_signature",
            SubmitErr.UnknownKey to "unknown_key",
            SubmitErr.KeyDistrusted to "key_distrusted",
            SubmitErr.CounterReused to "counter_reused",
            SubmitErr.ParentUnknown to "parent_unknown",
            SubmitErr.FingerprintMismatch to "fingerprint_mismatch",
        )
        for ((e, code) in rejected) assertEquals(SubmitResult.Rejected(code), err(e))
        assertEquals(10, variantsOf(SubmitErr::class.java))
    }

    @Test
    fun submissionCarriesFingerprintsBothWays() {
        val view = Vectors.claimView(keyBytes)
        val n = NSubmission(byteArrayOf(1), byteArrayOf(2), byteArrayOf(3), byteArrayOf(4, 4))
        val s = NativeMapping.submission("WERC8GMRZGE196QV", view, n)
        assertEquals(RecordId.parse("WERC8GMRZGE196QV"), s.recordId)
        assertTrue(s.isOriginal)
        assertEquals(1_700_000_000_000, s.wallMs)
        assertEquals(4, s.keyRef.counter)
        val back = NativeMapping.submission(s)
        assertContentEquals(n.claim, back.claim)
        assertContentEquals(n.cose, back.cose)
        assertContentEquals(n.fingerprints, back.fingerprints)
        assertNull(NativeMapping.submission(s.copy(fingerprints = null)).fingerprints)
        val derived = NativeMapping.submission("WERC8GMRZGE196QV", view.copy(kind = Kind.Derivative("WERC8GMRZGE196QV", 1uL, emptyList())), n)
        assertFalse(derived.isOriginal)
    }

    @Test
    fun keyInfoKeepsDevProvisioningAndConvertsTime() {
        val k = NativeMapping.keyInfo(Vectors.keyInfo(keyBytes))
        assertEquals(Provisioning.DEV, k.provisioning)
        assertEquals(SecurityLevel.TEE, k.security)
        assertEquals(Freshness.FRESH, k.freshness)
        assertEquals(10, k.enrolledAtMs)
        assertEquals(99, k.activeUntilMs)
        assertEquals(KeyStatus.ACTIVE, k.status)
        assertTrue(k.isDev)
        assertEquals(Provisioning.RKP, NativeMapping.keyInfo(Vectors.keyInfo(keyBytes, Prov.RKP, Security.STRONG_BOX)).provisioning)
    }

    @Test
    fun invalidHistoryFallsBackToEnrolledAt() {
        val bad = Vectors.keyInfo(keyBytes).copy(history = listOf(NStatusChange(NKeyStatus.ACTIVE, 5uL), NStatusChange(NKeyStatus.ACTIVE, 1uL)))
        assertEquals(KeyHistory.enrolled(10), NativeMapping.keyInfo(bad).history)
        assertEquals(KeyHistory.enrolled(10), NativeMapping.keyInfo(bad.copy(history = emptyList())).history)
    }

    @Test
    fun everyEnumMapsByName() {
        Prov.entries.forEach { NativeMapping.provisioning(it) }
        Security.entries.forEach { NativeMapping.security(it) }
        NKeyStatus.entries.forEach { NativeMapping.keyStatus(it) }
        Evidence.entries.forEach { NativeMapping.evidence(it) }
        Outcome.entries.forEach { Verdict.valueOf(it.name) }
        NVerdictFlag.entries.forEach { VerdictFlag.valueOf(it.name) }
        Reason.entries.forEach { VerdictReason.valueOf(it.name) }
        NBlurClass.entries.forEach { BlurClass.valueOf(it.name) }
        NBootState.entries.forEach { BootState.valueOf(it.name) }
        NSecurityLevel.entries.forEach { SecurityLevel.valueOf(it.name) }
        NImageFormat.entries.forEach { ImageFormat.valueOf(it.name) }
        Confidence.entries.forEach { LocationConfidence.valueOf(it.name) }
        assertEquals(Outcome.entries.map { it.name }, Verdict.entries.map { it.name })
        assertEquals(NVerdictFlag.entries.map { it.name }, VerdictFlag.entries.map { it.name })
        assertEquals(Reason.entries.map { it.name }, VerdictReason.entries.map { it.name })
    }

    @Test
    fun verdictMapsReferenceRegionsAndFlags() {
        val view = VerdictView(
            version = 1u,
            outcome = Outcome.MATCHES_WITH_CHANGES,
            flags = listOf(NVerdictFlag.DEV_ATTESTATION, NVerdictFlag.COULDNT_COMPARE),
            reasons = listOf(Reason.SEAL_VERIFIED),
            declared = listOf(uniffi.witness_android.Edit.Crop(uniffi.witness_android.Rect(0u, 0u, 4u, 4u))),
            regions = listOf(NRegion(1u, 2u, 3u, 4u)),
            reference = ReferenceView("WERC8GMRZGE196QV", Vectors.claimView(keyBytes), NAnchor(3uL, 9_000_000uL), Evidence.PIXELS, 60_000u, 50_000u, 10u),
        )
        val r = NativeMapping.verdict(view)
        assertEquals(Verdict.MATCHES_WITH_CHANGES, r.verdict)
        assertTrue(VerdictFlag.DEV_ATTESTATION in r.flags && VerdictFlag.COULDNT_COMPARE in r.flags)
        assertEquals(2, r.flags.size)
        assertTrue(VerdictReason.SEAL_VERIFIED in r.reasons)
        assertEquals(listOf(Edit.Crop(Rect(0, 0, 4, 4))), r.declared)
        assertEquals(app.witness.core.model.Region(1, 2, 3, 4), r.regions.single())
        val ref = r.reference!!
        assertEquals(Anchor(3, 9), ref.anchor)
        assertEquals(MatchEvidence.PIXELS, ref.evidence)
        assertEquals(60_000, ref.confidenceQ16)
        assertEquals(10, ref.changedQ16)
        assertNull(NativeMapping.verdict(view.copy(reference = null)).reference)
        assertFalse(r.proofsRejected)
        val rejected = NativeMapping.verdict(view.copy(outcome = Outcome.NO_RECORD, reference = null, proofsRejected = true))
        assertTrue(rejected.proofsRejected && rejected.registryUnverified)
    }

    @Test
    fun shardsKeepIds() {
        val m = NativeMapping.shards(FakeWitness().shards)
        assertEquals(listOf(0), m.index.map { it.id })
        assertEquals(listOf(0, 1), m.media.map { it.id })
    }

    @Test
    fun hexRoundTripsAndRejectsJunk() {
        val bytes = byteArrayOf(0, 1, -1, 0x7f)
        assertEquals("0001ff7f", Hex.of(bytes))
        assertContentEquals(bytes, Hex.parse("0001FF7f"))
        assertFailsWith<IllegalArgumentException> { Hex.parse("abc") }
        assertFailsWith<IllegalArgumentException> { Hex.parse("zz") }
        assertFailsWith<IllegalArgumentException> { Hex.parse("\uFF11\uFF12") }
        assertFailsWith<IllegalArgumentException> { Hex.parse("\u0661\u0662") }
        assertEquals("", Hex.of(ByteArray(0)))
    }

    @Test
    fun nanosecondsTruncateToMilliseconds() {
        assertEquals(0, NativeMapping.nsToMs(999_999uL))
        assertEquals(1, NativeMapping.nsToMs(1_999_999uL))
        assertEquals(18_446_744_073_709L, NativeMapping.nsToMs(ULong.MAX_VALUE))
    }
}
