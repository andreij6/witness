package app.witness.feature.home

import android.graphics.BitmapFactory
import android.util.LruCache
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import app.witness.core.domain.port.CaptureStore
import app.witness.core.model.RecordId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class CaptureThumbnails(private val captures: CaptureStore, maxBytes: Int = DEFAULT_CACHE_BYTES) : ThumbnailSource {
    private val cache = object : LruCache<String, ImageBitmap>(maxBytes) {
        override fun sizeOf(key: String, value: ImageBitmap): Int = value.width * value.height * BYTES_PER_PIXEL
    }

    override suspend fun load(recordId: RecordId, sizePx: Int): ImageBitmap? {
        val target = ThumbnailSource.bucket(sizePx)
        val key = recordId.text + '@' + target
        cache.get(key)?.let { return it }
        val jpeg = captures.readImage(recordId)?.toByteArray() ?: return null
        val bitmap = withContext(Dispatchers.Default) { decode(jpeg, target) } ?: return null
        cache.put(key, bitmap)
        return bitmap
    }

    companion object {
        const val DEFAULT_CACHE_BYTES: Int = 24 * 1024 * 1024
        private const val BYTES_PER_PIXEL = 4

        fun decode(jpeg: ByteArray, targetPx: Int): ImageBitmap? {
            val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size, bounds)
            if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
            val options = BitmapFactory.Options().apply { inSampleSize = ThumbnailSource.sampleSize(bounds.outWidth, bounds.outHeight, targetPx) }
            return BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size, options)?.asImageBitmap()
        }
    }
}
