package app.witness.core.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.graphics.ImageBitmap
import app.witness.core.model.ByteString
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

const val DISPLAY_PX: Int = 1280

fun sampleSize(width: Int, height: Int, maxDimension: Int): Int {
    if (width <= 0 || height <= 0 || maxDimension <= 0) return 1
    val longest = maxOf(width, height)
    var sample = 1
    while (longest / (sample * 2) >= maxDimension) sample *= 2
    return sample
}

expect fun decodeBitmap(bytes: ByteArray, maxDimension: Int): ImageBitmap?

@Composable
fun rememberBitmap(bytes: ByteString?, maxDimension: Int = DISPLAY_PX): ImageBitmap? {
    val bitmap by produceState<ImageBitmap?>(null, bytes, maxDimension) {
        value = bytes?.let {
            withContext(Dispatchers.Default) {
                try {
                    decodeBitmap(it.toByteArray(), maxDimension)
                } catch (e: CancellationException) {
                    throw e
                } catch (_: Exception) {
                    null
                }
            }
        }
    }
    return bitmap
}
