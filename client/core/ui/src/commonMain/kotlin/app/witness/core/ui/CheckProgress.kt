package app.witness.core.ui

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.LinearMeter
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon

enum class StageStatus { Done, Skipped, Running, Pending }

@Immutable
data class CheckStage(
    val title: String,
    val detail: String? = null,
    val status: StageStatus = StageStatus.Pending,
    val progress: Float? = null,
    val progressLabel: String? = null,
)

@Immutable
data class StageStyle(val icon: WitnessIcon, val iconTone: Tone, val titleTone: Tone, val detailTone: Tone?, val strong: Boolean)

fun StageStatus.style(): StageStyle = when (this) {
    StageStatus.Done -> StageStyle(WitnessIcon.CheckCircle, Tone.Ink, Tone.Ink, null, true)
    StageStatus.Skipped -> StageStyle(WitnessIcon.CheckCircle, Tone.Muted, Tone.Muted, null, false)
    StageStatus.Running -> StageStyle(WitnessIcon.Sync, Tone.Primary, Tone.Ink, Tone.Primary, true)
    StageStatus.Pending -> StageStyle(WitnessIcon.RadioButtonUnchecked, Tone.Muted, Tone.Muted, Tone.Muted, false)
}

fun StageStatus.accessibleLabel(): String = when (this) {
    StageStatus.Done -> "done"
    StageStatus.Skipped -> "not needed"
    StageStatus.Running -> "in progress"
    StageStatus.Pending -> "waiting"
}

fun overallProgress(stages: List<CheckStage>): Float {
    if (stages.isEmpty()) return 0f
    var sum = 0f
    for (s in stages) sum += when (s.status) {
        StageStatus.Done, StageStatus.Skipped -> 1f
        StageStatus.Running -> (s.progress ?: 0f).coerceIn(0f, 1f)
        StageStatus.Pending -> 0f
    }
    return sum / stages.size
}

@Composable
fun CheckProgress(stages: List<CheckStage>, modifier: Modifier = Modifier, dividers: Boolean = false) {
    val overall = overallProgress(stages)
    WitnessCard(
        modifier.fillMaxWidth().semantics { progressBarRangeInfo = ProgressBarRangeInfo(overall, 0f..1f) },
        contentPadding = PaddingValues(vertical = 8.dp),
    ) {
        stages.forEachIndexed { index, stage ->
            if (index > 0 && dividers) Hairline(Modifier.padding(horizontal = 16.dp))
            StageRow(stage)
        }
    }
}

@Composable
private fun StageRow(stage: CheckStage) {
    val palette = Witness.colors
    val type = Witness.type
    val style = stage.status.style()
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 64.dp)
            .semantics(mergeDescendants = true) { stateDescription = stage.status.accessibleLabel() }
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (stage.status == StageStatus.Running) SpinningIcon(style.icon, palette.tone(style.iconTone))
        else WIcon(style.icon, palette.tone(style.iconTone), size = 22.dp)
        Spacer(Modifier.width(14.dp))
        Column(Modifier.weight(1f)) {
            Text(stage.title, style = if (style.strong) type.bodyStrong else type.body, color = palette.tone(style.titleTone))
            if (stage.detail != null) Text(stage.detail, style = type.caption, color = style.detailTone?.let(palette::tone) ?: palette.ink2)
        }
        if (stage.status == StageStatus.Running && (stage.progress != null || stage.progressLabel != null)) {
            Column(horizontalAlignment = Alignment.End) {
                if (stage.progressLabel != null) Text(stage.progressLabel, style = type.captionStrong, color = palette.primary)
                if (stage.progress != null) LinearMeter(stage.progress, Modifier.padding(top = 6.dp))
            }
        }
    }
}

@Composable
private fun SpinningIcon(icon: WitnessIcon, color: Color) {
    val transition = rememberInfiniteTransition()
    val angle by transition.animateFloat(0f, -360f, infiniteRepeatable(tween(1200, easing = LinearEasing), RepeatMode.Restart))
    WIcon(icon, color, Modifier.graphicsLayer { rotationZ = angle }, size = 22.dp)
}
