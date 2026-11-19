package app.witness.core.verify

import app.witness.core.model.Verdict
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class VerifyWireTest {
    private val id = "WERC8GMRZGE196QV"

    @Test
    fun verdictCodesMatchRustOutcome() {
        assertEquals(Verdict.ORIGINAL, VerifyWire.verdict(0))
        assertEquals(Verdict.MATCHES_WITH_CHANGES, VerifyWire.verdict(1))
        assertEquals(Verdict.ALTERED, VerifyWire.verdict(2))
        assertEquals(Verdict.NO_RECORD, VerifyWire.verdict(3))
        assertEquals(Verdict.SEALED_UNANCHORED, VerifyWire.verdict(4))
        assertNull(VerifyWire.verdict(5))
        assertNull(VerifyWire.verdict(-1))
    }

    @Test
    fun verdictCodesRoundTrip() {
        for (v in Verdict.entries) assertEquals(v, VerifyWire.verdict(VerifyWire.code(v)))
    }

    @Test
    fun stageCodesMatchBridges() {
        assertEquals(CheckStage.READ_FILE, VerifyWire.stage(0))
        assertEquals(CheckStage.COMPARE, VerifyWire.stage(4))
        assertNull(VerifyWire.stage(5))
        assertEquals(StageState.WAITING, VerifyWire.state(0))
        assertEquals(StageState.SKIPPED, VerifyWire.state(3))
        assertNull(VerifyWire.state(4))
        assertNull(VerifyWire.state(-1))
    }

    @Test
    fun progressValidatesCounts() {
        assertEquals(CheckUpdate.Progress(CheckStage.FINGERPRINTS, StageState.RUNNING, 2, 3), VerifyWire.progress(2, 1, 2, 3))
        assertNull(VerifyWire.progress(2, 1, 4, 3))
        assertNull(VerifyWire.progress(2, 1, -1, 3))
        assertNull(VerifyWire.progress(9, 1, 0, 0))
        assertNull(VerifyWire.progress(0, 9, 0, 0))
    }

    @Test
    fun judgementMapsFields() {
        val j = VerifyWire.judgement(0, id.lowercase(), 0x11, fallback = false)
        assertEquals(Verdict.ORIGINAL, j.verdict)
        assertEquals(id, j.recordId?.text)
        assertEquals(setOf(VerdictFlag.KEY_REVOKED_AFTER_ANCHOR, VerdictFlag.NO_PUBLIC_REFERENCE), j.flags)
    }

    @Test
    fun positiveVerdictWithoutRecordDegradesToNoRecord() {
        assertEquals(Verdict.NO_RECORD, VerifyWire.judgement(0, null, 0, false).verdict)
        assertEquals(Verdict.NO_RECORD, VerifyWire.judgement(1, "garbage", 0, false).verdict)
    }

    @Test
    fun verdictsThatNeedNoRecord() {
        assertEquals(Verdict.SEALED_UNANCHORED, VerifyWire.judgement(4, null, 0, false).verdict)
        assertEquals(Verdict.ALTERED, VerifyWire.judgement(2, null, 0, false).verdict)
        assertEquals(Verdict.NO_RECORD, VerifyWire.judgement(3, null, 0, false).verdict)
    }

    @Test
    fun unknownVerdictIsNoRecordAndKeepsFallback() {
        val j = VerifyWire.judgement(77, id, 0, fallback = true)
        assertEquals(Verdict.NO_RECORD, j.verdict)
        assertTrue(j.fallback)
        assertNull(j.recordId)
    }
}
