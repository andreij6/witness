package app.witness.core.verify

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CheckStepsTest {
    @Test
    fun startsWaiting() {
        val s = CheckSteps.INITIAL
        assertTrue(CheckStage.entries.all { s[it] == StageState.WAITING })
        assertNull(s.current)
        assertFalse(s.isDone)
    }

    @Test
    fun tracksRunningStageAndShardCounts() {
        val s = CheckSteps.INITIAL
            .reduce(CheckUpdate.Progress(CheckStage.READ_FILE, StageState.DONE))
            .reduce(CheckUpdate.Progress(CheckStage.CREDENTIALS, StageState.SKIPPED))
            .reduce(CheckUpdate.Progress(CheckStage.FINGERPRINTS, StageState.RUNNING, 2, 3))
        assertEquals(CheckStage.FINGERPRINTS, s.current)
        assertEquals(2, s.answered)
        assertEquals(3, s.total)
        assertEquals(StageState.SKIPPED, s[CheckStage.CREDENTIALS])
    }

    @Test
    fun countsOnlyComeFromFingerprintStage() {
        val s = CheckSteps.INITIAL
            .reduce(CheckUpdate.Progress(CheckStage.FINGERPRINTS, StageState.RUNNING, 1, 4))
            .reduce(CheckUpdate.Progress(CheckStage.WATERMARK, StageState.RUNNING, 9, 9))
            .reduce(CheckUpdate.Progress(CheckStage.FINGERPRINTS, StageState.DONE))
        assertEquals(1, s.answered)
        assertEquals(4, s.total)
    }

    @Test
    fun doneSkipsUnfinishedStages() {
        val s = CheckSteps.INITIAL
            .reduce(CheckUpdate.Progress(CheckStage.READ_FILE, StageState.DONE))
            .reduce(CheckUpdate.Progress(CheckStage.CREDENTIALS, StageState.RUNNING))
            .reduce(CheckUpdate.Done(Judgement.NO_RECORD))
        assertTrue(s.isDone)
        assertEquals(StageState.DONE, s[CheckStage.READ_FILE])
        assertEquals(StageState.SKIPPED, s[CheckStage.CREDENTIALS])
        assertEquals(StageState.SKIPPED, s[CheckStage.COMPARE])
        assertNull(s.current)
        assertEquals(Judgement.NO_RECORD, s.judgement)
    }

    @Test
    fun reduceIsPureAndDeterministic() {
        val u = CheckUpdate.Progress(CheckStage.WATERMARK, StageState.RUNNING)
        val a = CheckSteps.INITIAL.reduce(u)
        assertEquals(a, CheckSteps.INITIAL.reduce(u))
        assertEquals(a.hashCode(), CheckSteps.INITIAL.reduce(u).hashCode())
        assertEquals(StageState.WAITING, CheckSteps.INITIAL[CheckStage.WATERMARK])
        assertTrue("WATERMARK=RUNNING" in a.toString())
    }
}
