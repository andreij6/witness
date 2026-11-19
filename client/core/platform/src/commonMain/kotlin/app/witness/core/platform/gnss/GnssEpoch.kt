package app.witness.core.platform.gnss

class SatelliteSample(
    val svid: Int,
    val constellation: Int,
    val cn0DbHz: Double,
    val pseudorangeRateMps: Double,
    val agcDb: Double? = null,
) {
    val key: Int get() = constellation * SVID_SPACE + svid

    private companion object {
        const val SVID_SPACE = 1024
    }
}

class GnssEpoch(
    val elapsedNs: Long,
    val timeNanos: Long,
    val fullBiasNanos: Long?,
    val biasNanos: Double,
    val driftNsPerS: Double?,
    val discontinuityCount: Int,
    val satellites: List<SatelliteSample>,
) {
    fun gpsTimeNanos(): Long? = fullBiasNanos?.let { timeNanos - it - biasNanos.toLong() }
}

class GnssRingBuffer(private val windowNs: Long = WINDOW_NS, capacity: Int = DEFAULT_CAPACITY) {
    private val slots = arrayOfNulls<GnssEpoch>(capacity)
    private var head = 0
    var size = 0
        private set

    init {
        require(capacity > 0) { "capacity must be positive" }
        require(windowNs > 0) { "window must be positive" }
    }

    fun add(epoch: GnssEpoch) {
        if (size > 0 && epoch.elapsedNs < this[size - 1].elapsedNs) clear()
        val index = (head + size) % slots.size
        slots[index] = epoch
        if (size < slots.size) size++ else head = (head + 1) % slots.size
        evictOlderThan(epoch.elapsedNs - windowNs)
    }

    fun snapshot(nowNs: Long): List<GnssEpoch> {
        evictOlderThan(nowNs - windowNs)
        return List(size) { this[it] }
    }

    fun clear() {
        slots.fill(null)
        head = 0
        size = 0
    }

    operator fun get(i: Int): GnssEpoch = slots[(head + i) % slots.size]!!

    private fun evictOlderThan(cutoffNs: Long) {
        while (size > 0 && slots[head]!!.elapsedNs < cutoffNs) {
            slots[head] = null
            head = (head + 1) % slots.size
            size--
        }
    }

    companion object {
        const val WINDOW_NS = 10_000_000_000L
        const val DEFAULT_CAPACITY = 32
    }
}
