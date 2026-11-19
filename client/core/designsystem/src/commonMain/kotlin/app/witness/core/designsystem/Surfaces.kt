package app.witness.core.designsystem

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

const val PanelFillAlpha = 0.07f
const val PanelStrokeAlpha = 0.4f

@Composable
fun WitnessCard(
    modifier: Modifier = Modifier,
    shape: Shape = WitnessShapes.Card,
    color: Color = Witness.colors.surface,
    border: Color? = Witness.colors.line,
    contentPadding: PaddingValues = PaddingValues(16.dp),
    onClick: (() -> Unit)? = null,
    content: @Composable ColumnScope.() -> Unit,
) {
    Column(
        modifier
            .clip(shape)
            .background(color)
            .then(if (border != null) Modifier.border(1.dp, border, shape) else Modifier)
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier)
            .padding(contentPadding),
        content = content,
    )
}

@Composable
fun TonePanel(
    color: Color,
    modifier: Modifier = Modifier,
    fill: Color = color.copy(alpha = PanelFillAlpha),
    stroke: Color = color.copy(alpha = PanelStrokeAlpha),
    contentPadding: PaddingValues = PaddingValues(16.dp),
    content: @Composable ColumnScope.() -> Unit,
) = WitnessCard(modifier, WitnessShapes.Panel, fill, stroke, contentPadding, content = content)

fun Modifier.dashedBorder(color: Color, cornerRadius: Dp, width: Dp = 1.2.dp, dash: Dp = 6.dp, gap: Dp = 5.dp): Modifier = drawWithCache {
    val strokePx = width.toPx()
    val effect = PathEffect.dashPathEffect(floatArrayOf(dash.toPx(), gap.toPx()))
    val stroke = Stroke(strokePx, pathEffect = effect)
    val inset = strokePx / 2
    val radius = CornerRadius(cornerRadius.toPx())
    onDrawBehind {
        drawRoundRect(color, Offset(inset, inset), Size(size.width - strokePx, size.height - strokePx), radius, style = stroke)
    }
}

@Composable
fun DropZone(modifier: Modifier = Modifier, onClick: (() -> Unit)? = null, content: @Composable ColumnScope.() -> Unit) {
    val palette = Witness.colors
    Column(
        modifier
            .clip(WitnessShapes.Panel)
            .background(palette.surface)
            .dashedBorder(palette.muted.copy(alpha = 0.6f), 20.dp)
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier)
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        content = content,
    )
}

@Composable
fun Overline(text: String, modifier: Modifier = Modifier, color: Color = Witness.colors.muted) {
    Text(overlineText(text), modifier.semantics { heading() }, style = Witness.type.overline, color = color, maxLines = 1)
}

fun overlineText(text: String): String = text.uppercase()

@Composable
fun SectionTitle(text: String, modifier: Modifier = Modifier, trailing: @Composable (() -> Unit)? = null) {
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(text, Modifier.weight(1f).semantics { heading() }, style = Witness.type.section, color = Witness.colors.ink)
        trailing?.invoke()
    }
}

@Composable
fun Hairline(modifier: Modifier = Modifier, color: Color = Witness.colors.line) {
    Box(modifier.fillMaxWidth().height(1.dp).background(color))
}

@Composable
fun IconDisc(icon: WitnessIcon, tint: Color, modifier: Modifier = Modifier, size: Dp = 36.dp, iconSize: Dp = 20.dp, fill: Color = tint.copy(alpha = TintAlpha)) {
    Box(modifier.size(size).clip(CircleShape).background(fill), contentAlignment = Alignment.Center) { WIcon(icon, tint, size = iconSize) }
}

@Composable
fun ActionRow(
    icon: WitnessIcon,
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    tint: Color = Witness.colors.ink,
    chevron: Boolean = true,
    onClick: (() -> Unit)? = null,
    trailing: @Composable (() -> Unit)? = null,
) {
    val palette = Witness.colors
    Row(
        modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier)
            .padding(horizontal = 24.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(icon, tint)
        Spacer(Modifier.width(20.dp))
        Column(Modifier.weight(1f)) {
            Text(title, style = Witness.type.actionTitle, color = tint)
            if (subtitle != null) Text(subtitle, style = Witness.type.subtitle, color = palette.ink2)
        }
        trailing?.invoke()
        if (chevron && onClick != null) WIcon(WitnessIcon.ChevronRight, palette.muted)
    }
}

@Composable
fun StatusLine(text: String, icon: WitnessIcon, modifier: Modifier = Modifier, tint: Color = Witness.colors.ink2, textColor: Color = tint, strong: Boolean = false) {
    Row(modifier, verticalAlignment = Alignment.CenterVertically) {
        WIcon(icon, tint, size = 18.dp)
        Spacer(Modifier.width(8.dp))
        Text(text, style = if (strong) Witness.type.labelStrong else Witness.type.subtitle, color = textColor)
    }
}
