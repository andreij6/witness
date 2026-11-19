package app.witness.feature.capture

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.layout.ContentScale
import app.witness.core.designsystem.WitnessColors
import app.witness.core.ui.decodeBitmap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class Photo(val bytes: ByteArray)

@Composable
fun PhotoImage(photo: Photo?, contentDescription: String?, modifier: Modifier = Modifier, maxDimension: Int = THUMB_PX) {
    val bitmap by produceState<ImageBitmap?>(null, photo, maxDimension) {
        value = photo?.let { withContext(Dispatchers.Default) { runCatching { decodeBitmap(it.bytes, maxDimension) }.getOrNull() } }
    }
    val image = bitmap
    if (image == null) Box(modifier.background(WitnessColors.CameraSurface))
    else Image(image, contentDescription, modifier, contentScale = ContentScale.Crop)
}

const val THUMB_PX: Int = 192
