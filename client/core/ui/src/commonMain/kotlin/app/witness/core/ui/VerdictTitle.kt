package app.witness.core.ui

import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessPalette
import app.witness.core.designsystem.WitnessState
import app.witness.core.model.Verdict
import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color

fun Verdict.title(): String = when (this) {
    Verdict.ORIGINAL -> "Original"
    Verdict.MATCHES_WITH_CHANGES -> "Matches an original, with changes"
    Verdict.ALTERED -> "Altered"
    Verdict.NO_RECORD -> "No Witness record"
    Verdict.SEALED_UNANCHORED -> "Sealed · not yet anchored"
}

fun Verdict.headline(): String = when (this) {
    Verdict.MATCHES_WITH_CHANGES -> "Matches an original"
    else -> title()
}

fun Verdict.qualifier(): String? = if (this == Verdict.MATCHES_WITH_CHANGES) "with changes" else null

fun Verdict.state(): WitnessState = when (this) {
    Verdict.ORIGINAL -> WitnessState.Original
    Verdict.MATCHES_WITH_CHANGES -> WitnessState.Changes
    Verdict.ALTERED -> WitnessState.Altered
    Verdict.NO_RECORD -> WitnessState.NoRecord
    Verdict.SEALED_UNANCHORED -> WitnessState.Unanchored
}

fun Verdict.summary(): String = when (this) {
    Verdict.ORIGINAL -> "Matches the sealed capture exactly."
    Verdict.MATCHES_WITH_CHANGES -> "This copy matches a sealed capture. No local edits found in the regions we could compare."
    Verdict.ALTERED -> "This copy matches a sealed capture, but one region was changed. The sealed reference is authentic — this copy is not."
    Verdict.NO_RECORD -> "We found no sealed capture for this image. That doesn't make it fake — it just wasn't proven with Witness, or its proof was removed."
    Verdict.SEALED_UNANCHORED -> "This file is validly sealed by an enrolled phone, but its proof hasn't reached the registry yet — usually it was shared straight from a phone that's offline."
}

@Immutable
data class VerdictColors(val fill: Color, val stroke: Color, val icon: Color, val title: Color)

fun Verdict.colors(palette: WitnessPalette): VerdictColors = when (this) {
    Verdict.NO_RECORD -> VerdictColors(palette.surface2, palette.muted.copy(alpha = 0.4f), palette.ink2, palette.ink)
    Verdict.SEALED_UNANCHORED -> VerdictColors(palette.ink.copy(alpha = 0.05f), palette.ink.copy(alpha = 0.25f), palette.ink, palette.ink)
    else -> {
        val tone: Tone = state().tone
        val color = palette.tone(tone)
        VerdictColors(color.copy(alpha = 0.07f), color.copy(alpha = 0.4f), color, palette.toneText(tone))
    }
}
