package app.witness.core.icp.witness

import app.witness.core.icp.witness.WitnessFixtures.FULL_CLAIM
import app.witness.core.icp.witness.WitnessFixtures.HASH_A
import app.witness.core.icp.witness.WitnessFixtures.KEY
import app.witness.core.icp.witness.WitnessFixtures.KEY_HEX
import app.witness.core.icp.witness.WitnessFixtures.LEAN_CLAIM
import app.witness.core.icp.witness.WitnessFixtures.PARENT_ID
import app.witness.core.icp.witness.WitnessFixtures.PROOF
import app.witness.core.icp.witness.WitnessFixtures.RECORD_ID
import app.witness.core.model.BlurClass
import app.witness.core.model.BootState
import app.witness.core.model.ByteString
import app.witness.core.model.ClaimKind
import app.witness.core.model.Edit
import app.witness.core.model.Freshness
import app.witness.core.model.GnssCheck
import app.witness.core.model.ImageFormat
import app.witness.core.model.KeyStatus
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationReason
import app.witness.core.model.Provisioning
import app.witness.core.model.Rect
import app.witness.core.model.SecurityLevel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class WireJsonTest {
    @Test
    fun fullClaimMapsEveryField() {
        val c = WireJson.claim(WireJson.parse(FULL_CLAIM))
        assertEquals(123456789L, c.captureId.value)
        assertEquals(ByteString.ofHex(HASH_A), c.assetHash)
        assertEquals(32, c.fpCommit.size)
        assertEquals(640, c.dims.width)
        assertEquals(1_700_000_000_000L, c.time.wallMs)
        assertEquals(1_700_000_000_500L, c.time.gnssMs)
        assertEquals(55L, c.time.elapsedNs)
        assertEquals(4, c.time.bootCount)
        assertEquals(LocationConfidence.MEDIUM, c.location.confidence)
        assertTrue(LocationReason.RAW_GNSS_PASSED in c.location.reasons && LocationReason.FUSED_FIX in c.location.reasons)
        assertEquals(32, c.location.commit?.size)
        assertEquals(SecurityLevel.DEV, c.checks.keySecurity)
        assertEquals(BootState.SELF_SIGNED, c.checks.boot)
        assertTrue(c.checks.deviceLocked)
        assertTrue(GnssCheck.RAW_AVAILABLE in c.checks.gnss && GnssCheck.AGC_STABLE in c.checks.gnss)
        assertEquals(3, c.checks.recapture)
        assertEquals(KEY_HEX, ByteString.wrap(c.keyRef.keyId.toBytes()).hex())
        assertEquals(3L, c.keyRef.counter)
        assertEquals("0a0b", c.certSerial)
        assertEquals("9q8yy", c.coarse)
        val kind = c.kind as ClaimKind.Derivative
        assertEquals(PARENT_ID, kind.parent.text)
        assertEquals(77L, kind.root.value)
        assertEquals(
            listOf(
                Edit.Blur(Rect(1, 2, 3, 4), BlurClass.HOUSE_NO, 900),
                Edit.Recompress(ImageFormat.JPEG, 80),
                Edit.Resize(10, 20, 1),
                Edit.Orient(6),
                Edit.Crop(Rect(0, 0, 5, 6)),
                Edit.Rotate(-150),
                Edit.Tone(-3, 4, 5),
                Edit.Filter(2, 50),
            ),
            kind.edits,
        )
    }

    @Test
    fun leanClaimFallsBackToNeutralEvidence() {
        val c = WireJson.claim(WireJson.parse(LEAN_CLAIM))
        assertEquals(ClaimKind.Original, c.kind)
        assertNull(c.time.gnssMs)
        assertEquals(LocationEvidence.NONE, c.location)
        assertEquals(WireJson.UNKNOWN_CHECKS, c.checks)
        assertNull(c.certSerial)
        assertNull(c.coarse)
        assertEquals(0, c.fpCommit.size)
    }

    @Test
    fun recordConvertsNanosecondsToMillis() {
        val r = WireJson.record(WireJson.parse(PROOF))
        assertEquals(7L, r.recordIx)
        assertEquals(RECORD_ID, r.recordId.text)
        assertEquals(1_700_000_001_000L, r.anchoredAtMs)
    }

    @Test
    fun keyMapsVariantsAndHistory() {
        val k = WireJson.key(WireJson.parse(KEY))
        assertEquals(SecurityLevel.STRONG_BOX, k.security)
        assertEquals(Provisioning.RKP, k.provisioning)
        assertEquals(Freshness.LATE, k.freshness)
        assertEquals(1000L, k.enrolledAtMs)
        assertEquals(9000L, k.activeUntilMs)
        assertEquals(KeyStatus.REVOKED, k.status)
        assertTrue(k.history.revokedAfter(2000))
    }

    @Test
    fun malformedInputIsRejected() {
        val bad = listOf(
            "", "[]", "{", "null",
            FULL_CLAIM.replace("\"captureId\":\"123456789\"", "\"captureId\":\"-1\""),
            FULL_CLAIM.replace("\"captureId\":\"123456789\"", "\"captureId\":\"99999999999999999999\""),
            FULL_CLAIM.replace("\"type\":\"derivative\"", "\"type\":\"collage\""),
            FULL_CLAIM.replace("\"kind\":\"rotate\"", "\"kind\":\"warp\""),
            FULL_CLAIM.replace("\"class\":\"house_no\"", "\"class\":\"cat\""),
            FULL_CLAIM.replace("\"keySecurity\":2", "\"keySecurity\":9"),
            FULL_CLAIM.replace("\"keyId\":\"$KEY_HEX\"", "\"keyId\":\"00\""),
            FULL_CLAIM.replace("\"assetHash\":\"$HASH_A\"", "\"assetHash\":\"zz\""),
            FULL_CLAIM.replace("\"width\":640", "\"width\":\"640\""),
            FULL_CLAIM.replace("\"parent\":\"$PARENT_ID\"", "\"parent\":\"nope\""),
        )
        for (text in bad) assertFailsWith<WireException>(text) { WireJson.claim(WireJson.parse(text)) }
        assertFailsWith<WireException> { WireJson.key(WireJson.parse(KEY.replace("\"Revoked\"", "\"Lost\""))) }
        assertFailsWith<WireException> { WireJson.key(WireJson.parse(KEY.replace("\"sinceNs\":\"5000000000\"", "\"sinceNs\":\"1000000\""))) }
        assertFailsWith<WireException> { WireJson.record(WireJson.parse(PROOF.replace(RECORD_ID, "short"))) }
    }

    @Test
    fun parsingIsDeterministic() {
        assertEquals(WireJson.claim(WireJson.parse(FULL_CLAIM)), WireJson.claim(WireJson.parse(FULL_CLAIM)))
    }
}
