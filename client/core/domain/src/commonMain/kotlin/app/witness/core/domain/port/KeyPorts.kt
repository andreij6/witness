package app.witness.core.domain.port

import app.witness.core.model.ByteString
import app.witness.core.model.DeviceKey
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId

data class GeneratedKey(val key: DeviceKey, val attestationChain: List<ByteString>, val challenge: ByteString)

interface KeyVault {
    suspend fun activeKey(): DeviceKey?

    suspend fun key(keyId: KeyId): DeviceKey?

    suspend fun generate(challenge: ByteString, freshness: Freshness): GeneratedKey

    suspend fun attestation(keyId: KeyId): GeneratedKey?

    suspend fun markEnrolled(keyId: KeyId, enrolledAtMs: Long, activeUntilMs: Long, certSerial: String, c2paCert: ByteString)

    suspend fun retire(keyId: KeyId)

    suspend fun delete(keyId: KeyId)

    suspend fun sign(keyId: KeyId, message: ByteArray): ByteString
}

fun interface CounterStore {
    suspend fun reserve(keyId: KeyId): Long
}
