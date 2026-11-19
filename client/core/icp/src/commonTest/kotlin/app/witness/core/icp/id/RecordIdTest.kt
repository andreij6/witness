package app.witness.core.icp.id

import app.witness.core.icp.XorShift
import app.witness.core.icp.hex
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class RecordIdTest {
    private val emptyClaimId = RecordId.of(hex("e3b0c44298fc1c149afb"))

    @Test
    fun goldenSharedWithRust() {
        assertEquals("WERC8GMRZGE196QV", emptyClaimId.text)
        assertEquals("WERC8GMRZGE196QV", emptyClaimId.toString())
        assertEquals("WERC-8GMR-ZGE1-96QV", emptyClaimId.grouped)
        assertEquals("0000000000000000", RecordId.of(ByteArray(10)).text)
        assertEquals("ZZZZZZZZZZZZZZZZ", RecordId.of(ByteArray(10) { -1 }).text)
        assertContentEquals(hex("e3b0c44298fc1c149afb"), emptyClaimId.bytes)
    }

    @Test
    fun parseVariantsResolveToCanonical() {
        for (text in listOf(
            "WERC8GMRZGE196QV",
            "werc-8gmr-zge1-96qv",
            "WERC-8GMR-ZGEI-96QV",
            "WERC-8GMR-ZGEL-96QV",
            "wErC8gMrZgEl96Qv",
            "-WERC--8GMR-ZGE1-96QV-",
        )) {
            assertEquals(emptyClaimId, RecordId.parse(text), text)
        }
        assertEquals(RecordId.of(ByteArray(10)), RecordId.parse("0OOO-oooo-0000-0000"))
    }

    @Test
    fun parseRejectsMalformed() {
        for (text in listOf("WERC8GMRZGE196Q", "WERC8GMRZGE196QVV", "WERC8GMRZGE196QU", "WERC 8GMR ZGE1 96QV", "", "é234567890123456")) {
            assertNull(RecordId.parse(text), text)
        }
        assertEquals(Base32Error.LENGTH, assertFailsWith<Base32Exception> { RecordId.parseOrThrow("WERC8GMRZGE196Q") }.error)
        assertEquals(Base32Error.SYMBOL, assertFailsWith<Base32Exception> { RecordId.parseOrThrow("WERC8GMRZGE196QU") }.error)
    }

    @Test
    fun ofRejectsWrongLength() {
        assertFailsWith<IllegalArgumentException> { RecordId.of(ByteArray(9)) }
        assertFailsWith<IllegalArgumentException> { RecordId.of(ByteArray(11)) }
    }

    @Test
    fun propertyRoundTrip() {
        val rng = XorShift(21)
        repeat(2000) {
            val id = RecordId.of(rng.bytes(10))
            assertEquals(RecordId.TEXT_LEN, id.text.length)
            assertEquals(RecordId.GROUPED_LEN, id.grouped.length)
            assertEquals(id, RecordId.parse(id.text))
            assertEquals(id, RecordId.parse(id.grouped))
            assertEquals(id, RecordId.parse(id.text.lowercase()))
            assertEquals(id, RecordId.of(id.bytes))
        }
    }

    @Test
    fun textOrderMatchesByteOrder() {
        val rng = XorShift(22)
        repeat(1000) {
            val a = rng.bytes(10)
            val b = rng.bytes(10)
            val byteOrder = (0 until 10).firstOrNull { a[it] != b[it] }?.let { (a[it].toInt() and 0xff).compareTo(b[it].toInt() and 0xff) } ?: 0
            assertEquals(byteOrder.coerceIn(-1, 1), RecordId.of(a).text.compareTo(RecordId.of(b).text).coerceIn(-1, 1))
        }
    }
}
