package app.witness.core.designsystem

import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

internal val ToggleWidth = 46.dp
internal val ToggleHeight = 26.dp
internal val ToggleThumb = 19.dp

internal fun toggleThumbOffset(checked: Boolean): Dp = if (checked) 33.dp - ToggleThumb / 2 else 13.dp - ToggleThumb / 2

@Composable
fun WitnessToggle(checked: Boolean, onCheckedChange: (Boolean) -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true) {
    val palette = Witness.colors
    val track by animateColorAsState(if (checked) palette.primary else palette.surface3)
    val offset by animateDpAsState(toggleThumbOffset(checked))
    Box(
        modifier
            .size(ToggleWidth, ToggleHeight)
            .alpha(if (enabled) 1f else DisabledAlpha)
            .clip(WitnessShapes.Pill)
            .background(track)
            .toggleable(value = checked, enabled = enabled, role = Role.Switch, onValueChange = onCheckedChange),
        contentAlignment = Alignment.CenterStart,
    ) {
        Box(Modifier.offset(x = offset).size(ToggleThumb).clip(CircleShape).background(WitnessColors.White))
    }
}

@Composable
fun SegmentedControl(options: List<String>, selectedIndex: Int, onSelect: (Int) -> Unit, modifier: Modifier = Modifier, height: Dp = 44.dp) {
    val palette = Witness.colors
    Row(modifier.selectableGroup(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        options.forEachIndexed { index, label ->
            val selected = index == selectedIndex
            Box(
                Modifier
                    .weight(1f)
                    .height(height)
                    .clip(WitnessShapes.Pill)
                    .background(if (selected) palette.ink else palette.surface)
                    .then(if (selected) Modifier else Modifier.border(1.dp, palette.line, WitnessShapes.Pill))
                    .selectable(selected = selected, role = Role.RadioButton) { onSelect(index) },
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    label,
                    style = if (selected) Witness.type.bodyStrong else Witness.type.body,
                    color = if (selected) palette.onInk else palette.ink2,
                    maxLines = 1,
                )
            }
        }
    }
}

@Composable
fun GlassSegmentedControl(options: List<String>, selectedIndex: Int, onSelect: (Int) -> Unit, modifier: Modifier = Modifier, segmentWidth: Dp = 71.dp) {
    Row(
        modifier
            .height(36.dp)
            .clip(WitnessShapes.Pill)
            .background(WitnessColors.Black.copy(alpha = 0.5f))
            .padding(3.dp)
            .selectableGroup(),
    ) {
        options.forEachIndexed { index, label ->
            val selected = index == selectedIndex
            Box(
                Modifier
                    .width(segmentWidth)
                    .fillMaxHeight()
                    .clip(WitnessShapes.Pill)
                    .background(if (selected) WitnessColors.White else Color.Transparent)
                    .selectable(selected = selected, role = Role.Tab) { onSelect(index) },
                contentAlignment = Alignment.Center,
            ) {
                Text(label, style = Witness.type.bodySmallStrong, color = if (selected) WitnessColors.Ink else WitnessColors.White)
            }
        }
    }
}

@Composable
fun LinearMeter(fraction: Float, modifier: Modifier = Modifier, width: Dp = 60.dp, color: Color = Witness.colors.primary) {
    val clamped = fraction.coerceIn(0f, 1f)
    Box(modifier.size(width, 4.dp).clip(WitnessShapes.Pill).background(Witness.colors.surface2)) {
        Box(Modifier.size(width * clamped, 4.dp).clip(WitnessShapes.Pill).background(color))
    }
}
