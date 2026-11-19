package app.witness.core.icp.index

import app.witness.core.icp.IcpException
import app.witness.core.icp.fake.InMemoryIndex
import app.witness.core.icp.fake.InMemoryRegistry
import app.witness.core.icp.registry.ShardInfo
import app.witness.core.icp.registry.ShardMap
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class FanOutIndexSearchTest {
    private val shards = listOf(ShardInfo("a", 0, 0), ShardInfo("b", 100, 0), ShardInfo("c", 200, 0))
    private val query = ProbeQuery(pdq = ByteArray(32))

    private fun hit(local: Int, d: Int) = ShardProbe.answered(5, localIx = intArrayOf(local), shape = intArrayOf(0), distance = intArrayOf(d))

    @Test
    fun keepsShardOrderAndMarksFailures() = runTest {
        val search = FanOutIndexSearch(InMemoryIndex(mapOf("a" to hit(1, 9), "c" to hit(2, 3))))
        val progress = mutableListOf<Pair<Int, Int>>()
        val probes = search.probe(shards, query) { a, t -> progress += a to t }
        assertEquals(listOf(hit(1, 9), ShardProbe.FAILED, hit(2, 3)), probes)
        assertEquals(3, progress.size)
        assertEquals(2 to 3, progress.last())
        assertTrue(progress.all { it.second == 3 })
    }

    @Test
    fun runsShardsConcurrently() = runTest {
        val gate = CompletableDeferred<Unit>()
        var started = 0
        val search = FanOutIndexSearch { shard, _ ->
            started++
            if (started == shards.size) gate.complete(Unit)
            gate.await()
            hit(shard.baseIx.toInt(), 1)
        }
        val probes = search.probe(shards, query) { _, _ -> }
        assertEquals(3, probes.count { it.answered })
    }

    @Test
    fun progressIsMonotonic() = runTest {
        val search = FanOutIndexSearch { shard, _ ->
            delay(300 - shard.baseIx)
            hit(0, 0)
        }
        val seen = mutableListOf<Int>()
        search.probe(shards, query) { a, _ -> seen += a }
        assertEquals(listOf(1, 2, 3), seen)
    }

    @Test
    fun cancellationIsNotSwallowed() = runTest {
        val search = FanOutIndexSearch { _, _ -> throw CancellationException("stop") }
        assertFailsWith<CancellationException> { search.probe(shards, query) { _, _ -> } }
    }

    @Test
    fun emptyShardListYieldsEmpty() = runTest {
        assertEquals(emptyList(), FanOutIndexSearch(InMemoryIndex()).probe(emptyList(), query) { _, _ -> error("no progress") })
    }

    @Test
    fun candidateSearchMergesAcrossShards() = runTest {
        val registry = InMemoryRegistry(shardMap = ShardMap.of(shards, emptyList()))
        val index = FanOutIndexSearch(InMemoryIndex(mapOf("a" to hit(1, 9), "b" to hit(1, 2), "c" to hit(1, 9))))
        val c = CandidateSearch(registry, index).search(query)
        assertContentEquals(longArrayOf(101, 1, 201), c.recordIx)
        assertEquals(3, c.answered)
        assertTrue(c.isComplete)
    }

    @Test
    fun candidateSearchShortCircuitsEmptyQuery() = runTest {
        val registry = InMemoryRegistry(shardMap = ShardMap.of(shards, emptyList()))
        val c = CandidateSearch(registry) { _, _, _ -> throw IcpException("must not probe") }.search(ProbeQuery())
        assertTrue(c.isEmpty)
        assertTrue(c.isComplete)
        assertFalse(c.truncated)
    }

    @Test
    fun candidateSearchWithoutShards() = runTest {
        val c = CandidateSearch(InMemoryRegistry()) { _, _, _ -> throw IcpException("must not probe") }.search(query)
        assertEquals(0, c.total)
        assertTrue(c.isEmpty)
    }

    @Test
    fun allShardsDownReportsZeroAnswered() = runTest {
        val registry = InMemoryRegistry(shardMap = ShardMap.of(shards, emptyList()))
        val c = CandidateSearch(registry, FanOutIndexSearch(InMemoryIndex())).search(query)
        assertEquals(0, c.answered)
        assertEquals(3, c.total)
        assertTrue(c.isEmpty)
    }

    @Test
    fun concurrentSearchesAreIndependent() = runTest {
        val registry = InMemoryRegistry(shardMap = ShardMap.of(shards, emptyList()))
        val search = CandidateSearch(registry, FanOutIndexSearch(InMemoryIndex(mapOf("a" to hit(1, 1)))))
        val results = List(20) { async { search.search(query) } }.map { it.await() }
        assertTrue(results.all { it == results.first() })
    }
}
