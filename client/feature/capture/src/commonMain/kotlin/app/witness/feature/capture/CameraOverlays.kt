package app.witness.feature.capture

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.solidChip
import app.witness.core.ui.mapToCropped

@Composable
internal fun TopScrim(modifier: Modifier = Modifier) {
    Box(
        modifier
            .fillMaxWidth()
            .height(CameraTokens.TopScrim)
            .background(Brush.verticalGradient(listOf(WitnessColors.Black.copy(alpha = CameraTokens.TopScrimAlpha), Color.Transparent))),
    )
}

@Composable
internal fun FrameGuides(modifier: Modifier = Modifier) {
    val color = WitnessColors.White.copy(alpha = CameraTokens.GuideAlpha)
    Spacer(
        modifier.drawWithCache {
            val inset = CameraTokens.GuideInset.toPx()
            val arm = CameraTokens.GuideArm.toPx()
            val stroke = Stroke(2.dp.toPx(), cap = StrokeCap.Round)
            val top = size.height * CameraTokens.GuideTop
            val bottom = size.height * CameraTokens.GuideBottom
            val right = size.width - inset
            val path = Path()
            fun corner(x: Float, y: Float, sx: Float, sy: Float) {
                path.moveTo(x, y + arm * sy)
                path.lineTo(x, y)
                path.lineTo(x + arm * sx, y)
            }
            corner(inset, top, 1f, 1f)
            corner(right, top, -1f, 1f)
            corner(inset, bottom, 1f, -1f)
            corner(right, bottom, -1f, -1f)
            onDrawBehind { drawPath(path, color, style = stroke) }
        },
    )
}

@Composable
internal fun DetectionOverlay(detections: List<Detection>, frameAspect: Float?, modifier: Modifier = Modifier) {
    if (detections.isEmpty()) return
    val palette = Witness.colors
    Layout(
        modifier = modifier,
        content = {
            Canvas(Modifier) {
                val stroke = Stroke(1.6.dp.toPx())
                val radius = CornerRadius(6.dp.toPx())
                for (d in detections) {
                    val r = mapToCropped(d.box, frameAspect, size)
                    drawRoundRect(CameraTokens.Detect, Offset(r.left, r.top), Size(r.width, r.height), radius, style = stroke)
                }
            }
            for (d in detections) {
                WitnessChip(d.label, palette.solidChip(CameraTokens.Detect, WitnessColors.Ink), height = 22.dp, fontSize = 10.5.sp)
            }
        },
    ) { measurables, constraints ->
        val frame = Size(constraints.maxWidth.toFloat(), constraints.maxHeight.toFloat())
        val canvas = measurables[0].measure(constraints.copy(minWidth = constraints.maxWidth, minHeight = constraints.maxHeight))
        val loose = constraints.copy(minWidth = 0, minHeight = 0)
        val chips = List(detections.size) { measurables[it + 1].measure(loose) }
        val gap = 6.dp.roundToPx()
        layout(constraints.maxWidth, constraints.maxHeight) {
            canvas.place(0, 0)
            chips.forEachIndexed { i, chip ->
                val r = mapToCropped(detections[i].box, frameAspect, frame)
                val x = (r.right.toInt() - chip.width).coerceIn(0, maxOf(0, constraints.maxWidth - chip.width))
                val y = (r.top.toInt() - gap - chip.height).coerceAtLeast(0)
                chip.place(x, y)
            }
        }
    }
}

