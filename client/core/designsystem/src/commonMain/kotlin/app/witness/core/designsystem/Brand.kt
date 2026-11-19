package app.witness.core.designsystem

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Immutable
data class MarkGeometry(
    val stroke: Float,
    val radius: Float,
    val center: Offset,
    val shutterTopLeft: Offset,
    val shutterSize: Size,
    val check: List<Offset>,
) {
    companion object {
        const val CenterYFraction = 0.577f

        fun of(size: Float): MarkGeometry {
            val sw = size * 0.11f
            val r = size * 0.36f
            val cx = size / 2f
            val cy = size * CenterYFraction
            return MarkGeometry(
                stroke = sw,
                radius = r,
                center = Offset(cx, cy),
                shutterTopLeft = Offset(cx + r * 0.42f, cy - r - sw * 1.9f),
                shutterSize = Size(r * 0.62f, sw * 1.05f),
                check = listOf(Offset(cx - r * 0.42f, cy + r * 0.02f), Offset(cx - r * 0.1f, cy + r * 0.34f), Offset(cx + r * 0.46f, cy - r * 0.3f)),
            )
        }
    }
}

@Composable
fun WitnessMark(modifier: Modifier = Modifier, size: Dp = 24.dp, ring: Color = Witness.colors.ink, check: Color = Witness.colors.ok, contentDescription: String? = null) {
    Box(
        modifier
            .size(size)
            .then(if (contentDescription != null) Modifier.semantics { this.contentDescription = contentDescription; role = Role.Image } else Modifier)
            .drawWithCache {
                val g = MarkGeometry.of(this.size.minDimension)
                val path = Path().apply {
                    moveTo(g.check[0].x, g.check[0].y)
                    lineTo(g.check[1].x, g.check[1].y)
                    lineTo(g.check[2].x, g.check[2].y)
                }
                val ringStroke = Stroke(g.stroke)
                val checkStroke = Stroke(g.stroke, cap = StrokeCap.Round, join = StrokeJoin.Round)
                val corner = CornerRadius(g.stroke / 2f)
                onDrawBehind {
                    drawCircle(ring, g.radius, g.center, style = ringStroke)
                    drawRoundRect(ring, g.shutterTopLeft, g.shutterSize, corner)
                    drawPath(path, check, style = checkStroke)
                }
            },
    )
}

@Composable
fun Wordmark(modifier: Modifier = Modifier, size: Dp = 22.dp, color: Color = Witness.colors.ink, check: Color = Witness.colors.ok) {
    Row(modifier.semantics(mergeDescendants = true) {}, verticalAlignment = Alignment.CenterVertically) {
        WitnessMark(size = size * 1.15f, ring = color, check = check)
        Spacer(Modifier.width(size * 0.2f))
        Text("Witness", style = Witness.type.wordmark.merge(TextStyle(fontSize = size.value.sp)), color = color, maxLines = 1)
    }
}

@Composable
fun AppIcon(modifier: Modifier = Modifier, size: Dp = 96.dp) {
    Box(
        modifier.size(size).clip(RoundedCornerShape(size * 0.23f)).background(WitnessColors.Ink),
        contentAlignment = Alignment.Center,
    ) { WitnessMark(size = size * 0.8f, ring = WitnessColors.White, check = WitnessColors.IconCheck, contentDescription = "Witness") }
}

@Composable
fun SealBadge(state: WitnessState, modifier: Modifier = Modifier, radius: Dp = 11.dp, ring: Color = Witness.colors.surface) {
    Box(
        modifier
            .size((radius + 2.dp) * 2)
            .clip(CircleShape)
            .background(ring)
            .semantics { contentDescription = state.label },
        contentAlignment = Alignment.Center,
    ) {
        Box(Modifier.size(radius * 2).clip(CircleShape).background(sealColor(state)), contentAlignment = Alignment.Center) {
            WIcon(state.icon, WitnessColors.White, size = radius * 1.32f)
        }
    }
}

fun sealColor(state: WitnessState): Color = WitnessPalette.Light.tone(state.tone)
