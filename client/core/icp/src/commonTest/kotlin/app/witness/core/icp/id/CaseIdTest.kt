package app.witness.core.icp.id

import app.witness.core.icp.XorShift
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class CaseIdTest {
    @Test
    fun layoutAndTextSharedWithRust() {
        val id = CaseId.of(3, ByteArray(15) { 0xaa.toByte() })
        assertEquals(3, id.shard)
        assertEquals(3, id.bytes[0].toInt())
        assertContentEquals(ByteArray(15) { 0xaa.toByte() }, id.bytes.copyOfRange(1, 16))
        assertEquals(CaseId.TEXT_LEN, id.text.length)
        assertEquals(id, CaseId.parse(id.text))
        assertEquals("00000000000000000000000000", CaseId.of(0, ByteArray(15)).text)
        assertEquals("ZZZZZZZZZZZZZZZZZZZZZZZZZW", CaseId.of(255, ByteArray(15) { -1 }).text)
    }

    @Test
    fun parseRejectsNonZeroPadding() {
        assertNull(CaseId.parse("ZZZZZZZZZZZZZZZZZZZZZZZZZZ"))
        assertEquals(Base32Error.PADDING, assertFailsWith<Base32Exception> { CaseId.parseOrThrow("ZZZZZZZZZZZZZZZZZZZZZZZZZZ") }.error)
    }

    @Test
    fun rejectsBadInputs() {
        assertFailsWith<IllegalArgumentException> { CaseId.of(256, ByteArray(15)) }
        assertFailsWith<IllegalArgumentException> { CaseId.of(-1, ByteArray(15)) }
        assertFailsWith<IllegalArgumentException> { CaseId.of(0, ByteArray(14)) }
        assertFailsWith<IllegalArgumentException> { CaseId.of(ByteArray(17)) }
        assertNull(CaseId.parse("0000000000000000000000000"))
        assertNull(CaseId.parse("U0000000000000000000000000"))
    }

    @Test
    fun shardPropertyForEveryByte() {
        val rng = XorShift(23)
        for (shard in 0..255) {
            val id = CaseId.of(shard, rng.bytes(15))
            assertEquals(shard, id.shard)
            assertEquals(id, CaseId.parse(id.text))
            assertEquals(shard, CaseId.parse(id.text.lowercase())!!.shard)
        }
    }
}
