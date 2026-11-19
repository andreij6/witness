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
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.GeneratedTag
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.model.LocalCaseId

@Composable
fun ColumnScope.CaseSwitcherContent(state: CaseSwitcherUiState, onSelect: (LocalCaseId) -> Unit, onNewCase: () -> Unit) {
    val palette = Witness.colors
    SheetTitle("Capture into")
    LazyColumn(Modifier.heightIn(max = 384.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        items(state.options, key = { it.id.value }) { option ->
            OptionRow(WitnessIcon.Folder, option.title, option.subtitle, option.selected, { onSelect(option.id) })
        }
    }
    Hairline(Modifier.padding(start = 16.dp, end = 16.dp, top = 12.dp))
    Row(
        Modifier
            .padding(start = 16.dp, end = 16.dp, top = 8.dp)
            .fillMaxWidth()
            .height(48.dp)
            .clip(OptionShape)
            .clickable(role = Role.Button, onClick = onNewCase)
            .padding(start = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(40.dp).clip(IconBoxShape).background(palette.primary.copy(alpha = 0.1f)), contentAlignment = Alignment.Center) {
            WIcon(WitnessIcon.Add, palette.primary, size = 24.dp)
        }
        Spacer(Modifier.width(12.dp))
        Text("New case", style = Witness.type.rowTitle, color = palette.primary)
    }
    InfoNote("Post mode captures don't go into a case.", Modifier.padding(start = 24.dp, end = 24.dp, top = 20.dp, bottom = 24.dp))
    ErrorLine(state.error, Modifier.padding(start = 24.dp, end = 24.dp, bottom = 16.dp))
}

@Composable
fun ColumnScope.NewCaseContent(state: NewCaseUiState, onName: (String) -> Unit, onSelect: (ReportType) -> Unit, onCreate: () -> Unit) {
    val palette = Witness.colors
    SheetTitle("New case")
    NameField(state.name, onName, onCreate, Modifier.padding(horizontal = 16.dp))
    Overline("What happened?", Modifier.padding(start = 24.dp, top = 24.dp, bottom = 10.dp))
    Column(Modifier.padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        ReportType.situations.chunked(2).forEach { pair ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                pair.forEach { type -> SituationChip(type, type == state.selected, Modifier.weight(1f)) { onSelect(type) } }
            }
        }
    }
    GeneratedTag("Report suggested on-device", Modifier.padding(start = 24.dp, top = 16.dp))
    Text(
        "Witness picks a report template from the name and your first photos. You can change it anytime.",
        Modifier.padding(start = 24.dp, end = 24.dp, top = 8.dp),
        style = Witness.type.subtitle,
        color = palette.ink2,
    )
    ErrorLine(state.error, Modifier.padding(start = 24.dp, end = 24.dp, top = 8.dp))
    WitnessButton("Create case", onCreate, Modifier.padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 24.dp).fillMaxWidth(), enabled = state.canCreate)
}

@Composable
private fun NameField(value: String, onValueChange: (String) -> Unit, onDone: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { runCatching { focus.requestFocus() } }
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth().focusRequester(focus),
        singleLine = true,
        textStyle = Witness.type.bodyLarge.merge(TextStyle(color = palette.ink)),
        cursorBrush = SolidColor(palette.primary),
        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences, imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = { onDone() }),
        decorationBox = { inner ->
            Column(
                Modifier
                    .fillMaxWidth()
                    .height(56.dp)
                    .clip(WitnessShapes.Field)
                    .background(palette.surface)
                    .border(1.5.dp, palette.primary, WitnessShapes.Field)
                    .padding(start = 16.dp, end = 16.dp, top = 7.dp),
            ) {
                Text("Name", style = Witness.type.meta.merge(MediumWeight), color = palette.primary)
                Spacer(Modifier.height(2.dp))
                inner()
            }
        },
    )
}

@Composable
private fun SituationChip(type: ReportType, selected: Boolean, modifier: Modifier = Modifier, onClick: () -> Unit) {
    val palette = Witness.colors
    Row(
        modifier
            .height(40.dp)
            .clip(WitnessShapes.Pill)
            .background(if (selected) palette.ink else Color.Transparent)
            .then(if (selected) Modifier else Modifier.border(1.dp, palette.line, WitnessShapes.Pill))
            .selectable(selected = selected, role = Role.RadioButton, onClick = onClick)
            .padding(start = 14.dp, end = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(type.chipIcon, if (selected) palette.onInk else palette.ink2, size = 20.dp)
        Spacer(Modifier.width(8.dp))
        Text(type.chipLabel, style = Witness.type.bodySmall.merge(if (selected) SemiBoldWeight else MediumWeight), color = if (selected) palette.onInk else palette.ink, maxLines = 1)
    }
}
