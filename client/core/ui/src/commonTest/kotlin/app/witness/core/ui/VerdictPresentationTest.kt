package app.witness.core.ui

import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessPalette
import app.witness.core.designsystem.WitnessState
import app.witness.core.model.Verdict
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class VerdictPresentationTest {
    private val light = WitnessPalette.Light

    @Test
    fun headlineAndQualifierComposeTitle() {
        Verdict.entries.forEach { v ->
            val composed = listOfNotNull(v.headline(), v.qualifier()).joinToString(", ")
            assertEquals(v.title(), composed, v.name)
        }
    }

    @Test
    fun onlyMatchesHasQualifier() {
        assertEquals("with changes", Verdict.MATCHES_WITH_CHANGES.qualifier())
        Verdict.entries.filter { it != Verdict.MATCHES_WITH_CHANGES }.forEach { assertNull(it.qualifier()) }
    }

    @Test
    fun verdictsMapToDesignStates() {
        assertEquals(WitnessState.Original, Verdict.ORIGINAL.state())
        assertEquals(WitnessState.Changes, Verdict.MATCHES_WITH_CHANGES.state())
        assertEquals(WitnessState.Altered, Verdict.ALTERED.state())
        assertEquals(WitnessState.NoRecord, Verdict.NO_RECORD.state())
        assertEquals(WitnessState.Unanchored, Verdict.SEALED_UNANCHORED.state())
        Verdict.entries.forEach { assertEquals(it.title(), it.state().label, it.name) }
    }

    @Test
    fun summariesAreDistinctSentences() {
        val all = Verdict.entries.map { it.summary() }
        assertEquals(all.size, all.toSet().size)
        all.forEach { assertTrue(it.endsWith(".")) }
    }

    @Test
    fun noRecordNeverSaysFake() {
        assertTrue("doesn't make it fake" in Verdict.NO_RECORD.summary())
    }

    @Test
    fun provenVerdictsUseToneTint() {
        val original = Verdict.ORIGINAL.colors(light)
        assertEquals(WitnessColors.Ok, original.icon)
        assertEquals(WitnessColors.OkText, original.title)
        assertEquals(0.07f, original.fill.alpha, 0.005f)
        assertEquals(0.4f, original.stroke.alpha, 0.005f)
        assertEquals(WitnessColors.Edit, Verdict.MATCHES_WITH_CHANGES.colors(light).title)
        assertEquals(WitnessColors.Crit, Verdict.ALTERED.colors(light).title)
    }

    @Test
    fun noRecordUsesNeutralWell() {
        val c = Verdict.NO_RECORD.colors(light)
        assertEquals(WitnessColors.Surface2, c.fill)
        assertEquals(WitnessColors.Ink2, c.icon)
        assertEquals(WitnessColors.Ink, c.title)
    }

    @Test
    fun unanchoredUsesInk() {
        val c = Verdict.SEALED_UNANCHORED.colors(light)
        assertEquals(WitnessColors.Ink, c.title)
        assertEquals(0.05f, c.fill.alpha, 0.005f)
        assertEquals(0.25f, c.stroke.alpha, 0.005f)
    }

    @Test
    fun greenAppearsOnlyForOriginal() {
        Verdict.entries.filter { it != Verdict.ORIGINAL }.forEach { v ->
            val c = v.colors(light)
            assertTrue(c.icon != WitnessColors.Ok && c.title != WitnessColors.OkText, v.name)
        }
    }
}
