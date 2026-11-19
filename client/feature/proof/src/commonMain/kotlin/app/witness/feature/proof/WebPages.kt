package app.witness.feature.proof

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.GeneratedSurface
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.StateChip
import app.witness.core.designsystem.TonePanel
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessTextField
import app.witness.core.designsystem.dashedBorder
import app.witness.core.designsystem.outlineChip
import app.witness.core.designsystem.solidChip
import app.witness.core.model.Verdict
import app.witness.core.ui.BrowserSafeLayout
import app.witness.core.ui.ProofFacts
import app.witness.core.ui.Proves
import app.witness.core.ui.ProvesCard
import app.witness.core.ui.VerdictHeadline
import app.witness.core.ui.WebFooter
import app.witness.core.ui.WebHeader
import app.witness.core.ui.WebLayoutMode
import app.witness.core.ui.WebLink
import app.witness.core.ui.state

@Immutable
class WebActions(
    val onCheckCopy: () -> Unit = {},
    val onReport: () -> Unit = {},
    val onRetry: () -> Unit = {},
    val onOpenItem: (Int) -> Unit = {},
    val footer: List<WebLink> = emptyList(),
    val header: List<WebLink> = emptyList(),
)

@Composable
private fun WebFrame(
    label: String,
    actions: WebActions,
    modifier: Modifier,
    wideHeader: (@Composable () -> Unit)? = null,
    secondary: (@Composable ColumnScope.(Boolean) -> Unit)? = null,
    primary: @Composable ColumnScope.(Boolean) -> Unit,
) {
    BrowserSafeLayout(
        modifier,
        header = { spec ->
            val wide = spec.mode == WebLayoutMode.Wide
            if (wide && wideHeader != null) Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.weight(1f)) { WebHeader(label = null, wide = true) }
                wideHeader()
            } else WebHeader(label = label, links = if (wide) actions.header else emptyList(), wide = wide)
        },
        footer = { spec -> WebFooter(actions.footer, wide = spec.mode == WebLayoutMode.Wide) },
        secondary = secondary?.let { s -> { spec -> if (spec.mode == WebLayoutMode.Wide) s(true) } },
    ) { spec -> primary(spec.mode == WebLayoutMode.Wide) }
}

@Composable
fun ProofPageScreen(state: ProofPageState, actions: WebActions, modifier: Modifier = Modifier) {
    val found = (state as? ProofPageState.Found)?.page
    WebFrame(
        "Verification",
        actions,
        modifier,
        secondary = found?.let { page -> { _ -> ProofDetails(page, actions, wide = true) } },
    ) { wide ->
        when (state) {
            ProofPageState.Loading -> LoadingBlock("Loading proof…")
            is ProofPageState.Found -> {
                ProofHero(state.page, wide)
                if (!wide) ProofDetails(state.page, actions, wide = false)
            }
            is ProofPageState.Removed -> GonePage(
                WitnessIcon.HideImage,
                "This proof page was removed",
                "Its owner removed it, or it was taken down after a report. The anonymous anchored proof still exists, but its image and page are gone.",
                "Removed pages can't be restored by visitors.",
                "What still exists",
                listOf(
                    GoneLine(WitnessIcon.Verified, "An anonymous proof record", "No image, no location · anchored"),
                    GoneLine(WitnessIcon.HideImage, "The image and this page are gone", "Copies elsewhere can still be checked"),
                ),
                "Check a copy",
                actions.onCheckCopy,
            )
            ProofPageState.NotFound -> GonePage(
                WitnessIcon.LinkOff,
                "No proof at this address",
                "This link doesn't match any Witness record. Check that it was copied in full.",
                null,
                null,
                emptyList(),
                "Check a photo instead",
                actions.onCheckCopy,
            )
            ProofPageState.Offline -> GonePage(
                WitnessIcon.WifiOff,
                "Couldn't reach the registry",
                "Check your connection and try again.",
                null,
                null,
                emptyList(),
                "Try again",
                actions.onRetry,
            )
        }
    }
}

