package app.witness.core.platform.keys

import app.witness.core.model.DeviceKey

object StrongBoxPolicy {
    const val BUDGET_P90_NS = 120_000_000L
    const val SAMPLES = 5

    fun p90(samplesNs: LongArray): Long {
        require(samplesNs.isNotEmpty()) { "no samples" }
        val sorted = samplesNs.copyOf().also { it.sort() }
        val rank = (sorted.size * 9 + 9) / 10
        return sorted[(rank - 1).coerceIn(0, sorted.size - 1)]
    }

    fun withinBudget(samplesNs: LongArray, budgetNs: Long = BUDGET_P90_NS): Boolean = p90(samplesNs) <= budgetNs

    fun active(keys: Collection<DeviceKey>): DeviceKey? {
        var best: DeviceKey? = null
        for (k in keys) if (!k.retired && (best == null || k.createdAtMs > best.createdAtMs)) best = k
        return best
    }

    fun needsRotation(key: DeviceKey, nowMs: Long, rotateAfterMs: Long = ROTATE_AFTER_MS): Boolean =
        key.retired || nowMs - (key.enrolledAtMs ?: key.createdAtMs) >= rotateAfterMs

    const val ROTATE_AFTER_MS = 30L * 24 * 60 * 60 * 1000
}
