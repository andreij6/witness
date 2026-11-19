package app.witness.feature.share

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.ChipColors
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessSheet
import app.witness.core.designsystem.WitnessToggle

private val OptionShape = RoundedCornerShape(16.dp)
private val IconShape = RoundedCornerShape(12.dp)

@Composable
fun ColumnScope.ExportContent(
    state: ExportUiState,
    onSelect: (ExportFormat) -> Unit,
    onExactLocation: (Boolean) -> Unit,
    onExport: () -> Unit,
) {
    val palette = Witness.colors
    Column(Modifier.fillMaxWidth().padding(start = 24.dp, end = 24.dp, top = 12.dp, bottom = 20.dp)) {
        Text("Export case", Modifier.semantics { heading() }, style = Witness.type.title, color = palette.ink)
        Spacer(Modifier.height(4.dp))
        Text(state.subtitle, style = Witness.type.subtitle, color = palette.ink2)
    }
    FormatOption(
        icon = WitnessIcon.FolderZip,
        title = "ZIP of sealed files + proof records",
        tag = "Free",
        tagIcon = null,
        subtitle = "Anyone can check them",
        selected = state.format == ExportFormat.ZIP,
        locked = false,
    ) { onSelect(ExportFormat.ZIP) }
    Spacer(Modifier.height(12.dp))
    FormatOption(
        icon = WitnessIcon.PictureAsPdf,
        title = "PDF report",
        tag = "Plus",
        tagIcon = WitnessIcon.WorkspacePremium,
        subtitle = "Formatted report with photos",
        selected = state.format == ExportFormat.PDF,
        locked = state.pdfLocked,
    ) { onSelect(ExportFormat.PDF) }
    Row(Modifier.fillMaxWidth().padding(start = 24.dp, end = 24.dp, top = 24.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("Include exact location", Modifier.weight(1f), style = Witness.type.actionTitle.merge(TextStyle(fontSize = 14.5.sp, fontWeight = FontWeight.Medium)), color = palette.ink)
        WitnessToggle(state.exactLocation, onExactLocation)
    }
    Text(state.locationHint, Modifier.padding(start = 24.dp, end = 24.dp, top = 4.dp), style = Witness.type.caption, color = palette.muted)
    state.error?.let { Text(it, Modifier.padding(start = 24.dp, end = 24.dp, top = 12.dp), style = Witness.type.bodySmallStrong, color = palette.crit) }
    WitnessButton(
        if (state.exporting) "Exporting…" else state.buttonLabel,
        onExport,
        Modifier.padding(start = 16.dp, end = 16.dp, top = 32.dp, bottom = 24.dp).fillMaxWidth(),
        icon = if (state.format == ExportFormat.PDF) WitnessIcon.PictureAsPdf else WitnessIcon.FolderZip,
        enabled = state.canExport,
    )
}

@Composable
private fun FormatOption(
    icon: WitnessIcon,
    title: String,
    tag: String,
    tagIcon: WitnessIcon?,
    subtitle: String,
    selected: Boolean,
    locked: Boolean,
    onClick: () -> Unit,
) {
    val palette = Witness.colors
    Row(
        Modifier
            .padding(horizontal = 16.dp)
            .fillMaxWidth()
            .height(80.dp)
            .clip(OptionShape)
            .background(if (selected) palette.primary.copy(alpha = 0.04f) else palette.surface)
            .border(if (selected) 1.5.dp else 1.dp, if (selected) palette.primary else palette.line, OptionShape)
            .selectable(selected, role = Role.RadioButton, onClick = onClick)
            .padding(start = 12.dp, end = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(
            Modifier.size(40.dp).clip(IconShape).background(if (selected) palette.primary.copy(alpha = 0.12f) else palette.surface2),
            verticalArrangement = androidx.compose.foundation.layout.Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) { WIcon(icon, if (selected) palette.primary else palette.muted, size = 24.dp) }
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(title, style = Witness.type.bodyStrong, color = if (selected) palette.ink else palette.ink2, maxLines = 1)
            Spacer(Modifier.height(6.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                val fg = if (selected) palette.ink2 else palette.ink
                WitnessChip(tag, ChipColors(fg.copy(alpha = 0.1f), fg, fg), icon = tagIcon, height = 22.dp, fontSize = 11.sp)
                Spacer(Modifier.width(8.dp))
                Text(subtitle, style = Witness.type.caption, color = if (selected) palette.ink2 else palette.muted, maxLines = 1)
            }
        }
        if (locked) {
            WIcon(WitnessIcon.Lock, palette.muted, size = 20.dp, contentDescription = "Witness Plus")
        } else {
            WIcon(
                if (selected) WitnessIcon.RadioButtonChecked else WitnessIcon.RadioButtonUnchecked,
                if (selected) palette.primary else palette.muted,
                size = 24.dp,
            )
        }
    }
}

@Composable
internal fun SheetDialog(onDismiss: () -> Unit, content: @Composable ColumnScope.() -> Unit) {
    ClearDialogDim()
    WitnessSheet(visible = true, onDismiss = onDismiss, content = content)
}
