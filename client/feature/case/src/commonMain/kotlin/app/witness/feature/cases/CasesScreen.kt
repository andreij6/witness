package app.witness.feature.cases

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.FilterChip
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessAppBar
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.ui.CaseCard
import app.witness.core.ui.stateCounts

@Composable
fun CasesScreen(
    state: CasesUiState,
    onFilter: (CaseFilter) -> Unit,
    onOpen: (CaseCardUi) -> Unit,
    onNewCase: () -> Unit,
    onBack: () -> Unit,
    onSearch: () -> Unit,
    modifier: Modifier = Modifier,
    image: RecordImage = {},
) {
    val palette = Witness.colors
    Box(modifier.fillMaxSize().background(palette.paper)) {
        Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars)) {
            WitnessAppBar("Cases", onNavigate = onBack, actions = { WitnessIconButton(WitnessIcon.Search, "Search", onSearch) })
            LazyColumn(
                Modifier.fillMaxSize(),
                contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 4.dp, bottom = 112.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                item(key = "filters") {
                    Row(Modifier.padding(bottom = 6.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        CaseFilter.entries.forEach { f -> FilterChip(f.label, f == state.filter, { onFilter(f) }) }
                    }
                }
                if (state.isEmpty) item(key = "empty") { EmptyCases(state.filter) }
                items(state.cases, key = { it.id.value }) { card ->
                    CaseCard(
                        title = card.title,
                        meta = card.meta,
                        counts = stateCounts(card.anchored, card.queued),
                        thumbnailCount = card.covers.size,
                        sharedLabel = card.sharedLabel,
                        onClick = { onOpen(card) },
                    ) { index -> Box(Modifier.fillMaxSize().background(palette.surface2)) { image(card.covers[index]) } }
                }
            }
        }
        NewCaseFab(onNewCase, Modifier.align(Alignment.BottomEnd).windowInsetsPadding(WindowInsets.navigationBars).padding(16.dp))
    }
}

@Composable
private fun EmptyCases(filter: CaseFilter) {
    val palette = Witness.colors
    Column(Modifier.fillMaxWidth().padding(top = 48.dp, start = 24.dp, end = 24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        WIcon(WitnessIcon.FolderOpen, palette.muted, size = 40.dp)
        Spacer(Modifier.height(12.dp))
        Text(emptyTitle(filter), style = Witness.type.section, color = palette.ink, textAlign = TextAlign.Center)
        Spacer(Modifier.height(4.dp))
        Text(EMPTY_HINT, style = Witness.type.subtitle, color = palette.ink2, textAlign = TextAlign.Center)
    }
}

internal const val EMPTY_HINT = "Cases keep evidence together and build a report on your phone."

internal fun emptyTitle(filter: CaseFilter): String = when (filter) {
    CaseFilter.ALL -> "No cases yet"
    CaseFilter.OPEN -> "No open cases"
    CaseFilter.SHARED -> "No shared cases"
}

@Composable
private fun NewCaseFab(onClick: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Row(
        modifier
            .height(56.dp)
            .clip(RoundedCornerShape(16.dp))
            .background(palette.primary)
            .clickable(role = Role.Button, onClick = onClick)
            .padding(start = 18.dp, end = 24.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(WitnessIcon.Add, palette.onPrimary, size = 24.dp)
        Spacer(Modifier.width(8.dp))
        Text("New case", style = Witness.type.button, color = palette.onPrimary)
    }
}
