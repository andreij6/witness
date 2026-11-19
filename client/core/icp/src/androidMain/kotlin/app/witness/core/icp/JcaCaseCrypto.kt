package app.witness.core.icp

import app.witness.core.icp.cases.CaseChunks
import app.witness.core.icp.cases.CaseCrypto
import java.security.GeneralSecurityException
import java.security.MessageDigest
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

class JcaCaseCrypto(private val random: SecureRandom = SecureRandom()) : CaseCrypto {
    private fun cipher(mode: Int, key: ByteArray, nonce: ByteArray, nonceOffset: Int, aad: ByteArray): Cipher =
        Cipher.getInstance(TRANSFORMATION).apply {
            init(mode, SecretKeySpec(key, "AES"), GCMParameterSpec(CaseChunks.TAG_BYTES * 8, nonce, nonceOffset, CaseChunks.NONCE_BYTES))
            updateAAD(aad)
        }

    override suspend fun sealChunk(key: ByteArray, nonce: ByteArray, plaintext: ByteArray, from: Int, to: Int, aad: ByteArray): ByteArray {
        require(nonce.size == CaseChunks.NONCE_BYTES) { "nonce needs ${CaseChunks.NONCE_BYTES} bytes" }
        val out = ByteArray(CaseChunks.NONCE_BYTES + (to - from) + CaseChunks.TAG_BYTES)
        nonce.copyInto(out)
        cipher(Cipher.ENCRYPT_MODE, key, nonce, 0, aad).doFinal(plaintext, from, to - from, out, CaseChunks.NONCE_BYTES)
        return out
    }

    override suspend fun openChunk(key: ByteArray, chunk: ByteArray, aad: ByteArray): ByteArray? {
        if (chunk.size < CaseChunks.OVERHEAD) return null
        return try {
            cipher(Cipher.DECRYPT_MODE, key, chunk, 0, aad).doFinal(chunk, CaseChunks.NONCE_BYTES, chunk.size - CaseChunks.NONCE_BYTES)
        } catch (_: GeneralSecurityException) {
            null
        }
    }

    override suspend fun sha256(data: ByteArray): ByteArray = MessageDigest.getInstance("SHA-256").digest(data)

    override fun randomBytes(count: Int): ByteArray = ByteArray(count).also(random::nextBytes)

    private companion object {
        const val TRANSFORMATION = "AES/GCM/NoPadding"
    }
}
