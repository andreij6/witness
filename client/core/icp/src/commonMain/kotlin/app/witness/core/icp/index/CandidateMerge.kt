package app.witness.core.icp.index

import app.witness.core.icp.registry.ShardInfo

class Candidates(
    val exact: LongArray,
    val recordIx: LongArray,
    val distance: IntArray,
    val truncated: Boolean,
    val answered: Int,
    val total: Int,
    val indexedUpto: Long,
) {
    val size: Int get() = recordIx.size
    val isComplete: Boolean get() = answered == total
    val isEmpty: Boolean get() = exact.isEmpty() && recordIx.isEmpty()

    override fun equals(other: Any?): Boolean =
        this === other || other is Candidates &&
            exact.contentEquals(other.exact) &&
            recordIx.contentEquals(other.recordIx) &&
            distance.contentEquals(other.distance) &&
            truncated == other.truncated &&
            answered == other.answered &&
            total == other.total &&
            indexedUpto == other.indexedUpto

    override fun hashCode(): Int = (exact.contentHashCode() * 31 + recordIx.contentHashCode()) * 31 + answered

    override fun toString(): String =
        "Candidates(exact=${exact.toList()}, recordIx=${recordIx.toList()}, distance=${distance.toList()}, " +
            "truncated=$truncated, answered=$answered/$total, indexedUpto=$indexedUpto)"
}

object CandidateMerge {
    private const val IX_BITS = 53
    private const val IX_MASK = (1L shl IX_BITS) - 1

    const val MAX_EXACT: Int = 8
    const val MAX_WATERMARK: Int = 32
    const val MAX_PROBE_HITS: Int = 32
    const val MAX_CANDIDATES: Int = 64
    const val NO_DISTANCE: Int = 257
    const val MAX_RECORD_IX: Long = IX_MASK

    fun merge(shards: List<ShardInfo>, probes: List<ShardProbe>): Candidates {
        require(shards.size == probes.size) { "${probes.size} probes for ${shards.size} shards" }
        var truncated = false
        var answered = 0
        var indexedUpto = Long.MAX_VALUE
        var pdqTotal = 0
        var wmTotal = 0
        var exactTotal = 0
        for (p in probes) {
            if (!p.answered) continue
            answered++
            truncated = truncated or p.truncated
            if (p.indexedUpto < indexedUpto) indexedUpto = p.indexedUpto
            pdqTotal += p.pdqCount
            wmTotal += p.watermark.size
            exactTotal += p.exact.size
        }
        val pdq = LongArray(pdqTotal)
        val watermark = LongArray(wmTotal)
        val exact = LongArray(exactTotal)
        var np = 0
        var nw = 0
        var ne = 0
        probes.forEachIndexed { s, p ->
            if (!p.answered) return@forEachIndexed
            val base = shards[s].baseIx
            for (i in 0 until p.pdqCount) {
                val ix = base + p.localIxAt(i)
                if (ix in 0..MAX_RECORD_IX) pdq[np++] = pack(p.distance[i].coerceAtMost(NO_DISTANCE), ix)
            }
            for (ix in p.watermark) if (ix in 0..MAX_RECORD_IX) watermark[nw++] = ix
            for (ix in p.exact) if (ix in 0..MAX_RECORD_IX) exact[ne++] = ix
        }
        val exactIx = sortedUnique(exact, ne)
        if (exactIx.size > MAX_EXACT) truncated = true
        val wmIx = sortedUnique(watermark, nw)
        if (wmIx.size > MAX_WATERMARK) truncated = true
        val pdqBest = bestPerRecord(pdq, np)
        if (pdqBest.size > MAX_PROBE_HITS) truncated = true
        val keepPdq = minOf(pdqBest.size, MAX_PROBE_HITS)
        val keepWm = minOf(wmIx.size, MAX_WATERMARK)
        val combined = LongArray(keepPdq + keepWm)
        pdqBest.copyInto(combined, 0, 0, keepPdq)
        for (i in 0 until keepWm) combined[keepPdq + i] = pack(NO_DISTANCE, wmIx[i])
        val ranked = bestPerRecord(combined, combined.size)
        if (ranked.size > MAX_CANDIDATES) truncated = true
        val keep = minOf(ranked.size, MAX_CANDIDATES)
        return Candidates(
            exact = exactIx.copyOf(minOf(exactIx.size, MAX_EXACT)),
            recordIx = LongArray(keep) { ranked[it] and IX_MASK },
            distance = IntArray(keep) { (ranked[it] ushr IX_BITS).toInt() },
            truncated = truncated,
            answered = answered,
            total = shards.size,
            indexedUpto = if (answered == 0) 0 else indexedUpto,
        )
    }

    private fun pack(distance: Int, ix: Long): Long = (distance.toLong() shl IX_BITS) or ix

    private fun sortedUnique(values: LongArray, n: Int): LongArray {
        if (n == 0) return values.copyOf(0)
        values.sort(0, n)
        var w = 1
        for (r in 1 until n) if (values[r] != values[w - 1]) values[w++] = values[r]
        return values.copyOf(w)
    }

    private fun bestPerRecord(packed: LongArray, n: Int): LongArray {
        if (n == 0) return LongArray(0)
        val byIx = LongArray(n) { ((packed[it] and IX_MASK) shl 10) or (packed[it] ushr IX_BITS) }
        byIx.sort()
        var w = 0
        var last = -1L
        for (v in byIx) {
            val ix = v ushr 10
            if (ix != last) {
                byIx[w++] = pack((v and 0x3ff).toInt(), ix)
                last = ix
            }
        }
        val out = byIx.copyOf(w)
        out.sort()
        return out
    }
}
