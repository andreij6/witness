package app.witness.feature.post

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.layout.ContentScale
import app.witness.core.designsystem.WitnessColors
import app.witness.core.model.Edit
import app.witness.core.ui.decodeBitmap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class Photo(val bytes: ByteArray)

internal expect fun blurPhoto(image: ImageBitmap, blurs: List<Edit.Blur>, scale: Float): ImageBitmap

@Composable
fun PhotoImage(
    photo: Photo?,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    maxDimension: Int = PREVIEW_PX,
    contentScale: ContentScale = ContentScale.Crop,
    colorFilter: ColorFilter? = null,
    placeholder: Color = WitnessColors.Surface2,
    blurs: List<Edit.Blur> = emptyList(),
    sourceWidth: Int = 0,
) {
    val decoded by produceState<ImageBitmap?>(null, photo, maxDimension) {
        value = photo?.let { withContext(Dispatchers.Default) { runCatching { decodeBitmap(it.bytes, maxDimension) }.getOrNull() } }
    }
    val bitmap by produceState<ImageBitmap?>(null, decoded, blurs, sourceWidth) {
        val base = decoded
        value = if (base == null || blurs.isEmpty()) base
        else withContext(Dispatchers.Default) { runCatching { blurPhoto(base, blurs, previewScale(base.width, sourceWidth)) }.getOrNull() }
    }
    val image = bitmap
    if (image == null) Box(modifier.background(placeholder))
    else Image(image, contentDescription, modifier, contentScale = contentScale, colorFilter = colorFilter)
}

const val PREVIEW_PX: Int = 1280
