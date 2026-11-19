package app.witness.feature.check

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.ButtonKind
import app.witness.core.designsystem.DropZone
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.StateChip
import app.witness.core.designsystem.TextAction
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessTextField
import app.witness.core.model.RecordId
import app.witness.core.model.Verdict
import app.witness.core.ui.ComparePane
import app.witness.core.ui.FactLayout
import app.witness.core.ui.Formatters
import app.witness.core.ui.ProofFacts
import app.witness.core.ui.ReferenceCompare
import app.witness.core.ui.VerdictCard
import app.witness.core.ui.state

@Immutable
class CheckActions(
    val onChoose: () -> Unit = {},
    val onLinkChange: (String) -> Unit = {},
    val onSubmitLink: (String) -> Unit = {},
    val onCancel: () -> Unit = {},
    val onOpenProof: (RecordId) -> Unit = {},
    val onShare: (String) -> Unit = {},
    val onNotify: (Boolean) -> Unit = {},
    val onClose: () -> Unit = {},
)

@Composable
fun BitmapBox(bitmap: ImageBitmap?, description: String?, modifier: Modifier = Modifier, contentScale: ContentScale = ContentScale.Crop) {
    Box(modifier.background(Witness.colors.surface2), contentAlignment = Alignment.Center) {
        if (bitmap != null) Image(bitmap, description, Modifier.fillMaxSize(), contentScale = contentScale)
        else WIcon(WitnessIcon.Image, Witness.colors.muted, size = 28.dp)
    }
}

fun dimsText(image: PickedImage, bitmap: ImageBitmap?): String? =
    image.dims?.let { Formatters.dimensions(it.width, it.height) } ?: bitmap?.let { Formatters.dimensions(it.width, it.height) }

fun appFileLine(image: PickedImage, bitmap: ImageBitmap?, kind: String): String =
    Formatters.joinDot(kind, dimsText(image, bitmap), Formatters.bytes(image.size.toLong()))

fun webFileLine(image: PickedImage, bitmap: ImageBitmap?): String =
    Formatters.joinDot(Formatters.bytes(image.size.toLong()), dimsText(image, bitmap), image.format)

@Composable
fun ChooseDropZone(onChoose: () -> Unit, subtitle: String, modifier: Modifier = Modifier, large: Boolean = false) {
    val palette = Witness.colors
    val type = Witness.type
    DropZone(modifier.fillMaxWidth(), onClick = onChoose) {
        Spacer(Modifier.height(if (large) 8.dp else 6.dp))
        WIcon(WitnessIcon.PhotoLibrary, palette.ink2, size = if (large) 36.dp else 32.dp)
        Spacer(Modifier.height(if (large) 14.dp else 10.dp))
        Text("Choose a photo or screenshot", style = if (large) type.section else type.actionTitle, color = palette.ink, textAlign = TextAlign.Center)
        Spacer(Modifier.height(4.dp))
        Text(subtitle, style = type.subtitle, color = palette.ink2, textAlign = TextAlign.Center)
        Spacer(Modifier.height(if (large) 20.dp else 10.dp))
        if (large) WitnessButton("Choose photo", onChoose, Modifier.width(176.dp), ButtonKind.Ink, icon = WitnessIcon.Upload)
        else WitnessButton("Choose photo", onChoose, Modifier.width(160.dp), ButtonKind.Ink, height = 30.dp, textStyle = type.bodySmallStrong.merge(fontSize = 13.sp))
    }
}

@Suppress("DEPRECATION")
@Composable
fun PasteField(value: String, error: String?, actions: CheckActions, modifier: Modifier = Modifier) {
    val clipboard = LocalClipboardManager.current
    Column(modifier.fillMaxWidth()) {
        WitnessTextField(
            value = value,
            onValueChange = actions.onLinkChange,
            modifier = Modifier.fillMaxWidth(),
            placeholder = "Paste a post or proof link",
            leadingIcon = WitnessIcon.Link,
            trailing = {
                TextAction("Paste", {
                    val text = if (value.isBlank()) clipboard.getText()?.text.orEmpty() else value
                    actions.onSubmitLink(text)
                }, color = Witness.colors.primary)
            },
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Go),
            keyboardActions = KeyboardActions(onGo = { actions.onSubmitLink(value) }),
        )
        if (error != null) Text(error, Modifier.padding(start = 16.dp, top = 6.dp), style = Witness.type.caption, color = Witness.colors.crit)
    }
}

@Composable
fun OutcomeVerdict(outcome: CheckOutcome, modifier: Modifier = Modifier) {
    VerdictCard(
        outcome.verdict,
        modifier,
        headline = outcome.headline,
        qualifier = outcome.qualifier,
        body = outcome.note?.let { outcome.body + "\n" + it } ?: outcome.body,
    )
}

