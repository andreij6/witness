package app.witness.feature.share

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.BottomBar
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.QrCode
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessAppBar
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessToggle
import app.witness.core.designsystem.outlineChip
import app.witness.core.designsystem.solidChip
import app.witness.core.designsystem.tintedChip
import app.witness.core.model.RecordId
import app.witness.core.model.ShareExpiry

typealias RecordImage = @Composable BoxScope.(RecordId) -> Unit

class ShareCaseActions(
    val onClose: () -> Unit,
    val onToggleItem: (RecordId) -> Unit,
    val onTogglePrivacy: (PrivacyOption) -> Unit,
    val onExpiry: (ShareExpiry) -> Unit,
    val onIncludeSummary: (Boolean) -> Unit,
    val onCopyLink: (String) -> Unit,
    val onPreview: (String) -> Unit,
    val onExportFile: () -> Unit,
    val onShareLink: () -> Unit,
    val onClearError: () -> Unit,
)

private val MetaStyle = TextStyle(fontWeight = FontWeight.Medium)
private val StrongStyle = TextStyle(fontWeight = FontWeight.SemiBold)

@Composable
fun ShareCaseScreen(state: ShareCaseUiState, actions: ShareCaseActions, modifier: Modifier = Modifier, image: RecordImage = {}) {
    val palette = Witness.colors
    Box(modifier.fillMaxSize().background(palette.paper)) {
        Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars)) {
            WitnessAppBar("Share case", subtitle = state.subtitle, navigationIcon = WitnessIcon.Close, navigationLabel = "Close", onNavigate = actions.onClose)
            Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(top = 8.dp, bottom = 104.dp)) {
                Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) {
                    Overline("Items", Modifier.weight(1f))
                    Text(state.selectedLabel, style = Witness.type.caption.merge(MetaStyle), color = palette.muted)
                }
                Spacer(Modifier.height(10.dp))
                ItemStrip(state.items, image, actions.onToggleItem)
                if (state.privacy.isNotEmpty()) {
                    Overline("Privacy", Modifier.padding(start = 16.dp, top = 20.dp, bottom = 10.dp))
                    Row(Modifier.padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        state.privacy.forEach { chip -> PrivacyChip(chip) { actions.onTogglePrivacy(chip.option) } }
                    }
                    Note(WitnessIcon.Info, "Edits are declared — recipients see exactly what changed.", Modifier.padding(start = 16.dp, end = 16.dp, top = 12.dp))
                }
                ExpiryCard(state.expiry, state.includeSummary, state.summaryLabel, actions.onExpiry, actions.onIncludeSummary)
                LinkCard(state, actions)
                state.error?.let {
                    Text(
                        it,
                        Modifier.padding(start = 16.dp, end = 16.dp, top = 12.dp).clickable(role = Role.Button, onClick = actions.onClearError),
                        style = Witness.type.bodySmallStrong,
                        color = palette.crit,
                    )
                }
            }
        }
        BottomBar {
            WitnessButton("Export file", actions.onExportFile, Modifier.width(150.dp), ButtonKind.Tonal, icon = WitnessIcon.Description)
            WitnessButton(
                if (state.sharing) "Encrypting…" else "Share link",
                actions.onShareLink,
                Modifier.weight(1f),
                icon = WitnessIcon.Link,
                enabled = state.canShare,
            )
        }
    }
}

@Composable
private fun ItemStrip(items: List<ShareItemUi>, image: RecordImage, onToggle: (RecordId) -> Unit) {
    val palette = Witness.colors
    LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        items(items, key = { it.recordId.text }) { item ->
            Box(
                Modifier
                    .size(88.dp, 116.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(palette.surface2)
                    .toggleable(item.selected, role = Role.Checkbox) { onToggle(item.recordId) },
            ) {
                Box(Modifier.fillMaxSize().alpha(if (item.selected) 1f else UNSELECTED_ALPHA)) { image(item.recordId) }
                SelectionMark(item.selected, Modifier.align(Alignment.TopEnd).padding(3.dp))
                if (item.edited) {
                    WitnessChip(
                        "Edited",
                        palette.solidChip(palette.edit),
                        Modifier.align(Alignment.BottomStart).padding(6.dp),
                        icon = WitnessIcon.Edit,
                        height = 22.dp,
                        fontSize = 10.5.sp,
                    )
                }
            }
        }
    }
}