@Composable
internal fun OfflinePill(modifier: Modifier = Modifier) {
    Row(
        modifier
            .height(30.dp)
            .clip(WitnessShapes.Pill)
            .background(WitnessColors.Black.copy(alpha = 0.5f))
            .background(Witness.colors.warn.copy(alpha = 0.4f))
            .border(1.dp, CameraTokens.Amber.copy(alpha = 0.5f), WitnessShapes.Pill)
            .padding(start = 10.dp, end = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        WIcon(WitnessIcon.CloudOff, CameraTokens.Amber, size = 16.dp)
        Spacer(Modifier.width(5.dp))
        Text(CameraCopy.OFFLINE, style = Witness.type.chip, color = WitnessColors.White, maxLines = 1)
    }
}

private val CardShape = RoundedCornerShape(20.dp)

@Composable
private fun OverlayCard(border: Color, alpha: Float, modifier: Modifier = Modifier, onClick: (() -> Unit)? = null, content: @Composable BoxScope.() -> Unit) {
    Box(
        modifier
            .fillMaxWidth()
            .height(72.dp)
            .clip(CardShape)
            .background(CameraTokens.Card.copy(alpha = alpha))
            .border(1.2.dp, border, CardShape)
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier),
        contentAlignment = Alignment.CenterStart,
    ) { content() }
}

@Composable
internal fun HintCard(hint: CameraHint, modifier: Modifier = Modifier) {
    val ai = Witness.colors.ai
    OverlayCard(ai.copy(alpha = 0.6f), CameraTokens.HintAlpha, modifier) {
        Box(Modifier.padding(top = 18.dp).size(3.dp, 36.dp).align(Alignment.TopStart).background(ai))
        Row(Modifier.padding(start = 16.dp, end = 16.dp), verticalAlignment = Alignment.Top) {
            WIcon(WitnessIcon.AutoAwesome, CameraTokens.AiLight, Modifier.padding(top = 2.dp), size = 18.dp)
            Spacer(Modifier.width(8.dp))
            Column {
                Text(hint.title, style = Witness.type.bodyStrong, color = WitnessColors.White, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Spacer(Modifier.height(3.dp))
                Text(hint.subtitle, style = Witness.type.caption, color = CameraTokens.CardSubtitle, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

@Composable
internal fun SealedToastCard(toast: SealedToast, onOpen: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    val statusColor = if (toast.tone == Tone.Warn) CameraTokens.Amber else if (toast.tone == Tone.Ok) WitnessColors.ProvenDark else CameraTokens.CardSubtitle
    OverlayCard(palette.ok.copy(alpha = 0.5f), CameraTokens.ToastAlpha, modifier.semantics { contentDescription = "${toast.title}, ${toast.status}" }, onOpen) {
        Row(Modifier.padding(start = 12.dp, end = 14.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(48.dp)) {
                PhotoImage(toast.photo, null, Modifier.size(48.dp).clip(WitnessShapes.Thumb))
                Box(
                    Modifier.offset(35.dp, (-9).dp).size(22.dp).clip(CircleShape).background(CameraTokens.Card),
                    contentAlignment = Alignment.Center,
                ) {
                    Box(Modifier.size(18.dp).clip(CircleShape).background(WitnessColors.Ok), contentAlignment = Alignment.Center) {
                        WIcon(WitnessIcon.Lock, WitnessColors.White, size = 12.dp)
                    }
                }
            }
            Spacer(Modifier.width(16.dp))
            Column(Modifier.weight(1f)) {
                Text(toast.title, style = Witness.type.bodyStrong, color = WitnessColors.White, maxLines = 1)
                Spacer(Modifier.height(4.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    WIcon(toast.icon, statusColor, size = 14.dp)
                    Spacer(Modifier.width(4.dp))
                    Text(toast.status, style = Witness.type.caption, color = statusColor, maxLines = 1)
                }
            }
            WIcon(WitnessIcon.ChevronRight, WitnessColors.CameraInk2, size = 20.dp)
        }
    }
}

@Composable
internal fun ErrorCard(message: String, onDismiss: () -> Unit, modifier: Modifier = Modifier) {
    OverlayCard(Witness.colors.crit.copy(alpha = 0.6f), CameraTokens.ToastAlpha, modifier, onDismiss) {
        Row(Modifier.padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) {
            WIcon(WitnessIcon.Error, Witness.colors.crit, size = 20.dp)
            Spacer(Modifier.width(10.dp))
            Text(message, Modifier.weight(1f), style = Witness.type.bodyStrong, color = WitnessColors.White, maxLines = 2)
            WIcon(WitnessIcon.Close, WitnessColors.CameraInk2, size = 18.dp)
        }
    }
}
