package app.witness.core.icp.cases

object CaseChunks {
    const val KEY_BYTES: Int = 32
    const val NONCE_BYTES: Int = 12
    const val TAG_BYTES: Int = 16
    const val OVERHEAD: Int = NONCE_BYTES + TAG_BYTES
    const val MAX_WIRE_CHUNK: Int = 1 shl 20
    const val PLAIN_CHUNK: Int = MAX_WIRE_CHUNK - OVERHEAD
    const val MAX_CASE_WIRE_BYTES: Long = 16L shl 20

    fun count(plainBytes: Long): Int {
        require(plainBytes >= 0) { "negative bundle size $plainBytes" }
        return if (plainBytes == 0L) 1 else ((plainBytes + PLAIN_CHUNK - 1) / PLAIN_CHUNK).toInt()
    }

    fun start(ix: Int): Long = ix.toLong() * PLAIN_CHUNK

    fun plainSize(ix: Int, plainBytes: Long): Int {
        require(ix in 0 until count(plainBytes)) { "chunk $ix out of range for $plainBytes bytes" }
        return minOf(PLAIN_CHUNK.toLong(), plainBytes - start(ix)).toInt()
    }

    fun wireSize(plainBytes: Long): Long = plainBytes + count(plainBytes).toLong() * OVERHEAD

    fun fits(plainBytes: Long): Boolean = plainBytes >= 0 && wireSize(plainBytes) <= MAX_CASE_WIRE_BYTES
}
