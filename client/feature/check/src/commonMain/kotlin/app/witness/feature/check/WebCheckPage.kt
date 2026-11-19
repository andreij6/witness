package app.witness.feature.check

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
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.dashedBorder
import app.witness.core.designsystem.outlineChip
import app.witness.core.model.Verdict
import app.witness.core.ui.BrowserSafeLayout
import app.witness.core.ui.CheckProgress
import app.witness.core.ui.Formatters
import app.witness.core.ui.WebFooter
import app.witness.core.ui.WebHeader
import app.witness.core.ui.WebLayoutMode
import app.witness.core.ui.WebLink
import app.witness.core.ui.rememberBitmap

@Composable
fun WebCheckPage(state: CheckUiState, actions: CheckActions, footerLinks: List<WebLink>, modifier: Modifier = Modifier, headerLinks: List<WebLink> = emptyList()) {
    BrowserSafeLayout(
        modifier,
        header = { spec ->
            val wide = spec.mode == WebLayoutMode.Wide
            WebHeader(label = "Check a photo", links = if (wide) headerLinks else emptyList(), wide = wide)
        },
        footer = { spec -> WebFooter(footerLinks, wide = spec.mode == WebLayoutMode.Wide) },
        secondary = { spec -> if (spec.mode == WebLayoutMode.Wide) WideResult(state, actions) },
    ) { spec ->
        if (spec.mode == WebLayoutMode.Wide) WideInput(state, actions)
        else CompactPage(state, actions)
    }
}

@Composable
private fun ColumnScope.CompactPage(state: CheckUiState, actions: CheckActions) {
    val outcome = state.outcome
    when (state.phase) {
        CheckPhase.Idle -> WebIdle(state, actions)
        CheckPhase.Checking -> WebChecking(state, actions)
        CheckPhase.Failed -> ErrorCard(state.error ?: CheckViewModel.FAILED, actions.onCancel)
        CheckPhase.Done -> if (outcome != null) WebResult(state, outcome, actions)
    }
}

@Composable
private fun ColumnScope.WebIdle(state: CheckUiState, actions: CheckActions) {
    val palette = Witness.colors
    val type = Witness.type
    Text("Is this photo real?", style = type.pageTitle, color = palette.ink)
    Text("Compare any photo with sealed Witness captures.", style = type.bodySmall, color = palette.ink2)
    ChooseDropZone(actions.onChoose, "Checked in your browser — nothing is uploaded", Modifier.padding(top = 4.dp), large = true)
    PasteField(state.link, state.linkError, actions)
    HowItWorks()
    Row(Modifier.padding(top = 4.dp)) {
        WIcon(WitnessIcon.PrivacyTip, palette.muted, size = 18.dp)
        Spacer(Modifier.width(8.dp))
        Text("Your photo stays on this device. Only a short fingerprint is sent to search.", style = type.subtitle, color = palette.muted)
    }
}

@Composable
private fun HowItWorks() {
    val palette = Witness.colors
    WitnessCard(Modifier.fillMaxWidth()) {
        Overline("How it works")
        HOW_IT_WORKS.forEachIndexed { k, (icon, title, detail) ->
            Row(Modifier.padding(top = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                IconDisc(icon, palette.ink, fill = palette.surface2)
                Spacer(Modifier.width(12.dp))
                Column {
                    Text("${k + 1}. $title", style = Witness.type.bodySmallStrong, color = palette.ink)
                    Text(detail, style = Witness.type.caption, color = palette.ink2)
                }
            }
        }
    }
}

@Composable
private fun ColumnScope.WebChecking(state: CheckUiState, actions: CheckActions) {
    val palette = Witness.colors
    val type = Witness.type
    state.image?.let { FileCard(it, rememberBitmap(it.bytes)) }
    Text("Checking…", Modifier.padding(top = 12.dp), style = type.headline, color = palette.ink)
    Text("Usually under 5 seconds", style = type.bodyCompact, color = palette.ink2)
    CheckProgress(state.stages, dividers = true)
    Row(
        Modifier.fillMaxWidth().clip(WitnessShapes.Card).background(palette.surface2).padding(16.dp),
        verticalAlignment = Alignment.Top,
    ) {
        WIcon(WitnessIcon.Fingerprint, palette.ink2, size = 20.dp)
        Spacer(Modifier.width(10.dp))
        Text("Only a short fingerprint leaves your browser. The photo itself is never sent.", style = type.subtitle, color = palette.ink2)
    }
    WitnessButton("Cancel", actions.onCancel, Modifier.fillMaxWidth(), ButtonKind.Outlined)
}

@Composable
private fun FileCard(image: PickedImage, bitmap: ImageBitmap?) {
    val palette = Witness.colors
    WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(12.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            ImageThumb(bitmap, 80.dp, 12.dp, image.name)
            Spacer(Modifier.width(16.dp))
            Column(Modifier.weight(1f)) {
                Text(image.name ?: CheckViewModel.DEFAULT_LABEL, style = Witness.type.bodyStrong, color = palette.ink, maxLines = 1)
                Text(webFileLine(image, bitmap), style = Witness.type.caption, color = palette.ink2)
                Spacer(Modifier.height(6.dp))
                WitnessChip("Stays in your browser", palette.outlineChip(), icon = WitnessIcon.Lock, height = 22.dp, fontSize = 10.5.sp)
            }
        }
    }
}

