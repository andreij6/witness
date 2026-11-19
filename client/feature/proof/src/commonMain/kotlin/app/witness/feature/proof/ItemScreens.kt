package app.witness.feature.proof

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.ActionRow
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.GlassIconButton
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.SheetHandle
import app.witness.core.designsystem.SheetHeader
import app.witness.core.designsystem.TonePanel
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessAppBar
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessDialog
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessSheet
import app.witness.core.designsystem.solidChip
import app.witness.core.model.RecordId
import app.witness.core.ui.ChainOfCustody
import app.witness.core.ui.ProofHeader

@Immutable
class ItemActions(
    val onBack: () -> Unit = {},
    val onShowActions: () -> Unit = {},
    val onHideActions: () -> Unit = {},
    val onTechnical: () -> Unit = {},
    val onShare: () -> Unit = {},
    val onMove: () -> Unit = {},
    val onCreatePost: () -> Unit = {},
    val onProveLocation: () -> Unit = {},
    val onDelete: () -> Unit = {},
    val onErase: () -> Unit = {},
    val onConfirmErase: () -> Unit = {},
    val onDismissErase: () -> Unit = {},
    val onPickCase: (CaseOption) -> Unit = {},
    val onConfirmMove: () -> Unit = {},
    val onDismissMove: () -> Unit = {},
    val onConfirmDelete: () -> Unit = {},
    val onDismissDelete: () -> Unit = {},
    val onCopy: (String) -> Unit = {},
    val onOpenWeb: () -> Unit = {},
    val onCreateLocationProof: () -> Unit = {},
)

@Composable
fun ItemProofScreen(state: ItemUiState, actions: ItemActions, showActions: Boolean, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Box(modifier.fillMaxSize().background(palette.paper)) {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
            Box(Modifier.fillMaxWidth().height(330.dp)) {
                BytesImage(state.image, state.itemTitle, Modifier.fillMaxSize())
                Box(Modifier.fillMaxWidth().height(130.dp).background(Brush.verticalGradient(listOf(Color.Black.copy(alpha = 0.42f), Color.Transparent))))
                state.position?.let {
                    WitnessChip(it, palette.solidChip(WitnessColors.Black), Modifier.align(Alignment.BottomCenter).padding(bottom = 32.dp), height = 26.dp)
                }
            }
            Column(
                Modifier.offset(y = (-24).dp).fillMaxWidth().clip(WitnessShapes.Sheet).background(palette.paper).padding(bottom = 96.dp),
            ) {
                SheetHandle()
                if (state.missing) {
                    Text("This item is no longer on this phone.", Modifier.padding(24.dp), style = Witness.type.body, color = palette.ink2)
                }
                state.header?.let { h ->
                    ProofHeader(h.state, h.title, h.subtitle, Modifier.padding(horizontal = 16.dp, vertical = 8.dp), context = state.caseTitle?.let { "In case: $it" })
                }
                if (state.custody.isNotEmpty()) {
                    Text("Checks & chain of custody", Modifier.padding(start = 16.dp, top = 20.dp, bottom = 12.dp), style = Witness.type.cardTitle, color = palette.ink)
                    ChainOfCustody(state.custody, Modifier.padding(horizontal = 16.dp))
                }
            }
        }
        Row(Modifier.windowInsetsPadding(WindowInsets.statusBars).fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
            GlassIconButton(WitnessIcon.ArrowBack, "Back", actions.onBack)
            Spacer(Modifier.weight(1f))
            GlassIconButton(WitnessIcon.MoreVert, "Item actions", actions.onShowActions)
        }
        Row(
            Modifier.align(Alignment.BottomCenter).fillMaxWidth().background(palette.paper).windowInsetsPadding(WindowInsets.navigationBars).padding(16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            WitnessButton("Technical record", actions.onTechnical, kind = ButtonKind.Outlined, icon = WitnessIcon.Description, height = 50.dp)
            WitnessButton("Share", actions.onShare, Modifier.weight(1f), icon = WitnessIcon.Share, height = 50.dp)
        }
        WitnessSheet(showActions, actions.onHideActions) { ItemActionsSheet(state, actions) }
        val picking = state.move as? MoveState.Picking
        WitnessSheet(picking != null, actions.onDismissMove) { picking?.let { MoveSheet(it, actions) } }
        EraseDialogs(state, actions)
        MoveDialogs(state.move, actions)
        DeleteDialogs(state.delete, actions)
    }
}

