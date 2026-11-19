package app.witness.feature.post

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.AiHeader
import app.witness.core.designsystem.BottomBar
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.GeneratedSurface
import app.witness.core.designsystem.GeneratedTag
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessAppBar
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessTextField
import app.witness.core.designsystem.WitnessToggle
import app.witness.core.designsystem.outlineChip
import app.witness.core.designsystem.solidChip
import app.witness.core.designsystem.tintedChip
import app.witness.core.model.PostDraft

@Composable
fun ComposerScreen(state: ComposerUiState, actions: ComposerActions, onClose: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    Box(modifier.fillMaxSize().background(palette.paper)) {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).statusBarsPadding().padding(bottom = 96.dp)) {
            WitnessAppBar(PostCopy.TITLE, subtitle = state.subtitle.ifEmpty { null }, navigationIcon = WitnessIcon.Close, navigationLabel = "Discard post", onNavigate = onClose)
            Spacer(Modifier.height(2.dp))
            val draft = state.draft
            ComposerPhoto(state, draft, actions)
            if (draft != null) {
                Spacer(Modifier.height(12.dp))
                BeforeYouPost(draft, state.place, actions)
                Spacer(Modifier.height(12.dp))
                CaptionCard(draft, state, actions)
                ProofLinkToggle(state.proofLink, actions::setProofLink)
                Text(PostCopy.PROOF_LINK_NOTE, Modifier.padding(start = 32.dp, end = 32.dp, top = 12.dp), style = Witness.type.caption, color = palette.muted)
            }
            state.error?.let { ErrorLine(it, actions::dismissError) }
        }
        BottomBar {
            WitnessButton("Copy link", actions::copyLink, Modifier.width(140.dp), ButtonKind.Tonal, WitnessIcon.ContentCopy, enabled = state.canShare)
            WitnessButton("Share post", actions::share, Modifier.weight(1f), ButtonKind.Filled, WitnessIcon.Share, enabled = state.canShare)
        }
        LocationSheet(state.sheet as? ComposerSheet.Location, state.place, actions)
        AltTextSheet(state.sheet as? ComposerSheet.AltText, actions)
        state.editor?.let { EditPhotoScreen(it, state.photo, actions) }
    }
}

private val PhotoShape = RoundedCornerShape(16.dp)