@Composable
private fun ColumnScope.WebResult(state: CheckUiState, outcome: CheckOutcome, actions: CheckActions) {
    val input = rememberBitmap(state.image?.bytes)
    if (outcome.verdict == Verdict.NO_RECORD) {
        BitmapBox(input, state.image?.name, Modifier.fillMaxWidth().height(236.dp).clip(WitnessShapes.ImageLarge))
    }
    OutcomeVerdict(outcome)
    if (outcome.hasReference && state.reference != null) {
        val caption = state.image?.let { img -> Formatters.joinDot(if (outcome.crop != null) "Square crop" else null, img.format) }
        CompareSection(outcome, input, rememberBitmap(state.reference), "Your screenshot", caption?.takeIf { it.isNotEmpty() })
    }
    if (outcome.changes.isNotEmpty() && outcome.verdict != Verdict.ALTERED) ChangesCard(outcome.changes)
    OutcomeFacts(outcome)
    if (outcome.verdict == Verdict.NO_RECORD && !outcome.registryUnverified) SawThisOnline()
    ResultLinks(outcome, actions)
}

@Composable
private fun ResultLinks(outcome: CheckOutcome, actions: CheckActions) {
    val id = outcome.recordId
    Row(horizontalArrangement = Arrangement.spacedBy(24.dp)) {
        if (id != null && outcome.verdict != Verdict.NO_RECORD) LinkText("Open proof page", WitnessIcon.OpenInNew, { actions.onOpenProof(id) })
        LinkText("Check another", WitnessIcon.ImageSearch, actions.onCancel)
    }
}

