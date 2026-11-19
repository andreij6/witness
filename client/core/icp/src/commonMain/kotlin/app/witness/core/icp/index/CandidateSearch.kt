package app.witness.core.icp.index

import app.witness.core.icp.registry.RegistryGateway

class CandidateSearch(private val registry: RegistryGateway, private val index: IndexSearch) {
    suspend fun search(query: ProbeQuery, progress: ShardProgress = ShardProgress { _, _ -> }): Candidates {
        val shards = registry.shards().index
        if (query.isEmpty || shards.isEmpty()) {
            return Candidates(LongArray(0), LongArray(0), IntArray(0), false, shards.size, shards.size, 0)
        }
        return CandidateMerge.merge(shards, index.probe(shards, query, progress))
    }
}
