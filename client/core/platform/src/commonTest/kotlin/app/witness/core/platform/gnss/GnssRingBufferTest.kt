package app.witness.core.platform.gnss

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class GnssRingBufferTest {
    @Test
    fun keepsOnlyWindow() {
        val b = GnssRingBuffer()
        for (t in 0..20) b.add(epoch(t))
        val snap = b.snapshot(20 * SEC)
        assertEquals((10..20).map { it * SEC }, snap.map { it.elapsedNs })
    }

    @Test
    fun snapshotEvictsByNow() {
        val b = GnssRingBuffer()
        for (t in 0..5) b.add(epoch(t))
        assertEquals(listOf(5 * SEC), b.snapshot(15 * SEC).map { it.elapsedNs })
        assertTrue(b.snapshot(100 * SEC).isEmpty())
    }

    @Test
    fun capacityOverwritesOldest() {
        val b = GnssRingBuffer(windowNs = 1_000 * SEC, capacity = 3)
        for (t in 0..9) b.add(epoch(t))
        assertEquals(listOf(7, 8, 9).map { it * SEC }, b.snapshot(9 * SEC).map { it.elapsedNs })
        assertEquals(3, b.size)
    }

    @Test
    fun timeGoingBackwardsResets() {
        val b = GnssRingBuffer()
        b.add(epoch(5))
        b.add(epoch(6))
        b.add(epoch(2))
        assertEquals(listOf(2 * SEC), b.snapshot(2 * SEC).map { it.elapsedNs })
    }

    @Test
    fun clearEmpties() {
        val b = GnssRingBuffer()
        b.add(epoch(1))
        b.clear()
        assertEquals(0, b.size)
        assertTrue(b.snapshot(1 * SEC).isEmpty())
    }

    @Test
    fun invalidConfigRejected() {
        assertFailsWith<IllegalArgumentException> { GnssRingBuffer(capacity = 0) }
        assertFailsWith<IllegalArgumentException> { GnssRingBuffer(windowNs = 0) }
    }

    @Test
    fun manyEpochsStayBounded() {
        val b = GnssRingBuffer()
        for (t in 0 until 100_000) b.add(GnssEpoch(t * 100_000_000L, 0, null, 0.0, null, 0, emptyList()))
        assertEquals(GnssRingBuffer.DEFAULT_CAPACITY, b.size)
    }
}
