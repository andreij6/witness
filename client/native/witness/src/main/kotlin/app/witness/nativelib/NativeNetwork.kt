package app.witness.nativelib

import app.witness.core.domain.port.IndexGateway
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.PdqPosting
import app.witness.core.domain.port.PreviewPublisher
import app.witness.core.domain.port.ProbeQuery
import app.witness.core.domain.port.ProbeResult
import app.witness.core.domain.port.ShardInfo
import app.witness.core.domain.port.UploadRequest
import app.witness.core.domain.port.UploadTicket
import app.witness.core.model.ByteString
import app.witness.core.model.CaseId
import app.witness.core.model.Entitlement
import app.witness.core.model.KeyId
import app.witness.core.model.RecordId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.witness_android.CaseInfo
import uniffi.witness_android.UploadTarget
import uniffi.witness_android.WitnessInterface
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap

class NativeIndex(private val witness: WitnessInterface) : IndexGateway {
    override suspend fun probe(shard: ShardInfo, query: ProbeQuery): ProbeResult {
        val r = remote { witness.probe(query.pdq.map { it.toByteArray() }, emptyList(), query.exact.map { it.toByteArray() }) }
        return ProbeResult(r.pdq.map { PdqPosting(it.recordIx.toLong(), it.distance.toInt()) }, r.exact.map { it.toLong() }, r.truncatedCandidates)
    }

    override suspend fun derivatives(shard: ShardInfo, rootIx: Long): List<Long> = emptyList()
}

fun interface HttpFetch {
    suspend fun get(url: String): ByteArray?

    companion object {
        const val MAX_BYTES: Int = 4 shl 20
        const val TIMEOUT_MS: Int = 10_000

        val Default: HttpFetch = HttpFetch { url ->
            withContext(Dispatchers.IO) {
                val c = URL(url).openConnection() as HttpURLConnection
                try {
                    c.connectTimeout = TIMEOUT_MS
                    c.readTimeout = TIMEOUT_MS
                    if (c.responseCode != HttpURLConnection.HTTP_OK || c.contentLengthLong > MAX_BYTES) null
                    else c.inputStream.use { it.readNBytes(MAX_BYTES + 1) }.takeIf { it.size <= MAX_BYTES }
                } finally {
                    c.disconnect()
                }
            }
        }
    }
}

class NativeMedia(
    private val witness: WitnessInterface,
    private val keys: NativeKeys,
    private val gateway: String,
    private val fetch: HttpFetch = HttpFetch.Default,
    private val caseInfo: suspend (UByte, String) -> CaseInfo? = { _, _ -> null },
    private val previewOf: (ByteArray) -> ByteArray = { it },
) : MediaGateway, PreviewPublisher {
    private val uploads = ConcurrentHashMap<String, UploadRequest>()

    override val maxCaseBytes: Long = Entitlement.FREE_CASE_BYTES

    override suspend fun begin(request: UploadRequest): UploadTicket {
        require(request.chunkCount in 1..MAX_CASE_CHUNKS) { "chunk count out of range" }
        val ticket = UploadTicket(request.caseId.shard, request.caseId.text)
        uploads[ticket.uploadId] = request
        return ticket
    }

    override suspend fun putChunk(ticket: UploadTicket, index: Int, bytes: ByteArray) {
        val request = requireNotNull(uploads[ticket.uploadId]) { "unknown upload" }
        require(index in 0 until request.chunkCount) { "chunk index out of range" }
        require(bytes.size in 1..MediaGateway.MAX_CHUNK_BYTES) { "chunk size out of range" }
        val keyId = keys.activeEnrolled() ?: throw NativeException("no_key", "an enrolled key is required to upload")
        val signer = keys.signer(keyId)
        val shard = ticket.shard.toUByte()
        val target = UploadTarget.CaseChunk(request.caseId.text, index.toUInt(), request.chunkCount.toUInt())
        val digest = MessageDigest.getInstance("SHA-256").digest(bytes)
        remote(NativeTimeouts.LONG_MS) {
            val begun = witness.beginUpload(shard, target, bytes.size.toULong(), digest, keyId.toBytes(), signer, null)
            if (begun.missing.isNotEmpty()) witness.putChunk(shard, begun.upload, 0u, bytes)
            witness.commit(shard, begun.upload)
        }
    }

    override suspend fun commit(ticket: UploadTicket) {
        requireNotNull(uploads.remove(ticket.uploadId)) { "unknown upload" }
    }

    override suspend fun reference(recordId: RecordId): ByteString? {
        val media = remote { witness.shards() }.media
        for (m in media) fetch.get("$gateway/p/${recordId.text}?canisterId=${m.canister}")?.let { return ByteString.wrap(it) }
        return null
    }

    override suspend fun isPublished(recordId: RecordId): Boolean = reference(recordId) != null

    override suspend fun publish(recordId: RecordId, keyId: KeyId, jpeg: ByteString) {
        val preview = previewOf(jpeg.toByteArray())
        require(preview.size in 1..MAX_PREVIEW_BYTES) { "preview size out of range" }
        val signer = keys.signer(keyId)
        val digest = MessageDigest.getInstance("SHA-256").digest(preview)
        remote(NativeTimeouts.LONG_MS) {
            val shard = witness.get(recordId.text)?.mediaShard ?: throw OfflineException("record ${recordId.text} is not in the registry yet")
            val begun = witness.beginUpload(shard, UploadTarget.Preview(recordId.text), preview.size.toULong(), digest, keyId.toBytes(), signer, null)
            if (begun.missing.isNotEmpty()) witness.putChunk(shard, begun.upload, 0u, preview)
            witness.commit(shard, begun.upload)
        }
    }

    override suspend fun caseExpiry(caseId: CaseId): Long? =
        try {
            remote { caseInfo(caseId.shard.toUByte(), caseId.text) }?.let { NativeMapping.nsToMs(it.expiresNs) }
        } catch (_: OfflineException) {
            null
        } catch (_: NativeException) {
            null
        }

    companion object {
        const val MAX_CASE_CHUNKS: Int = 256
        const val MAX_PREVIEW_BYTES: Int = 150 * 1024
    }
}
