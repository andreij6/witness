package app.witness.feature.home

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import app.witness.core.designsystem.Witness
import app.witness.core.model.RecordId

fun interface ThumbnailSource {
    suspend fun load(recordId: RecordId, sizePx: Int): ImageBitmap?

    companion object {
        val None: ThumbnailSource = ThumbnailSource { _, _ -> null }
        const val MIN_BUCKET: Int = 64
        const val MAX_BUCKET: Int = 2048

        fun bucket(sizePx: Int): Int {
            var b = MIN_BUCKET
            while (b < sizePx && b < MAX_BUCKET) b = b shl 1
            return b
        }

        fun sampleSize(width: Int, height: Int, targetPx: Int): Int {
            if (width <= 0 || height <= 0 || targetPx <= 0) return 1
            val shortSide = minOf(width, height)
            var sample = 1
            while (shortSide / (sample shl 1) >= targetPx) sample = sample shl 1
            return sample
        }
    }
}

val LocalThumbnails = staticCompositionLocalOf { ThumbnailSource.None }

@Composable
fun RecordThumb(recordId: RecordId, size: Dp, modifier: Modifier = Modifier) {
    val source = LocalThumbnails.current
    val px = with(LocalDensity.current) { ThumbnailSource.bucket(size.roundToPx()) }
    val bitmap by produceState<ImageBitmap?>(null, recordId, px, source) { value = source.load(recordId, px) }
    Box(modifier.fillMaxSize().background(Witness.colors.surface2)) {
        bitmap?.let { Image(it, null, Modifier.fillMaxSize(), contentScale = ContentScale.Crop) }
    }
}
