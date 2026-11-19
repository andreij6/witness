package app.witness.core.model

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class ByteStringTest {
    @Test
    fun equalityIsStructural() {
        val a = ByteString.of(byteArrayOf(1, 2, 3))
        val b = ByteString.of(byteArrayOf(1, 2, 3))
        assertEquals(a, b)
        assertEquals(a.hashCode(), b.hashCode())
        assertNotEquals(a, ByteString.of(byteArrayOf(1, 2)))
        assertNotEquals<Any>(a, byteArrayOf(1, 2, 3))
    }

    @Test
    fun ofCopiesInputAndOutput() {
        val src = byteArrayOf(1, 2, 3)
        val s = ByteString.of(src)
        src[0] = 9
        assertEquals(1, s[0].toInt())
        val out = s.toByteArray()
        out[1] = 9
        assertEquals(2, s[1].toInt())
    }

    @Test
    fun hexRoundTrip() {
        val s = ByteString.ofHex("00ff10Ab")
        assertEquals("00ff10ab", s.hex())
        assertEquals(4, s.size)
        assertEquals(s, ByteString.ofHex(s.hex()))
        assertEquals("", ByteString.EMPTY.hex())
    }

    @Test
    fun hexRejectsMalformed() {
        assertFailsWith<IllegalArgumentException> { ByteString.ofHex("abc") }
        assertFailsWith<IllegalArgumentException> { ByteString.ofHex("zz") }
    }

    @Test
    fun compareIsUnsignedLexicographic() {
        assertTrue(ByteString.ofHex("7f") < ByteString.ofHex("80"))
        assertTrue(ByteString.ofHex("00") < ByteString.ofHex("0000"))
        assertTrue(ByteString.ofHex("ff") > ByteString.ofHex("00ff"))
        assertEquals(0, ByteString.ofHex("0102").compareTo(ByteString.ofHex("0102")))
    }

    @Test
    fun copyIntoOffsets() {
        val dst = ByteArray(5)
        ByteString.ofHex("0102").copyInto(dst, 3)
        assertContentEquals(byteArrayOf(0, 0, 0, 1, 2), dst)
    }

    @Test
    fun toStringDoesNotLeakContent() {
        assertEquals("ByteString(2)", ByteString.ofHex("dead").toString())
    }
}
