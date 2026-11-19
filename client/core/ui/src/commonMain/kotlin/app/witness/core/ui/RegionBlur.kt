package app.witness.core.ui

import app.witness.core.model.Rect
import kotlin.math.ceil
import kotlin.math.roundToInt
import kotlin.math.sqrt

object RegionBlur {
    const val PASSES: Int = 3

    fun boxRadii(sigmaE2: Int, passes: Int = PASSES): IntArray {
        require(passes > 0) { "passes must be positive" }
        if (sigmaE2 <= 0) return IntArray(passes)
        val variance = (sigmaE2 / 100.0) * (sigmaE2 / 100.0)
        var lower = sqrt(12.0 * variance / passes + 1.0).toInt()
        if (lower % 2 == 0) lower--
        val upper = lower + 2
        val m = ((12.0 * variance - passes * lower * lower - 4.0 * passes * lower - 3.0 * passes) / (-4.0 * lower - 4.0)).roundToInt().coerceIn(0, passes)
        return IntArray(passes) { (if (it < m) lower else upper) / 2 }
    }

    fun scaledSigma(sigmaE2: Int, scale: Float): Int = if (scale <= 0f) 0 else (sigmaE2 * scale).roundToInt()

    fun blur(pixels: IntArray, width: Int, height: Int, sigmaE2: Int, scratch: IntArray? = null) {
        require(width >= 0 && height >= 0) { "negative size" }
        val n = width * height
        require(pixels.size >= n && (scratch == null || scratch.size >= n)) { "buffer too small" }
        if (n == 0) return
        val radii = boxRadii(sigmaE2)
        if (radii.all { it == 0 }) return
        val tmp = scratch ?: IntArray(n)
        for (r in radii) {
            if (r == 0) continue
            boxPass(pixels, tmp, height, width, width, 1, r)
            boxPass(tmp, pixels, width, height, 1, width, r)
        }
    }

    private fun boxPass(src: IntArray, dst: IntArray, lines: Int, len: Int, lineStep: Int, step: Int, r: Int) {
        val div = 2 * r + 1
        val half = div / 2
        val last = len - 1
        for (line in 0 until lines) {
            val base = line * lineStep
            var a = 0
            var red = 0
            var g = 0
            var b = 0
            for (k in -r - 1 until r) {
                val p = src[base + k.coerceIn(0, last) * step]
                a += p ushr 24
                red += (p shr 16) and 0xFF
                g += (p shr 8) and 0xFF
                b += p and 0xFF
            }
            for (x in 0..last) {
                val pin = src[base + minOf(x + r, last) * step]
                val pout = src[base + maxOf(x - r - 1, 0) * step]
                a += (pin ushr 24) - (pout ushr 24)
                red += ((pin shr 16) and 0xFF) - ((pout shr 16) and 0xFF)
                g += ((pin shr 8) and 0xFF) - ((pout shr 8) and 0xFF)
                b += (pin and 0xFF) - (pout and 0xFF)
                dst[base + x * step] = ((a + half) / div shl 24) or ((red + half) / div shl 16) or ((g + half) / div shl 8) or ((b + half) / div)
            }
        }
    }
}

fun Rect.scaled(scale: Float): Rect {
    if (scale == 1f) return this
    val l = (x * scale).toInt()
    val t = (y * scale).toInt()
    val r = ceil(right * scale).toInt()
    val b = ceil(bottom * scale).toInt()
    return Rect(l, t, maxOf(1, r - l), maxOf(1, b - t))
}
