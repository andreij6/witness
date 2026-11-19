package app.witness.core.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
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
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.SealBadge
import app.witness.core.designsystem.StateChip
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.ToneChip
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessState
import app.witness.core.designsystem.solidChip

typealias Thumbnail = @Composable BoxScope.(index: Int) -> Unit

@Immutable
data class StateCount(val state: WitnessState, val count: Int)

fun stateCounts(anchored: Int, queued: Int, altered: Int = 0): List<StateCount> = buildList {
    if (anchored > 0) add(StateCount(WitnessState.Anchored, anchored))
    if (queued > 0) add(StateCount(WitnessState.Queued, queued))
    if (altered > 0) add(StateCount(WitnessState.Altered, altered))
}

fun mosaicShapes(count: Int, outer: Dp, inner: Dp = 2.dp): List<Shape> = when {
    count >= 3 -> listOf(
        RoundedCornerShape(outer, inner, inner, outer),
        RoundedCornerShape(inner, outer, inner, inner),
        RoundedCornerShape(inner, inner, outer, inner),
    )
    count >= 1 -> listOf(RoundedCornerShape(outer))
    else -> emptyList()
}

@Composable
fun CaseMosaic(count: Int, modifier: Modifier = Modifier, corner: Dp = 10.dp, leadFraction: Float = 0.6f, thumbnail: Thumbnail) {
    val shapes = mosaicShapes(count, corner)
    Row(modifier.clip(RoundedCornerShape(corner)), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
        if (shapes.isEmpty()) {
            Box(Modifier.fillMaxSize().clip(RoundedCornerShape(corner)).background(Witness.colors.surface2))
            return@Row
        }
        Box(Modifier.weight(if (shapes.size == 3) leadFraction else 1f).fillMaxHeight().clip(shapes[0])) { thumbnail(0) }
        if (shapes.size == 3) {
            Column(Modifier.weight(1f - leadFraction).fillMaxHeight(), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Box(Modifier.weight(1f).fillMaxWidth().clip(shapes[1])) { thumbnail(1) }
                Box(Modifier.weight(1f).fillMaxWidth().clip(shapes[2])) { thumbnail(2) }
            }
        }
    }
}

@Composable
private fun Counts(counts: List<StateCount>) {
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        counts.forEach { StateChip(it.state, text = Formatters.groupThousands(it.count), height = 24.dp, fontSize = 11.sp) }
    }
}

@Composable
fun CaseRow(title: String, meta: String, counts: List<StateCount>, thumbnailCount: Int, modifier: Modifier = Modifier, onClick: (() -> Unit)? = null, thumbnail: Thumbnail) {
    val palette = Witness.colors
    WitnessCard(modifier.fillMaxWidth(), shape = WitnessShapes.Row, contentPadding = PaddingValues(8.dp), onClick = onClick) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            CaseMosaic(thumbnailCount, Modifier.padding(start = 2.dp).size(76.dp, 56.dp), thumbnail = thumbnail)
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(title, style = Witness.type.rowTitle, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Spacer(Modifier.height(4.dp))
                Text(meta, style = Witness.type.caption, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            Counts(counts)
            Spacer(Modifier.width(6.dp))
        }
    }
}

@Composable
fun CaseCard(
    title: String,
    meta: String,
    counts: List<StateCount>,
    thumbnailCount: Int,
    modifier: Modifier = Modifier,
    sharedLabel: String? = null,
    onClick: (() -> Unit)? = null,
    thumbnail: Thumbnail,
) {
    val palette = Witness.colors
    WitnessCard(modifier.fillMaxWidth(), contentPadding = PaddingValues(8.dp), onClick = onClick) {
        Box {
            CaseMosaic(thumbnailCount, Modifier.fillMaxWidth().height(128.dp), corner = 12.dp, leadFraction = 0.66f, thumbnail = thumbnail)
            if (sharedLabel != null) {
                WitnessChip(sharedLabel, palette.solidChip(palette.surface, palette.primary), Modifier.padding(8.dp), icon = WitnessIcon.Link, fontSize = 11.5.sp)
            }
        }
        Row(Modifier.padding(start = 8.dp, end = 8.dp, top = 10.dp, bottom = 6.dp), verticalAlignment = Alignment.Top) {
            Column(Modifier.weight(1f)) {
                Text(title, style = Witness.type.cardTitle, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Spacer(Modifier.height(4.dp))
                Text(meta, style = Witness.type.subtitle, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            Counts(counts)
        }
    }
}

@Composable
fun CaseItemRow(title: String, meta: String, state: WitnessState, modifier: Modifier = Modifier, onClick: (() -> Unit)? = null, thumbnail: Thumbnail) {
    val palette = Witness.colors
    Row(
        modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier)
            .semantics(mergeDescendants = true) {}
            .padding(horizontal = 14.dp, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(40.dp, 44.dp).clip(RoundedCornerShape(8.dp)).background(palette.surface2)) { thumbnail(0) }
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(title, style = Witness.type.bodySmallStrong, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.height(2.dp))
            Text(meta, style = Witness.type.meta, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        WIcon(state.icon, palette.tone(state.tone), size = 20.dp, contentDescription = state.label)
        Spacer(Modifier.width(4.dp))
        if (onClick != null) WIcon(WitnessIcon.ChevronRight, palette.muted, size = 22.dp)
    }
}

@Composable
fun PostCard(
    title: String,
    meta: String,
    state: WitnessState,
    modifier: Modifier = Modifier,
    checks: Long? = null,
    kindLabel: String = "Post",
    onClick: (() -> Unit)? = null,
    image: @Composable BoxScope.() -> Unit,
) {
    val palette = Witness.colors
    WitnessCard(modifier.fillMaxWidth(), contentPadding = PaddingValues(8.dp), onClick = onClick) {
        Box(Modifier.fillMaxWidth().height(180.dp).clip(WitnessShapes.Image).background(palette.surface2)) {
            image()
            WitnessChip(kindLabel, palette.solidChip(WitnessColors.Black), Modifier.padding(10.dp), icon = WitnessIcon.Public, height = 24.dp, fontSize = 11.5.sp)
        }
        Column(Modifier.padding(start = 8.dp, end = 8.dp, top = 10.dp, bottom = 6.dp)) {
            Text(title, style = Witness.type.cardHeadline, color = palette.ink, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.height(4.dp))
            Text(meta, style = Witness.type.subtitle, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.height(8.dp))
            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                StateChip(state, height = 24.dp, fontSize = 11.sp)
                if (checks != null) ToneChip(Formatters.checkedBadge(checks), Tone.Ok, icon = WitnessIcon.TrendingUp, height = 24.dp, fontSize = 11.sp)
            }
        }
    }
}

@Composable
fun CaptureTile(title: String, meta: String, state: WitnessState, modifier: Modifier = Modifier, onClick: (() -> Unit)? = null, image: @Composable BoxScope.() -> Unit) {
    val palette = Witness.colors
    Column(modifier.then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier).semantics(mergeDescendants = true) {}) {
        Box(Modifier.fillMaxWidth().height(112.dp).clip(RoundedCornerShape(14.dp)).background(palette.surface2)) {
            image()
            SealBadge(state, Modifier.align(Alignment.BottomEnd).padding(3.dp), radius = 9.dp, ring = palette.paper)
        }
        Spacer(Modifier.height(8.dp))
        Text(title, Modifier.padding(horizontal = 2.dp), style = Witness.type.tileTitle, color = palette.ink, maxLines = 1, overflow = TextOverflow.Ellipsis)
        Text(meta, Modifier.padding(horizontal = 2.dp), style = Witness.type.caption, color = palette.ink2, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}
