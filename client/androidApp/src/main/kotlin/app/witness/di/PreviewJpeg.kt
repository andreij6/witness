package app.witness.di

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import java.io.ByteArrayOutputStream
import kotlin.math.max
import kotlin.math.roundToInt

object PreviewJpeg {
    const val MAX_SIDE: Int = 1080
    const val MAX_BYTES: Int = 150 * 1024
    const val FIRST_QUALITY: Int = 85
    const val MIN_QUALITY: Int = 40
    const val QUALITY_STEP: Int = 10

    fun encode(jpeg: ByteArray): ByteArray {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size, bounds)
        require(bounds.outWidth > 0 && bounds.outHeight > 0) { "post image is not decodable" }
        val decoded = BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size, BitmapFactory.Options().apply { inSampleSize = sampleSize(bounds.outWidth, bounds.outHeight) })
            ?: throw IllegalArgumentException("post image is not decodable")
        var side = MAX_SIDE
        while (true) {
            val scaled = scaled(decoded, side)
            val out = smallest(scaled)
            if (scaled !== decoded) scaled.recycle()
            if (out.size <= MAX_BYTES || side <= MIN_SIDE) {
                decoded.recycle()
                return out
            }
            side /= 2
        }
    }

    fun sampleSize(w: Int, h: Int): Int = app.witness.core.ui.sampleSize(w, h, MAX_SIDE)

    fun fit(w: Int, h: Int, side: Int): Pair<Int, Int> {
        val longest = max(w, h)
        if (longest <= side) return w to h
        val f = side.toDouble() / longest
        return (w * f).roundToInt().coerceAtLeast(1) to (h * f).roundToInt().coerceAtLeast(1)
    }

    private fun scaled(b: Bitmap, side: Int): Bitmap {
        val (w, h) = fit(b.width, b.height, side)
        return if (w == b.width && h == b.height) b else Bitmap.createScaledBitmap(b, w, h, true)
    }

    private fun smallest(b: Bitmap): ByteArray {
        var q = FIRST_QUALITY
        while (true) {
            val out = ByteArrayOutputStream()
            b.compress(Bitmap.CompressFormat.JPEG, q, out)
            if (out.size() <= MAX_BYTES || q <= MIN_QUALITY) return out.toByteArray()
            q -= QUALITY_STEP
        }
    }

    private const val MIN_SIDE = 135
}
