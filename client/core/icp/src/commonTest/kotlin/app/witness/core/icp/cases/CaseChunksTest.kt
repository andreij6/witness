package app.witness.core.icp.cases

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class CaseChunksTest {
    private val p = CaseChunks.PLAIN_CHUNK.toLong()

    @Test
    fun wireChunkNeverExceedsOneMebibyte() {
        assertEquals(1 shl 20, CaseChunks.PLAIN_CHUNK + CaseChunks.OVERHEAD)
        assertEquals(28, CaseChunks.OVERHEAD)
    }

    @Test
    fun countAtBoundaries() {
        assertEquals(1, CaseChunks.count(0))
        assertEquals(1, CaseChunks.count(1))
        assertEquals(1, CaseChunks.count(p))
        assertEquals(2, CaseChunks.count(p + 1))
        assertEquals(3, CaseChunks.count(3 * p))
        assertFailsWith<IllegalArgumentException> { CaseChunks.count(-1) }
    }

    @Test
    fun plainSizesSumToTotal() {
        for (total in listOf(0L, 1L, p - 1, p, p + 1, 2 * p + 17, 5 * p)) {
            val n = CaseChunks.count(total)
            assertEquals(total, (0 until n).sumOf { CaseChunks.plainSize(it, total).toLong() }, "total $total")
            for (ix in 0 until n - 1) assertEquals(CaseChunks.PLAIN_CHUNK, CaseChunks.plainSize(ix, total))
            assertEquals(total, CaseChunks.start(n - 1) + CaseChunks.plainSize(n - 1, total))
        }
    }

    @Test
    fun plainSizeRejectsOutOfRange() {
        assertFailsWith<IllegalArgumentException> { CaseChunks.plainSize(1, p) }
        assertFailsWith<IllegalArgumentException> { CaseChunks.plainSize(-1, p) }
    }

    @Test
    fun wireSizeAddsOverheadPerChunk() {
        assertEquals(28, CaseChunks.wireSize(0))
        assertEquals(p + 28, CaseChunks.wireSize(p))
        assertEquals(p + 1 + 56, CaseChunks.wireSize(p + 1))
    }

    @Test
    fun sixteenMebibyteLimit() {
        assertTrue(CaseChunks.fits(16 * p))
        assertFalse(CaseChunks.fits(16 * p + 1))
        assertFalse(CaseChunks.fits(-1))
        assertTrue(CaseChunks.fits(0))
    }
}