@Composable
private fun ColumnScope.MoveSheet(picking: MoveState.Picking, actions: ItemActions) {
    val subtitle = if (picking.options.size <= 1) "No cases yet — start one from the camera" else "Its seal and proof don't change"
    SheetHeader("Move to case", subtitle = subtitle)
    for (option in picking.options) {
        ActionRow(
            if (option.id == null) WitnessIcon.FolderOpen else WitnessIcon.Folder,
            option.title,
            subtitle = if (option.current) "Current" else null,
            chevron = false,
            onClick = { actions.onPickCase(option) },
            trailing = if (option.current) ({ WIcon(WitnessIcon.Check, Witness.colors.ok, size = 20.dp) }) else null,
        )
    }
    Spacer(Modifier.height(8.dp))
}

@Composable
private fun MoveDialogs(move: MoveState, actions: ItemActions) {
    when (move) {
        is MoveState.Confirm -> WitnessDialog(
            ItemViewModel.moveTitle(move),
            actions.onDismissMove,
            if (move.target.id == null) "Remove" else "Move",
            actions.onConfirmMove,
            text = ItemViewModel.moveText(move),
            icon = WitnessIcon.Folder,
        )
        MoveState.Failed -> WitnessDialog(
            "Couldn't move this photo",
            actions.onDismissMove,
            "OK",
            actions.onDismissMove,
            text = "Nothing changed. Try again.",
            dismissLabel = null,
        )
        else -> Unit
    }
}

@Composable
private fun DeleteDialogs(delete: DeleteState, actions: ItemActions) {
    when (delete) {
        is DeleteState.Confirm -> WitnessDialog(
            "Delete from this phone?",
            actions.onDismissDelete,
            "Delete",
            actions.onConfirmDelete,
            text = ItemViewModel.deleteText(delete.anchored),
            icon = WitnessIcon.Delete,
            destructive = true,
        )
        DeleteState.Failed -> WitnessDialog(
            "Couldn't delete this photo",
            actions.onDismissDelete,
            "OK",
            actions.onDismissDelete,
            text = "It's still on this phone. Try again.",
            dismissLabel = null,
        )
        else -> Unit
    }
}

@Composable
private fun ColumnScope.ItemActionsSheet(state: ItemUiState, actions: ItemActions) {
    val crit = Witness.colors.crit
    Column(Modifier.padding(start = 24.dp, end = 24.dp, top = 8.dp, bottom = 12.dp)) {
        Text(state.itemTitle, style = Witness.type.section, color = Witness.colors.ink)
        Text(state.itemSubtitle, style = Witness.type.subtitle, color = Witness.colors.ink2)
    }
    Hairline(Modifier.padding(horizontal = 16.dp))
    ActionRow(WitnessIcon.Folder, "Move to case", subtitle = state.moveSubtitle, onClick = actions.onMove)
    ActionRow(WitnessIcon.Public, "Create post", subtitle = "Share a redacted copy with a public proof page", onClick = actions.onCreatePost)
    ActionRow(WitnessIcon.PinDrop, "Prove exact location", subtitle = "A private file that shows where you stood", onClick = actions.onProveLocation)
    ActionRow(WitnessIcon.DataObject, "Technical record", subtitle = "Hashes, key, certificate and anchor", onClick = actions.onTechnical)
    Hairline(Modifier.padding(horizontal = 16.dp, vertical = 8.dp))
    ActionRow(WitnessIcon.Delete, "Delete from this phone", subtitle = "Anchored proof stays — it holds no image", tint = crit, chevron = false, onClick = actions.onDelete)
    ActionRow(WitnessIcon.HideImage, "Erase public copies", subtitle = "Posted images come down; links show “Removed”", tint = crit, chevron = false, onClick = actions.onErase)
    Spacer(Modifier.height(8.dp))
}

@Composable
private fun EraseDialogs(state: ItemUiState, actions: ItemActions) {
    when (val erase = state.erase) {
        EraseState.Confirm -> WitnessDialog(
            "Erase public copies?",
            actions.onDismissErase,
            "Erase",
            actions.onConfirmErase,
            text = "Posted images and proof pages for this item come down, and their links show “Removed”. The anonymous anchored proof stays. Anyone who saved a copy keeps it.",
            icon = WitnessIcon.HideImage,
            destructive = true,
        )
        is EraseState.Finished -> WitnessDialog(
            "Public copies",
            actions.onDismissErase,
            "Done",
            actions.onDismissErase,
            text = ItemViewModel.eraseMessage(erase.report),
            dismissLabel = null,
        )
        else -> Unit
    }
}

