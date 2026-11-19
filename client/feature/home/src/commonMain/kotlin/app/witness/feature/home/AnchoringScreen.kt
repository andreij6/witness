package app.witness.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.StateChip
import app.witness.core.designsystem.TextAction
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessAppBar
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessState
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.model.RecordId
import app.witness.core.ui.Formatters

@Composable
fun AnchoringScreen(state: AnchoringUiState, onBack: () -> Unit, onRetry: () -> Unit, onItem: (RecordId) -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Column(modifier.fillMaxSize().background(palette.paper).windowInsetsPadding(WindowInsets.statusBars)) {
        WitnessAppBar(AnchoringCopy.TITLE, onNavigate = onBack)
        if (state.loading) return@Column
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(start = Gutter, end = Gutter, top = 4.dp, bottom = 32.dp)) {
            item(key = "summary") { SummaryCard(state) }
            if (state.queue.isNotEmpty()) {
                item(key = "queue-title") {
                    Row(Modifier.fillMaxWidth().padding(top = 22.dp, bottom = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                        Overline(AnchoringCopy.QUEUE, Modifier.weight(1f))
                        TextAction(AnchoringCopy.RETRY, onRetry)
                    }
                }
                items(state.queue, key = { "q-" + it.recordId.text }) { row -> QueueCard(row, Modifier.padding(bottom = 8.dp)) { onItem(row.recordId) } }
            }
            if (state.recent.isNotEmpty()) {
                item(key = "recent") {
                    Overline(AnchoringCopy.RECENT, Modifier.padding(top = 20.dp, bottom = 12.dp))
                    WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(0.dp)) {
                        state.recent.forEachIndexed { i, row ->
                            if (i > 0) Hairline(Modifier.padding(horizontal = 16.dp))
                            RecentRow(row) { onItem(row.recordId) }
                        }
                    }
                }
            }
            state.key?.let { key ->
                item(key = "key") {
                    Overline(AnchoringCopy.YOUR_KEY, Modifier.padding(top = 28.dp, bottom = 12.dp))
                    WitnessCard(Modifier.fillMaxWidth()) {
                        Row {
                            IconDisc(WitnessIcon.VpnKey, palette.ink, size = 40.dp, iconSize = 22.dp, fill = palette.surface2)
                            HSpace(12.dp)
                            Column(Modifier.weight(1f)) {
                                Text(key.title, style = Witness.type.bodyStrong, color = palette.ink)
                                VSpace(6.dp)
                                Text(key.detail, style = Witness.type.subtitle, color = palette.ink2)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun SummaryCard(state: AnchoringUiState) {
    val palette = Witness.colors
    WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(0.dp)) {
        Row(Modifier.fillMaxWidth().height(100.dp)) {
            Stat(Formatters.groupThousands(state.anchored), AnchoringCopy.ANCHORED, WitnessIcon.Verified, palette.ok, Modifier.width(172.dp))
            Box(Modifier.padding(top = 24.dp).size(1.dp, 58.dp).background(palette.line))
            Stat(Formatters.groupThousands(state.queued), AnchoringCopy.QUEUED, WitnessIcon.HourglassTop, palette.warn, Modifier.weight(1f))
        }
        Hairline(Modifier.padding(horizontal = 16.dp))
        Column(Modifier.padding(start = 20.dp, end = 16.dp, top = 12.dp, bottom = 14.dp)) {
            InfoLine(WitnessIcon.Schedule, state.lastAnchored)
            VSpace(8.dp)
            InfoLine(WitnessIcon.VerifiedUser, AnchoringCopy.CERTIFIED)
        }
    }
}

@Composable
private fun Stat(value: String, label: String, icon: WitnessIcon, tint: Color, modifier: Modifier) {
    val palette = Witness.colors
    Column(modifier.padding(start = 20.dp, top = 12.dp).semantics(mergeDescendants = true) {}) {
        Text(value, style = Witness.type.display.copy(fontSize = 32.sp), color = palette.ink, maxLines = 1)
        VSpace(6.dp)
        Row(verticalAlignment = Alignment.CenterVertically) {
            WIcon(icon, tint, size = 16.dp)
            HSpace(6.dp)
            Text(label, style = Witness.type.label.copy(fontSize = 13.sp), color = palette.ink2)
        }
    }
}

@Composable
private fun InfoLine(icon: WitnessIcon, text: String) {
    val palette = Witness.colors
    Row(verticalAlignment = Alignment.CenterVertically) {
        WIcon(icon, palette.ink2, size = 16.dp)
        HSpace(8.dp)
        Text(text, style = Witness.type.subtitle, color = palette.ink2)
    }
}

@Composable
private fun QueueCard(row: AnchorRow, modifier: Modifier, onClick: () -> Unit) {
    val palette = Witness.colors
    WitnessCard(modifier.fillMaxWidth(), shape = WitnessShapes.Row, contentPadding = PaddingValues(12.dp), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(56.dp).clip(WitnessShapes.Thumb)) { RecordThumb(row.recordId, 56.dp) }
            HSpace(14.dp)
            Column(Modifier.weight(1f)) {
                Text(row.title, style = Witness.type.bodyStrong, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
                row.reason?.let { reason ->
                    VSpace(6.dp)
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        WIcon(reason.icon, palette.warn, size = 14.dp)
                        HSpace(5.dp)
                        Text(reason.text, style = Witness.type.subtitle, color = palette.warnText, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                }
            }
            HSpace(8.dp)
            StateChip(row.state, height = 24.dp, fontSize = 11.sp)
        }
    }
}

@Composable
private fun RecentRow(row: AnchorRow, onClick: () -> Unit) {
    val palette = Witness.colors
    Row(
        Modifier.fillMaxWidth().clickable(role = Role.Button, onClick = onClick).padding(horizontal = 12.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(52.dp).clip(WitnessShapes.Thumb)) { RecordThumb(row.recordId, 52.dp) }
        HSpace(14.dp)
        Column(Modifier.weight(1f)) {
            Text(row.title, style = Witness.type.bodyStrong, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
            VSpace(4.dp)
            Text(row.detail, style = Witness.type.subtitle, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        WIcon(WitnessIcon.Verified, palette.ok, size = 20.dp, contentDescription = row.state.label)
        HSpace(4.dp)
    }
}

internal object SnapshotData {
    private fun id(n: Int) = RecordId.fromBytes(ByteArray(RecordId.SIZE) { (it * n + n).toByte() })

    val anchoring = AnchoringUiState(
        loading = false,
        anchored = 30,
        queued = 1,
        lastAnchored = "Last anchored 2 min ago",
        queue = listOf(AnchorRow(id(1), "Wide shot · Fender bender", "", WitnessState.Queued, AnchoringCopy.waitingNetwork)),
        recent = listOf(
            AnchorRow(id(2), "Golf-ball hail out of nowhere", "Post · 2 min ago", WitnessState.Anchored),
            AnchorRow(id(3), "Plate 7MNV562", "Fender bender · Photo 3 · 9 min ago", WitnessState.Anchored),
            AnchorRow(id(5), "Rear bumper", "Fender bender · Photo 2 · 11 min ago", WitnessState.Anchored),
        ),
        key = KeyLine("Active · StrongBox · rotates in 12 days", AnchoringCopy.KEY_DETAIL),
    )

    val search = SearchUiState(
        terms = listOf("7mnv"),
        items = listOf(
            SearchItemRow(id(6), "Plate 7MNV562 · Photo 3", "Fender bender · Today 2:09 PM", WitnessState.Anchored),
            SearchItemRow(id(7), "Rear damage · Photo 2", "Note: hit by 7MNV562, silver sedan", WitnessState.Anchored),
        ),
        cases = listOf(SearchCaseRow(app.witness.core.model.LocalCaseId(1), "Fender bender", "Oak St & 5th Ave · plate 7MNV562", listOf(id(8), id(9), id(10)))),
    )
}

@Preview
@Composable
private fun AnchoringPreview() = WitnessTheme { AnchoringScreen(SnapshotData.anchoring, {}, {}, {}) }
