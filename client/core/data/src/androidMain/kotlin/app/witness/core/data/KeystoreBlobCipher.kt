package app.witness.core.data

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class AesGcmBlobCipher(private val key: () -> SecretKey) : BlobCipher {
    override fun seal(plain: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION).apply { init(Cipher.ENCRYPT_MODE, key()) }
        val iv = cipher.iv
        check(iv.size == IV_BYTES) { "unexpected IV size ${iv.size}" }
        val out = ByteArray(1 + IV_BYTES + plain.size + TAG_BYTES)
        out[0] = VERSION
        iv.copyInto(out, 1)
        val written = cipher.doFinal(plain, 0, plain.size, out, 1 + IV_BYTES)
        check(written == plain.size + TAG_BYTES) { "unexpected ciphertext size" }
        return out
    }

    override fun open(sealed: ByteArray): ByteArray {
        require(sealed.size >= 1 + IV_BYTES + TAG_BYTES) { "sealed blob too short" }
        require(sealed[0] == VERSION) { "unsupported blob version ${sealed[0]}" }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(TAG_BYTES * 8, sealed, 1, IV_BYTES))
        return cipher.doFinal(sealed, 1 + IV_BYTES, sealed.size - 1 - IV_BYTES)
    }

    companion object {
        const val VERSION: Byte = 1
        const val IV_BYTES = 12
        const val TAG_BYTES = 16
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val KEYSTORE = "AndroidKeyStore"
        const val ALIAS = "witness.loc_opening.v1"

        fun keystore(alias: String = ALIAS): AesGcmBlobCipher {
            var cached: SecretKey? = null
            return AesGcmBlobCipher { cached ?: loadOrCreate(alias).also { cached = it } }
        }

        private fun loadOrCreate(alias: String): SecretKey {
            val ks = KeyStore.getInstance(KEYSTORE).apply { load(null) }
            (ks.getKey(alias, null) as? SecretKey)?.let { return it }
            val spec = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setRandomizedEncryptionRequired(true)
                .build()
            return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE).apply { init(spec) }.generateKey()
        }
    }
}