@Composable
private fun ProofHero(page: ProofPage, wide: Boolean) {
    val palette = Witness.colors
    Box(Modifier.fillMaxWidth().then(if (wide) Modifier.aspectRatio(1f) else Modifier.height(214.dp)).clip(if (wide) RoundedCornerShape(20.dp) else WitnessShapes.ImageLarge)) {
        BytesImage(page.image, "Sealed photo", Modifier.fillMaxSize())
        val tone = palette.tone(page.verdict.state().tone)
        WitnessChip(page.headline, palette.solidChip(tone), Modifier.align(Alignment.TopStart).padding(12.dp), icon = page.verdict.state().icon, height = if (wide) 30.dp else 28.dp, fontSize = if (wide) 13.sp else 12.sp)
        page.badge?.let {
            WitnessChip(it, palette.solidChip(if (wide) palette.edit else WitnessColors.Ink), Modifier.align(Alignment.BottomStart).padding(12.dp), icon = WitnessIcon.Edit, height = if (wide) 28.dp else 26.dp, fontSize = if (wide) 12.sp else 11.5.sp)
        }
    }
}

@Composable
private fun ColumnScope.ProofDetails(page: ProofPage, actions: WebActions, wide: Boolean) {
    val palette = Witness.colors
    VerdictHeadline(page.verdict, qualifier = page.qualifier, note = page.qualifierNote, large = wide)
    if (wide) Text(page.summary, style = Witness.type.bodyLarge, color = palette.ink)
    page.warning?.let { Text(it, style = Witness.type.bodySmallStrong, color = palette.warnText) }
    ProofFacts(page.facts, labelWidth = if (wide) 104.dp else 76.dp)
    ProvesCard(lines = if (page.dev) Proves.Dev else Proves.Default)
    if (wide) {
        Row(
            Modifier.fillMaxWidth().clip(WitnessShapes.Card).background(palette.surface).dashedBorder(palette.muted.copy(alpha = 0.7f), 18.dp)
                .clickable(role = Role.Button, onClick = actions.onCheckCopy).padding(horizontal = 22.dp, vertical = 22.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            WIcon(WitnessIcon.PhotoLibrary, palette.ink2, size = 26.dp)
            Spacer(Modifier.width(14.dp))
            Column {
                Text("Check a copy", style = Witness.type.actionTitle, color = palette.ink)
                Text("Drop a screenshot here or paste a link — checked in your browser", style = Witness.type.bodyCompact, color = palette.ink2)
            }
        }
    }
}

@Composable
private fun LoadingBlock(text: String) {
    Box(Modifier.fillMaxWidth().height(320.dp), contentAlignment = Alignment.Center) {
        Text(text, style = Witness.type.body, color = Witness.colors.muted)
    }
}

@Immutable
data class GoneLine(val icon: WitnessIcon, val title: String, val detail: String)

@Composable
fun GonePage(icon: WitnessIcon, title: String, body: String, note: String?, cardTitle: String?, lines: List<GoneLine>, button: String?, onButton: () -> Unit) {
    val palette = Witness.colors
    val type = Witness.type
    Column(Modifier.fillMaxWidth().padding(top = 32.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        IconDisc(icon, palette.ink2, size = 88.dp, iconSize = 44.dp, fill = palette.surface2)
        Spacer(Modifier.height(24.dp))
        Text(title, Modifier.semantics { heading() }, style = type.headline, color = palette.ink, textAlign = TextAlign.Center)
        Spacer(Modifier.height(12.dp))
        Text(body, Modifier.padding(horizontal = 16.dp), style = type.body, color = palette.ink2, textAlign = TextAlign.Center)
        if (note != null) Text(note, Modifier.padding(horizontal = 16.dp, vertical = 12.dp), style = type.label, color = palette.muted, textAlign = TextAlign.Center)
    }
    if (cardTitle != null && lines.isNotEmpty()) {
        WitnessCard(Modifier.fillMaxWidth().padding(top = 12.dp)) {
            Overline(cardTitle)
            lines.forEach { l ->
                Row(Modifier.padding(top = 12.dp)) {
                    WIcon(l.icon, palette.ink2, size = 20.dp)
                    Spacer(Modifier.width(10.dp))
                    Column {
                        Text(l.title, style = type.bodySmallStrong, color = palette.ink)
                        Text(l.detail, style = type.caption, color = palette.ink2)
                    }
                }
            }
        }
    }
    if (button != null) WitnessButton(button, onButton, Modifier.fillMaxWidth().padding(top = 12.dp), ButtonKind.Outlined, WitnessIcon.ImageSearch)
}

@Composable
fun CaseLinkScreen(state: CaseLinkState, actions: WebActions, modifier: Modifier = Modifier) {
    val open = state as? CaseLinkState.Open
    WebFrame(
        "Private case",
        actions,
        modifier,
        wideHeader = {
            Row(verticalAlignment = Alignment.CenterVertically) {
                open?.expires?.let { Text("End-to-end encrypted · expires $it", style = Witness.type.bodyCompact, color = Witness.colors.ink2) }
                Spacer(Modifier.width(12.dp))
                WitnessChip("Private case", Witness.colors.outlineChip(), icon = WitnessIcon.Lock, height = 30.dp, fontSize = 13.sp)
            }
        },
        secondary = open?.let { o -> { _ -> CaseGrid(o, actions) } },
    ) { wide ->
        when (state) {
            CaseLinkState.Opening -> LoadingBlock("Decrypting in your browser…")
            is CaseLinkState.Open -> CaseOverview(state, actions, wide)
            is CaseLinkState.Expired -> GonePage(
                WitnessIcon.TimerOff,
                "This case link has expired",
                (state.on?.let { "The sender set it to expire on $it. " } ?: "") + "Expired cases are deleted — ask the sender for a new link.",
                "Any copies opened before expiry may still exist.",
                "Why links expire",
                EXPIRY_LINES,
                "Check a saved copy",
                actions.onCheckCopy,
            )
            CaseLinkState.NotFound -> GonePage(WitnessIcon.LinkOff, "This case link doesn't exist", "It may have been deleted by the sender. Ask them for a new link.", null, null, emptyList(), "Check a saved copy", actions.onCheckCopy)
            CaseLinkState.MissingKey -> GonePage(WitnessIcon.VpnKey, "This link is missing its key", "Case links carry their key after the # sign. Copy the full link from the sender and open it again.", null, null, emptyList(), null) {}
            CaseLinkState.Tampered -> GonePage(WitnessIcon.GppBad, "This case couldn't be opened", "Some of its encrypted files are missing or were changed, so nothing is shown. Ask the sender to share it again.", null, null, emptyList(), null) {}
            CaseLinkState.Offline -> GonePage(WitnessIcon.WifiOff, "Couldn't reach Witness", "Check your connection and try again.", null, null, emptyList(), "Try again", actions.onRetry)
        }
    }
}

@Composable
private fun ColumnScope.CaseOverview(state: CaseLinkState.Open, actions: WebActions, wide: Boolean) {
    val palette = Witness.colors
    val type = Witness.type
    Column {
        Text(state.title, Modifier.semantics { heading() }, style = if (wide) type.display.merge(fontSize = 30.sp) else type.title, color = palette.ink)
        Text(state.meta, style = if (wide) type.body else type.subtitle, color = palette.ink2)
    }
    val verdict = state.verdict
    if (verdict == null) {
        WitnessCard(Modifier.fillMaxWidth()) {
            Text("Verifying ${state.items.size} items in your browser…", style = type.bodyStrong, color = palette.ink)
        }
    } else {
        val tone = palette.tone(verdict.state.tone)
        TonePanel(tone, Modifier.fillMaxWidth(), contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 18.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                WIcon(verdict.state.icon, tone, size = if (wide) 34.dp else 30.dp)
                Spacer(Modifier.width(10.dp))
                Text(verdict.headline, style = if (wide) type.display else type.headline, color = palette.toneText(verdict.state.tone))
            }
            Spacer(Modifier.height(12.dp))
            Text(verdict.detail, style = if (wide) type.bodyLarge else type.bodySmall, color = palette.ink)
        }
    }
    state.summary?.let { summary ->
        val source = state.summarySource
        val body: @Composable () -> Unit = {
            Spacer(Modifier.height(8.dp))
            Text(summary, style = type.subtitle, color = palette.ink2)
            if (wide) Text(SenderSummaryCopy.footnote(source), Modifier.padding(top = 10.dp), style = type.meta, color = palette.muted)
        }
        if (SenderSummaryCopy.isGenerated(source)) {
            GeneratedSurface(Modifier.fillMaxWidth()) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    WIcon(WitnessIcon.AutoAwesome, palette.ai, size = 16.dp)
                    Spacer(Modifier.width(6.dp))
                    Overline(SenderSummaryCopy.label(source), color = palette.ai)
                }
                body()
            }
        } else {
            WitnessCard(Modifier.fillMaxWidth()) {
                Overline(SenderSummaryCopy.label(source))
                body()
            }
        }
    }
    if (wide) {
        WitnessCard(Modifier.fillMaxWidth()) {
            Overline("About this link")
            listOf(
                GoneLine(WitnessIcon.Lock, "End-to-end encrypted", "The key lives in the link, never on Witness servers"),
                GoneLine(WitnessIcon.Schedule, state.expires?.let { "Expires $it" } ?: "No expiry set", "Then the photos are deleted from Witness"),
                GoneLine(WitnessIcon.VerifiedUser, "Checked in your browser", "Each item is verified against its anchor"),
            ).forEachIndexed { i, l ->
                if (i > 0) Hairline(Modifier.padding(top = 10.dp))
                Row(Modifier.padding(top = 10.dp)) {
                    WIcon(l.icon, palette.ink2, size = 20.dp)
                    Spacer(Modifier.width(12.dp))
                    Column {
                        Text(l.title, style = type.bodySmallStrong, color = palette.ink)
                        Text(l.detail, style = type.subtitle, color = palette.ink2)
                    }
                }
            }
        }
    } else {
        WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(vertical = 6.dp)) {
            state.items.forEachIndexed { i, item ->
                if (i > 0) Hairline(Modifier.padding(horizontal = 16.dp))
                CaseItemRow(item) { actions.onOpenItem(i) }
            }
        }
        ProvesCard(lines = if (state.dev) Proves.CaseLinkDev else Proves.CaseLink)
    }
}

