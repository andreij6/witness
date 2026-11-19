package app.witness.core.icp.registry

import app.witness.core.icp.id.CaptureId
import app.witness.core.icp.id.RecordId

interface RegistryGateway {
    suspend fun get(recordId: RecordId): CertifiedRecord?

    suspend fun byCapture(captureId: CaptureId): List<RecordId>

    suspend fun shards(): ShardMap
}
