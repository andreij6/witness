package app.witness.core.designsystem

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

enum class ButtonKind { Filled, Ink, Tonal, Outlined, Glass, Danger }

@Immutable
data class ButtonColors(val container: Color, val content: Color, val border: Color?)

const val GlassAlpha = 0.42f
const val DisabledAlpha = 0.38f

fun ButtonKind.colors(palette: WitnessPalette): ButtonColors = when (this) {
    ButtonKind.Filled -> ButtonColors(palette.primary, palette.onPrimary, null)
    ButtonKind.Ink -> ButtonColors(palette.ink, palette.onInk, null)
    ButtonKind.Tonal -> ButtonColors(palette.surface3, palette.ink, null)
    ButtonKind.Outlined -> ButtonColors(Color.Transparent, palette.ink, palette.line)
    ButtonKind.Glass -> ButtonColors(WitnessColors.Black.copy(alpha = GlassAlpha), WitnessColors.White, null)
    ButtonKind.Danger -> ButtonColors(palette.crit, WitnessColors.White, null)
}

@Composable
fun WitnessButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    kind: ButtonKind = ButtonKind.Filled,
    icon: WitnessIcon? = null,
    height: Dp = 48.dp,
    enabled: Boolean = true,
    textStyle: TextStyle = Witness.type.button,
) {
    val colors = kind.colors(Witness.colors)
    Row(
        modifier
            .height(height)
            .alpha(if (enabled) 1f else DisabledAlpha)
            .clip(WitnessShapes.Pill)
            .background(colors.container)
            .then(if (colors.border != null) Modifier.border(1.dp, colors.border, WitnessShapes.Pill) else Modifier)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .padding(horizontal = 24.dp),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) {
            WIcon(icon, colors.content, size = 20.dp)
            Spacer(Modifier.width(6.dp))
        }
        Text(text, style = textStyle, color = colors.content, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

@Composable
fun WitnessIconButton(
    icon: WitnessIcon,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    tint: Color = Witness.colors.ink,
    container: Color = Color.Transparent,
    size: Dp = 40.dp,
    iconSize: Dp = 24.dp,
    enabled: Boolean = true,
) {
    Box(
        modifier
            .size(size)
            .alpha(if (enabled) 1f else DisabledAlpha)
            .clip(CircleShape)
            .background(container)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .semantics { this.contentDescription = contentDescription },
        contentAlignment = Alignment.Center,
    ) { WIcon(icon, tint, size = iconSize) }
}

@Composable
fun GlassIconButton(icon: WitnessIcon, contentDescription: String, onClick: () -> Unit, modifier: Modifier = Modifier) =
    WitnessIconButton(icon, contentDescription, onClick, modifier, WitnessColors.White, WitnessColors.Black.copy(alpha = GlassAlpha))

@Composable
fun TextAction(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, color: Color = Witness.colors.primary, icon: WitnessIcon? = null) {
    Row(
        modifier.clip(WitnessShapes.Field).clickable(role = Role.Button, onClick = onClick).padding(vertical = 4.dp, horizontal = 2.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) {
            WIcon(icon, color, size = 18.dp)
            Spacer(Modifier.width(4.dp))
        }
        Text(text, style = Witness.type.bodyStrong, color = color)
    }
}