@Composable
private fun CaseItemRow(item: CaseItemUi, onClick: () -> Unit) {
    val palette = Witness.colors
    Row(
        Modifier.fillMaxWidth().clickable(role = Role.Button, onClick = onClick).padding(horizontal = 14.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        BytesImage(item.image, item.label, Modifier.size(40.dp, 44.dp).clip(RoundedCornerShape(8.dp)))
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(item.label, style = Witness.type.bodySmallStrong, color = palette.ink, maxLines = 1)
            Text(item.meta, style = Witness.type.meta, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        item.state?.let { WIcon(it.icon, palette.tone(it.tone), size = 22.dp, contentDescription = it.label) }
        Spacer(Modifier.width(6.dp))
        WIcon(WitnessIcon.ChevronRight, palette.muted, size = 22.dp)
    }
}

@Composable
private fun ColumnScope.CaseGrid(state: CaseLinkState.Open, actions: WebActions) {
    val palette = Witness.colors
    state.items.chunked(2).forEachIndexed { row, pair ->
        Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            pair.forEachIndexed { col, item ->
                WitnessCard(Modifier.weight(1f), contentPadding = PaddingValues(8.dp), onClick = { actions.onOpenItem(row * 2 + col) }) {
                    BytesImage(item.image, item.label, Modifier.fillMaxWidth().height(148.dp).clip(WitnessShapes.Image))
                    Row(Modifier.padding(start = 8.dp, end = 8.dp, top = 12.dp, bottom = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(item.label.substringAfter(" · "), style = Witness.type.rowTitle, color = palette.ink, maxLines = 1)
                            Text(item.meta, style = Witness.type.subtitle, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                        item.state?.let { StateChip(it, text = item.stateText ?: it.shortLabel) }
                    }
                }
            }
            if (pair.size == 1) Spacer(Modifier.weight(1f))
        }
    }
    ProvesCard(lines = if (state.dev) Proves.CaseLinkDev else Proves.CaseLink)
}

@Composable
fun ReportScreen(state: ReportUiState, actions: ReportActions, web: WebActions, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    val type = Witness.type
    WebFrame("Report", web, modifier) {
        Text("Report this page", Modifier.semantics { heading() }, style = type.headline, color = palette.ink)
        Text("Tell us what's wrong. No account needed.", style = type.bodyCompact, color = palette.ink2)
        if (state.phase == ReportPhase.Sent) {
            TonePanel(palette.ok, Modifier.fillMaxWidth()) {
                Text("Thanks — your report was sent", style = type.bodyStrong, color = palette.okText)
                Text("A person will review it. We remove pages, never anchored proofs.", style = type.subtitle, color = palette.ink2)
            }
            return@WebFrame
        }
        WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(vertical = 4.dp)) {
            ReportReason.entries.forEachIndexed { i, reason ->
                if (i > 0) Hairline(Modifier.padding(horizontal = 16.dp))
                val on = reason == state.reason
                Row(
                    Modifier.fillMaxWidth().clickable(role = Role.RadioButton) { actions.onReason(reason) }.padding(horizontal = 16.dp, vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    WIcon(if (on) WitnessIcon.RadioButtonChecked else WitnessIcon.RadioButtonUnchecked, if (on) palette.primary else palette.muted, size = 22.dp)
                    Spacer(Modifier.width(12.dp))
                    Text(reason.label, style = if (on) type.bodyStrong else type.body, color = palette.ink)
                }
            }
        }
        FieldLabel("Details", null)
        WitnessTextField(state.details, actions.onDetails, Modifier.fillMaxWidth(), placeholder = "What should we look at?", shape = WitnessShapes.Field, minHeight = 72.dp, singleLine = false)
        FieldLabel("Email (optional)", "Only to tell you the outcome")
        WitnessTextField(state.email, actions.onEmail, Modifier.fillMaxWidth(), placeholder = "you@example.com", shape = WitnessShapes.Field)
        state.emailError?.let { Text(it, style = type.caption, color = palette.crit) }
        if (state.phase == ReportPhase.Failed) Text("Couldn't send your report. Try again.", style = type.caption, color = palette.crit)
        WitnessButton(
            if (state.phase == ReportPhase.Sending) "Sending…" else "Send report",
            actions.onSend,
            Modifier.fillMaxWidth(),
            icon = WitnessIcon.Send,
            enabled = state.canSend,
        )
        Row {
            WIcon(WitnessIcon.Info, palette.muted, size = 16.dp)
            Spacer(Modifier.width(8.dp))
            Text("Reports are reviewed by people. We remove pages, never anchored proofs.", style = type.subtitle, color = palette.muted)
        }
    }
}

@Immutable
class ReportActions(
    val onReason: (ReportReason) -> Unit = {},
    val onDetails: (String) -> Unit = {},
    val onEmail: (String) -> Unit = {},
    val onSend: () -> Unit = {},
)

@Composable
private fun FieldLabel(text: String, hint: String?) {
    Row(Modifier.fillMaxWidth().padding(top = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(text, Modifier.weight(1f), style = Witness.type.bodySmallStrong, color = Witness.colors.ink)
        if (hint != null) Text(hint, style = Witness.type.meta, color = Witness.colors.muted)
    }
}

private val EXPIRY_LINES = listOf(
    GoneLine(WitnessIcon.VpnKey, "The key is in the link", "Witness never sees it or the case"),
    GoneLine(WitnessIcon.Delete, "Deleted on expiry", "Encrypted files are removed from storage"),
    GoneLine(WitnessIcon.Link, "New link, new key", "The sender can share again anytime"),
)

