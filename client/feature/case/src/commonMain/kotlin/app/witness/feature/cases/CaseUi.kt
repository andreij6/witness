package app.witness.feature.cases

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.DisabledAlpha
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessSheet
import app.witness.core.designsystem.outlineChip

internal val OptionShape = RoundedCornerShape(14.dp)
internal val IconBoxShape = RoundedCornerShape(12.dp)
internal const val SelectedRowAlpha = 0.06f
internal const val SelectedIconAlpha = 0.12f

@Composable
internal fun SheetTitle(text: String, modifier: Modifier = Modifier, subtitle: String? = null, bottom: Dp = 16.dp) {
    Column(modifier.fillMaxWidth().padding(start = 24.dp, end = 24.dp, top = 12.dp, bottom = bottom)) {
        Text(text, Modifier.semantics { heading() }, style = Witness.type.title, color = Witness.colors.ink)
        if (subtitle != null) {
            Spacer(Modifier.height(4.dp))
            Text(subtitle, style = Witness.type.subtitle, color = Witness.colors.ink2)
        }
    }
}

@Composable
internal fun Radio(selected: Boolean, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    WIcon(if (selected) WitnessIcon.RadioButtonChecked else WitnessIcon.RadioButtonUnchecked, if (selected) palette.primary else palette.muted, modifier, 24.dp)
}

@Composable
internal fun LaterChip() = WitnessChip("Later", Witness.colors.outlineChip(), height = 26.dp, fontSize = 11.5.sp)

@Composable
internal fun OptionRow(
    icon: WitnessIcon,
    title: String,
    subtitle: String,
    selected: Boolean,
    onClick: (() -> Unit)?,
    modifier: Modifier = Modifier,
    muted: Boolean = false,
    trailing: @Composable () -> Unit = { Radio(selected) },
) {
    val palette = Witness.colors
    Row(
        modifier
            .padding(horizontal = 16.dp)
            .fillMaxWidth()
            .height(56.dp)
            .clip(OptionShape)
            .background(if (selected) palette.primary.copy(alpha = SelectedRowAlpha) else Color.Transparent)
            .selectable(selected = selected, enabled = onClick != null, role = Role.RadioButton) { onClick?.invoke() }
            .padding(start = 8.dp, end = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier.size(40.dp).clip(IconBoxShape).background(if (selected) palette.primary.copy(alpha = SelectedIconAlpha) else palette.surface2),
            contentAlignment = Alignment.Center,
        ) { WIcon(icon, if (selected) palette.primary else if (muted) palette.muted else palette.ink2, size = 24.dp) }
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(title, style = Witness.type.rowTitle, color = if (muted) palette.muted else palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(subtitle, style = Witness.type.caption, color = if (muted) palette.muted else palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        trailing()
    }
}

@Composable
internal fun InfoNote(text: String, modifier: Modifier = Modifier, icon: WitnessIcon = WitnessIcon.Info, style: TextStyle = Witness.type.caption) {
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.Top) {
        WIcon(icon, Witness.colors.muted, Modifier.padding(top = 1.dp), 16.dp)
        Spacer(Modifier.width(6.dp))
        Text(text, style = style, color = Witness.colors.muted)
    }
}

@Composable
internal fun ErrorLine(text: String?, modifier: Modifier = Modifier, onDismiss: (() -> Unit)? = null) {
    if (text == null) return
    Text(
        text,
        modifier.fillMaxWidth().then(if (onDismiss != null) Modifier.clickable(role = Role.Button, onClick = onDismiss) else Modifier),
        style = Witness.type.bodySmallStrong,
        color = Witness.colors.crit,
    )
}

@Composable
internal fun OutlinedTone(text: String, icon: WitnessIcon, color: Color, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true) {
    Row(
        modifier
            .height(48.dp)
            .alpha(if (enabled) 1f else DisabledAlpha)
            .clip(WitnessShapes.Pill)
            .border(1.dp, Witness.colors.line, WitnessShapes.Pill)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .padding(horizontal = 20.dp),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(icon, color, size = 20.dp)
        Spacer(Modifier.width(6.dp))
        Text(text, style = Witness.type.button, color = color, maxLines = 1)
    }
}

@Composable
internal fun NoteField(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    modifier: Modifier = Modifier,
    minHeight: Dp = 88.dp,
) {
    val palette = Witness.colors
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth(),
        textStyle = Witness.type.body.merge(TextStyle(color = palette.ink)),
        cursorBrush = SolidColor(palette.primary),
        decorationBox = { inner ->
            Box(
                Modifier
                    .fillMaxWidth()
                    .height(minHeight)
                    .clip(WitnessShapes.Field)
                    .background(palette.surface)
                    .border(1.dp, palette.line, WitnessShapes.Field)
                    .padding(16.dp),
            ) {
                if (value.isEmpty()) Text(placeholder, style = Witness.type.body, color = palette.muted)
                inner()
            }
        },
    )
}

@Composable
internal fun SheetDialog(onDismiss: () -> Unit, content: @Composable ColumnScope.() -> Unit) {
    ClearDialogDim()
    WitnessSheet(visible = true, onDismiss = onDismiss, content = content)
}

@Composable
internal fun <T : Any> rememberLast(value: T?): T? {
    val holder = remember { arrayOfNulls<Any>(1) }
    if (value != null) holder[0] = value
    @Suppress("UNCHECKED_CAST")
    return holder[0] as T?
}

internal val MediumWeight = TextStyle(fontWeight = FontWeight.Medium)
internal val SemiBoldWeight = TextStyle(fontWeight = FontWeight.SemiBold)
