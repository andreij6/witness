package app.witness.feature.check

import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.BottomBar
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.StatusLine
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessAppBar
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessToggle
import app.witness.core.model.Verdict
import app.witness.core.ui.CheckProgress
import app.witness.core.ui.Formatters
import app.witness.core.ui.rememberBitmap

@Composable
fun CheckScreen(state: CheckUiState, actions: CheckActions, modifier: Modifier = Modifier, shared: Boolean = false, now: Long = 0L, time: TimeText = TimeText.UTC) {
    val subtitle = if (shared) "Shared from another app" else "Is this photo real?"
    val outcome = state.outcome
    Box(modifier.fillMaxSize().background(Witness.colors.paper).windowInsetsPadding(WindowInsets.statusBars)) {
        Column(Modifier.fillMaxSize()) {
            val idle = state.phase == CheckPhase.Idle
            WitnessAppBar(
                "Check",
                subtitle = subtitle,
                navigationIcon = if (idle && !shared) null else WitnessIcon.Close,
                navigationLabel = "Close",
                onNavigate = if (shared) actions.onClose else actions.onCancel,
            )
            Column(
                Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Spacer(Modifier.height(4.dp))
                when (state.phase) {
                    CheckPhase.Idle -> IdleContent(state, actions, now, time)
                    CheckPhase.Checking -> CheckingContent(state)
                    CheckPhase.Failed -> ErrorCard(state.error ?: CheckViewModel.FAILED, actions.onCancel)
                    CheckPhase.Done -> if (outcome != null) {
                        if (outcome.verdict == Verdict.SEALED_UNANCHORED) UnanchoredContent(state, outcome, actions)
                        else ResultContent(state, outcome)
                    }
                }
                Spacer(Modifier.height(if (state.phase == CheckPhase.Done || state.phase == CheckPhase.Checking) 96.dp else 24.dp))
            }
        }
        when (state.phase) {
            CheckPhase.Checking -> WitnessButton(
                "Cancel",
                actions.onCancel,
                Modifier.align(Alignment.BottomCenter).padding(bottom = 24.dp).width(120.dp),
                ButtonKind.Outlined,
                height = 44.dp,
            )
            CheckPhase.Done -> BottomBar {
                when {
                    outcome?.verdict == Verdict.SEALED_UNANCHORED ->
                        WitnessButton("Check again later", actions.onCancel, Modifier.weight(1f), ButtonKind.Tonal, WitnessIcon.Schedule)
                    outcome?.recordId != null -> {
                        state.shareText?.let { text -> WitnessButton("Share result", { actions.onShare(text) }, kind = ButtonKind.Tonal, icon = WitnessIcon.Share) }
                        WitnessButton("Open proof page", { actions.onOpenProof(outcome.recordId) }, Modifier.weight(1f), icon = WitnessIcon.OpenInNew)
                    }
                    else -> WitnessButton("Check another photo", actions.onCancel, Modifier.weight(1f), ButtonKind.Tonal, WitnessIcon.ImageSearch)
                }
            }
            else -> Unit
        }
    }
}

@Composable
private fun ColumnScope.IdleContent(state: CheckUiState, actions: CheckActions, now: Long, time: TimeText) {
    ChooseDropZone(actions.onChoose, "Checked on your phone — nothing is uploaded")
    PasteField(state.link, state.linkError, actions)
    StatusLine("Tip: in any app, use Share → Witness", WitnessIcon.Share, tint = Witness.colors.muted)
    if (state.recent.isNotEmpty()) {
        Text("Recent checks", Modifier.padding(top = 12.dp), style = Witness.type.section, color = Witness.colors.ink)
        state.recent.forEach { check ->
            val meta = Formatters.joinDot(time.relative(check.checkedAtMs, now), check.note)
            RecentRow(check, meta, check.recordId?.let { id -> { actions.onOpenProof(id) } })
        }
    }
}

@Composable
private fun ColumnScope.CheckingContent(state: CheckUiState) {
    val image = state.image ?: return
    val bitmap = rememberBitmap(image.bytes)
    BitmapBox(bitmap, image.name ?: "Photo being checked", Modifier.align(Alignment.CenterHorizontally).size(272.dp).clip(WitnessShapes.Panel))
    Text(
        appFileLine(image, bitmap, image.format ?: "Image"),
        Modifier.fillMaxWidth(),
        style = Witness.type.proof,
        color = Witness.colors.muted,
        textAlign = TextAlign.Center,
    )
    CheckProgress(state.stages, Modifier.padding(top = 8.dp))
    StatusLine("Checked on your phone — nothing is uploaded", WitnessIcon.PhonelinkLock, Modifier.padding(top = 8.dp, start = 4.dp))
}

@Composable
private fun ColumnScope.ResultContent(state: CheckUiState, outcome: CheckOutcome) {
    OutcomeVerdict(outcome)
    val input = rememberBitmap(state.image?.bytes)
    if (outcome.hasReference && state.reference != null) {
        val reference = rememberBitmap(state.reference)
        CompareSection(outcome, input, reference, "Your screenshot", "Tap either to compare", inputCaptionTone = Tone.Primary)
    }
    if (outcome.changes.isNotEmpty()) ChangesCard(outcome.changes)
    OutcomeFacts(outcome)
    if (outcome.verdict == Verdict.NO_RECORD && !outcome.registryUnverified) SawThisOnline()
    outcome.captured?.let { Text(it, Modifier.padding(horizontal = 4.dp), style = Witness.type.caption, color = Witness.colors.ink2) }
}

@Composable
private fun ColumnScope.UnanchoredContent(state: CheckUiState, outcome: CheckOutcome, actions: CheckActions) {
    val palette = Witness.colors
    val type = Witness.type
    OutcomeVerdict(outcome)
    state.image?.let { image ->
        val bitmap = rememberBitmap(image.bytes)
        WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                ImageThumb(bitmap, 64.dp, 10.dp, image.name)
                Spacer(Modifier.width(14.dp))
                Column(Modifier.weight(1f)) {
                    Text(image.name ?: CheckViewModel.DEFAULT_LABEL, style = type.bodyStrong, color = palette.ink, maxLines = 1)
                    Text(Formatters.bytes(image.size.toLong()), style = type.caption, color = palette.ink2)
                    outcome.captured?.let { Text(it.substringBefore(Formatters.Dot), style = type.caption, color = palette.ink2) }
                }
            }
        }
    }
    OutcomeFacts(outcome)
    Row(Modifier.padding(top = 12.dp, start = 4.dp)) {
        WIcon(WitnessIcon.Info, palette.ink2, size = 18.dp)
        Spacer(Modifier.width(8.dp))
        Column {
            Text("Check again later for the full verdict.", style = type.bodySmallStrong, color = palette.ink)
            Text(
                "Until it's anchored we can't say when it was first registered or whether its key was revoked.",
                style = type.subtitle,
                color = palette.ink2,
            )
        }
    }
    WitnessCard(Modifier.fillMaxWidth().padding(top = 8.dp), shape = WitnessShapes.Row) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            WIcon(WitnessIcon.Notifications, palette.ink, size = 22.dp)
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text("Tell me when it's anchored", style = type.bodyStrong, color = palette.ink)
                Text("We'll recheck quietly in the background", style = type.caption, color = palette.ink2)
            }
            WitnessToggle(state.notifyWhenAnchored, actions.onNotify)
        }
    }
}
