package app.witness.core.designsystem.qr

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class ReedSolomonTest {
    private fun bytes(vararg v: Int) = ByteArray(v.size) { v[it].toByte() }

    @Test
    fun multiplyIsFieldMultiplication() {
        assertEquals(0, ReedSolomon.multiply(0, 7))
        assertEquals(7, ReedSolomon.multiply(1, 7))
        assertEquals(4, ReedSolomon.multiply(2, 2))
        assertEquals(0x1D, ReedSolomon.multiply(0x80, 2))
        for (a in 0..255) for (b in listOf(1, 2, 3, 0x53, 0xCA, 0xFF)) assertEquals(ReedSolomon.multiply(a, b), ReedSolomon.multiply(b, a))
    }

    @Test
    fun everyNonZeroElementHasInverse() {
        for (a in 1..255) assertEquals(1, (1..255).count { ReedSolomon.multiply(a, it) == 1 }, "a=$a")
    }

    @Test
    fun divisorOfDegreeSevenMatchesStandardGenerator() {
        val logs = intArrayOf(87, 229, 146, 149, 238, 102, 21)
        val expected = logs.map { log -> var x = 1; repeat(log) { x = ReedSolomon.multiply(x, 2) }; x }.toIntArray()
        assertContentEquals(expected, ReedSolomon.divisor(7))
    }

    @Test
    fun divisorRejectsInvalidDegree() {
        assertFailsWith<IllegalArgumentException> { ReedSolomon.divisor(0) }
        assertFailsWith<IllegalArgumentException> { ReedSolomon.divisor(256) }
    }

    @Test
    fun helloWorldOneMEccMatchesKnownVector() {
        val data = bytes(32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17)
        val out = ByteArray(10)
        ReedSolomon.remainder(data, 0, data.size, ReedSolomon.divisor(10), out, 0)
        assertContentEquals(bytes(196, 35, 39, 119, 235, 215, 231, 226, 93, 23), out)
    }

    @Test
    fun remainderHonoursOffsets() {
        val data = bytes(9, 9, 32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17, 9)
        val out = ByteArray(14)
        ReedSolomon.remainder(data, 2, 16, ReedSolomon.divisor(10), out, 3)
        assertContentEquals(bytes(196, 35, 39, 119, 235, 215, 231, 226, 93, 23), out.copyOfRange(3, 13))
    }

    @Test
    fun remainderOfZerosIsZero() {
        val out = ByteArray(16) { 1 }
        ReedSolomon.remainder(ByteArray(40), 0, 40, ReedSolomon.divisor(16), out, 0)
        assertContentEquals(ByteArray(16), out)
    }
}
