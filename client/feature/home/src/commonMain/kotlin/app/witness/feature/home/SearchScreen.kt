package app.witness.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
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
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.FilterChip
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.SealBadge
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessState
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.ui.CaseMosaic
import app.witness.core.ui.Formatters

const val HIGHLIGHT_ALPHA = 0.14f

@Composable
fun SearchScreen(
    state: SearchUiState,
    query: String,
    onQuery: (String) -> Unit,
    onFilter: (SearchFilter) -> Unit,
    onBack: () -> Unit,
    onItem: (RecordId) -> Unit,
    onCase: (LocalCaseId) -> Unit,
    modifier: Modifier = Modifier,
    autoFocus: Boolean = true,
) {
    val palette = Witness.colors
    Column(modifier.fillMaxSize().background(palette.paper).windowInsetsPadding(WindowInsets.statusBars).imePadding()) {
        Row(Modifier.fillMaxWidth().padding(start = 8.dp, end = Gutter, top = 10.dp), verticalAlignment = Alignment.CenterVertically) {
            WitnessIconButton(WitnessIcon.ArrowBack, "Back", onBack)
            HSpace(8.dp)
            SearchField(query, onQuery, Modifier.weight(1f), autoFocus)
        }
        Row(Modifier.padding(start = Gutter, end = Gutter, top = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SearchFilter.entries.forEach { f -> FilterChip(f.label, f == state.filter, { onFilter(f) }) }
        }
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(start = Gutter, end = Gutter, top = 8.dp, bottom = 32.dp)) {
            if (state.items.isNotEmpty()) {
                item(key = "items") {
                    Overline(Formatters.joinDot(SearchCopy.ITEMS, state.items.size.toString()), Modifier.padding(top = 16.dp, bottom = 10.dp))
                    WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(0.dp)) {
                        state.items.forEachIndexed { i, row ->
                            if (i > 0) Hairline(Modifier.padding(horizontal = 16.dp))
                            ResultRow(row.title, row.subtitle, state.terms, onClick = { onItem(row.recordId) }) {
                                Box(Modifier.size(56.dp)) {
                                    Box(Modifier.size(56.dp).clip(WitnessShapes.Thumb)) { RecordThumb(row.recordId, 56.dp) }
                                    SealBadge(row.state, Modifier.align(Alignment.BottomEnd).offset(4.dp, 4.dp), radius = 8.dp)
                                }
                            }
                        }
                    }
                }
            }
            if (state.cases.isNotEmpty()) {
                item(key = "cases") {
                    Overline(Formatters.joinDot(SearchCopy.CASES, state.cases.size.toString()), Modifier.padding(top = 20.dp, bottom = 10.dp))
                    WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(0.dp)) {
                        state.cases.forEachIndexed { i, row ->
                            if (i > 0) Hairline(Modifier.padding(horizontal = 16.dp))
                            ResultRow(row.title, row.subtitle, state.terms, onClick = { onCase(row.id) }) {
                                CaseMosaic(row.covers.size, Modifier.size(76.dp, 56.dp)) { i -> RecordThumb(row.covers[i], 76.dp) }
                            }
                        }
                    }
                }
            }
            if (state.noResults) item(key = "none") { Footnote(SearchCopy.noResults(query), Modifier.padding(top = 32.dp), color = palette.ink2) }
            item(key = "on-device") {
                Row(Modifier.fillMaxWidth().padding(top = 48.dp), horizontalArrangement = Arrangement.Center, verticalAlignment = Alignment.CenterVertically) {
                    WIcon(WitnessIcon.Lock, palette.muted, size = 14.dp)
                    HSpace(6.dp)
                    Text(SearchCopy.ON_DEVICE, style = Witness.type.caption, color = palette.muted)
                }
            }
        }
    }
}

@Composable
private fun SearchField(query: String, onQuery: (String) -> Unit, modifier: Modifier, autoFocus: Boolean) {
    val palette = Witness.colors
    val focus = remember { FocusRequester() }
    val focusManager = LocalFocusManager.current
    if (autoFocus) LaunchedEffect(Unit) { focus.requestFocus() }
    val style = Witness.type.bodyLarge.merge(TextStyle(color = palette.ink))
    BasicTextField(
        query,
        onQuery,
        modifier.focusRequester(focus).semantics { contentDescription = SearchCopy.PLACEHOLDER },
        singleLine = true,
        textStyle = style,
        cursorBrush = SolidColor(palette.primary),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
        keyboardActions = KeyboardActions(onSearch = { focusManager.clearFocus() }),
        decorationBox = { inner ->
            Row(
                Modifier.height(44.dp).clip(WitnessShapes.Pill).background(palette.surface).border(1.5.dp, palette.primary, WitnessShapes.Pill).padding(start = 20.dp, end = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(Modifier.weight(1f)) {
                    if (query.isEmpty()) Text(SearchCopy.PLACEHOLDER, style = style, color = palette.muted, maxLines = 1)
                    inner()
                }
                if (query.isNotEmpty()) WitnessIconButton(WitnessIcon.Close, "Clear search", { onQuery("") }, tint = palette.ink2, size = 36.dp, iconSize = 20.dp)
            }
        },
    )
}

@Composable
private fun ResultRow(title: String, subtitle: String, terms: List<String>, onClick: () -> Unit, leading: @Composable () -> Unit) {
    val palette = Witness.colors
    val mark = palette.primary.copy(alpha = HIGHLIGHT_ALPHA)
    Row(
        Modifier.fillMaxWidth().clickable(role = Role.Button, onClick = onClick).padding(start = 12.dp, end = 12.dp, top = 12.dp, bottom = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        leading()
        HSpace(14.dp)
        Column(Modifier.weight(1f)) {
            Text(Highlight.annotate(title, terms, mark), style = Witness.type.bodyStrong, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
            VSpace(6.dp)
            Text(Highlight.annotate(subtitle, terms, mark), style = Witness.type.subtitle, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        WIcon(WitnessIcon.ChevronRight, palette.muted)
    }
}

@Preview
@Composable
private fun SearchPreview() = WitnessTheme { SearchScreen(SnapshotData.search, "7MNV", {}, {}, {}, {}, {}, autoFocus = false) }
