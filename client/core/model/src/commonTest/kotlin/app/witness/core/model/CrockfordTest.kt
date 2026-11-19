package app.witness.core.model

import kotlin.random.Random
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CrockfordTest {
    private fun bytes(vararg v: Int) = ByteArray(v.size) { v[it].toByte() }

    @Test
    fun encodedLengthMatchesBitCount() {
        assertEquals(0, Crockford.encodedLength(0))
        assertEquals(2, Crockford.encodedLength(1))
        assertEquals(8, Crockford.encodedLength(5))
        assertEquals(16, Crockford.encodedLength(10))
        assertEquals(26, Crockford.encodedLength(16))
    }

    @Test
    fun goldenVectorsMatchRust() {
        assertEquals("00000000", Crockford.encode(ByteArray(5)))
        assertEquals("ZZZZZZZZ", Crockford.encode(ByteArray(5) { -1 }))
        assertEquals("Z0", Crockford.encode(bytes(0xf8)))
        assertEquals("12345678", Crockford.encode(bytes(0x08, 0x86, 0x42, 0x98, 0xe8)))
        assertEquals("01234567", Crockford.encode(bytes(0x00, 0x44, 0x32, 0x14, 0xc7)))
    }

    @Test
    fun decodeIsCaseInsensitiveAndMapsConfusables() {
        assertContentEquals(Crockford.decode("ABCDEFGH", 5), Crockford.decode("abcdefgh", 5))
        assertContentEquals(Crockford.decode("11111111", 5), Crockford.decode("1I1L1i1l", 5))
        assertContentEquals(ByteArray(5), Crockford.decode("0OoO0000", 5))
    }

    @Test
    fun decodeToleratesHyphensAnywhere() {
        val want = bytes(0x08, 0x86, 0x42, 0x98, 0xe8)
        assertContentEquals(want, Crockford.decode("-1234-5678-", 5))
        assertContentEquals(want, Crockford.decode("1-2-3-4-5-6-7-8", 5))
    }

    @Test
    fun decodeRejectsBadSymbols() {
        for (bad in listOf('U', 'u', ' ', '=', '*', 'é', '\u0000', 'ÿ', '中')) {
            val text = "123${bad}5678"
            val e = assertFailsWith<IdFormatException> { Crockford.decode(text, 5) }
            assertEquals(Base32Error.SYMBOL, e.error)
        }
    }

    @Test
    fun decodeRejectsWrongLength() {
        for (text in listOf("1234567", "123456789", "", "--------")) {
            val e = assertFailsWith<IdFormatException> { Crockford.decode(text, 5) }
            assertEquals(Base32Error.LENGTH, e.error, text)
        }
    }

    @Test
    fun decodeRejectsNonZeroPadding() {
        assertContentEquals(bytes(0xf8), Crockford.decode("Z0", 1))
        assertEquals(Base32Error.PADDING, assertFailsWith<IdFormatException> { Crockford.decode("Z1", 1) }.error)
        assertEquals(Base32Error.PADDING, assertFailsWith<IdFormatException> { Crockford.decode("Z3", 1) }.error)
        assertContentEquals(bytes(0xf9), Crockford.decode("Z4", 1))
    }

    @Test
    fun emptyRoundTrips() {
        assertEquals("", Crockford.encode(ByteArray(0)))
        assertContentEquals(ByteArray(0), Crockford.decode("", 0))
        assertContentEquals(ByteArray(0), Crockford.decode("---", 0))
    }

    @Test
    fun seededRoundTripAllLengths() {
        val rnd = Random(0x5eed)
        for (len in 0 until 40) {
            repeat(25) {
                val b = rnd.nextBytes(len)
                val text = Crockford.encode(b)
                assertEquals(Crockford.encodedLength(len), text.length)
                assertTrue(text.all { it in Crockford.ALPHABET })
                assertContentEquals(b, Crockford.decode(text, len))
                assertContentEquals(b, Crockford.decode(text.lowercase(), len))
            }
        }
    }

    @Test
    fun encodingPreservesByteOrder() {
        val rnd = Random(7)
        val values = List(200) { rnd.nextBytes(10) }
        val byBytes = values.sortedWith { a, b -> ByteString.wrap(a).compareTo(ByteString.wrap(b)) }.map(Crockford::encode)
        assertEquals(byBytes.sorted(), byBytes)
    }

    @Test
    fun canonicalOrNull() {
        assertEquals("12345678", Crockford.canonicalOrNull("1234-5678", 5))
        assertEquals("10000000", Crockford.canonicalOrNull("i0oO0000", 5))
        assertNull(Crockford.canonicalOrNull("1234567U", 5))
        assertNull(Crockford.canonicalOrNull("123", 5))
    }

    @Test
    fun grouping() {
        assertEquals("WERC-8GMR-ZGE1-96QV", Crockford.group("WERC8GMRZGE196QV"))
        assertEquals("ABCD-EF", Crockford.group("ABCDEF"))
        assertEquals("ABCD", Crockford.group("ABCD"))
        assertEquals("", Crockford.group(""))
        assertEquals("AB-CD-E", Crockford.group("ABCDE", 2))
    }

    @Test
    fun deterministic() {
        val b = Random(1).nextBytes(16)
        assertEquals(Crockford.encode(b), Crockford.encode(b.copyOf()))
    }
}
