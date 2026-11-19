package app.witness.core.domain.port

import app.witness.core.model.Anchor
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.CaseId
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.RecordId
import app.witness.core.model.Submission

class OfflineException(message: String = "offline", cause: Throwable? = null) : Exception(message, cause)

data class EnrollmentRequest(
    val keyId: KeyId,
    val attestationChain: List<ByteString>,
    val freshness: Freshness,
    val challenge: ByteString,
)

sealed interface EnrollOutcome {
    data class Enrolled(val info: KeyInfo, val c2paCert: ByteString, val certSerial: String) : EnrollOutcome

    data class Rejected(val reason: String) : EnrollOutcome
}

sealed interface SubmitResult {
    data class Anchored(val anchor: Anchor) : SubmitResult

    data class Duplicate(val anchor: Anchor) : SubmitResult

    data class RetryAfter(val delayMs: Long) : SubmitResult

    data object KeyExpired : SubmitResult

    data class Rejected(val reason: String) : SubmitResult
}

data class ShardInfo(val id: Int, val baseIx: Long, val indexedUpto: Long = Long.MAX_VALUE)

data class ShardMap(val index: List<ShardInfo>, val media: List<ShardInfo>)

data class EraseRequest(val recordId: RecordId, val keyId: KeyId, val nonce: ByteString, val signature: ByteString)

sealed interface EraseOutcome {
    data object Erased : EraseOutcome

    data class Rejected(val reason: String) : EraseOutcome
}

interface RegistryGateway {
    suspend fun challenge(): ByteString

    suspend fun enroll(request: EnrollmentRequest): EnrollOutcome

    suspend fun submit(batch: List<Submission>): List<SubmitResult>

    suspend fun get(recordId: RecordId): AnchoredRecord?

    suspend fun getByIndex(recordIx: Long): AnchoredRecord?

    suspend fun getKey(keyId: KeyId): KeyInfo?

    suspend fun byCapture(captureId: CaptureId): List<RecordId>

    suspend fun shards(): ShardMap

    suspend fun eraseMine(request: EraseRequest): EraseOutcome

    companion object {
        const val MAX_BATCH: Int = 64
    }
}

data class ProbeQuery(val pdq: List<ByteString> = emptyList(), val exact: List<ByteString> = emptyList())

data class PdqPosting(val recordIx: Long, val distance: Int)

data class ProbeResult(val pdq: List<PdqPosting> = emptyList(), val exact: List<Long> = emptyList(), val truncated: Boolean = false)

interface IndexGateway {
    suspend fun probe(shard: ShardInfo, query: ProbeQuery): ProbeResult

    suspend fun derivatives(shard: ShardInfo, rootIx: Long): List<Long>
}

data class UploadRequest(
    val caseId: CaseId,
    val chunkCount: Int,
    val totalBytes: Long,
    val bundleHash: ByteString,
    val expiresAtMs: Long?,
)

data class UploadTicket(val shard: Int, val uploadId: String)

interface PreviewPublisher {
    suspend fun isPublished(recordId: RecordId): Boolean

    suspend fun publish(recordId: RecordId, keyId: KeyId, jpeg: ByteString)

    object None : PreviewPublisher {
        override suspend fun isPublished(recordId: RecordId): Boolean = true

        override suspend fun publish(recordId: RecordId, keyId: KeyId, jpeg: ByteString) = Unit
    }
}

interface MediaGateway {
    suspend fun begin(request: UploadRequest): UploadTicket

    suspend fun putChunk(ticket: UploadTicket, index: Int, bytes: ByteArray)

    suspend fun commit(ticket: UploadTicket)

    suspend fun reference(recordId: RecordId): ByteString?

    suspend fun caseExpiry(caseId: CaseId): Long? = null

    val maxCaseBytes: Long get() = Long.MAX_VALUE

    companion object {
        const val MAX_CHUNK_BYTES: Int = 1 shl 20
    }
}
