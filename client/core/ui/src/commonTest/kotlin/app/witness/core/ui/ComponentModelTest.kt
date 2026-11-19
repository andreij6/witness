package app.witness.core.ui

import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import kotlin.test.Test
import kotlin.test.assertEquals

class ComponentModelTest {
    @Test
    fun factStylesFollowStatus() {
        val neutral = ProofFact("Captured", "Nov 18", WitnessIcon.Schedule)
        assertEquals(FactStyle(WitnessIcon.Schedule, null, null), neutral.style())
        assertEquals(FactStyle(WitnessIcon.CheckCircle, Tone.Ok, null), neutral.copy(status = FactStatus.Passed).style())
        assertEquals(FactStyle(WitnessIcon.Error, Tone.Crit, Tone.Crit), neutral.copy(status = FactStatus.Failed).style())
    }

    @Test
    fun custodyLineConnectsDiscs() {
        assertEquals(emptyList(), (0 until 0).map { custodyLine(it, 0) })
        assertEquals(listOf(LineSegment.None), (0 until 1).map { custodyLine(it, 1) })
        assertEquals(listOf(LineSegment.Lower, LineSegment.Upper), (0 until 2).map { custodyLine(it, 2) })
        assertEquals(
            listOf(LineSegment.Lower, LineSegment.Full, LineSegment.Full, LineSegment.Full, LineSegment.Upper),
            (0 until 5).map { custodyLine(it, 5) },
        )
    }

    @Test
    fun strengthLabelsAndTones() {
        assertEquals("Strong", Strength.Strong.label)
        assertEquals(Tone.Ok, Strength.Strong.tone)
        assertEquals(Tone.Warn, Strength.Medium.tone)
        assertEquals(Tone.Muted, Strength.Weak.tone)
    }

    @Test
    fun provesDefaultsSayWhatIsNotProven() {
        assertEquals(4, Proves.Default.size)
        assertEquals(3, Proves.Default.count { it.proven })
        assertEquals("Doesn't prove the scene wasn't staged", Proves.Default.last().text)
        assertEquals(false, Proves.CaseLink.last().proven)
        assertEquals("Proves: Genuine phone, real Witness app", Proves.accessibleText(Proves.CaseLink.first()))
        assertEquals(Proves.Default.last().text, Proves.accessibleText(Proves.Default.last()))
    }

    @Test
    fun stageStylesFollowDesign() {
        assertEquals(WitnessIcon.CheckCircle, StageStatus.Done.style().icon)
        assertEquals(Tone.Ink, StageStatus.Done.style().iconTone)
        assertEquals(Tone.Muted, StageStatus.Skipped.style().iconTone)
        assertEquals(WitnessIcon.Sync, StageStatus.Running.style().icon)
        assertEquals(Tone.Primary, StageStatus.Running.style().detailTone)
        assertEquals(WitnessIcon.RadioButtonUnchecked, StageStatus.Pending.style().icon)
        assertEquals(StageStatus.entries.size, StageStatus.entries.map { it.accessibleLabel() }.toSet().size)
    }

    @Test
    fun overallProgressAveragesStages() {
        assertEquals(0f, overallProgress(emptyList()))
        val stages = listOf(
            CheckStage("Reading the file", status = StageStatus.Done),
            CheckStage("Content Credentials", status = StageStatus.Skipped),
            CheckStage("Searching fingerprints", status = StageStatus.Running, progress = 0.5f),
            CheckStage("Reading the watermark"),
        )
        assertEquals(2.5f / 4f, overallProgress(stages), 1e-6f)
        assertEquals(0.25f, overallProgress(listOf(CheckStage("x", status = StageStatus.Running, progress = 7f), CheckStage("y"), CheckStage("z"), CheckStage("w"))), 1e-6f)
        assertEquals(0f, overallProgress(listOf(CheckStage("x", status = StageStatus.Running, progress = -1f))))
        assertEquals(0f, overallProgress(listOf(CheckStage("x", status = StageStatus.Running))))
    }

    @Test
    fun stateCountsSkipZeros() {
        assertEquals(listOf(StateCount(WitnessState.Anchored, 3), StateCount(WitnessState.Queued, 1)), stateCounts(3, 1))
        assertEquals(listOf(StateCount(WitnessState.Anchored, 6)), stateCounts(6, 0))
        assertEquals(emptyList(), stateCounts(0, 0))
        assertEquals(WitnessState.Altered, stateCounts(0, 0, 2).single().state)
    }

    @Test
    fun mosaicShapesMatchThumbnailCount() {
        assertEquals(0, mosaicShapes(0, 10.dp).size)
        assertEquals(1, mosaicShapes(1, 10.dp).size)
        assertEquals(1, mosaicShapes(2, 10.dp).size)
        assertEquals(3, mosaicShapes(3, 10.dp).size)
        assertEquals(3, mosaicShapes(12, 10.dp).size)
    }

    @Test
    fun locationTextsFollowDesign() {
        assertEquals("Denver area · Nov 18, 4:12 PM", locationPillText("Denver area", "Nov 18, 4:12 PM"))
        assertEquals("Denver area", locationPillText("Denver area"))
        assertEquals("Location · High", LocationConfidence.High.pillText())
        assertEquals(WitnessIcon.SatelliteAlt, LocationConfidence.High.icon)
        assertEquals(Tone.Ok, LocationConfidence.High.tone)
        assertEquals(3, LocationConfidence.entries.map { it.reason }.toSet().size)
    }
}