@Composable
private fun ColumnScope.WideInput(state: CheckUiState, actions: CheckActions) {
    val palette = Witness.colors
    val type = Witness.type
    val image = state.image
    Column(
        Modifier
            .fillMaxWidth()
            .clip(WitnessShapes.Panel)
            .background(palette.surface)
            .dashedBorder(palette.muted.copy(alpha = 0.7f), 20.dp)
            .clickable(role = Role.Button, onClick = actions.onChoose)
            .padding(horizontal = 56.dp, vertical = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        if (image == null) {
            Spacer(Modifier.height(120.dp))
            WIcon(WitnessIcon.PhotoLibrary, palette.ink2, size = 40.dp)
            Spacer(Modifier.height(16.dp))
            Text("Drop a screenshot or choose a photo", style = type.section, color = palette.ink, textAlign = TextAlign.Center)
            Spacer(Modifier.height(4.dp))
            Text("Checked in your browser · the file is never uploaded", style = type.subtitle, color = palette.ink2, textAlign = TextAlign.Center)
            Spacer(Modifier.height(20.dp))
            WitnessButton("Choose photo", actions.onChoose, Modifier.width(176.dp), ButtonKind.Ink, icon = WitnessIcon.Upload)
            Spacer(Modifier.height(120.dp))
        } else {
            val bitmap = rememberBitmap(image.bytes)
            BitmapBox(bitmap, image.name, Modifier.fillMaxWidth().aspectRatio(1f).clip(WitnessShapes.Card))
            Spacer(Modifier.height(16.dp))
            Row(
                Modifier.fillMaxWidth().clip(WitnessShapes.Image).background(palette.surface2).padding(horizontal = 14.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                WIcon(WitnessIcon.Image, palette.ink2, size = 20.dp)
                Spacer(Modifier.width(10.dp))
                Text(Formatters.joinDot(image.name ?: CheckViewModel.DEFAULT_LABEL, Formatters.bytes(image.size.toLong())), Modifier.weight(1f), style = type.bodySmallStrong, color = palette.ink, maxLines = 1)
                Text(Formatters.joinDot(bitmap?.let { Formatters.dimensions(it.width, it.height) }, image.format), style = type.proofSmall, color = palette.muted)
                Spacer(Modifier.width(12.dp))
                Box(Modifier.size(24.dp).clickable(role = Role.Button, onClick = actions.onCancel), contentAlignment = Alignment.Center) {
                    WIcon(WitnessIcon.Close, palette.ink2, size = 20.dp, contentDescription = "Remove")
                }
            }
            Spacer(Modifier.height(20.dp))
            Text("Drop another screenshot, paste an image or a link", style = type.bodyStrong, color = palette.ink, textAlign = TextAlign.Center)
            Text("Checked in your browser · the file is never uploaded", style = type.subtitle, color = palette.ink2, textAlign = TextAlign.Center)
        }
    }
    PasteField(state.link, state.linkError, actions)
}

@Composable
private fun ColumnScope.WideResult(state: CheckUiState, actions: CheckActions) {
    val outcome = state.outcome
    when (state.phase) {
        CheckPhase.Idle -> HowItWorks()
        CheckPhase.Checking -> {
            Text("Checking…", style = Witness.type.headline, color = Witness.colors.ink)
            CheckProgress(state.stages, dividers = true)
            WitnessButton("Cancel", actions.onCancel, Modifier.width(160.dp), ButtonKind.Outlined, height = 40.dp)
        }
        CheckPhase.Failed -> ErrorCard(state.error ?: CheckViewModel.FAILED, actions.onCancel)
        CheckPhase.Done -> if (outcome != null) {
            OutcomeVerdict(outcome)
            if (outcome.hasReference && state.reference != null) WideCompare(outcome, rememberBitmap(state.image?.bytes), rememberBitmap(state.reference))
            if (outcome.changes.isNotEmpty()) ChangesCard(outcome.changes, attribution = true)
            if (outcome.verdict != Verdict.MATCHES_WITH_CHANGES && outcome.verdict != Verdict.ORIGINAL) OutcomeFacts(outcome)
            outcome.captured?.let { CapturedLine(it) }
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                val id = outcome.recordId
                if (id != null && outcome.verdict != Verdict.NO_RECORD) {
                    WitnessButton("Open proof page", { actions.onOpenProof(id) }, height = 40.dp, icon = WitnessIcon.OpenInNew)
                }
                WitnessButton("Check another", actions.onCancel, height = 40.dp, kind = ButtonKind.Outlined)
            }
        }
    }
}

@Composable
private fun WideCompare(outcome: CheckOutcome, input: ImageBitmap?, reference: ImageBitmap?) {
    val palette = Witness.colors
    val type = Witness.type
    WitnessCard(Modifier.fillMaxWidth()) {
        Overline("This screenshot vs. sealed reference")
        Spacer(Modifier.height(12.dp))
        Row(Modifier.height(168.dp)) {
            Column(Modifier.weight(1f)) {
                BitmapBox(input, "Screenshot", Modifier.fillMaxWidth().weight(1f).clip(WitnessShapes.Image))
                Dot("Screenshot", palette.edit)
            }
            Box(Modifier.width(36.dp).fillMaxHeight(), contentAlignment = Alignment.Center) { WIcon(WitnessIcon.Compare, palette.muted) }
            Column(Modifier.weight(0.75f)) {
                BitmapBox(reference, "Reference", Modifier.fillMaxWidth().weight(1f).clip(WitnessShapes.Image))
                Dot("Reference", palette.ok)
            }
            Spacer(Modifier.width(20.dp))
            Box(Modifier.width(1.dp).fillMaxHeight().background(palette.line))
            Spacer(Modifier.width(20.dp))
            Column(Modifier.weight(1.2f), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                listOf("Matched by" to outcome.matchedBy, "Record" to outcome.recordId?.let { Formatters.abbreviateMiddle(it.grouped(), 8, 0) }, "Aligned" to outcome.aligned)
                    .forEach { (label, value) ->
                        if (value != null) Column {
                            Text(label, style = type.meta, color = palette.muted)
                            Text(value, style = if (label == "Record") type.proofStrong else type.bodySmallStrong, color = palette.ink)
                        }
                    }
            }
        }
    }
}

@Composable
private fun Dot(label: String, color: androidx.compose.ui.graphics.Color) {
    Row(Modifier.padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(8.dp).clip(WitnessShapes.Pill).background(color))
        Spacer(Modifier.width(8.dp))
        Text(label, style = Witness.type.labelStrong, color = Witness.colors.ink)
    }
}

private val HOW_IT_WORKS = listOf(
    Triple(WitnessIcon.Fingerprint, "Fingerprint", "Survives screenshots and recompression"),
    Triple(WitnessIcon.Tag, "Watermark", "Reads the invisible ID in Witness photos"),
    Triple(WitnessIcon.Compare, "Compare", "Region by region against the sealed original"),
)
