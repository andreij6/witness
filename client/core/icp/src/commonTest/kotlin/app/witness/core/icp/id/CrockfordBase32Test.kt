package app.witness.core.icp.id

import app.witness.core.icp.XorShift
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CrockfordBase32Test {
    private fun decodeError(text: String, len: Int): Base32Exception? = CrockfordBase32.decodeInto(text, ByteArray(len))

    @Test
    fun encodedLengthMatchesRustBitCount() {
        assertEquals(0, CrockfordBase32.encodedLength(0))
        assertEquals(2, CrockfordBase32.encodedLength(1))
        assertEquals(8, CrockfordBase32.encodedLength(5))
        assertEquals(16, CrockfordBase32.encodedLength(10))
        assertEquals(26, CrockfordBase32.encodedLength(16))
    }

    @Test
    fun goldenVectorsSharedWithRust() {
        assertEquals("00000000", CrockfordBase32.encode(ByteArray(5)))
        assertEquals("ZZZZZZZZ", CrockfordBase32.encode(ByteArray(5) { -1 }))
        assertEquals("Z0", CrockfordBase32.encode(byteArrayOf(0xf8.toByte())))
        assertEquals("12345678", CrockfordBase32.encode(byteArrayOf(0x08, 0x86.toByte(), 0x42, 0x98.toByte(), 0xe8.toByte())))
        assertEquals("01234567", CrockfordBase32.encode(byteArrayOf(0x00, 0x44, 0x32, 0x14, 0xc7.toByte())))
    }

    @Test
    fun decodeIsCaseInsensitiveAndMapsConfusables() {
        assertContentEquals(CrockfordBase32.decode("ABCDEFGH", 5), CrockfordBase32.decode("abcdefgh", 5))
        assertContentEquals(CrockfordBase32.decode("11111111", 5), CrockfordBase32.decode("1I1L1i1l", 5))
        assertContentEquals(ByteArray(5), CrockfordBase32.decode("0OoO0000", 5))
    }

    @Test
    fun decodeToleratesHyphensAnywhere() {
        val want = byteArrayOf(0x08, 0x86.toByte(), 0x42, 0x98.toByte(), 0xe8.toByte())
        assertContentEquals(want, CrockfordBase32.decode("-1234-5678-", 5))
        assertContentEquals(want, CrockfordBase32.decode("1-2-3-4-5-6-7-8", 5))
    }

    @Test
    fun decodeRejectsBadSymbols() {
        for (bad in listOf('U', 'u', ' ', '=', '*', '\u0000', '\u0080', 'é', 'ÿ', '￿')) {
            val text = "123" + bad + "5678"
            val e = decodeError(text, 5)
            assertEquals(Base32Error.SYMBOL, e?.error, "symbol $bad")
            assertEquals(bad, e?.symbol)
        }
    }

    @Test
    fun decodeRejectsWrongLength() {
        for (text in listOf("1234567", "123456789", "", "--------")) {
            assertEquals(Base32Error.LENGTH, decodeError(text, 5)?.error, text)
        }
    }

    @Test
    fun decodeRejectsNonZeroPadding() {
        assertNull(decodeError("Z0", 1))
        assertEquals(Base32Error.PADDING, decodeError("Z1", 1)?.error)
        assertEquals(Base32Error.PADDING, decodeError("Z3", 1)?.error)
        assertContentEquals(byteArrayOf(0xf9.toByte()), CrockfordBase32.decode("Z4", 1))
    }

    @Test
    fun emptyRoundTrip() {
        assertEquals("", CrockfordBase32.encode(ByteArray(0)))
        assertNull(decodeError("", 0))
        assertNull(decodeError("---", 0))
    }

    @Test
    fun decodeThrowsTypedException() {
        val e = assertFailsWith<Base32Exception> { CrockfordBase32.decode("U", 1) }
        assertEquals(Base32Error.SYMBOL, e.error)
        assertTrue(e.message!!.isNotEmpty())
    }

    @Test
    fun seededRoundTripAllLengths() {
        val rng = XorShift(0x7e37_79b9_7f4a_7c15)
        for (len in 0 until 40) {
            repeat(50) {
                val bytes = rng.bytes(len)
                val text = CrockfordBase32.encode(bytes)
                assertEquals(CrockfordBase32.encodedLength(len), text.length)
                assertTrue(text.all { it in CrockfordBase32.ALPHABET })
                assertContentEquals(bytes, CrockfordBase32.decode(text, len))
                assertContentEquals(bytes, CrockfordBase32.decode(text.lowercase(), len))
            }
        }
    }

    @Test
    fun encodingPreservesByteOrder() {
        var prev = CrockfordBase32.encode(ByteArray(10))
        var v = 0L
        for (step in 1L until 200L) {
            v += step * 0x1234_5678_9abcL
            val b = ByteArray(10)
            for (i in 0 until 8) b[9 - i] = (v ushr (8 * i)).toByte()
            val cur = CrockfordBase32.encode(b)
            assertTrue(cur > prev, "$cur > $prev")
            prev = cur
        }
    }

    @Test
    fun encodeIsDeterministic() {
        val b = XorShift(9).bytes(64)
        assertEquals(CrockfordBase32.encode(b), CrockfordBase32.encode(b.copyOf()))
    }
}
