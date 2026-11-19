package app.witness.feature.capture

import app.witness.core.model.BlurProposal
import app.witness.core.model.Dims

class FrameThrottle(private val minIntervalMs: Long) {
    private var last: Long = 0
    private var started: Boolean = false

    init {
        require(minIntervalMs >= 0) { "interval must not be negative" }
    }

    fun tryAcquire(nowMs: Long): Boolean {
        if (started && nowMs - last < minIntervalMs) return false
        started = true
        last = nowMs
        return true
    }

    fun reset() {
        started = false
    }
}

object LiveFrames {
    const val MAX_FPS: Int = 5
    const val MIN_INTERVAL_MS: Long = 1_000L / MAX_FPS

    fun upright(width: Int, height: Int, rotationDegrees: Int): Dims =
        if (((rotationDegrees % 360) + 360) % 180 == 90) Dims(height, width) else Dims(width, height)

    fun detections(dims: Dims, proposals: List<BlurProposal>, mirrored: Boolean): FrameDetections {
        if (!dims.isValid || proposals.isEmpty()) return FrameDetections(dims, emptyList())
        val out = ArrayList<BlurProposal>(minOf(proposals.size, CameraCopy.MAX_DETECTIONS))
        for (p in proposals) {
            if (out.size == CameraCopy.MAX_DETECTIONS) break
            val r = p.rect.clippedTo(dims.width, dims.height) ?: continue
            out += p.copy(rect = if (mirrored) r.copy(x = dims.width - r.right) else r)
        }
        return FrameDetections(dims, out)
    }
}
