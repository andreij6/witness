package app.witness.core.icp.index

import app.witness.core.icp.registry.ShardInfo
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

fun interface ShardProgress {
    fun onShard(answered: Int, total: Int)
}

fun interface IndexSearch {
    suspend fun probe(shards: List<ShardInfo>, query: ProbeQuery, progress: ShardProgress): List<ShardProbe>
}

fun interface IndexGateway {
    suspend fun probe(shard: ShardInfo, query: ProbeQuery): ShardProbe
}

class FanOutIndexSearch(private val gateway: IndexGateway) : IndexSearch {
    override suspend fun probe(shards: List<ShardInfo>, query: ProbeQuery, progress: ShardProgress): List<ShardProbe> {
        val lock = Mutex()
        var answered = 0
        return coroutineScope {
            shards.map { shard ->
                async {
                    val result = try {
                        gateway.probe(shard, query)
                    } catch (e: CancellationException) {
                        throw e
                    } catch (_: Exception) {
                        ShardProbe.FAILED
                    }
                    lock.withLock {
                        if (result.answered) answered++
                        progress.onShard(answered, shards.size)
                    }
                    result
                }
            }.awaitAll()
        }
    }
}
