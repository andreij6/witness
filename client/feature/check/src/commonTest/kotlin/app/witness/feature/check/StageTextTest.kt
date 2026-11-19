package app.witness.feature.check

import app.witness.core.domain.usecase.CheckProgress
import app.witness.core.model.CheckStage
import app.witness.core.model.StageState
import app.witness.core.model.StageStatus
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import app.witness.core.ui.StageStatus as Ui

class StageTextTest {
    private fun app(stage: CheckStage, state: StageState, done: Int = 0, total: Int = 0, detail: String? = null) =
        StageText.stage(StageStatus(stage, state, done, total, detail), CheckSurface.App)

    private fun web(stage: CheckStage, state: StageState, done: Int = 0, total: Int = 0, detail: String? = null) =
        StageText.stage(StageStatus(stage, state, done, total, detail), CheckSurface.Web)

    @Test
    fun statesMapOntoUiStatuses() {
        assertEquals(Ui.Pending, StageText.status(StageState.WAITING))
        assertEquals(Ui.Running, StageText.status(StageState.RUNNING))
        assertEquals(Ui.Done, StageText.status(StageState.DONE))
        assertEquals(Ui.Skipped, StageText.status(StageState.SKIPPED))
        assertEquals(Ui.Skipped, StageText.status(StageState.FAILED))
    }

    @Test
    fun appTitlesFollowDesign() {
        assertEquals("Reading the file", app(CheckStage.READ_FILE, StageState.DONE).title)
        assertEquals("Content Credentials · none found", app(CheckStage.CREDENTIALS, StageState.SKIPPED).title)
        assertEquals("Stripped on upload — expected for screenshots", app(CheckStage.CREDENTIALS, StageState.SKIPPED).detail)
        assertEquals("Searching fingerprints", app(CheckStage.FINGERPRINTS, StageState.RUNNING).title)
        assertEquals("Reading the watermark", app(CheckStage.WATERMARK, StageState.WAITING).title)
        assertEquals("Comparing with the reference", app(CheckStage.COMPARE, StageState.WAITING).title)
    }

    @Test
    fun webTitlesAreShort() {
        assertEquals("Reading file", web(CheckStage.READ_FILE, StageState.DONE).title)
        assertEquals("Content Credentials", web(CheckStage.CREDENTIALS, StageState.SKIPPED).title)
        assertEquals("None in this file", web(CheckStage.CREDENTIALS, StageState.SKIPPED).detail)
        assertEquals("Fingerprints", web(CheckStage.FINGERPRINTS, StageState.RUNNING).title)
        assertEquals("Loads only if needed", web(CheckStage.WATERMARK, StageState.WAITING).detail)
        assertEquals("Waiting", web(CheckStage.COMPARE, StageState.WAITING).detail)
    }

    @Test
    fun shardProgressBecomesFractionAndLabel() {
        val w = web(CheckStage.FINGERPRINTS, StageState.RUNNING, 2, 3)
        assertEquals(2f / 3f, w.progress)
        assertEquals("2 of 3", w.progressLabel)
        assertEquals("Searching 3 shards…", w.detail)
        val a = app(CheckStage.FINGERPRINTS, StageState.RUNNING, 2, 3)
        assertEquals("3 index shards · 2 answered", a.detail)
        assertNull(a.progressLabel)
    }

    @Test
    fun zeroTotalHasNoProgressAndFinishedStagesHaveNone() {
        assertNull(app(CheckStage.FINGERPRINTS, StageState.RUNNING).progress)
        assertNull(app(CheckStage.FINGERPRINTS, StageState.DONE, 3, 3).progress)
        assertEquals("3 index shards · 3 answered", app(CheckStage.FINGERPRINTS, StageState.DONE, 3, 3).detail)
    }

    @Test
    fun overCountIsClamped() {
        assertEquals(1f, web(CheckStage.FINGERPRINTS, StageState.RUNNING, 9, 3).progress)
    }

    @Test
    fun engineDetailWins() {
        assertEquals("JPEG", app(CheckStage.READ_FILE, StageState.DONE, detail = "JPEG").detail)
        assertEquals("C2PA from Pixel", web(CheckStage.CREDENTIALS, StageState.SKIPPED, detail = "C2PA from Pixel").detail)
    }

    @Test
    fun mapKeepsOrderAndSize() {
        val mapped = StageText.map(CheckProgress.INITIAL.stages, CheckSurface.App)
        assertEquals(CheckStage.entries.size, mapped.size)
        assertEquals("Reading the file", mapped.first().title)
        assertEquals(Ui.Pending, mapped.last().status)
    }
}
