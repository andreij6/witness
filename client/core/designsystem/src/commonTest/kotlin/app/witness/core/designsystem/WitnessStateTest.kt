package app.witness.core.designsystem

import kotlin.test.Test
import kotlin.test.assertEquals

class WitnessStateTest {
    private val designStates = mapOf(
        WitnessState.Original to Triple("Original", Tone.Ok, WitnessIcon.CheckCircle),
        WitnessState.Changes to Triple("Matches an original, with changes", Tone.Edit, WitnessIcon.Compare),
        WitnessState.Altered to Triple("Altered", Tone.Crit, WitnessIcon.Error),
        WitnessState.NoRecord to Triple("No Witness record", Tone.Muted, WitnessIcon.Help),
        WitnessState.Anchored to Triple("Anchored", Tone.Ok, WitnessIcon.Verified),
        WitnessState.Queued to Triple("Queued", Tone.Warn, WitnessIcon.HourglassTop),
        WitnessState.Sealed to Triple("Sealed", Tone.Ink, WitnessIcon.Lock),
        WitnessState.Edited to Triple("Declared edit", Tone.Edit, WitnessIcon.Edit),
        WitnessState.Unverified to Triple("Unverified import", Tone.Muted, WitnessIcon.Help),
        WitnessState.Unanchored to Triple("Sealed · not yet anchored", Tone.Ink, WitnessIcon.Lock),
    )

    @Test
    fun everyStateMatchesDesignLibrary() {
        assertEquals(WitnessState.entries.toSet(), designStates.keys)
        designStates.forEach { (state, spec) ->
            assertEquals(spec.first, state.label, state.name)
            assertEquals(spec.second, state.tone, state.name)
            assertEquals(spec.third, state.icon, state.name)
        }
    }

    @Test
    fun labelsAreUnique() {
        assertEquals(WitnessState.entries.size, WitnessState.entries.map { it.label }.toSet().size)
        assertEquals(WitnessState.entries.size, WitnessState.entries.map { it.shortLabel }.toSet().size)
    }

    @Test
    fun recentCheckShortLabelsFollowDesign() {
        assertEquals("With changes", WitnessState.Changes.shortLabel)
        assertEquals("No record", WitnessState.NoRecord.shortLabel)
        assertEquals("Original", WitnessState.Original.shortLabel)
    }

    @Test
    fun sealBadgeColorsUseLightTokensInEveryTheme() {
        assertEquals(WitnessColors.Ok, sealColor(WitnessState.Anchored))
        assertEquals(WitnessColors.Warn, sealColor(WitnessState.Queued))
        assertEquals(WitnessColors.Ink, sealColor(WitnessState.Sealed))
        assertEquals(WitnessColors.Edit, sealColor(WitnessState.Edited))
        assertEquals(WitnessColors.Crit, sealColor(WitnessState.Altered))
        assertEquals(WitnessColors.Muted, sealColor(WitnessState.Unverified))
    }
}
