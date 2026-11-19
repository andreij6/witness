package app.witness.core.icp.index

import app.witness.core.icp.id.CaptureId
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ProbeQueryTest {
    @Test
    fun countsHashesAndKeys() {
        val q = ProbeQuery(pdq = ByteArray(64), exact = ByteArray(32))
        assertEquals(2, q.pdqCount)
        assertEquals(2, q.exactCount)
        assertFalse(q.isEmpty)
        assertTrue(ProbeQuery().isEmpty)
    }

    @Test
    fun rejectsMisalignedOrOversized() {
        assertFailsWith<IllegalArgumentException> { ProbeQuery(pdq = ByteArray(31)) }
        assertFailsWith<IllegalArgumentException> { ProbeQuery(pdq = ByteArray(32 * 17)) }
        assertFailsWith<IllegalArgumentException> { ProbeQuery(exact = ByteArray(15)) }
        assertFailsWith<IllegalArgumentException> { ProbeQuery(exact = ByteArray(16 * 5)) }
        assertFailsWith<IllegalArgumentException> { ProbeQuery(exact = ByteArray(32), exactStride = 0) }
        assertFailsWith<IllegalArgumentException> { ProbeQuery(watermarks = List(5) { CaptureId.of(it.toLong(), 0)!! }) }
    }

    @Test
    fun watermarkPairsSplitHighLow() {
        val id = CaptureId.of(0x00ab_cdef_0123_4567, 0x1f)!!
        assertContentEquals(intArrayOf(0x1fabcdef, 0x01234567), ProbeQuery(watermarks = listOf(id)).watermarkPairs())
        assertContentEquals(IntArray(0), ProbeQuery().watermarkPairs())
    }
}
