package app.witness.core.platform.keys

import android.content.SharedPreferences
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import android.util.Base64
import app.witness.core.domain.Clock
import app.witness.core.domain.port.GeneratedKey
import app.witness.core.domain.port.KeyVault
import app.witness.core.model.ByteString
import app.witness.core.model.DeviceKey
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.SecurityLevel
import app.witness.core.platform.crypto.EcdsaSignature
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.security.KeyFactory
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.MessageDigest
import java.security.PrivateKey
import java.security.Signature
import java.security.spec.ECGenParameterSpec

class KeystoreKeyVault(
    private val prefs: SharedPreferences,
    private val clock: Clock,
    private val hasStrongBox: Boolean,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : KeyVault {
    private val keyStore: KeyStore by lazy { KeyStore.getInstance(PROVIDER).apply { load(null) } }
    private val signing = Mutex()

    override suspend fun activeKey(): DeviceKey? = withContext(io) { StrongBoxPolicy.active(all()) }

    override suspend fun key(keyId: KeyId): DeviceKey? = withContext(io) { record(keyId) }

    override suspend fun generate(challenge: ByteString, freshness: Freshness): GeneratedKey = withContext(io) {
        require(challenge.size in 1..MAX_CHALLENGE) { "attestation challenge must be 1..$MAX_CHALLENGE bytes" }
        val now = clock.nowEpochMillis()
        val alias = ALIAS_PREFIX + now + "." + System.nanoTime()
        val raw = challenge.toByteArray()
        val strongBox = hasStrongBox && tryStrongBox(alias, raw)
        if (!strongBox) generatePair(alias, raw, strongBox = false)
        val chain = keyStore.getCertificateChain(alias).map { ByteString.wrap(it.encoded) }
        val keyId = KeyId.fromDigest(sha256(keyStore.getCertificate(alias).publicKey.encoded))
        val key = DeviceKey(keyId, securityOf(alias, strongBox), freshness, now)
        prefs.edit()
            .putString(recordKey(keyId), KeyRecordCodec.encode(key))
            .putString(aliasKey(keyId), alias)
            .putString(challengeKey(keyId), Base64.encodeToString(raw, Base64.NO_WRAP))
            .apply()
        GeneratedKey(key, chain, challenge)
    }

    override suspend fun attestation(keyId: KeyId): GeneratedKey? = withContext(io) {
        val key = record(keyId) ?: return@withContext null
        val alias = prefs.getString(aliasKey(keyId), null) ?: return@withContext null
        val challenge = prefs.getString(challengeKey(keyId), null) ?: return@withContext null
        val chain = keyStore.getCertificateChain(alias)?.map { ByteString.wrap(it.encoded) } ?: return@withContext null
        GeneratedKey(key, chain, ByteString.wrap(Base64.decode(challenge, Base64.NO_WRAP)))
    }

    override suspend fun markEnrolled(keyId: KeyId, enrolledAtMs: Long, activeUntilMs: Long, certSerial: String, c2paCert: ByteString) =
        withContext(io) {
            val key = requireNotNull(record(keyId)) { "unknown key" }
            val updated = key.copy(enrolledAtMs = enrolledAtMs, activeUntilMs = activeUntilMs, certSerial = certSerial)
            prefs.edit()
                .putString(recordKey(keyId), KeyRecordCodec.encode(updated))
                .putString(certKey(keyId), Base64.encodeToString(c2paCert.toByteArray(), Base64.NO_WRAP))
                .apply()
        }

    override suspend fun retire(keyId: KeyId) = withContext(io) {
        val key = record(keyId) ?: return@withContext
        prefs.edit().putString(recordKey(keyId), KeyRecordCodec.encode(key.copy(retired = true))).apply()
    }

    override suspend fun delete(keyId: KeyId) = withContext(io) {
        prefs.getString(aliasKey(keyId), null)?.let(keyStore::deleteEntry)
        prefs.edit().remove(recordKey(keyId)).remove(aliasKey(keyId)).remove(certKey(keyId)).remove(challengeKey(keyId)).apply()
    }

    override suspend fun sign(keyId: KeyId, message: ByteArray): ByteString = signing.withLock {
        withContext(io) { ByteString.wrap(EcdsaSignature.derToRaw(signDer(aliasOf(keyId), message))) }
    }

    suspend fun c2paCertificate(keyId: KeyId): ByteString? = withContext(io) {
        prefs.getString(certKey(keyId), null)?.let { ByteString.wrap(Base64.decode(it, Base64.NO_WRAP)) }
    }

    private fun record(keyId: KeyId): DeviceKey? = prefs.getString(recordKey(keyId), null)?.let(KeyRecordCodec::decode)

    private fun all(): List<DeviceKey> = prefs.all.mapNotNull { (k, v) ->
        if (k.startsWith(RECORD_PREFIX)) (v as? String)?.let(KeyRecordCodec::decode) else null
    }

    private fun aliasOf(keyId: KeyId): String = requireNotNull(prefs.getString(aliasKey(keyId), null)) { "unknown key" }

    private fun tryStrongBox(alias: String, challenge: ByteArray): Boolean {
        val ok = try {
            generatePair(alias, challenge, strongBox = true)
            StrongBoxPolicy.withinBudget(LongArray(StrongBoxPolicy.SAMPLES) { timeSign(alias) })
        } catch (_: StrongBoxUnavailableException) {
            false
        }
        if (!ok) keyStore.deleteEntry(alias)
        return ok
    }

    private fun timeSign(alias: String): Long {
        val start = System.nanoTime()
        signDer(alias, PROBE)
        return System.nanoTime() - start
    }

    private fun generatePair(alias: String, challenge: ByteArray, strongBox: Boolean) {
        val spec = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_SIGN)
            .setAlgorithmParameterSpec(ECGenParameterSpec(CURVE))
            .setDigests(KeyProperties.DIGEST_SHA256)
            .setAttestationChallenge(challenge)
            .setIsStrongBoxBacked(strongBox)
            .build()
        KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC, PROVIDER).apply { initialize(spec) }.generateKeyPair()
    }

    private fun signDer(alias: String, message: ByteArray): ByteArray {
        val key = keyStore.getKey(alias, null) as PrivateKey
        return Signature.getInstance(SIGNATURE).run {
            initSign(key)
            update(message)
            sign()
        }
    }

    private fun securityOf(alias: String, strongBox: Boolean): SecurityLevel {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            val key = keyStore.getKey(alias, null) as PrivateKey
            val info = KeyFactory.getInstance(key.algorithm, PROVIDER).getKeySpec(key, KeyInfo::class.java)
            return when (info.securityLevel) {
                KeyProperties.SECURITY_LEVEL_STRONGBOX -> SecurityLevel.STRONG_BOX
                KeyProperties.SECURITY_LEVEL_TRUSTED_ENVIRONMENT -> SecurityLevel.TEE
                else -> SecurityLevel.DEV
            }
        }
        return if (strongBox) SecurityLevel.STRONG_BOX else SecurityLevel.TEE
    }

    companion object {
        const val PROVIDER = "AndroidKeyStore"
        const val CURVE = "secp256r1"
        const val SIGNATURE = "SHA256withECDSA"
        const val MAX_CHALLENGE = 128
        const val PREFS_NAME = "witness_keys"
        private const val ALIAS_PREFIX = "witness.key."
        private const val RECORD_PREFIX = "key."
        private const val ALIAS_KEY_PREFIX = "alias."
        private const val CERT_PREFIX = "cert."
        private const val CHALLENGE_PREFIX = "challenge."
        private val PROBE = ByteArray(32)

        private fun recordKey(keyId: KeyId) = RECORD_PREFIX + keyId.text
        private fun aliasKey(keyId: KeyId) = ALIAS_KEY_PREFIX + keyId.text
        private fun certKey(keyId: KeyId) = CERT_PREFIX + keyId.text
        private fun challengeKey(keyId: KeyId) = CHALLENGE_PREFIX + keyId.text

        fun sha256(bytes: ByteArray): ByteArray = MessageDigest.getInstance("SHA-256").digest(bytes)

        fun hasStrongBox(packageManager: android.content.pm.PackageManager): Boolean =
            packageManager.hasSystemFeature(android.content.pm.PackageManager.FEATURE_STRONGBOX_KEYSTORE)

        fun offlineChallenge(installNonce: ByteArray, wallMs: Long): ByteString =
            ByteString.wrap(sha256(app.witness.core.platform.crypto.Preimages.offlineChallenge(installNonce, wallMs)))
    }
}
