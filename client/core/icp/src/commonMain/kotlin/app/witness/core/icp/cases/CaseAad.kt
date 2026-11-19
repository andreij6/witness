package app.witness.core.icp.cases

import app.witness.core.icp.id.CaseId

object CaseAad {
    val DOMAIN: ByteArray = "witness/case/v1".encodeToByteArray()
    const val HASH_BYTES: Int = 32
    val SIZE: Int = DOMAIN.size + CaseId.LEN + 4 + 4 + HASH_BYTES

    fun build(caseId: CaseId, ix: Int, count: Int, bundleHash: ByteArray): ByteArray {
        require(count >= 1) { "chunk count $count must be positive" }
        require(ix in 0 until count) { "chunk $ix out of range 0 until $count" }
        require(bundleHash.size == HASH_BYTES) { "bundle hash needs $HASH_BYTES bytes, got ${bundleHash.size}" }
        val out = ByteArray(SIZE)
        DOMAIN.copyInto(out)
        var o = DOMAIN.size
        caseId.bytes.copyInto(out, o)
        o += CaseId.LEN
        o = putU32(out, o, ix)
        o = putU32(out, o, count)
        bundleHash.copyInto(out, o)
        return out
    }

    private fun putU32(out: ByteArray, at: Int, v: Int): Int {
        out[at] = (v ushr 24).toByte()
        out[at + 1] = (v ushr 16).toByte()
        out[at + 2] = (v ushr 8).toByte()
        out[at + 3] = v.toByte()
        return at + 4
    }
}
