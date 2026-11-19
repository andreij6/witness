package app.witness.core.domain.usecase

import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.ShardInfo
import app.witness.core.domain.port.ShardMap
import app.witness.core.domain.port.StageSink
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.ByteString
import app.witness.core.model.CheckStage
import app.witness.core.model.Platform
import app.witness.core.model.StageState
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class GatewayOracleTest {
    private val h1 = ByteString.of(ByteArray(32) { 1 })
    private val h2 = ByteString.of(ByteArray(32) { 2 })

    private suspend fun graphWithRecords(n: Int): Pair<TestGraph, List<AnchoredRecord>> {
        val g = TestGraph()
        g.enrolledKey()
        val recs = List(n) { g.sealOne() }.also { g.anchor() }.map { g.registry.record(it.recordId)!! }
        return g to recs
    }

    private fun oracle(g: TestGraph, options: JudgeOptions = JudgeOptions.forPlatform(Platform.ANDROID), sink: StageSink = StageSink { _, _, _, _, _ -> }) =
        GatewayOracle(g.registry, g.index, g.media, options, sink)

    @Test
    fun shardForUsesBaseBoundaries() {
        val shards = listOf(ShardInfo(0, 0), ShardInfo(1, 100), ShardInfo(2, 250))
        assertEquals(0, GatewayOracle.shardFor(shards, 0)?.id)
        assertEquals(0, GatewayOracle.shardFor(shards, 99)?.id)
        assertEquals(1, GatewayOracle.shardFor(shards, 100)?.id)
        assertEquals(1, GatewayOracle.shardFor(shards, 249)?.id)
        assertEquals(2, GatewayOracle.shardFor(shards, Long.MAX_VALUE)?.id)
        assertNull(GatewayOracle.shardFor(shards.drop(1), 5))
        assertNull(GatewayOracle.shardFor(emptyList(), 5))
    }

    @Test
    fun rankByDistanceTieBreaksOnLowestIndex() {
        val ranked = GatewayOracle.rankByDistance(mapOf(5L to 10, 3L to 10, 9L to 2, 1L to 40), 3)
        assertEquals(listOf(9L to 2, 3L to 10, 5L to 10), ranked)
        assertEquals(emptyList(), GatewayOracle.rankByDistance(emptyMap(), 3))
        assertEquals(emptyList(), GatewayOracle.rankByDistance(mapOf(1L to 1), 0))
    }

    @Test
    fun keyAndRecordLookupsAreMemoized() = runTest {
        val (g, recs) = graphWithRecords(1)
        val o = oracle(g)
        val before = g.registry.getCalls
        val a = o.record(recs[0].recordId)
        val b = o.record(recs[0].recordId)
        assertSame(a, b)
        assertEquals(before + 1, g.registry.getCalls)
        assertEquals(recs[0].claim.keyRef.keyId, o.key(recs[0].claim.keyRef.keyId)?.keyId)
    }

    @Test
    fun pdqMergesShardsByMinimumDistance() = runTest {
        val (g, recs) = graphWithRecords(3)
        g.registry.shardMap = ShardMap(listOf(ShardInfo(1, 0), ShardInfo(0, 0)), emptyList())
        g.index.addPdq(0, h1, recs[0].recordIx, 40)
        g.index.addPdq(1, h1, recs[0].recordIx, 12)
        g.index.addPdq(0, h2, recs[1].recordIx, 12)
        g.index.addPdq(1, h2, recs[2].recordIx, 5)
        val probe = oracle(g).pdq(listOf(h1, h2))
        assertEquals(listOf(recs[2].recordIx to 5, recs[0].recordIx to 12, recs[1].recordIx to 12), probe.hits.map { it.record.recordIx to it.distance })
        assertFalse(probe.truncated)
        assertEquals(2, probe.shardsAsked)
        assertEquals(2, probe.shardsAnswered)
    }

    @Test
    fun pdqCapsQueryHashesAndCandidates() = runTest {
        val (g, recs) = graphWithRecords(5)
        recs.forEachIndexed { i, r -> g.index.addPdq(0, h1, r.recordIx, i) }
        val opts = JudgeOptions.forPlatform(Platform.WEB).copy(maxPdqCandidates = 2, maxQueryHashes = 1)
        val probe = oracle(g, opts).pdq(listOf(h2, h1))
        assertTrue(probe.hits.isEmpty())
        val probe2 = oracle(g, opts).pdq(listOf(h1, h2))
        assertEquals(listOf(recs[0].recordIx, recs[1].recordIx), probe2.hits.map { it.record.recordIx })
    }

    @Test
    fun downOrTruncatedShardsMarkSearchPartial() = runTest {
        val (g, recs) = graphWithRecords(1)
        g.registry.shardMap = ShardMap(listOf(ShardInfo(0, 0), ShardInfo(1, 1000), ShardInfo(2, 2000)), emptyList())
        g.index.addPdq(0, h1, recs[0].recordIx, 3)
        g.index.down += 2
        val updates = ArrayList<Pair<Int, Int>>()
        val probe = oracle(g, sink = StageSink { s, st, done, total, _ -> if (s == CheckStage.FINGERPRINTS && st == StageState.RUNNING) updates += done to total }).pdq(listOf(h1))
        assertTrue(probe.truncated)
        assertEquals(3, probe.shardsAsked)
        assertEquals(2, probe.shardsAnswered)
        assertEquals(1, probe.hits.size)
        assertEquals(0 to 3, updates.first())
        assertEquals(2 to 3, updates.last())

        g.index.down.clear()
        g.index.truncatedShards += 1
        assertTrue(oracle(g).pdq(listOf(h1)).truncated)
    }

    @Test
    fun exactReturnsEarliestAnchoredCapped() = runTest {
        val (g, recs) = graphWithRecords(10)
        recs.reversed().forEach { g.index.addExact(0, h1, it.recordIx) }
        g.index.addExact(0, h2, recs[0].recordIx)
        val opts = JudgeOptions.forPlatform(Platform.ANDROID)
        val out = oracle(g, opts).exact(h1, h2)
        assertEquals(opts.maxExact, out.size)
        assertEquals(recs.take(8).map { it.recordIx }, out.map { it.recordIx })
        assertEquals(emptyList(), oracle(g).exact(h2.copyHash(9), null))
    }

    @Test
    fun byWatermarkResolvesEarliestFirst() = runTest {
        val (g, recs) = graphWithRecords(2)
        val hits = oracle(g).byWatermark(recs[1].claim.captureId)
        assertEquals(listOf(recs[1].recordId), hits.map { it.recordId })
    }

    @Test
    fun derivativesRouteToOwningShard() = runTest {
        val (g, recs) = graphWithRecords(3)
        g.registry.shardMap = ShardMap(listOf(ShardInfo(0, 0), ShardInfo(1, 2)), emptyList())
        g.index.derivatives[recs[0].recordIx] = mutableListOf(recs[2].recordIx, recs[1].recordIx, recs[1].recordIx)
        assertEquals(listOf(recs[1].recordIx, recs[2].recordIx), oracle(g).derivatives(recs[0]).map { it.recordIx })
        g.index.down += 0
        assertEquals(emptyList(), oracle(g).derivatives(recs[0]))
    }

    @Test
    fun referenceComesFromMediaAndFailsSoft() = runTest {
        val (g, recs) = graphWithRecords(1)
        assertNull(oracle(g).reference(recs[0]))
        g.media.references[recs[0].recordId] = h1
        assertEquals(h1, oracle(g).reference(recs[0]))
    }

    private fun ByteString.copyHash(v: Int) = ByteString.of(ByteArray(size) { v.toByte() })
}
