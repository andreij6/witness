package app.witness.core.icp.registry

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class ShardMapTest {
    private val a = ShardInfo("a", 0, 99)
    private val b = ShardInfo("b", 100, 150)
    private val c = ShardInfo("c", 1_000, 1_000)

    @Test
    fun ofSortsByBase() {
        val map = ShardMap.of(listOf(c, a, b), listOf(ShardInfo("m0", 0, 0)))
        assertEquals(listOf(a, b, c), map.index)
    }

    @Test
    fun indexShardOfUsesBinarySearchBoundaries() {
        val map = ShardMap.of(listOf(a, b, c), emptyList())
        assertEquals(a, map.indexShardOf(0))
        assertEquals(a, map.indexShardOf(99))
        assertEquals(b, map.indexShardOf(100))
        assertEquals(b, map.indexShardOf(999))
        assertEquals(c, map.indexShardOf(1_000))
        assertEquals(c, map.indexShardOf(Long.MAX_VALUE))
        assertNull(map.indexShardOf(-1))
        assertNull(ShardMap.EMPTY.indexShardOf(0))
    }

    @Test
    fun indexShardOfMatchesLinearScanOnManyShards() {
        val shards = (0 until 257).map { ShardInfo("s$it", it * 37L + 5, 0) }
        val map = ShardMap.of(shards.reversed(), emptyList())
        for (ix in 0L until 257 * 37 + 50) {
            assertEquals(shards.lastOrNull { it.baseIx <= ix }, map.indexShardOf(ix), "ix $ix")
        }
    }

    @Test
    fun rejectsDuplicateOrNegativeBases() {
        assertFailsWith<IllegalArgumentException> { ShardMap.of(listOf(a, a.copy(canisterId = "z")), emptyList()) }
        assertFailsWith<IllegalArgumentException> { ShardMap.of(listOf(ShardInfo("n", -1, 0)), emptyList()) }
    }

    @Test
    fun mediaShardLookup() {
        val m0 = ShardInfo("m0", 0, 0)
        val m1 = ShardInfo("m1", 0, 0)
        val map = ShardMap.of(emptyList(), listOf(m0, m1))
        assertEquals(m0, map.mediaShard(0))
        assertEquals(m1, map.mediaShard(1))
        assertNull(map.mediaShard(2))
        assertNull(map.mediaShard(-1))
    }

    @Test
    fun equalityIsStructural() {
        assertEquals(ShardMap.of(listOf(b, a), emptyList()), ShardMap.of(listOf(a, b), emptyList()))
        assertEquals(ShardMap.of(listOf(b, a), emptyList()).hashCode(), ShardMap.of(listOf(a, b), emptyList()).hashCode())
    }
}
