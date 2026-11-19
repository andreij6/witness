package app.witness.core.icp.index

import app.witness.core.icp.IcpException

object ProbeWire {
    const val SHARD_OK: Int = 0
    const val SHARD_FAILED: Int = 1
    const val HEADER: Int = 7

    fun u64(high: Int, low: Int): Long = (high.toLong() shl 32) or (low.toLong() and 0xffff_ffffL)

    fun decode(wire: IntArray): List<ShardProbe> {
        val r = Reader(wire)
        val count = r.count()
        val out = ArrayList<ShardProbe>(count)
        repeat(count) { out.add(r.shard()) }
        if (r.at != wire.size) throw IcpException("probe wire has ${wire.size - r.at} trailing ints")
        return out
    }

    fun encodedSize(probes: List<ShardProbe>): Int =
        1 + probes.sumOf { HEADER + if (it.answered) it.pdqCount * 3 + (it.watermark.size + it.exact.size) * 2 else 0 }

    fun encode(probes: List<ShardProbe>): IntArray {
        val out = IntArray(encodedSize(probes))
        out[0] = probes.size
        var o = 1
        for (p in probes) {
            if (!p.answered) {
                out[o] = SHARD_FAILED
                o += HEADER
                continue
            }
            out[o++] = SHARD_OK
            out[o++] = if (p.truncated) 1 else 0
            o = putU64(out, o, p.indexedUpto)
            out[o++] = p.pdqCount
            out[o++] = p.watermark.size
            out[o++] = p.exact.size
            for (i in 0 until p.pdqCount) {
                out[o++] = p.localIx[i]
                out[o++] = p.shape[i]
                out[o++] = p.distance[i]
            }
            for (v in p.watermark) o = putU64(out, o, v)
            for (v in p.exact) o = putU64(out, o, v)
        }
        return out
    }

    private fun putU64(out: IntArray, at: Int, v: Long): Int {
        out[at] = (v ushr 32).toInt()
        out[at + 1] = v.toInt()
        return at + 2
    }

    private class Reader(val wire: IntArray) {
        var at = 0

        fun take(): Int {
            if (at >= wire.size) throw IcpException("probe wire truncated at $at")
            return wire[at++]
        }

        fun count(): Int {
            val n = take()
            if (n < 0 || n > wire.size - at) throw IcpException("probe wire count $n out of range at ${at - 1}")
            return n
        }

        fun u64(): Long {
            val v = u64(take(), take())
            if (v < 0) throw IcpException("probe wire u64 exceeds 63 bits at ${at - 2}")
            return v
        }

        fun shard(): ShardProbe {
            val status = take()
            if (status == SHARD_FAILED) {
                skipHeader()
                return ShardProbe.FAILED
            }
            if (status != SHARD_OK) throw IcpException("probe wire status $status at ${at - 1}")
            val truncated = take() != 0
            val indexedUpto = u64()
            val nPdq = count()
            val nWm = count()
            val nExact = count()
            val need = nPdq.toLong() * 3 + (nWm.toLong() + nExact) * 2
            if (need > wire.size - at) throw IcpException("probe wire body of $need ints exceeds remaining ${wire.size - at}")
            val localIx = IntArray(nPdq)
            val shape = IntArray(nPdq)
            val distance = IntArray(nPdq)
            for (i in 0 until nPdq) {
                localIx[i] = take()
                shape[i] = take()
                distance[i] = take()
                if (shape[i] !in 0..255 || distance[i] !in 0..0xffff) throw IcpException("probe wire hit $i out of range")
            }
            val watermark = LongArray(nWm) { u64() }
            val exact = LongArray(nExact) { u64() }
            return ShardProbe(true, truncated, indexedUpto, localIx, shape, distance, watermark, exact)
        }

        private fun skipHeader() {
            if (wire.size - at < HEADER - 1) throw IcpException("probe wire truncated in failed shard header")
            at += HEADER - 1
        }
    }
}
