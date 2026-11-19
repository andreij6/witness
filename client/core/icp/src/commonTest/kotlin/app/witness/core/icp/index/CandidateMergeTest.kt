package app.witness.core.icp.index

import app.witness.core.icp.XorShift
import app.witness.core.icp.registry.ShardInfo
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class CandidateMergeTest {
    private val s0 = ShardInfo("s0", 0, 0)
    private val s1 = ShardInfo("s1", 1_000, 0)

    private fun pdq(indexedUpto: Long = 10, vararg hits: Pair<Int, Int>, truncated: Boolean = false) = ShardProbe.answered(
        indexedUpto = indexedUpto,
        truncated = truncated,
        localIx = IntArray(hits.size) { hits[it].first },
        shape = IntArray(hits.size),
        distance = IntArray(hits.size) { hits[it].second },
    )

    @Test
    fun mapsLocalIndexThroughShardBase() {
        val c = CandidateMerge.merge(listOf(s0, s1), listOf(pdq(10, 5 to 20), pdq(10, 5 to 10)))
        assertContentEquals(longArrayOf(1_005, 5), c.recordIx)
        assertContentEquals(intArrayOf(10, 20), c.distance)
    }

    @Test
    fun dedupesKeepingSmallestDistance() {
        val c = CandidateMerge.merge(listOf(s0), listOf(pdq(10, 4 to 30, 4 to 8, 4 to 50, 2 to 8)))
        assertContentEquals(longArrayOf(2, 4), c.recordIx)
        assertContentEquals(intArrayOf(8, 8), c.distance)
    }

    @Test
    fun ordersByDistanceThenLowestRecordIx() {
        val c = CandidateMerge.merge(listOf(s0), listOf(pdq(10, 9 to 3, 7 to 3, 1 to 90, 3 to 0)))
        assertContentEquals(longArrayOf(3, 7, 9, 1), c.recordIx)
        assertContentEquals(intArrayOf(0, 3, 3, 90), c.distance)
    }

    @Test
    fun watermarkHitsRankAfterPdqWithNoDistance() {
        val probe = ShardProbe.answered(
            indexedUpto = 10,
            localIx = intArrayOf(4),
            shape = intArrayOf(0),
            distance = intArrayOf(40),
            watermark = longArrayOf(9, 4, 2, 9),
        )
        val c = CandidateMerge.merge(listOf(s0), listOf(probe))
        assertContentEquals(longArrayOf(4, 2, 9), c.recordIx)
        assertContentEquals(intArrayOf(40, CandidateMerge.NO_DISTANCE, CandidateMerge.NO_DISTANCE), c.distance)
        assertFalse(c.truncated)
    }

    @Test
    fun exactHitsAreSortedUniqueAndCapped() {
        val a = ShardProbe.answered(indexedUpto = 1, exact = longArrayOf(30, 10, 10))
        val b = ShardProbe.answered(indexedUpto = 1, exact = LongArray(10) { 100L - it })
        val c = CandidateMerge.merge(listOf(s0, s1), listOf(a, b))
        assertContentEquals(longArrayOf(10, 30, 91, 92, 93, 94, 95, 96), c.exact)
        assertTrue(c.truncated)
    }

    @Test
    fun capsPdqHitsAndFlagsTruncation() {
        val hits = Array(40) { it to 40 - it }
        val c = CandidateMerge.merge(listOf(s0), listOf(pdq(10, *hits)))
        assertEquals(CandidateMerge.MAX_PROBE_HITS, c.size)
        assertEquals(1, c.distance.first())
        assertTrue(c.truncated)
    }

    @Test
    fun capsWatermarkHits() {
        val probe = ShardProbe.answered(indexedUpto = 1, watermark = LongArray(40) { it.toLong() })
        val c = CandidateMerge.merge(listOf(s0), listOf(probe))
        assertEquals(CandidateMerge.MAX_WATERMARK, c.size)
        assertEquals(0L, c.recordIx.first())
        assertTrue(c.truncated)
    }

    @Test
    fun combinedCandidatesNeverExceedCap() {
        val probe = ShardProbe.answered(
            indexedUpto = 1,
            localIx = IntArray(32) { it },
            shape = IntArray(32),
            distance = IntArray(32) { 1 },
            watermark = LongArray(32) { 100L + it },
        )
        val c = CandidateMerge.merge(listOf(s0), listOf(probe))
        assertEquals(CandidateMerge.MAX_CANDIDATES, c.size)
        assertFalse(c.truncated)
    }

    @Test
    fun shardTruncationPropagates() {
        assertTrue(CandidateMerge.merge(listOf(s0), listOf(pdq(10, truncated = true))).truncated)
    }

    @Test
    fun failedShardsAreCountedButIgnored() {
        val c = CandidateMerge.merge(listOf(s0, s1), listOf(ShardProbe.FAILED, pdq(77, 1 to 2)))
        assertEquals(1, c.answered)
        assertEquals(2, c.total)
        assertFalse(c.isComplete)
        assertEquals(77, c.indexedUpto)
        assertContentEquals(longArrayOf(1_001), c.recordIx)
    }

    @Test
    fun indexedUptoIsMinimumOverAnsweredShards() {
        val c = CandidateMerge.merge(listOf(s0, s1), listOf(pdq(500), pdq(40)))
        assertEquals(40, c.indexedUpto)
        assertTrue(c.isComplete)
        assertEquals(0, CandidateMerge.merge(listOf(s0), listOf(ShardProbe.FAILED)).indexedUpto)
    }

    @Test
    fun unsignedLocalIndexIsHonoured() {
        val c = CandidateMerge.merge(listOf(s0), listOf(pdq(10, -1 to 5)))
        assertContentEquals(longArrayOf(0xffff_ffffL), c.recordIx)
    }

    @Test
    fun outOfRangeRecordIndicesAreDropped() {
        val far = ShardInfo("far", CandidateMerge.MAX_RECORD_IX, 0)
        val probe = ShardProbe.answered(
            indexedUpto = 1,
            localIx = intArrayOf(0, 1),
            shape = intArrayOf(0, 0),
            distance = intArrayOf(1, 1),
            watermark = longArrayOf(Long.MAX_VALUE),
            exact = longArrayOf(CandidateMerge.MAX_RECORD_IX + 1),
        )
        val c = CandidateMerge.merge(listOf(far), listOf(probe))
        assertContentEquals(longArrayOf(CandidateMerge.MAX_RECORD_IX), c.recordIx)
        assertContentEquals(LongArray(0), c.exact)
    }

    @Test
    fun distanceIsClampedToNoDistance() {
        val c = CandidateMerge.merge(listOf(s0), listOf(pdq(1, 3 to 65_535)))
        assertContentEquals(intArrayOf(CandidateMerge.NO_DISTANCE), c.distance)
    }

    @Test
    fun rejectsMismatchedInputs() {
        assertFailsWith<IllegalArgumentException> { CandidateMerge.merge(listOf(s0), emptyList()) }
    }

    @Test
    fun emptyInputs() {
        val c = CandidateMerge.merge(emptyList(), emptyList())
        assertTrue(c.isEmpty)
        assertTrue(c.isComplete)
        assertEquals(0, c.total)
    }

    @Test
    fun resultIsIndependentOfHitOrder() {
        val rng = XorShift(41)
        repeat(50) {
            val hits = Array(200) { rng.below(80) to rng.below(257) }
            val shuffled = hits.copyOf().also { it.shuffle(kotlin.random.Random(it.size + rng.below(1000))) }
            assertEquals(
                CandidateMerge.merge(listOf(s0), listOf(pdq(1, *hits))),
                CandidateMerge.merge(listOf(s0), listOf(pdq(1, *shuffled))),
            )
        }
    }

    @Test
    fun matchesReferenceImplementation() {
        val rng = XorShift(42)
        repeat(100) {
            val shards = List(1 + rng.below(4)) { ShardInfo("s$it", it * 100L, 0) }
            val probes = shards.map {
                val n = rng.below(60)
                ShardProbe.answered(
                    indexedUpto = 1,
                    localIx = IntArray(n) { rng.below(120) },
                    shape = IntArray(n),
                    distance = IntArray(n) { rng.below(256) },
                    watermark = LongArray(rng.below(6)) { rng.below(400).toLong() },
                )
            }
            val pdqBest = shards.indices.flatMap { s -> (0 until probes[s].pdqCount).map { shards[s].baseIx + probes[s].localIx[it] to probes[s].distance[it] } }
                .groupBy({ it.first }, { it.second }).map { (ix, d) -> ix to d.min() }
                .sortedWith(compareBy({ it.second }, { it.first })).take(CandidateMerge.MAX_PROBE_HITS)
            val wm = probes.flatMap { p -> p.watermark.toList() }.distinct().sorted().take(CandidateMerge.MAX_WATERMARK)
                .map { it to CandidateMerge.NO_DISTANCE }
            val want = (pdqBest + wm).groupBy({ it.first }, { it.second }).map { (ix, d) -> ix to d.min() }
                .sortedWith(compareBy({ it.second }, { it.first })).take(CandidateMerge.MAX_CANDIDATES)
            val got = CandidateMerge.merge(shards, probes)
            assertEquals(want.map { it.first }, got.recordIx.toList())
            assertEquals(want.map { it.second }, got.distance.toList())
        }
    }

    @Test
    fun largeMergeIsFast() {
        val n = 200_000
        val rng = XorShift(43)
        val probe = ShardProbe.answered(
            indexedUpto = 1,
            localIx = IntArray(n) { rng.below(50_000) },
            shape = IntArray(n),
            distance = IntArray(n) { rng.below(257) },
        )
        val c = CandidateMerge.merge(listOf(s0), listOf(probe))
        assertEquals(CandidateMerge.MAX_PROBE_HITS, c.size)
        assertTrue((1 until c.size).all { c.distance[it - 1] <= c.distance[it] })
    }
}
