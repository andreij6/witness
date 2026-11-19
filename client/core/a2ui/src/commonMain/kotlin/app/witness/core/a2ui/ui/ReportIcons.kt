package app.witness.core.a2ui.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp

internal enum class ReportIcon { Sparkle, Flag, CheckCircle, Add }

@Composable
internal fun ReportIconView(icon: ReportIcon, size: Dp, color: Color, modifier: Modifier = Modifier) {
    Canvas(modifier.size(size)) {
        when (icon) {
            ReportIcon.Sparkle -> sparkle(color)
            ReportIcon.Flag -> flag(color)
            ReportIcon.CheckCircle -> checkCircle(color)
            ReportIcon.Add -> add(color)
        }
    }
}

private fun DrawScope.star(cx: Float, cy: Float, r: Float, color: Color) {
    val path = Path().apply {
        moveTo(cx, cy - r)
        quadraticTo(cx, cy, cx + r, cy)
        quadraticTo(cx, cy, cx, cy + r)
        quadraticTo(cx, cy, cx - r, cy)
        quadraticTo(cx, cy, cx, cy - r)
        close()
    }
    drawPath(path, color)
}

private fun DrawScope.sparkle(color: Color) {
    val s = size.minDimension
    star(s * 0.40f, s * 0.58f, s * 0.36f, color)
    star(s * 0.78f, s * 0.20f, s * 0.17f, color)
    star(s * 0.80f, s * 0.84f, s * 0.13f, color)
}

private fun DrawScope.flag(color: Color) {
    val s = size.minDimension
    val stroke = Stroke(width = s * 0.09f, cap = StrokeCap.Round, join = StrokeJoin.Round)
    drawLine(color, Offset(s * 0.22f, s * 0.12f), Offset(s * 0.22f, s * 0.90f), s * 0.09f, StrokeCap.Round)
    val pennant = Path().apply {
        moveTo(s * 0.22f, s * 0.16f)
        lineTo(s * 0.56f, s * 0.16f)
        lineTo(s * 0.60f, s * 0.26f)
        lineTo(s * 0.84f, s * 0.26f)
        lineTo(s * 0.84f, s * 0.64f)
        lineTo(s * 0.58f, s * 0.64f)
        lineTo(s * 0.54f, s * 0.55f)
        lineTo(s * 0.22f, s * 0.55f)
    }
    drawPath(pennant, color, style = stroke)
}

private fun DrawScope.checkCircle(color: Color) {
    val s = size.minDimension
    val width = s * 0.09f
    drawCircle(color, radius = s * 0.42f, style = Stroke(width))
    val tick = Path().apply {
        moveTo(s * 0.30f, s * 0.52f)
        lineTo(s * 0.44f, s * 0.65f)
        lineTo(s * 0.71f, s * 0.38f)
    }
    drawPath(tick, color, style = Stroke(width, cap = StrokeCap.Round, join = StrokeJoin.Round))
}

private fun DrawScope.add(color: Color) {
    val s = size.minDimension
    val width = s * 0.09f
    drawLine(color, Offset(s * 0.5f, s * 0.2f), Offset(s * 0.5f, s * 0.8f), width, StrokeCap.Round)
    drawLine(color, Offset(s * 0.2f, s * 0.5f), Offset(s * 0.8f, s * 0.5f), width, StrokeCap.Round)
}
