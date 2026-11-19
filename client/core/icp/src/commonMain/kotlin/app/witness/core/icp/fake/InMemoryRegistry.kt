package app.witness.core.icp.fake

import app.witness.core.icp.id.CaptureId
import app.witness.core.icp.id.RecordId
import app.witness.core.icp.registry.CertifiedRecord
import app.witness.core.icp.registry.RegistryGateway
import app.witness.core.icp.registry.ShardMap

class InMemoryRegistry(
    records: Collection<CertifiedRecord> = emptyList(),
    captures: Map<CaptureId, List<RecordId>> = emptyMap(),
    private val shardMap: ShardMap = ShardMap.EMPTY,
) : RegistryGateway {
    private val byId = records.associateBy { it.recordId }
    private val byCapture = captures

    override suspend fun get(recordId: RecordId): CertifiedRecord? = byId[recordId]

    override suspend fun byCapture(captureId: CaptureId): List<RecordId> = byCapture[captureId].orEmpty()

    override suspend fun shards(): ShardMap = shardMap
}
