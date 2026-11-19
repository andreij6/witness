package app.witness.core.icp.id

import app.witness.core.icp.XorShift
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class CaptureIdTest {
    @Test
    fun layoutSharedWithRust() {
        val id = CaptureId.of(0x00ab_cdef_0123_4567, 0x1f)!!
        assertEquals(0x1fab_cdef_0123_4567, id.value)
        assertEquals(0x1f, id.schema)
        assertEquals(0x00ab_cdef_0123_4567, id.random)
        assertEquals(0L, CaptureId.of(0, 0)!!.value)
        assertEquals(CaptureId.MAX, CaptureId.of(CaptureId.RANDOM_MASK, 31)!!.value)
        assertEquals((1L shl 61) - 1, CaptureId.MAX)
    }

    @Test
    fun rejectsOutOfRange() {
        assertNull(CaptureId.of(1L shl 56, 0))
        assertNull(CaptureId.of(0, 32))
        assertNull(CaptureId.of(-1, 0))
        assertNull(CaptureId.of(0, -1))
        assertNull(CaptureId.fromLong(1L shl 61))
        assertNull(CaptureId.fromLong(-1))
        assertEquals(CaptureId.MAX, CaptureId.fromLong(CaptureId.MAX)?.value)
    }

    @Test
    fun highLowSplitRoundTrips() {
        val rng = XorShift(20)
        repeat(2000) {
            val id = CaptureId.of(rng.next() and CaptureId.RANDOM_MASK, rng.below(32))!!
            assertEquals(id.value, (id.high.toLong() shl 32) or (id.low.toLong() and 0xffff_ffffL))
            assertEquals(id, CaptureId.fromLong(id.value))
        }
    }
}
