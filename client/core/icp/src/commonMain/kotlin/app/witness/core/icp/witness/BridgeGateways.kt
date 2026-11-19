package app.witness.core.icp.witness

import app.witness.core.domain.port.EnrollOutcome
import app.witness.core.domain.port.EnrollmentRequest
import app.witness.core.domain.port.EraseOutcome
import app.witness.core.domain.port.EraseRequest
import app.witness.core.domain.port.IndexGateway
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.ProbeQuery
import app.witness.core.domain.port.ProbeResult
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.ShardInfo
import app.witness.core.domain.port.ShardMap
import app.witness.core.domain.port.SubmitResult
import app.witness.core.domain.port.UploadRequest
import app.witness.core.domain.port.UploadTicket
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.KeyId
import app.witness.core.model.KeyInfo
import app.witness.core.model.RecordId
import app.witness.core.model.Submission

private fun readOnly(): Nothing = throw UnsupportedOperationException("the web client is read-only")

class BridgeRegistry(private val bridge: WitnessBridge) : RegistryGateway {
    override suspend fun get(recordId: RecordId): AnchoredRecord? =
        bridge.proof(recordId.text)?.let { WireJson.record(WireJson.parse(it)) }

    override suspend fun getKey(keyId: KeyId): KeyInfo? =
        bridge.key(ByteString.wrap(keyId.toBytes()).hex())?.let { WireJson.key(WireJson.parse(it)) }

    override suspend fun challenge(): ByteString = readOnly()

    override suspend fun enroll(request: EnrollmentRequest): EnrollOutcome = readOnly()

    override suspend fun submit(batch: List<Submission>): List<SubmitResult> = readOnly()

    override suspend fun getByIndex(recordIx: Long): AnchoredRecord? = readOnly()

    override suspend fun byCapture(captureId: CaptureId): List<RecordId> = readOnly()

    override suspend fun shards(): ShardMap = readOnly()

    override suspend fun eraseMine(request: EraseRequest): EraseOutcome = readOnly()
}

class BridgeMedia(private val bridge: WitnessBridge) : MediaGateway {
    override suspend fun reference(recordId: RecordId): ByteString? = bridge.preview(recordId.text)?.let(ByteString::wrap)

    override suspend fun begin(request: UploadRequest): UploadTicket = readOnly()

    override suspend fun putChunk(ticket: UploadTicket, index: Int, bytes: ByteArray) = readOnly()

    override suspend fun commit(ticket: UploadTicket) = readOnly()
}

object BridgeIndex : IndexGateway {
    override suspend fun probe(shard: ShardInfo, query: ProbeQuery): ProbeResult = readOnly()

    override suspend fun derivatives(shard: ShardInfo, rootIx: Long): List<Long> = readOnly()
}