private const val UNSELECTED_ALPHA = 0.45f

@Composable
private fun SelectionMark(selected: Boolean, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Box(
        modifier
            .size(22.dp)
            .clip(CircleShape)
            .background(if (selected) palette.ink else Color.Black.copy(alpha = 0.25f))
            .border(2.dp, Color.White, CircleShape),
        contentAlignment = Alignment.Center,
    ) { if (selected) WIcon(WitnessIcon.Check, Color.White, size = 16.dp) }
}

@Composable
private fun PrivacyChip(chip: PrivacyChipUi, onClick: () -> Unit) {
    val palette = Witness.colors
    val icon = when (chip.option) {
        PrivacyOption.BLUR_FACES -> WitnessIcon.Face
        PrivacyOption.BLUR_PLATES -> WitnessIcon.DirectionsCar
        PrivacyOption.HIDE_LOCATION -> WitnessIcon.LocationOn
    }
    WitnessChip(
        chip.label,
        if (chip.on) palette.tintedChip(Tone.Edit) else palette.outlineChip(),
        Modifier.toggleable(chip.on, role = Role.Switch) { onClick() },
        icon = icon,
        height = 34.dp,
        fontSize = 12.5.sp,
        weight = if (chip.on) FontWeight.SemiBold else FontWeight.Medium,
    )
}

@Composable
private fun Note(icon: WitnessIcon, text: String, modifier: Modifier = Modifier) {
    Row(modifier, verticalAlignment = Alignment.CenterVertically) {
        WIcon(icon, Witness.colors.muted, size = 16.dp)
        Spacer(Modifier.width(6.dp))
        Text(text, style = Witness.type.caption, color = Witness.colors.muted)
    }
}

@Composable
private fun ExpiryCard(expiry: ShareExpiry, includeSummary: Boolean, summaryLabel: String, onExpiry: (ShareExpiry) -> Unit, onIncludeSummary: (Boolean) -> Unit) {
    WitnessCard(Modifier.padding(start = 16.dp, end = 16.dp, top = 20.dp).fillMaxWidth()) {
        Overline("Link expires")
        Spacer(Modifier.height(8.dp))
        ExpirySegments(expiry, onExpiry)
        Hairline(Modifier.padding(vertical = 14.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(summaryLabel, Modifier.weight(1f), style = Witness.type.body.merge(MetaStyle), color = Witness.colors.ink)
            WitnessToggle(includeSummary, onIncludeSummary)
        }
    }
}

@Composable
internal fun ExpirySegments(selected: ShareExpiry, onSelect: (ShareExpiry) -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Row(modifier.fillMaxWidth().height(38.dp).clip(RoundedCornerShape(19.dp)).background(palette.surface2).padding(3.dp).selectableGroup()) {
        ShareExpiry.entries.forEach { option ->
            val on = option == selected
            Box(
                Modifier
                    .weight(1f)
                    .fillMaxHeight()
                    .clip(RoundedCornerShape(16.dp))
                    .then(if (on) Modifier.background(palette.surface).border(1.dp, palette.line, RoundedCornerShape(16.dp)) else Modifier)
                    .selectable(on, role = Role.RadioButton) { onSelect(option) },
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    ShareText.expiryLabel(option),
                    style = Witness.type.bodyCompact.merge(if (on) StrongStyle else MetaStyle),
                    color = if (on) palette.ink else palette.ink2,
                    textAlign = TextAlign.Center,
                    maxLines = 1,
                )
            }
        }
    }
}

