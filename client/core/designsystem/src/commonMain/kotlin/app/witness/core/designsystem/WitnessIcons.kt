package app.witness.core.designsystem

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.paint
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.graphics.vector.group
import androidx.compose.ui.graphics.vector.rememberVectorPainter
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

internal const val IconViewport = 960f

private val vectorCache = arrayOfNulls<ImageVector>(WitnessIcon.entries.size)

val WitnessIcon.imageVector: ImageVector
    get() = vectorCache[ordinal] ?: buildVector().also { vectorCache[ordinal] = it }

private fun WitnessIcon.buildVector(): ImageVector =
    ImageVector.Builder(symbolName, 24.dp, 24.dp, IconViewport, IconViewport, autoMirror = false)
        .group(translationY = IconViewport) {
            addPath(pathData = addPathNodes(pathData), fill = SolidColor(Color.Black))
        }
        .build()

@Composable
fun WIcon(icon: WitnessIcon, tint: Color, modifier: Modifier = Modifier, size: Dp = 24.dp, contentDescription: String? = null) {
    val painter = rememberVectorPainter(icon.imageVector)
    val described = if (contentDescription == null) modifier else modifier.semantics {
        this.contentDescription = contentDescription
        role = Role.Image
    }
    Box(described.size(size).paint(painter, colorFilter = ColorFilter.tint(tint)))
}
