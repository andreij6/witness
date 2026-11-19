package app.witness.core.icp.index

import app.witness.core.icp.IcpException
import app.witness.core.icp.XorShift
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ProbeWireTest {
    private val sharedVector = intArrayOf(
        3,
        0, 1, 1, 2, 2, 1, 1, 5, 3, 12, -1, 255, 65535, 0, 7, 256, 1,
        1, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0,
    )

    private val sharedProbes = listOf(
        ShardProbe.answered(
            indexedUpto = 0x1_0000_0002L,
            truncated = true,
            localIx = intArrayOf(5, -1),
            shape = intArrayOf(3, 255),
            distance = intArrayOf(12, 65535),
            watermark = longArrayOf(7),
            exact = longArrayOf((1L shl 40) + 1),
        ),
        ShardProbe.FAILED,
        ShardProbe.answered(indexedUpto = 0),
    )

    @Test
    fun decodesVectorSharedWithBridgesJs() {
        val probes = ProbeWire.decode(sharedVector)
        assertEquals(sharedProbes, probes)
        assertEquals(0xffff_ffffL, probes[0].localIxAt(1))
        assertFalse(probes[1].answered)
    }

    @Test
    fun encodesVectorSharedWithBridgesJs() {
        assertContentEquals(sharedVector, ProbeWire.encode(sharedProbes))
        assertEquals(sharedVector.size, ProbeWire.encodedSize(sharedProbes))
    }

    @Test
    fun emptyWire() {
        assertEquals(emptyList(), ProbeWire.decode(intArrayOf(0)))
        assertContentEquals(intArrayOf(0), ProbeWire.encode(emptyList()))
    }

    @Test
    fun u64JoinsHalves() {
        assertEquals(0L, ProbeWire.u64(0, 0))
        assertEquals(0xffff_ffffL, ProbeWire.u64(0, -1))
        assertEquals(1L shl 32, ProbeWire.u64(1, 0))
        assertEquals(Long.MAX_VALUE, ProbeWire.u64(Int.MAX_VALUE, -1))
    }

    @Test
    fun rejectsMalformedWire() {
        val cases = listOf(
            intArrayOf(),
            intArrayOf(-1),
            intArrayOf(1),
            intArrayOf(1, 2, 0, 0, 0, 0, 0, 0),
            intArrayOf(1, 1, 0, 0),
            intArrayOf(1, 0, 0, 0, 0, 1, 0, 0),
            intArrayOf(1, 0, 0, -1, 0, 0, 0, 0),
            intArrayOf(1, 0, 0, 0, 0, -3, 0, 0),
            intArrayOf(1, 0, 0, 0, 0, Int.MAX_VALUE, Int.MAX_VALUE, Int.MAX_VALUE),
            intArrayOf(1, 0, 0, 0, 0, 1, 0, 0, 1, 256, 0),
            intArrayOf(1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 65536),
            intArrayOf(1, 0, 0, 0, 0, 1, 0, 0, 1, -1, 0),
            intArrayOf(1, 0, 0, 0, 0, 0, 1, 0, -1, 0),
            intArrayOf(0, 9),
            sharedVector + 0,
        )
        for (wire in cases) assertFailsWith<IcpException>(wire.toList().toString()) { ProbeWire.decode(wire) }
    }

    @Test
    fun seededRoundTrip() {
        val rng = XorShift(31)
        repeat(200) {
            val probes = List(rng.below(6)) {
                if (rng.below(4) == 0) {
                    ShardProbe.FAILED
                } else {
                    val n = rng.below(40)
                    ShardProbe.answered(
                        indexedUpto = rng.next() ushr 1,
                        truncated = rng.below(2) == 0,
                        localIx = IntArray(n) { rng.next().toInt() },
                        shape = IntArray(n) { rng.below(256) },
                        distance = IntArray(n) { rng.below(65536) },
                        watermark = LongArray(rng.below(5)) { rng.next() ushr 1 },
                        exact = LongArray(rng.below(9)) { rng.next() ushr 1 },
                    )
                }
            }
            val wire = ProbeWire.encode(probes)
            assertEquals(probes, ProbeWire.decode(wire))
            assertTrue(wire.size == ProbeWire.encodedSize(probes))
        }
    }
}
