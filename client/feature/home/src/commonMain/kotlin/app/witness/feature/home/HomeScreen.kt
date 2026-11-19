package app.witness.feature.home

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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.FilterChip
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.NavItem
import app.witness.core.designsystem.SectionTitle
import app.witness.core.designsystem.TextAction
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessBottomNav
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.WitnessMark
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessState
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.designsystem.Wordmark
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.ui.CaptureTile
import app.witness.core.ui.CaseRow
import app.witness.core.ui.PostCard
import app.witness.core.ui.stateCounts

class HomeActions(
    val onFilter: (HomeFilter) -> Unit = {},
    val onSearch: () -> Unit = {},
    val onSettings: () -> Unit = {},
    val onAnchoring: () -> Unit = {},
    val onCamera: () -> Unit = {},
    val onCheck: () -> Unit = {},
    val onItem: (RecordId) -> Unit = {},
    val onCase: (LocalCaseId) -> Unit = {},
    val onNewCase: () -> Unit = {},
)

internal val homeTabs = NavItem("Home", WitnessIcon.Home) to NavItem("Check", WitnessIcon.VerifiedUser)

@Composable
fun HomeScreen(state: HomeUiState, actions: HomeActions, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Column(modifier.fillMaxSize().background(palette.paper)) {
        HomeHeader(state.trust, actions, Modifier.windowInsetsPadding(WindowInsets.statusBars))
        Box(Modifier.weight(1f).fillMaxWidth()) {
            when {
                state.loading -> Unit
                state.firstRun -> EmptyHome(actions)
                else -> HomeFeed(state, actions)
            }
        }
        WitnessBottomNav(homeTabs.first, homeTabs.second, 0, { if (it == 1) actions.onCheck() }, actions.onCamera)
    }
}

@Composable
private fun HomeHeader(trust: TrustLine, actions: HomeActions, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Column(modifier.fillMaxWidth().padding(start = Gutter, end = Gutter, top = 10.dp)) {
        Row(Modifier.fillMaxWidth().height(40.dp), verticalAlignment = Alignment.CenterVertically) {
            Wordmark(size = 23.dp)
            Spacer(Modifier.weight(1f))
            WitnessIconButton(WitnessIcon.Search, "Search", actions.onSearch, size = 44.dp)
            HSpace(4.dp)
            WitnessIconButton(WitnessIcon.Settings, "Settings", actions.onSettings, tint = palette.ink2, container = palette.surface3, size = 36.dp, iconSize = 22.dp)
        }
        VSpace(6.dp)
        if (trust.text.isNotEmpty()) {
            Row(
                Modifier.clip(WitnessShapes.Field).clickable(role = Role.Button, onClick = actions.onAnchoring).padding(vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                WIcon(trust.icon, palette.tone(trust.tone), size = 17.dp)
                HSpace(6.dp)
                Text(trust.text, style = Witness.type.label, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        } else {
            VSpace(25.dp)
        }
    }
}

@Composable
private fun EmptyHome(actions: HomeActions) {
    val palette = Witness.colors
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = Gutter),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        VSpace(56.dp)
        EmptyIllustration()
        VSpace(12.dp)
        ScreenTitle(HomeCopy.EMPTY_TITLE, textAlign = TextAlign.Center, style = Witness.type.pageTitle)
        VSpace(10.dp)
        Lead(HomeCopy.EMPTY_LEAD, Modifier.padding(horizontal = 24.dp), textAlign = TextAlign.Center)
        VSpace(40.dp)
        WitnessButton(
            HomeCopy.OPEN_CAMERA,
            actions.onCamera,
            Modifier.width(220.dp),
            icon = WitnessIcon.PhotoCamera,
            height = LargeButtonHeight,
            textStyle = Witness.type.button.merge(LargeButtonText),
        )
        VSpace(60.dp)
        WitnessCard(Modifier.fillMaxWidth(), shape = WitnessShapes.Row, contentPadding = PaddingValues(16.dp), onClick = actions.onCheck) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                IconDisc(WitnessIcon.VerifiedUser, palette.ink, size = 44.dp, iconSize = 22.dp, fill = palette.surface2)
                HSpace(16.dp)
                Column(Modifier.weight(1f)) {
                    Text(HomeCopy.CHECK_TITLE, style = Witness.type.rowTitle, color = palette.ink)
                    VSpace(4.dp)
                    Text(HomeCopy.CHECK_DETAIL, style = Witness.type.subtitle, color = palette.ink2)
                }
                WIcon(WitnessIcon.ChevronRight, palette.muted)
            }
        }
        VSpace(24.dp)
    }
}

internal val emptyDots = listOf(Triple(-96f, -64f, 6f), Triple(88f, -84f, 4f), Triple(104f, 46f, 7f), Triple(-84f, 78f, 4f))