@Composable
fun TechnicalRecordScreen(state: ItemUiState, actions: ItemActions, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    val type = Witness.type
    ItemScaffold("Technical record", state.itemSubtitleShort(), actions.onBack, modifier, bottom = {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            WitnessButton("Copy JSON", { actions.onCopy(state.json) }, kind = ButtonKind.Tonal, icon = WitnessIcon.ContentCopy, height = 50.dp)
            state.webUrl?.let { WitnessButton("Open on the web", actions.onOpenWeb, Modifier.weight(1f), ButtonKind.Outlined, WitnessIcon.OpenInNew, height = 50.dp) }
        }
    }) {
        WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(vertical = 4.dp)) {
            state.technical.forEachIndexed { i, f ->
                if (i > 0) Hairline(Modifier.padding(horizontal = 16.dp))
                Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 6.dp, top = 12.dp, bottom = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text(f.key, style = type.proofSmall.merge(fontSize = 11.5.sp), color = palette.muted)
                        Spacer(Modifier.height(4.dp))
                        Text(f.display, style = type.proofStrong, color = palette.ink)
                    }
                    WitnessIconButton(WitnessIcon.ContentCopy, "Copy ${f.key}", { actions.onCopy(f.copy) }, tint = palette.muted, iconSize = 18.dp)
                }
            }
        }
        val certified = state.certified
        TonePanel(if (certified) palette.ok else palette.warn, Modifier.fillMaxWidth(), contentPadding = PaddingValues(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                WIcon(if (certified) WitnessIcon.Verified else WitnessIcon.HourglassTop, if (certified) palette.ok else palette.warn)
                Spacer(Modifier.width(12.dp))
                Column {
                    Text(
                        if (certified) "Certificate verified" else "Not anchored yet",
                        style = type.rowTitle,
                        color = if (certified) palette.okText else palette.warnText,
                    )
                    Text(
                        if (certified) "Witness CA chain · MMR proof matches the root" else "The registry hasn't certified this record yet",
                        style = type.caption,
                        color = palette.ink2,
                    )
                }
            }
        }
        Overline("Check it yourself", Modifier.padding(start = 4.dp, top = 12.dp))
        Text(
            "Anyone can recompute asset_hash from the file and check the MMR proof against the root the registry certifies. The web page does this in the browser.",
            Modifier.padding(horizontal = 4.dp),
            style = type.subtitle,
            color = palette.ink2,
        )
    }
}

@Composable
fun ProveLocationScreen(state: ItemUiState, actions: ItemActions, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    val type = Witness.type
    val location = state.location
    ItemScaffold("Prove exact location", state.itemSubtitleShort(), actions.onBack, modifier, stacked = true, bottom = {
        WitnessButton("Create location proof", actions.onCreateLocationProof, Modifier.fillMaxWidth(), icon = WitnessIcon.PinDrop, height = 50.dp, enabled = location != null)
        WitnessButton("Cancel", actions.onBack, Modifier.fillMaxWidth(), ButtonKind.Outlined, height = 40.dp)
    }) {
        MapPreview(location?.coordinates ?: "No sealed location", Modifier.fillMaxWidth().height(306.dp))
        TonePanel(palette.warn, Modifier.fillMaxWidth(), contentPadding = PaddingValues(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                WIcon(WitnessIcon.Warning, palette.warn)
                Spacer(Modifier.width(12.dp))
                Text(
                    if (location != null) "Anyone with this file can see exactly where you stood." else "This photo was sealed without a precise location, so there's nothing to prove.",
                    style = type.bodySmallStrong,
                    color = palette.warnText,
                )
            }
        }
        WitnessCard(Modifier.fillMaxWidth()) {
            LOCATION_EXPLAINER.forEachIndexed { i, (icon, title, body) ->
                Row(Modifier.padding(top = if (i == 0) 4.dp else 16.dp)) {
                    WIcon(icon, palette.ink, size = 22.dp)
                    Spacer(Modifier.width(14.dp))
                    Column {
                        Text(title, style = type.bodyStrong, color = palette.ink)
                        Text(body, style = type.subtitle, color = palette.ink2)
                    }
                }
            }
        }
    }
}