@Composable
private fun ComposerPhoto(state: ComposerUiState, draft: PostDraft?, actions: ComposerActions) {
    Box(Modifier.padding(horizontal = 16.dp).fillMaxWidth().height(290.dp).clip(PhotoShape)) {
        val blurs = remember(draft?.blurs, draft?.dims) { draft?.let { blurEdits(it.blurs, it.dims) }.orEmpty() }
        PhotoImage(state.photo, "Sealed photo", Modifier.fillMaxSize(), blurs = blurs, sourceWidth = draft?.dims?.width ?: 0)
        if (draft != null) {
            Row(
                Modifier
                    .align(Alignment.TopEnd)
                    .padding(top = 12.dp, end = 12.dp)
                    .height(32.dp)
                    .clip(WitnessShapes.Pill)
                    .background(WitnessColors.Black.copy(alpha = 0.5f))
                    .clickable(role = Role.Button, onClick = actions::openEditor)
                    .padding(start = 10.dp, end = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                WIcon(WitnessIcon.Tune, WitnessColors.White, size = 18.dp)
                Spacer(Modifier.width(4.dp))
                Text("Edit", style = Witness.type.labelStrong, color = WitnessColors.White)
            }
            if (state.enabledBlurs > 0) {
                WitnessChip(
                    PostCopy.declaredBlurs(state.enabledBlurs),
                    Witness.colors.solidChip(WitnessColors.Ink),
                    Modifier.align(Alignment.BottomStart).padding(start = 12.dp, bottom = 12.dp),
                    icon = WitnessIcon.Edit,
                    height = 26.dp,
                    fontSize = 11.5.sp,
                )
            }
        }
    }
}

@Composable
private fun BeforeYouPost(draft: PostDraft, place: String?, actions: ComposerActions) {
    val palette = Witness.colors
    GeneratedSurface(Modifier.padding(horizontal = 16.dp).fillMaxWidth()) {
        AiHeader(PostCopy.BEFORE_YOU_POST, onFlag = actions::flagAi)
        Spacer(Modifier.height(8.dp))
        PrepRow(WitnessIcon.Face, PostCopy.BLURS_TITLE) { BlurChips(draft, actions) }
        Hairline(Modifier.padding(vertical = 10.dp))
        val tone = PostCopy.precisionTone(draft.precision)
        PrepRow(
            WitnessIcon.NearMe,
            PostCopy.locationTitle(draft.precision, place, draft.coarse),
            action = "Change",
            onAction = actions::openLocation,
        ) {
            Text(PostCopy.locationSubtitle(draft.precision), style = Witness.type.caption, color = tone?.let(palette::toneText) ?: palette.ink2, maxLines = 1)
        }
        Hairline(Modifier.padding(vertical = 10.dp))
        PrepRow(WitnessIcon.ClosedCaption, PostCopy.altTitle(draft.aiGenerated), action = "Edit", onAction = actions::openAltText) {
            Text(draft.altText, style = Witness.type.caption, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

@Composable
private fun PrepRow(icon: WitnessIcon, title: String, action: String? = null, onAction: () -> Unit = {}, body: @Composable () -> Unit) {
    val palette = Witness.colors
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        IconDisc(icon, palette.ai, size = 30.dp, iconSize = 18.dp, fill = palette.ai.copy(alpha = 0.1f))
        Spacer(Modifier.width(8.dp))
        Column(Modifier.weight(1f)) {
            Text(title, style = Witness.type.bodySmallStrong, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.height(4.dp))
            body()
        }
        if (action != null) {
            Text(
                action,
                Modifier.clip(WitnessShapes.Field).clickable(role = Role.Button, onClick = onAction).padding(horizontal = 4.dp, vertical = 6.dp),
                style = Witness.type.labelStrong,
                color = palette.primary,
            )
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun BlurChips(draft: PostDraft, actions: ComposerActions) {
    val palette = Witness.colors
    if (draft.blurs.isEmpty()) {
        Text(PostCopy.NO_BLURS, style = Witness.type.caption, color = palette.ink2)
        return
    }
    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        draft.blurs.forEachIndexed { i, b ->
            val name = PostCopy.blurName(b.blurClass)
            if (b.enabled) {
                WitnessChip("$name  ×", palette.tintedChip(Tone.Edit), height = 26.dp, fontSize = 11.5.sp, onClick = { actions.toggleBlur(i) })
            } else {
                WitnessChip(name, palette.outlineChip(palette.muted), icon = WitnessIcon.Undo, height = 26.dp, fontSize = 11.5.sp, onClick = { actions.toggleBlur(i) })
            }
        }
    }
}

private val CaptionShape = RoundedCornerShape(16.dp)
private val FieldShape = RoundedCornerShape(8.dp)

@Composable
private fun CaptionCard(draft: PostDraft, state: ComposerUiState, actions: ComposerActions) {
    val palette = Witness.colors
    Column(
        Modifier
            .padding(horizontal = 16.dp)
            .fillMaxWidth()
            .clip(CaptionShape)
            .background(palette.surface)
            .border(1.2.dp, palette.ai.copy(alpha = 0.35f), CaptionShape)
            .padding(horizontal = 14.dp, vertical = 10.dp),
    ) {
        Row(Modifier.fillMaxWidth().height(24.dp), verticalAlignment = Alignment.CenterVertically) {
            Overline("Caption", Modifier.weight(1f).padding(start = 2.dp))
            if (state.captionSuggested) GeneratedTag("Suggested")
        }
        Spacer(Modifier.height(6.dp))
        WitnessTextField(
            draft.caption,
            actions::setCaption,
            Modifier.fillMaxWidth(),
            placeholder = "Write a caption",
            textStyle = Witness.type.bodySmall,
            shape = FieldShape,
            minHeight = 30.dp,
            singleLine = false,
        )
        Spacer(Modifier.height(8.dp))
        Row(
            Modifier.height(32.dp).clip(WitnessShapes.Pill).background(palette.primary.copy(alpha = 0.08f)).padding(horizontal = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            WIcon(if (state.proofLink) WitnessIcon.Link else WitnessIcon.LinkOff, palette.primary, size = 18.dp)
            Spacer(Modifier.width(6.dp))
            Text(state.linkText, style = Witness.type.proof.copy(fontSize = 12.5.sp), color = palette.primary, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

@Composable
private fun ProofLinkToggle(checked: Boolean, onChange: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth().padding(start = 32.dp, end = 16.dp, top = 12.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(PostCopy.PROOF_LINK_TOGGLE, Modifier.weight(1f), style = Witness.type.body.copy(fontWeight = FontWeight.Medium), color = Witness.colors.ink)
        WitnessToggle(checked, onChange)
    }
}

@Composable
private fun ErrorLine(message: String, onDismiss: () -> Unit) {
    val palette = Witness.colors
    Row(
        Modifier.padding(16.dp).fillMaxWidth().clip(WitnessShapes.Field).background(palette.crit.copy(alpha = 0.08f)).clickable(onClick = onDismiss).padding(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(WitnessIcon.Error, palette.crit, size = 18.dp)
        Spacer(Modifier.width(8.dp))
        Text(message, Modifier.weight(1f), style = Witness.type.bodySmall, color = palette.crit)
        WIcon(WitnessIcon.Close, palette.muted, size = 18.dp)
    }
}