@Composable
private fun EmptyIllustration() {
    val palette = Witness.colors
    Box(
        Modifier.size(240.dp).drawBehind {
            val c = center
            drawCircle(palette.surface2, 104.dp.toPx(), c)
            drawCircle(palette.surface, 72.dp.toPx(), c)
            for ((dx, dy, r) in emptyDots) drawCircle(palette.surface3, r.dp.toPx(), Offset(c.x + dx.dp.toPx(), c.y + dy.dp.toPx()))
        },
        contentAlignment = Alignment.Center,
    ) { WitnessMark(size = 84.dp) }
}

@Composable
private fun HomeFeed(state: HomeUiState, actions: HomeActions) {
    val palette = Witness.colors
    val pairs = (state.tiles.size + 1) / 2
    LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(start = Gutter, end = Gutter, top = 12.dp, bottom = 24.dp)) {
        item(key = "filters") {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                HomeFilter.entries.forEach { f -> FilterChip(f.label, f == state.filter, { actions.onFilter(f) }) }
            }
        }
        if (state.showPosts) {
            val featured = state.featured
            if (featured == null) {
                item(key = "no-posts") { EmptyNote(HomeCopy.NO_POSTS) }
            } else {
                item(key = featured.recordId.text) {
                    PostCard(featured.title, featured.meta, featured.state, Modifier.padding(top = 10.dp), kindLabel = "Post", onClick = { actions.onItem(featured.recordId) }) {
                        RecordThumb(featured.recordId, 400.dp)
                    }
                }
            }
            items(pairs, key = { "tiles-" + state.tiles[it * 2].recordId.text }) { row ->
                Row(Modifier.padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Tile(state.tiles[row * 2], actions, Modifier.weight(1f))
                    val right = state.tiles.getOrNull(row * 2 + 1)
                    if (right != null) Tile(right, actions, Modifier.weight(1f)) else Spacer(Modifier.weight(1f))
                }
            }
        }
        if (state.showCases) {
            item(key = "cases-title") {
                SectionTitle(HomeCopy.CASES, Modifier.padding(top = 14.dp, bottom = 2.dp)) { TextAction(HomeCopy.NEW_CASE, actions.onNewCase, color = palette.primary) }
            }
            if (state.cases.isEmpty()) item(key = "no-cases") { EmptyNote(HomeCopy.NO_CASES) }
            items(state.cases, key = { "case-" + it.id.value }) { c ->
                CaseRow(c.title, c.meta, c.counts, c.covers.size, Modifier.padding(top = 8.dp), onClick = { actions.onCase(c.id) }) { i -> RecordThumb(c.covers[i], 80.dp) }
            }
        }
    }
}

@Composable
private fun Tile(item: FeedItem, actions: HomeActions, modifier: Modifier) {
    CaptureTile(item.title, item.meta, item.state, modifier, onClick = { actions.onItem(item.recordId) }) { RecordThumb(item.recordId, 200.dp) }
}

@Composable
private fun EmptyNote(text: String) {
    Text(text, Modifier.fillMaxWidth().padding(vertical = 20.dp), style = Witness.type.subtitle, color = Witness.colors.muted, textAlign = TextAlign.Center)
}

internal object HomePreviewData {
    private fun id(n: Int) = RecordId.fromBytes(ByteArray(RecordId.SIZE) { (it + n).toByte() })

    val populated = HomeUiState(
        loading = false,
        trust = TrustLine("31 sealed · 30 anchored · 1 queued", Tone.Ok, WitnessIcon.Verified),
        featured = FeedItem(id(1), "Golf-ball hail out of nowhere.", "Today 4:12 PM · Denver area", WitnessState.Anchored),
        tiles = listOf(
            FeedItem(id(2), "Double rainbow", "Nov 16", WitnessState.Anchored),
            FeedItem(id(3), "Moose on Route 9", IMPORTED_FROM_GALLERY, WitnessState.Unverified),
        ),
        cases = listOf(
            CaseItem(LocalCaseId(1), "Fender bender", "Today · 4 items", stateCounts(3, 1), listOf(id(4), id(5), id(6))),
            CaseItem(LocalCaseId(2), "Kitchen ceiling leak", "Nov 12 · 6 items", stateCounts(6, 0), listOf(id(7))),
        ),
    )
    val empty = HomeUiState(loading = false, firstRun = true, trust = HomeCopy.ready)
}

@Preview
@Composable
private fun HomePreview() = WitnessTheme { HomeScreen(HomePreviewData.populated, HomeActions()) }

@Preview
@Composable
private fun HomeEmptyPreview() = WitnessTheme { HomeScreen(HomePreviewData.empty, HomeActions()) }

@Deprecated("Register homeEntries in the app shell", ReplaceWith("homeEntries(deps, navigator)"))
@Composable
fun HomeScreen(modifier: Modifier = Modifier) = HomeScreen(HomePreviewData.empty, HomeActions(), modifier)
