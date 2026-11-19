package app.witness.core.platform.blur

import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.Rect

object BlurGeometry {
    const val TARGET_EDGE = 1280
    const val PADDING_PERCENT = 10
    const val MAX_PROPOSALS = 64

    fun sampleSize(width: Int, height: Int, targetEdge: Int = TARGET_EDGE): Int {
        var sample = 1
        val edge = maxOf(width, height)
        while (edge / (sample * 2) >= targetEdge) sample *= 2
        return sample
    }

    fun toOriginal(left: Int, top: Int, right: Int, bottom: Int, scale: Int, width: Int, height: Int): Rect? {
        val w = right - left
        val h = bottom - top
        if (w <= 0 || h <= 0) return null
        val padX = w * PADDING_PERCENT / 100
        val padY = h * PADDING_PERCENT / 100
        return Rect((left - padX) * scale, (top - padY) * scale, (w + 2 * padX) * scale, (h + 2 * padY) * scale).clippedTo(width, height)
    }

    fun dedupe(proposals: List<BlurProposal>): List<BlurProposal> {
        val sorted = proposals.sortedByDescending { it.rect.area }
        val out = ArrayList<BlurProposal>(minOf(sorted.size, MAX_PROPOSALS))
        for (p in sorted) {
            if (out.size == MAX_PROPOSALS) break
            if (out.none { it.blurClass == p.blurClass && contains(it.rect, p.rect) }) out += p
        }
        return out
    }

    fun contains(outer: Rect, inner: Rect): Boolean =
        inner.x >= outer.x && inner.y >= outer.y && inner.right <= outer.right && inner.bottom <= outer.bottom

    fun proposal(rect: Rect, blurClass: BlurClass, confidence: Float = 1f) = BlurProposal(rect, blurClass, true, confidence.coerceIn(0f, 1f))
}
