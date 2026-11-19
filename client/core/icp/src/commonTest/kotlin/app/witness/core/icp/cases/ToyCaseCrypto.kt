package app.witness.core.icp.cases

class ToyCaseCrypto(private var counter: Int = 1) : CaseCrypto {
    private fun fnv(seed: Int, vararg parts: ByteArray): Int {
        var h = seed
        for (p in parts) for (b in p) h = (h xor (b.toInt() and 0xff)) * 0x01000193
        return h
    }

    private fun xorKeystream(key: ByteArray, nonce: ByteArray, src: ByteArray, from: Int, n: Int): ByteArray {
        var s = fnv(-0x7ee3623b, key, nonce) or 1
        return ByteArray(n) {
            s = s xor (s shl 13)
            s = s xor (s ushr 17)
            s = s xor (s shl 5)
            (src[from + it].toInt() xor s).toByte()
        }
    }

    private fun tag(key: ByteArray, nonce: ByteArray, aad: ByteArray, ct: ByteArray): ByteArray {
        val words = IntArray(4) { fnv(it + 1, key, nonce, aad) }
        for (i in ct.indices) {
            val w = i and 3
            words[w] = (words[w] xor (ct[i].toInt() and 0xff)) * 0x01000193
        }
        return ByteArray(CaseChunks.TAG_BYTES) { (words[it / 4] ushr (8 * (it % 4))).toByte() }
    }

    override suspend fun sealChunk(key: ByteArray, nonce: ByteArray, plaintext: ByteArray, from: Int, to: Int, aad: ByteArray): ByteArray {
        val ct = xorKeystream(key, nonce, plaintext, from, to - from)
        return nonce + ct + tag(key, nonce, aad, ct)
    }

    override suspend fun openChunk(key: ByteArray, chunk: ByteArray, aad: ByteArray): ByteArray? {
        if (chunk.size < CaseChunks.OVERHEAD) return null
        val nonce = chunk.copyOfRange(0, CaseChunks.NONCE_BYTES)
        val ct = chunk.copyOfRange(CaseChunks.NONCE_BYTES, chunk.size - CaseChunks.TAG_BYTES)
        if (!tag(key, nonce, aad, ct).contentEquals(chunk.copyOfRange(chunk.size - CaseChunks.TAG_BYTES, chunk.size))) return null
        return xorKeystream(key, nonce, ct, 0, ct.size)
    }

    override suspend fun sha256(data: ByteArray): ByteArray {
        val words = IntArray(8) { it * 0x9e3779b1.toInt() }
        for (i in data.indices) {
            val w = i and 7
            words[w] = (words[w] xor (data[i].toInt() and 0xff)) * 0x01000193
        }
        return ByteArray(32) { (words[it / 4] ushr (8 * (it % 4))).toByte() }
    }

    override fun randomBytes(count: Int): ByteArray = ByteArray(count) { (counter++ * -0x61c88647 ushr 13).toByte() }
}
