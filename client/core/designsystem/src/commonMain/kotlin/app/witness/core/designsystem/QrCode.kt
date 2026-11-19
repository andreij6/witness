package app.witness.core.designsystem

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.qr.QrEcc
import app.witness.core.designsystem.qr.QrEncoder
import app.witness.core.designsystem.qr.QrMatrix

inline fun QrMatrix.forEachDarkRun(action: (x: Int, y: Int, length: Int) -> Unit) {
    for (y in 0 until size) {
        var x = 0
        while (x < size) {
            if (!get(x, y)) {
                x++
                continue
            }
            val start = x
            while (x < size && get(x, y)) x++
            action(start, y, x - start)
        }
    }
}

fun QrMatrix.toPath(cell: Float, origin: Float = 0f): Path {
    val path = Path()
    forEachDarkRun { x, y, length ->
        path.addRect(Rect(origin + x * cell, origin + y * cell, origin + (x + length) * cell, origin + (y + 1) * cell))
    }
    return path
}

@Composable
fun QrCode(
    data: String,
    modifier: Modifier = Modifier,
    size: Dp? = null,
    ecc: QrEcc = QrEcc.M,
    color: Color = WitnessColors.Ink,
    background: Color = WitnessColors.Surface,
    contentDescription: String = "QR code for $data",
) {
    val matrix = remember(data, ecc) { QrEncoder.encode(data, ecc) }
    QrCode(matrix, modifier, size, color, background, contentDescription)
}

@Composable
fun QrCode(matrix: QrMatrix, modifier: Modifier = Modifier, size: Dp? = null, color: Color = WitnessColors.Ink, background: Color = WitnessColors.Surface, contentDescription: String? = null) {
    Box(
        modifier
            .clip(WitnessShapes.Thumb)
            .background(background)
            .padding(8.dp)
            .then(if (size != null) Modifier.size(size) else Modifier.fillMaxWidth().aspectRatio(1f))
            .semantics {
                role = Role.Image
                if (contentDescription != null) this.contentDescription = contentDescription
            }
            .drawWithCache {
                val cell = this.size.minDimension / matrix.size
                val path = matrix.toPath(cell)
                onDrawBehind { drawPath(path, color) }
            },
    )
}
