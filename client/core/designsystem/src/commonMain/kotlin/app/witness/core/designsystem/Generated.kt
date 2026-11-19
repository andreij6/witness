package app.witness.core.designsystem

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

const val GeneratedBorderAlpha = 0.35f
const val GeneratedRailAlpha = 0.9f
const val GeneratedDescription = "Generated on-device"

@Composable
fun GeneratedSurface(
    modifier: Modifier = Modifier,
    contentPadding: PaddingValues = PaddingValues(start = 18.dp, end = 16.dp, top = 14.dp, bottom = 14.dp),
    content: @Composable ColumnScope.() -> Unit,
) {
    val palette = Witness.colors
    val rail = palette.ai.copy(alpha = GeneratedRailAlpha)
    Column(
        modifier
            .clip(WitnessShapes.Card)
            .background(palette.surface)
            .border(1.2.dp, palette.ai.copy(alpha = GeneratedBorderAlpha), WitnessShapes.Card)
            .drawBehind {
                val inset = 18.dp.toPx()
                val w = 3.dp.toPx()
                drawRoundRect(rail, Offset(0f, inset), Size(w, size.height - inset * 2), CornerRadius(w / 2))
            }
            .semantics { stateDescription = GeneratedDescription }
            .padding(contentPadding),
        content = content,
    )
}

@Composable
fun GeneratedTag(text: String = GeneratedDescription, modifier: Modifier = Modifier, onClick: (() -> Unit)? = null) =
    WitnessChip(text, Witness.colors.tintedChip(Tone.Ai), modifier, WitnessIcon.AutoAwesome, height = 24.dp, fontSize = 11.sp, onClick = onClick)

@Composable
fun AiHeader(label: String, modifier: Modifier = Modifier, onFlag: (() -> Unit)? = null, trailing: @Composable (() -> Unit)? = null) {
    val palette = Witness.colors
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        WIcon(WitnessIcon.AutoAwesome, palette.ai, size = 16.dp)
        Spacer(Modifier.width(6.dp))
        Overline(label, Modifier.weight(1f), palette.ai)
        trailing?.invoke()
        if (onFlag != null) FlagButton(onFlag)
    }
}

@Composable
fun FlagButton(onClick: () -> Unit, modifier: Modifier = Modifier, label: String = "Flag generated content") =
    WitnessIconButton(WitnessIcon.Flag, label, onClick, modifier, Witness.colors.muted, size = 32.dp, iconSize = 18.dp)
