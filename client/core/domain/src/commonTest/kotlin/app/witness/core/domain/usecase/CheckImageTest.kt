package app.witness.core.domain.usecase

import app.cash.turbine.test
import app.witness.core.domain.fake.SoftwareSha256
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.usecase.CheckImage.Companion.fail
import app.witness.core.domain.usecase.CheckImage.Companion.settle
import app.witness.core.domain.usecase.CheckImage.Companion.withStage
import app.witness.core.model.ByteString
import app.witness.core.model.CheckStage
import app.witness.core.model.MatchEvidence
import app.witness.core.model.Platform
import app.witness.core.model.StageState
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.model.VerdictReason
import kotlinx.coroutines.flow.last
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class CheckImageTest {
    private val input = ByteString.of("some screenshot".encodeToByteArray())
    private val digest = ByteString.of(SoftwareSha256.digest("some screenshot".encodeToByteArray()))

    @Test
    fun initialProgressHasAllStagesWaiting() {
        val p = CheckProgress.INITIAL
        assertEquals(CheckStage.entries, p.stages.map { it.stage })
        assertTrue(p.stages.all { it.state == StageState.WAITING })
        assertEquals(CheckStage.WATERMARK, p[CheckStage.WATERMARK].stage)
    }

    @Test
    fun noRecordFlowEmitsStagedProgressThenResult() = runTest {
        val g = TestGraph()
        g.check()(input).test {
            assertEquals(CheckProgress.INITIAL, awaitItem())
            assertEquals(StageState.RUNNING, awaitItem()[CheckStage.READ_FILE].state)
            val events = ArrayList<CheckProgress>()
            while (true) {
                val p = awaitItem()
                events += p
                if (p.isDone) break
            }
            val done = events.last()
            assertEquals(Verdict.NO_RECORD, done.result!!.verdict)
            assertTrue(VerdictReason.NO_CANDIDATES in done.result.reasons)
            assertEquals(StageState.DONE, done[CheckStage.READ_FILE].state)
            assertEquals(StageState.DONE, done[CheckStage.FINGERPRINTS].state)
            assertEquals(StageState.SKIPPED, done[CheckStage.WATERMARK].state)
            assertEquals(StageState.SKIPPED, done[CheckStage.COMPARE].state)
            awaitComplete()
        }
    }

    @Test
    fun exactHashHitIsOriginal() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.anchor()
        g.index.addExact(0, digest, g.registry.record(c.recordId)!!.recordIx)
        val last = g.check()(input).last()
        assertEquals(Verdict.ORIGINAL, last.result!!.verdict)
        assertEquals(c.recordId, last.result.reference!!.recordId)
        assertEquals(MatchEvidence.EXACT_HASH, last.result.reference!!.evidence)
    }

    @Test
    fun pdqHitWithReferenceMatchesWithChanges() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.anchor()
        g.index.addPdq(0, digest, g.registry.record(c.recordId)!!.recordIx, 20)
        val noRef = g.check()(input).last().result!!
        assertEquals(Verdict.MATCHES_WITH_CHANGES, noRef.verdict)
        assertTrue(VerdictFlag.NO_PUBLIC_REFERENCE in noRef.flags)
        assertEquals(MatchEvidence.GRID, noRef.reference!!.evidence)
        g.media.references[c.recordId] = input
        val withRef = g.check()(input).last().result!!
        assertTrue(VerdictFlag.NO_PUBLIC_REFERENCE !in withRef.flags)
        assertEquals(MatchEvidence.PIXELS, withRef.reference!!.evidence)
    }

    @Test
    fun fingerprintStageReportsShardProgress() = runTest {
        val g = TestGraph()
        val all = g.check()(input).toList()
        val fp = all.map { it[CheckStage.FINGERPRINTS] }.filter { it.state == StageState.RUNNING }
        assertEquals(0 to 1, fp.first().completed to fp.first().total)
        assertEquals(1 to 1, fp.last().completed to fp.last().total)
    }

    @Test
    fun emptyInputIsUndecodable() = runTest {
        val g = TestGraph()
        val all = g.check()(ByteString.EMPTY).toList()
        val last = all.last()
        assertEquals(StageState.FAILED, last[CheckStage.READ_FILE].state)
        assertEquals(StageState.SKIPPED, last[CheckStage.COMPARE].state)
        assertTrue(VerdictReason.UNDECODABLE in last.result!!.reasons)
        assertTrue(g.verify.seen.isEmpty())
    }

    @Test
    fun engineFailureSurfacesErrorAndFailsRunningStage() = runTest {
        val g = TestGraph()
        g.verify.failure = IllegalStateException("decoder crashed")
        val last = g.check()(input).last()
        assertIs<IllegalStateException>(last.error)
        assertNull(last.result)
        assertTrue(last.isDone)
        assertEquals(StageState.FAILED, last[CheckStage.READ_FILE].state)
        assertEquals(StageState.SKIPPED, last[CheckStage.FINGERPRINTS].state)
        assertEquals(1, g.telemetry.errors.size)
    }

    @Test
    fun topKCapsFollowPlatform() = runTest {
        val g = TestGraph()
        g.check(Platform.ANDROID)(input).last()
        g.check(Platform.WEB)(input).last()
        assertEquals(listOf(8, 3), g.verify.seen.map { it.topK })
        assertEquals(listOf(Platform.ANDROID, Platform.WEB), g.verify.seen.map { it.platform })
    }

    @Test
    fun completionTelemetryHasVerdictOnly() = runTest {
        val g = TestGraph()
        g.check(Platform.WEB)(input).last()
        val (event, metrics) = g.telemetry.events.last()
        assertEquals(AnalyticsEvent.CHECK_COMPLETED, event)
        assertEquals(mapOf("verdict" to Verdict.NO_RECORD.ordinal.toLong(), "platform" to Platform.WEB.ordinal.toLong()), metrics)
    }

    @Test
    fun terminalStagesNeverRegress() {
        val s = CheckProgress.INITIAL.stages.withStage(CheckStage.READ_FILE, StageState.DONE, detail = "JPEG")
        assertSame(s, s.withStage(CheckStage.READ_FILE, StageState.RUNNING))
        assertEquals(StageState.FAILED, s.withStage(CheckStage.READ_FILE, StageState.FAILED)[0].state)
        assertEquals("JPEG", s.withStage(CheckStage.READ_FILE, StageState.FAILED)[0].detail)
        assertSame(s, s.withStage(CheckStage.READ_FILE, StageState.DONE, detail = "JPEG"))
    }

    @Test
    fun settleAndFailFinalizeStages() {
        val running = CheckProgress.INITIAL.stages.withStage(CheckStage.CREDENTIALS, StageState.RUNNING)
        val settled = running.settle()
        assertEquals(StageState.DONE, settled[1].state)
        assertEquals(StageState.SKIPPED, settled[0].state)
        val failed = running.fail()
        assertEquals(StageState.FAILED, failed[1].state)
        assertEquals(StageState.SKIPPED, failed[4].state)
    }

    @Test
    fun optionsClampTopK() {
        assertEquals(1, app.witness.core.domain.port.JudgeOptions.forPlatform(Platform.WEB, 0).topK)
        assertEquals(16, app.witness.core.domain.port.JudgeOptions.forPlatform(Platform.WEB, 99).topK)
        assertNotNull(app.witness.core.domain.port.JudgeOptions.forPlatform(Platform.ANDROID))
    }
}