@Composable
private fun MapPreview(label: String, modifier: Modifier) {
    val palette = Witness.colors
    Box(modifier.clip(WitnessShapes.Card).background(palette.surface2).border(1.dp, palette.line, WitnessShapes.Card)) {
        Canvas(Modifier.fillMaxSize()) {
            val step = 34.dp.toPx()
            var x = 0f
            while (x < size.width) {
                drawLine(palette.line, Offset(x, 0f), Offset(x, size.height), 1.dp.toPx())
                x += step
            }
            var y = 0f
            while (y < size.height) {
                drawLine(palette.line, Offset(0f, y), Offset(size.width, y), 1.dp.toPx())
                y += step
            }
            val r = CornerRadius(6.dp.toPx())
            val w = size.width
            val h = size.height
            drawRoundRect(palette.surface3, Offset(14.dp.toPx(), 18.dp.toPx()), Size(96.dp.toPx(), 64.dp.toPx()), r)
            drawRoundRect(palette.surface3, Offset(14.dp.toPx(), 150.dp.toPx()), Size(96.dp.toPx(), h - 168.dp.toPx()), r)
            drawRoundRect(palette.surface3, Offset(w - 128.dp.toPx(), 18.dp.toPx()), Size(112.dp.toPx(), 64.dp.toPx()), r)
            drawRoundRect(palette.surface3, Offset(w - 128.dp.toPx(), 150.dp.toPx()), Size(112.dp.toPx(), h - 168.dp.toPx()), r)
            drawRect(palette.surface, Offset(0f, 100.dp.toPx()), Size(w, 32.dp.toPx()))
            drawRect(palette.surface, Offset(w / 2 - 70.dp.toPx(), 0f), Size(32.dp.toPx(), h))
            drawLine(palette.surface.copy(alpha = 0.9f), Offset(0f, h - 10.dp.toPx()), Offset(w, 30.dp.toPx()), 10.dp.toPx())
            val pin = Offset(w / 2 - 54.dp.toPx(), 116.dp.toPx())
            drawCircle(palette.ink.copy(alpha = 0.08f), 30.dp.toPx(), pin)
            drawCircle(palette.ink.copy(alpha = 0.25f), 5.dp.toPx(), pin.copy(y = pin.y + 2.dp.toPx()))
        }
        WIcon(WitnessIcon.LocationOn, palette.ink, Modifier.align(Alignment.TopCenter).offset(x = (-54).dp, y = 80.dp), size = 36.dp)
        Text(
            label,
            Modifier.align(Alignment.BottomCenter).padding(bottom = 14.dp).clip(WitnessShapes.Pill).background(palette.surface)
                .border(1.dp, palette.line, WitnessShapes.Pill).padding(horizontal = 12.dp, vertical = 7.dp),
            style = Witness.type.proof.merge(fontWeight = androidx.compose.ui.text.font.FontWeight.Medium),
            color = palette.ink,
        )
    }
}

@Composable
private fun ItemScaffold(
    title: String,
    subtitle: String,
    onBack: () -> Unit,
    modifier: Modifier,
    stacked: Boolean = false,
    bottom: @Composable ColumnScope.() -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    Box(modifier.fillMaxSize().background(Witness.colors.paper).windowInsetsPadding(WindowInsets.statusBars)) {
        Column(Modifier.fillMaxSize()) {
            WitnessAppBar(title, subtitle = subtitle, onNavigate = onBack)
            Column(
                Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = 16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Spacer(Modifier.height(0.dp))
                content()
                Spacer(Modifier.height(if (stacked) 120.dp else 80.dp))
            }
        }
        Column(
            Modifier.align(Alignment.BottomCenter).fillMaxWidth().background(Witness.colors.paper).windowInsetsPadding(WindowInsets.navigationBars).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
            content = bottom,
        )
    }
}

private fun ItemUiState.itemSubtitleShort(): String = app.witness.core.ui.Formatters.joinDot(position?.substringBefore(" of "), caseTitle)

private val LOCATION_EXPLAINER = listOf(
    Triple(WitnessIcon.Description, "What's created", "A signed opening of the sealed location — a small file you send to one person."),
    Triple(WitnessIcon.VerifiedUser, "Who can verify", "Anyone you give it to, on witness.app. It must match what was sealed at capture."),
    Triple(WitnessIcon.VisibilityOff, "Still never public", "Posts and proof pages keep showing only the area."),
)

fun locationProofName(recordId: RecordId): String = "witness-location-${recordId.text}.json"
