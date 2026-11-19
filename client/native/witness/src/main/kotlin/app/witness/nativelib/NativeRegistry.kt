package app.witness.nativelib

import app.witness.core.domain.port.EnrollOutcome
import app.witness.core.domain.port.EnrollmentRequest
import app.witness.core.domain.port.EraseOutcome
import app.witness.core.domain.port.EraseRequest
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.ShardMap
import app.witness.core.domain.port.SubmitResult
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.RecordId
import app.witness.core.model.Submission
import uniffi.witness_android.ClaimView
import uniffi.witness_android.Enrollment
import uniffi.witness_android.WitnessException
import uniffi.witness_android.WitnessInterface

class NativeRegistry(
    private val witness: WitnessInterface,
    private val keys: NativeKeys,
    private val trust: NativeTrust,
    private val decode: (ByteArray) -> ClaimView,
) : RegistryGateway {
    override suspend fun challenge(): ByteString = remote { ByteString.wrap(witness.challenge()) }

    override suspend fun enroll(request: EnrollmentRequest): EnrollOutcome {
        if (request.freshness == Freshness.LATE) throw OfflineException("late enrollment needs the install opening")
        val enrolled = try {
            remote { witness.enroll(Enrollment(null, request.attestationChain.map { it.toByteArray() }, null)) }
        } catch (e: NativeException) {
            val reason = (e.cause as? WitnessException.Rejected)?.reason ?: throw e
            if (isTransient(reason)) throw OfflineException(reason, e)
            return EnrollOutcome.Rejected(reason)
        }
        val keyId = KeyId.fromBytes(enrolled.keyId)
        if (keyId != request.keyId) return EnrollOutcome.Rejected("registry enrolled a different key")
        trust.remember(enrolled.caCert)
        val info = getKey(keyId) ?: throw OfflineException("enrolled key not visible yet")
        return EnrollOutcome.Enrolled(info, ByteString.wrap(enrolled.c2paCert), NativeKeys.serialHex(enrolled.c2paCert))
    }

    override suspend fun submit(batch: List<Submission>): List<SubmitResult> {
        if (batch.isEmpty()) return emptyList()
        require(batch.size <= RegistryGateway.MAX_BATCH) { "batch too large" }
        return remote { witness.submit(batch.map(NativeMapping::submission)) }.map(NativeMapping::submitResult)
    }

    override suspend fun get(recordId: RecordId): AnchoredRecord? =
        remote { witness.get(recordId.text) }?.let { NativeMapping.record(it, decode) }

    override suspend fun getByIndex(recordIx: Long): AnchoredRecord? = null

    suspend fun claimCbor(recordId: RecordId): ByteString? = remote { witness.get(recordId.text) }?.claim?.let(ByteString::wrap)

    override suspend fun getKey(keyId: KeyId): KeyInfo? = remote { witness.getKey(keyId.toBytes()) }?.let { NativeMapping.keyInfo(it.key) }

    override suspend fun byCapture(captureId: CaptureId): List<RecordId> =
        remote { witness.byCapture(captureId.value.toULong()) }.mapNotNull(RecordId::parseOrNull)

    override suspend fun shards(): ShardMap = NativeMapping.shards(remote { witness.shards() })

    override suspend fun eraseMine(request: EraseRequest): EraseOutcome {
        val signer = keys.signer(request.keyId)
        return try {
            remote { witness.eraseMine(request.recordId.text, request.nonce.toByteArray(), signer) }
            EraseOutcome.Erased
        } catch (e: NativeException) {
            EraseOutcome.Rejected((e.cause as? WitnessException.Rejected)?.reason ?: throw e)
        }
    }

    companion object {
        private val TRANSIENT = listOf("quota", "not ready", "enrollment pending")

        fun isTransient(reason: String): Boolean = TRANSIENT.any(reason::startsWith)
    }
}
