package app.witness.core.icp.cases

interface CaseCrypto {
    suspend fun sealChunk(key: ByteArray, nonce: ByteArray, plaintext: ByteArray, from: Int, to: Int, aad: ByteArray): ByteArray

    suspend fun openChunk(key: ByteArray, chunk: ByteArray, aad: ByteArray): ByteArray?

    suspend fun sha256(data: ByteArray): ByteArray

    fun randomBytes(count: Int): ByteArray
}
