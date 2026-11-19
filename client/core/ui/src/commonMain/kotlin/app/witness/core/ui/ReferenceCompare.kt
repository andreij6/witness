package app.witness.core.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.solidChip
import kotlin.math.max

@Immutable
data class NormRect(val left: Float, val top: Float, val right: Float, val bottom: Float) {
    init {
        require(left in 0f..1f && right in 0f..1f && top in 0f..1f && bottom in 0f..1f) { "NormRect must lie within the unit square: $this" }
        require(left <= right && top <= bottom) { "NormRect edges are inverted: $this" }
    }

    val width: Float get() = right - left
    val height: Float get() = bottom - top
    val area: Float get() = width * height

    companion object {
        val Full = NormRect(0f, 0f, 1f, 1f)

        fun centeredCrop(sourceAspect: Float, targetAspect: Float): NormRect {
            require(sourceAspect > 0f && targetAspect > 0f) { "aspect ratios must be positive" }
            return if (targetAspect >= sourceAspect) {
                val h = sourceAspect / targetAspect
                NormRect(0f, (1f - h) / 2f, 1f, (1f + h) / 2f)
            } else {
                val w = targetAspect / sourceAspect
                NormRect((1f - w) / 2f, 0f, (1f + w) / 2f, 1f)
            }
        }
    }
}

fun mapToCropped(rect: NormRect, sourceAspect: Float?, frame: Size): Rect {
    if (sourceAspect == null || frame.width <= 0f || frame.height <= 0f) {
        return Rect(rect.left * frame.width, rect.top * frame.height, rect.right * frame.width, rect.bottom * frame.height)
    }
    val k = max(frame.width / sourceAspect, frame.height)
    val w = sourceAspect * k
    val h = k
    val dx = (w - frame.width) / 2f
    val dy = (h - frame.height) / 2f
    return Rect(rect.left * w - dx, rect.top * h - dy, rect.right * w - dx, rect.bottom * h - dy)
}

@Immutable
data class ComparePane(
    val label: String,
    val caption: String? = null,
    val captionTone: Tone? = null,
    val dot: Tone? = null,
    val sourceAspect: Float? = null,
    val crop: NormRect? = null,
    val cropLabel: String = "Crop",
    val changedRegions: List<NormRect> = emptyList(),
)

@Composable
fun ReferenceCompare(
    start: ComparePane,
    end: ComparePane,
    startContent: @Composable BoxScope.() -> Unit,
    endContent: @Composable BoxScope.() -> Unit,
    modifier: Modifier = Modifier,
    title: String? = null,
    paneAspect: Float = 3f / 4f,
    onPaneClick: ((Int) -> Unit)? = null,
) {
    WitnessCard(modifier.fillMaxWidth()) {
        if (title != null) {
            Overline(title)
            Spacer(Modifier.height(12.dp))
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            listOf(start to startContent, end to endContent).forEachIndexed { index, (pane, content) ->
                Column(Modifier.weight(1f)) {
                    if (title == null) {
                        Overline(pane.label)
                        Spacer(Modifier.height(12.dp))
                    }
                    PaneFrame(pane, paneAspect, onPaneClick?.let { { it(index) } }, content)
                    PaneCaption(pane)
                }
            }
        }
    }
}

@Composable
private fun PaneFrame(pane: ComparePane, aspect: Float, onClick: (() -> Unit)?, content: @Composable BoxScope.() -> Unit) {
    val palette = Witness.colors
    val edit = palette.edit
    val crit = palette.crit
    BoxWithConstraints(
        Modifier
            .fillMaxWidth()
            .aspectRatio(aspect)
            .clip(WitnessShapes.Image)
            .background(palette.surface2)
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier)
            .semantics { contentDescription = pane.label },
    ) {
        Box(Modifier.fillMaxSize(), content = content)
        Box(
            Modifier.fillMaxSize().drawWithCache {
                val crop = pane.crop?.let { mapToCropped(it, pane.sourceAspect, size) }
                val regions = pane.changedRegions.map { mapToCropped(it, pane.sourceAspect, size) }
                val dashed = Stroke(2.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(6.dp.toPx(), 4.dp.toPx())))
                val solid = Stroke(2.dp.toPx())
                val pad = 6.dp.toPx()
                onDrawBehind {
                    crop?.let { drawRoundRect(edit, it.topLeft, it.size, CornerRadius(2.dp.toPx()), style = dashed) }
                    regions.forEach {
                        drawRoundRect(crit, Offset(it.left - pad, it.top - pad), Size(it.width + pad * 2, it.height + pad * 2), CornerRadius(4.dp.toPx()), style = solid)
                    }
                }
            },
        )
        val crop = pane.crop
        if (crop != null) {
            val r = mapToCropped(crop, pane.sourceAspect, Size(maxWidth.value, maxHeight.value))
            WitnessChip(
                pane.cropLabel,
                palette.solidChip(palette.edit),
                Modifier.offset(x = r.left.dp + 6.dp, y = r.bottom.dp - 28.dp),
                icon = WitnessIcon.Compare,
                height = 22.dp,
                fontSize = 10.5.sp,
            )
        }
    }
}

@Composable
private fun PaneCaption(pane: ComparePane) {
    val palette = Witness.colors
    if (pane.caption == null) return
    Row(Modifier.padding(top = 8.dp, start = if (pane.dot != null) 8.dp else 0.dp), verticalAlignment = Alignment.CenterVertically) {
        if (pane.dot != null) {
            Box(Modifier.size(8.dp).clip(CircleShape).background(palette.tone(pane.dot)))
            Spacer(Modifier.width(8.dp))
        }
        Text(
            pane.caption,
            style = if (pane.dot != null) Witness.type.labelStrong else Witness.type.label,
            color = pane.captionTone?.let(palette::tone) ?: if (pane.dot != null) palette.ink else palette.ink2,
        )
    }
}