@Composable
private fun LinkCard(state: ShareCaseUiState, actions: ShareCaseActions) {
    val palette = Witness.colors
    val link = state.link
    WitnessCard(Modifier.padding(start = 16.dp, end = 16.dp, top = 14.dp).fillMaxWidth()) {
        Row(Modifier.alpha(if (link?.stale == true) STALE_ALPHA else 1f)) {
            if (link != null) {
                QrCode(link.url, size = 112.dp, contentDescription = "QR code for the private case link")
            } else {
                Box(Modifier.size(128.dp).clip(RoundedCornerShape(10.dp)).background(palette.surface2), contentAlignment = Alignment.Center) {
                    WIcon(WitnessIcon.QrCode2, palette.muted, size = 40.dp)
                }
            }
            Spacer(Modifier.width(16.dp))
            Column(Modifier.weight(1f).padding(top = 6.dp)) {
                Text("Private case link", style = Witness.type.actionTitle, color = palette.ink)
                Spacer(Modifier.height(4.dp))
                if (link != null) {
                    Text(
                        link.display,
                        Modifier.semantics { contentDescription = "Private case link, key hidden" },
                        style = Witness.type.proof.merge(TextStyle(fontSize = 12.5.sp, lineHeight = 1.35.em)),
                        color = palette.primary,
                    )
                    Spacer(Modifier.height(6.dp))
                    Text(link.meta, style = Witness.type.caption, color = palette.muted)
                    Spacer(Modifier.height(10.dp))
                    CopyPill { actions.onCopyLink(link.url) }
                } else {
                    Text(PENDING_LINK, style = Witness.type.caption, color = palette.muted)
                }
            }
        }
        Spacer(Modifier.height(14.dp))
        Row(
            Modifier.fillMaxWidth().height(38.dp).clip(RoundedCornerShape(12.dp)).background(palette.ok.copy(alpha = 0.08f)).padding(horizontal = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            WIcon(WitnessIcon.Lock, palette.ok, size = 18.dp)
            Spacer(Modifier.width(8.dp))
            Text("End-to-end encrypted · the key stays in the link", style = Witness.type.labelStrong, color = palette.okText, maxLines = 1)
        }
        Hairline(Modifier.padding(vertical = 14.dp))
        Overline("Recipients will see")
        Spacer(Modifier.height(8.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            WIcon(WitnessIcon.CheckCircle, palette.ok, size = 20.dp)
            Spacer(Modifier.width(8.dp))
            Text(state.recipients, style = Witness.type.bodySmallStrong, color = palette.okText)
        }
        if (link != null) {
            Row(
                Modifier
                    .align(Alignment.End)
                    .padding(top = 8.dp)
                    .clip(RoundedCornerShape(8.dp))
                    .clickable(role = Role.Button) { actions.onPreview(link.url) }
                    .padding(horizontal = 6.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Preview", style = Witness.type.labelStrong, color = palette.primary)
                Spacer(Modifier.width(6.dp))
                WIcon(WitnessIcon.ArrowForward, palette.primary, size = 18.dp)
            }
        }
    }
}

internal const val PENDING_LINK = "Created when you tap Share link. Only people with the link can open it."
private const val STALE_ALPHA = 0.5f

@Composable
private fun CopyPill(onClick: () -> Unit) {
    val palette = Witness.colors
    Row(
        Modifier
            .height(30.dp)
            .clip(RoundedCornerShape(15.dp))
            .background(palette.surface2)
            .clickable(role = Role.Button, onClick = onClick)
            .padding(start = 12.dp, end = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(WitnessIcon.ContentCopy, palette.ink, size = 18.dp)
        Spacer(Modifier.width(6.dp))
        Text("Copy link", style = Witness.type.labelStrong, color = palette.ink)
    }
}