@Composable
fun CompareSection(
    outcome: CheckOutcome,
    input: ImageBitmap?,
    reference: ImageBitmap?,
    inputLabel: String,
    inputCaption: String?,
    modifier: Modifier = Modifier,
    inputCaptionTone: Tone? = null,
) {
    val altered = outcome.verdict == Verdict.ALTERED
    val refPane = ComparePane(
        if (altered) "Reference" else "Sealed reference",
        caption = if (altered) Formatters.joinDot("Reference", outcome.referenceDate) else outcome.referenceCaption,
        dot = if (altered) Tone.Ok else null,
        sourceAspect = outcome.referenceAspect,
        crop = outcome.crop,
    )
    val copyPane = ComparePane(
        inputLabel,
        caption = if (altered) "This file" else inputCaption,
        captionTone = if (altered) null else inputCaptionTone,
        dot = if (altered) Tone.Crit else null,
        changedRegions = outcome.regions,
    )
    val copy: @Composable androidx.compose.foundation.layout.BoxScope.() -> Unit = { BitmapBox(input, inputLabel, Modifier.fillMaxSize()) }
    val ref: @Composable androidx.compose.foundation.layout.BoxScope.() -> Unit = { BitmapBox(reference, "Sealed reference", Modifier.fillMaxSize()) }
    if (altered) ReferenceCompare(refPane, copyPane, ref, copy, modifier, title = "Sealed reference vs. this file")
    else ReferenceCompare(copyPane, refPane, copy, ref, modifier)
}

@Composable
fun ChangesCard(changes: List<ChangeLine>, modifier: Modifier = Modifier, attribution: Boolean = false) {
    val palette = Witness.colors
    val type = Witness.type
    WitnessCard(modifier.fillMaxWidth()) {
        Overline("What changed")
        changes.forEach { line ->
            Row(Modifier.fillMaxWidth().padding(top = 10.dp).semantics(mergeDescendants = true) {}, verticalAlignment = Alignment.Top) {
                WIcon(line.icon, palette.tone(line.tone), size = 20.dp)
                Spacer(Modifier.width(10.dp))
                Column(Modifier.weight(1f)) {
                    Text(line.title, style = type.bodySmallStrong, color = palette.ink)
                    if (line.detail != null) Text(line.detail, style = type.caption, color = palette.ink2)
                }
                if (attribution && line.by != null) {
                    Text(line.by, style = type.labelStrong, color = if (line.by == "This copy") palette.ink2 else palette.edit)
                }
            }
        }
    }
}

@Composable
fun OutcomeFacts(outcome: CheckOutcome, modifier: Modifier = Modifier) {
    when (outcome.verdict) {
        Verdict.NO_RECORD -> ProofFacts(outcome.facts, modifier, title = "What we checked", dividers = false, labelWidth = 116.dp)
        Verdict.SEALED_UNANCHORED -> ProofFacts(outcome.facts, modifier, title = "What we could check", layout = FactLayout.Stacked)
        Verdict.MATCHES_WITH_CHANGES -> Unit
        else -> if (outcome.facts.isNotEmpty()) ProofFacts(outcome.facts, modifier)
    }
}

@Composable
fun SawThisOnline(modifier: Modifier = Modifier) {
    WitnessCard(modifier.fillMaxWidth()) {
        Text("Saw this online?", style = Witness.type.bodyStrong, color = Witness.colors.ink)
        Spacer(Modifier.height(4.dp))
        Text(
            "Ask the poster for a Witness proof link — originals captured with Witness always have one.",
            style = Witness.type.subtitle,
            color = Witness.colors.ink2,
        )
    }
}

@Composable
fun CapturedLine(text: String, modifier: Modifier = Modifier) {
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        WIcon(WitnessIcon.Schedule, Witness.colors.ink2, size = 18.dp)
        Spacer(Modifier.width(8.dp))
        Text(text, style = Witness.type.caption, color = Witness.colors.ink2)
    }
}

@Composable
fun RecentRow(check: RecentCheck, meta: String, onClick: (() -> Unit)?, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    val state = check.verdict.state()
    WitnessCard(modifier.fillMaxWidth(), shape = WitnessShapes.Row, contentPadding = androidx.compose.foundation.layout.PaddingValues(10.dp), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconDisc(state.icon, palette.tone(state.tone), size = 54.dp, iconSize = 26.dp)
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(check.label, style = Witness.type.bodyStrong, color = palette.ink, maxLines = 1)
                Text(meta, style = Witness.type.caption, color = palette.ink2, maxLines = 1)
                Spacer(Modifier.height(4.dp))
                StateChip(state, text = state.shortLabel, height = 22.dp, fontSize = 10.5.sp)
            }
        }
    }
}

@Composable
fun ErrorCard(message: String, onRetry: () -> Unit, modifier: Modifier = Modifier) {
    WitnessCard(modifier.fillMaxWidth()) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            WIcon(WitnessIcon.Error, Witness.colors.crit, size = 22.dp)
            Spacer(Modifier.width(10.dp))
            Text(message, Modifier.weight(1f), style = Witness.type.body, color = Witness.colors.ink)
        }
        Spacer(Modifier.height(12.dp))
        WitnessButton("Try another photo", onRetry, Modifier.fillMaxWidth(), ButtonKind.Outlined)
    }
}

@Composable
fun ImageThumb(bitmap: ImageBitmap?, size: Dp, corner: Dp, description: String?) {
    BitmapBox(bitmap, description, Modifier.size(size).clip(androidx.compose.foundation.shape.RoundedCornerShape(corner)))
}

@Composable
fun LinkText(text: String, icon: WitnessIcon, onClick: () -> Unit, modifier: Modifier = Modifier) {
    Row(modifier.clickable(role = Role.Button, onClick = onClick).padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text, style = Witness.type.bodyStrong, color = Witness.colors.primary)
        WIcon(icon, Witness.colors.primary, size = 18.dp)
    }
}
