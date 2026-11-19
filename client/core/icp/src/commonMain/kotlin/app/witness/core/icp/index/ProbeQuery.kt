package app.witness.core.icp.index

import app.witness.core.icp.id.CaptureId

class ProbeQuery(
    val pdq: ByteArray = ByteArray(0),
    val watermarks: List<CaptureId> = emptyList(),
    val exact: ByteArray = ByteArray(0),
    val exactStride: Int = EXACT_BYTES,
) {
    init {
        require(pdq.size % PDQ_BYTES == 0) { "pdq length ${pdq.size} is not a multiple of $PDQ_BYTES" }
        require(pdqCount <= MAX_PDQ) { "at most $MAX_PDQ pdq hashes, got $pdqCount" }
        require(exactStride > 0 && exact.size % exactStride == 0) { "exact length ${exact.size} is not a multiple of $exactStride" }
        require(exactCount <= MAX_EXACT) { "at most $MAX_EXACT exact keys, got $exactCount" }
        require(watermarks.size <= MAX_WATERMARKS) { "at most $MAX_WATERMARKS watermark ids, got ${watermarks.size}" }
    }

    val pdqCount: Int get() = pdq.size / PDQ_BYTES
    val exactCount: Int get() = exact.size / exactStride
    val isEmpty: Boolean get() = pdq.isEmpty() && watermarks.isEmpty() && exact.isEmpty()

    fun watermarkPairs(): IntArray {
        val out = IntArray(watermarks.size * 2)
        watermarks.forEachIndexed { i, id ->
            out[2 * i] = id.high
            out[2 * i + 1] = id.low
        }
        return out
    }

    companion object {
        const val PDQ_BYTES: Int = 32
        const val EXACT_BYTES: Int = 16
        const val MAX_PDQ: Int = 16
        const val MAX_EXACT: Int = 4
        const val MAX_WATERMARKS: Int = 4
    }
}
