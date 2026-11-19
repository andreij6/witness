package app.witness.core.icp.fake

import app.witness.core.icp.IcpException
import app.witness.core.icp.index.IndexGateway
import app.witness.core.icp.index.ProbeQuery
import app.witness.core.icp.index.ShardProbe
import app.witness.core.icp.registry.ShardInfo

class InMemoryIndex(private val answers: Map<String, ShardProbe> = emptyMap()) : IndexGateway {
    override suspend fun probe(shard: ShardInfo, query: ProbeQuery): ShardProbe =
        answers[shard.canisterId] ?: throw IcpException("shard ${shard.canisterId} unreachable")
}
