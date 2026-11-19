package app.witness.core.verify

import app.witness.core.icp.id.RecordId
import app.witness.core.model.Verdict
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

class FakeVerifyEngineTest {
    private val original = Judgement(Verdict.ORIGINAL, RecordId.parse("WERC8GMRZGE196QV"))

    @Test
    fun returnsConfiguredFallback() = runTest {
        assertEquals(original, FakeVerifyEngine(original).judge(byteArrayOf(1)))
    }

    @Test
    fun emptyInputHasNoRecord() = runTest {
        assertEquals(Judgement.NO_RECORD, FakeVerifyEngine(original).judge(ByteArray(0)))
    }

    @Test
    fun knownInputsMatchByContent() = runTest {
        val engine = FakeVerifyEngine(known = mapOf(byteArrayOf(1, 2, 3) to original))
        assertEquals(original, engine.judge(byteArrayOf(1, 2, 3)))
        assertEquals(Judgement.NO_RECORD, engine.judge(byteArrayOf(1, 2)))
        assertEquals(Judgement.NO_RECORD, engine.judge(byteArrayOf(3, 2, 1)))
    }

    @Test
    fun knownTableIsCopied() = runTest {
        val key = byteArrayOf(5)
        val engine = FakeVerifyEngine(known = mapOf(key to original))
        key[0] = 6
        assertEquals(original, engine.judge(byteArrayOf(5)))
    }

    @Test
    fun emitsEveryStageThenDone() = runTest {
        val updates = FakeVerifyEngine().check(byteArrayOf(1)).toList()
        assertIs<CheckUpdate.Done>(updates.last())
        val steps = updates.fold(CheckSteps.INITIAL) { s, u -> s.reduce(u) }
        assertTrue(CheckStage.entries.all { steps[it] == StageState.DONE })
    }

    @Test
    fun flowIsColdAndRepeatable() = runTest {
        val flow = FakeVerifyEngine(original).check(byteArrayOf(1))
        assertEquals(flow.toList(), flow.toList())
    }
}
