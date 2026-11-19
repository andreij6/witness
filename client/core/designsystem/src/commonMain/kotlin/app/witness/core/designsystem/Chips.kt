package app.witness.core.designsystem

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

const val TintAlpha = 0.12f

@Immutable
data class ChipColors(val container: Color, val content: Color, val text: Color, val border: Color? = null)

fun WitnessPalette.tintedChip(tone: Tone): ChipColors = ChipColors(tone(tone).copy(alpha = TintAlpha), tone(tone), toneText(tone))

fun WitnessPalette.solidChip(container: Color, content: Color = WitnessColors.White): ChipColors = ChipColors(container, content, content)

fun WitnessPalette.outlineChip(content: Color = ink2): ChipColors = ChipColors(Color.Transparent, content, content, line)

@Composable
fun WitnessChip(
    text: String,
    colors: ChipColors,
    modifier: Modifier = Modifier,
    icon: WitnessIcon? = null,
    dot: Boolean = false,
    height: Dp = 26.dp,
    fontSize: TextUnit = 12.sp,
    weight: FontWeight = FontWeight.SemiBold,
    mono: Boolean = false,
    onClick: (() -> Unit)? = null,
) {
    val base = if (mono) Witness.type.proof else Witness.type.chip
    val style = base.merge(TextStyle(fontSize = fontSize, fontWeight = weight, letterSpacing = 0.sp))
    Row(
        modifier
            .height(height)
            .clip(WitnessShapes.Pill)
            .background(colors.container)
            .then(if (colors.border != null) Modifier.border(1.dp, colors.border, WitnessShapes.Pill) else Modifier)
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier)
            .padding(start = if (icon != null) 9.dp else 10.dp, end = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        when {
            icon != null -> {
                WIcon(icon, colors.content, size = 15.dp)
                Spacer(Modifier.width(7.dp))
            }
            dot -> {
                Box(Modifier.size(7.dp).clip(CircleShape).background(colors.content))
                Spacer(Modifier.width(6.dp))
            }
        }
        Text(text, style = style, color = colors.text, maxLines = 1)
    }
}

@Composable
fun ToneChip(text: String, tone: Tone, modifier: Modifier = Modifier, icon: WitnessIcon? = null, height: Dp = 26.dp, fontSize: TextUnit = 12.sp) =
    WitnessChip(text, Witness.colors.tintedChip(tone), modifier, icon, height = height, fontSize = fontSize)

@Composable
fun StateChip(
    state: WitnessState,
    modifier: Modifier = Modifier,
    text: String = state.label,
    height: Dp = 26.dp,
    fontSize: TextUnit = 12.sp,
    solid: Boolean = false,
) {
    val palette = Witness.colors
    val colors = if (solid) palette.solidChip(palette.tone(state.tone)) else palette.tintedChip(state.tone)
    WitnessChip(text, colors, modifier, state.icon, height = height, fontSize = fontSize)
}

@Composable
fun FilterChip(text: String, selected: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    val colors = if (selected) palette.solidChip(palette.ink, palette.onInk) else palette.outlineChip()
    WitnessChip(
        text,
        colors,
        modifier.selectable(selected = selected, role = Role.Tab, onClick = onClick),
        height = 34.dp,
        fontSize = 13.sp,
        weight = if (selected) FontWeight.SemiBold else FontWeight.Medium,
    )
}

@Composable
fun GlassPill(text: String, icon: WitnessIcon, modifier: Modifier = Modifier, iconTint: Color = WitnessColors.White, trailing: WitnessIcon? = null, onClick: (() -> Unit)? = null) {
    Row(
        modifier
            .height(30.dp)
            .clip(WitnessShapes.Pill)
            .background(WitnessColors.Black.copy(alpha = 0.5f))
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier)
            .padding(start = 10.dp, end = if (trailing != null) 6.dp else 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(icon, iconTint, size = 16.dp)
        Spacer(Modifier.width(5.dp))
        Text(text, style = Witness.type.chip, color = WitnessColors.White, maxLines = 1)
        if (trailing != null) {
            Spacer(Modifier.width(2.dp))
            WIcon(trailing, WitnessColors.White, size = 18.dp)
        }
    }
}
