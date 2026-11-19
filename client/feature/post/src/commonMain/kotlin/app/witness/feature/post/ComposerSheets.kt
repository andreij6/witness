package app.witness.feature.post

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.WitnessSheet
import app.witness.core.designsystem.WitnessTextField

@Composable
private fun <T : Any> rememberLast(value: T?): T? {
    var last by remember { mutableStateOf(value) }
    if (value != null && value != last) last = value
    return value ?: last
}

@Composable
private fun SheetTitle(title: String, subtitle: String) {
    Spacer(Modifier.height(12.dp))
    Text(title, Modifier.semantics { heading() }, style = Witness.type.title, color = Witness.colors.ink)
    Spacer(Modifier.height(6.dp))
    Text(subtitle, style = Witness.type.bodyCompact, color = Witness.colors.ink2)
}

private val OptionShape = RoundedCornerShape(14.dp)

@Composable
internal fun LocationSheet(sheet: ComposerSheet.Location?, place: String?, actions: ComposerActions) {
    val shown = rememberLast(sheet)
    WitnessSheet(sheet != null, actions::dismissSheet) {
        val selected = shown?.selected ?: return@WitnessSheet
        val palette = Witness.colors
        Column(Modifier.padding(horizontal = 16.dp)) {
            SheetTitle(PostCopy.SHEET_TITLE, PostCopy.SHEET_SUBTITLE)
            Spacer(Modifier.height(18.dp))
            Column(Modifier.selectableGroup(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                for (o in PostCopy.precisionOptions(place)) {
                    val on = o.precision == selected
                    val sub = o.tone?.let(palette::toneText) ?: palette.ink2
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .height(60.dp)
                            .clip(OptionShape)
                            .background(if (on) palette.primary.copy(alpha = 0.07f) else Color.Transparent)
                            .selectable(on, role = Role.RadioButton) { actions.selectPrecision(o.precision) }
                            .padding(start = 12.dp, end = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        WIcon(if (on) WitnessIcon.RadioButtonChecked else WitnessIcon.RadioButtonUnchecked, if (on) palette.primary else palette.ink2)
                        Spacer(Modifier.width(12.dp))
                        Column {
                            Text(o.title, style = Witness.type.actionTitle.copy(fontWeight = if (on) FontWeight.SemiBold else FontWeight.Medium), color = palette.ink, maxLines = 1)
                            Spacer(Modifier.height(4.dp))
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                if (o.icon != null) {
                                    WIcon(o.icon, sub, size = 14.dp)
                                    Spacer(Modifier.width(4.dp))
                                }
                                Text(o.subtitle, style = Witness.type.subtitle, color = sub, maxLines = 1)
                            }
                        }
                    }
                }
            }
            Spacer(Modifier.height(16.dp))
            Row(verticalAlignment = Alignment.Top) {
                WIcon(WitnessIcon.Lock, palette.ink2, Modifier.padding(top = 2.dp), size = 16.dp)
                Spacer(Modifier.width(8.dp))
                Text(PostCopy.SHEET_NOTE, style = Witness.type.subtitle, color = palette.ink2)
            }
            Spacer(Modifier.height(30.dp))
            WitnessButton("Done", actions::confirmLocation, Modifier.fillMaxWidth())
            Spacer(Modifier.height(12.dp))
        }
    }
}

@Composable
internal fun AltTextSheet(sheet: ComposerSheet.AltText?, actions: ComposerActions) {
    val shown = rememberLast(sheet)
    WitnessSheet(sheet != null, actions::dismissSheet) {
        val text = shown?.text ?: return@WitnessSheet
        Column(Modifier.padding(horizontal = 16.dp)) {
            SheetTitle("Alt text", "Describes the photo for people who use screen readers.")
            Spacer(Modifier.height(16.dp))
            WitnessTextField(
                text,
                actions::editAltText,
                Modifier.fillMaxWidth(),
                placeholder = "Describe the photo",
                shape = RoundedCornerShape(12.dp),
                minHeight = 96.dp,
                singleLine = false,
            )
            Spacer(Modifier.height(24.dp))
            WitnessButton("Done", actions::confirmAltText, Modifier.fillMaxWidth())
            Spacer(Modifier.height(12.dp))
        }
    }
}
