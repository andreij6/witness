package app.witness.core.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.ToneChip
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.designsystem.outlineChip

enum class Strength(val label: String, val tone: Tone) {
    Strong("Strong", Tone.Ok),
    Medium("Medium", Tone.Warn),
    Weak("Weak", Tone.Muted),
}

@Immutable
data class CustodyStep(val icon: WitnessIcon, val title: String, val detail: String, val strength: Strength? = null)

enum class LineSegment { None, Lower, Upper, Full }

fun custodyLine(index: Int, count: Int): LineSegment = when {
    count <= 1 -> LineSegment.None
    index == 0 -> LineSegment.Lower
    index == count - 1 -> LineSegment.Upper
    else -> LineSegment.Full
}

private val StepHeight = 62.dp
private val DiscSize = 36.dp

@Composable
fun ChainOfCustody(steps: List<CustodyStep>, modifier: Modifier = Modifier) {
    WitnessCard(modifier.fillMaxWidth(), contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 10.dp)) {
        steps.forEachIndexed { index, step -> CustodyRow(step, custodyLine(index, steps.size)) }
    }
}

@Composable
private fun CustodyRow(step: CustodyStep, line: LineSegment) {
    val palette = Witness.colors
    val type = Witness.type
    val lineColor = palette.line
    Row(Modifier.fillMaxWidth().height(StepHeight).semantics(mergeDescendants = true) {}) {
        Box(
            Modifier.width(DiscSize).fillMaxHeight().drawBehind {
                val x = size.width / 2
                val center = DiscSize.toPx() / 2
                val stroke = 2.dp.toPx()
                when (line) {
                    LineSegment.Lower -> drawLine(lineColor, Offset(x, center), Offset(x, size.height), stroke)
                    LineSegment.Upper -> drawLine(lineColor, Offset(x, 0f), Offset(x, center), stroke)
                    LineSegment.Full -> drawLine(lineColor, Offset(x, 0f), Offset(x, size.height), stroke)
                    LineSegment.None -> Unit
                }
            },
            contentAlignment = Alignment.TopCenter,
        ) {
            val proven = step.strength != null
            IconDisc(step.icon, if (proven) palette.ok else palette.ink, size = DiscSize, fill = if (proven) palette.ok.copy(alpha = 0.12f) else palette.surface2)
        }
        Spacer(Modifier.width(14.dp))
        Column(Modifier.weight(1f).padding(top = 2.dp)) {
            Text(step.title, style = type.rowTitle, color = palette.ink)
            Spacer(Modifier.height(3.dp))
            Text(step.detail, style = type.caption, color = palette.ink2)
        }
        if (step.strength != null) ToneChip(step.strength.label, step.strength.tone, Modifier.padding(top = 2.dp), height = 24.dp, fontSize = 11.sp)
    }
}

@Composable
fun ProofHeader(state: WitnessState, title: String, subtitle: String, modifier: Modifier = Modifier, context: String? = null, contextIcon: WitnessIcon = WitnessIcon.Folder) {
    val palette = Witness.colors
    val tone = palette.tone(state.tone)
    Row(modifier.fillMaxWidth().semantics(mergeDescendants = true) {}, verticalAlignment = Alignment.Top) {
        IconDisc(state.icon, tone, size = 56.dp, iconSize = 32.dp)
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(title, style = Witness.type.title, color = palette.ink)
            Text(subtitle, style = Witness.type.bodyCompact, color = palette.ink2)
            if (context != null) {
                Spacer(Modifier.size(6.dp))
                WitnessChip(context, palette.outlineChip(), icon = contextIcon, height = 24.dp, fontSize = 11.sp)
            }
        }
    }
}
