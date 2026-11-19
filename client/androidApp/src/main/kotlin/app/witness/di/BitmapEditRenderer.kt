package app.witness.di

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import app.witness.core.model.ByteString
import app.witness.core.model.Edit
import app.witness.core.ui.blurRegion
import app.witness.nativelib.EditRenderer
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream

object BitmapEditRenderer : EditRenderer {
    const val QUALITY: Int = 95

    override suspend fun render(jpeg: ByteString, edits: List<Edit>): ByteString {
        if (edits.none(::isRendered)) return jpeg
        return withContext(Dispatchers.Default) {
            val bytes = jpeg.toByteArray()
            var bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply { inMutable = true })
                ?: throw IllegalArgumentException("parent image is not decodable")
            for (e in edits) bitmap = apply(bitmap, e)
            val out = ByteArrayOutputStream(bytes.size)
            bitmap.compress(Bitmap.CompressFormat.JPEG, QUALITY, out)
            bitmap.recycle()
            ByteString.wrap(out.toByteArray())
        }
    }

    fun isRendered(e: Edit): Boolean = e is Edit.Crop || e is Edit.Blur || e is Edit.Resize

    private fun apply(bitmap: Bitmap, e: Edit): Bitmap = when (e) {
        is Edit.Crop -> e.rect.clippedTo(bitmap.width, bitmap.height)?.let { replace(bitmap, Bitmap.createBitmap(bitmap, it.x, it.y, it.w, it.h)) } ?: bitmap
        is Edit.Resize -> replace(bitmap, Bitmap.createScaledBitmap(bitmap, e.w, e.h, true))
        is Edit.Blur -> mutable(bitmap).also { it.blurRegion(e.rect, e.sigmaE2) }
        else -> bitmap
    }

    private fun mutable(bitmap: Bitmap): Bitmap = if (bitmap.isMutable) bitmap else replace(bitmap, bitmap.copy(Bitmap.Config.ARGB_8888, true))

    private fun replace(old: Bitmap, new: Bitmap): Bitmap {
        if (new !== old) old.recycle()
        return new
    }
}
