package app.witness.core.model

import kotlin.random.Random
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class IdsTest {
    private val emptySha256 = ByteString.ofHex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855").toByteArray()

    @Test
    fun captureIdPacksSchemaAndRandom() {
        val id = assertNotNull(CaptureId.of(0x00ab_cdef_0123_4567, 0x1f))
        assertEquals(0x1fab_cdef_0123_4567, id.value)
        assertEquals(0x1f, id.schema)
        assertEquals(0x00ab_cdef_0123_4567, id.random)
        assertEquals(0L, CaptureId.of(0, 0)?.value)
        assertEquals((1L shl 61) - 1, CaptureId.MAX)
    }

    @Test
    fun captureIdRejectsOutOfRange() {
        assertNull(CaptureId.of(1L shl 56, 0))
        assertNull(CaptureId.of(-1, 0))
        assertNull(CaptureId.of(0, 32))
        assertNull(CaptureId.of(0, -1))
        assertNull(CaptureId.fromLong(1L shl 61))
        assertNull(CaptureId.fromLong(-1))
        assertNull(CaptureId.fromLong(Long.MAX_VALUE))
    }

    @Test
    fun captureIdFromRandomBytes() {
        val id = assertNotNull(CaptureId.fromRandomBytes(byteArrayOf(0, 1, 2, 3, 4, 5, 6, 7).copyOfRange(1, 8), 3))
        assertEquals(0x0001_0203_0405_0607, id.random)
        assertEquals(3, id.schema)
        assertNull(CaptureId.fromRandomBytes(ByteArray(7) { -1 }, 32))
        assertNull(CaptureId.fromRandomBytes(ByteArray(6), 0))
        assertNull(CaptureId.fromRandomBytes(ByteArray(8), 0))
        assertEquals(CaptureId.RANDOM_MASK, CaptureId.fromRandomBytes(ByteArray(7) { -1 }, 0)?.random)
    }

    @Test
    fun captureIdPropertyRoundTrip() {
        val rnd = Random(42)
        repeat(500) {
            val r = rnd.nextLong(0, CaptureId.RANDOM_MASK + 1)
            val s = rnd.nextInt(0, 32)
            val id = assertNotNull(CaptureId.of(r, s))
            assertEquals(r, id.random)
            assertEquals(s, id.schema)
            assertEquals(id, CaptureId.fromLong(id.value))
        }
    }

    @Test
    fun recordIdGoldenMatchesRust() {
        val id = RecordId.fromDigest(emptySha256)
        assertContentEquals(emptySha256.copyOf(10), id.toBytes())
        assertEquals("WERC8GMRZGE196QV", id.text)
        assertEquals("WERC8GMRZGE196QV", id.toString())
        assertEquals("WERC-8GMR-ZGE1-96QV", id.grouped())
        assertEquals(RecordId.GROUPED_LENGTH, id.grouped().length)
        assertEquals("0000000000000000", RecordId.fromBytes(ByteArray(10)).text)
        assertEquals("ZZZZZZZZZZZZZZZZ", RecordId.fromBytes(ByteArray(10) { -1 }).text)
    }

    @Test
    fun recordIdParseVariants() {
        val id = RecordId.fromDigest(emptySha256)
        for (text in listOf(
            "WERC8GMRZGE196QV",
            "werc-8gmr-zge1-96qv",
            "WERC-8GMR-ZGEI-96QV",
            "WERC-8GMR-ZGEL-96QV",
            "wErC8gMrZgEl96Qv",
            "-WERC--8GMR-ZGE1-96QV-",
        )) {
            assertEquals(id, RecordId.parse(text), text)
            assertEquals(id, RecordId.parseOrNull(text), text)
        }
        assertEquals(RecordId.fromBytes(ByteArray(10)), RecordId.parse("0OOO-oooo-0000-0000"))
    }

    @Test
    fun recordIdParseRejects() {
        assertEquals(Base32Error.LENGTH, assertFailsWith<IdFormatException> { RecordId.parse("WERC8GMRZGE196Q") }.error)
        assertEquals(Base32Error.LENGTH, assertFailsWith<IdFormatException> { RecordId.parse("WERC8GMRZGE196QVV") }.error)
        assertEquals(Base32Error.SYMBOL, assertFailsWith<IdFormatException> { RecordId.parse("WERC8GMRZGE196QU") }.error)
        assertEquals(Base32Error.SYMBOL, assertFailsWith<IdFormatException> { RecordId.parse("WERC 8GMR ZGE1 96QV") }.error)
        assertEquals(Base32Error.LENGTH, assertFailsWith<IdFormatException> { RecordId.parse("") }.error)
        assertNull(RecordId.parseOrNull("é234567890123456"))
        assertNull(CaseId.parseOrNull("Z".repeat(26)))
        assertEquals(Base32Error.PADDING, assertFailsWith<IdFormatException> { KeyId.parse("Z".repeat(26)) }.error)
    }

    @Test
    fun recordIdRequiresExactSizes() {
        assertFailsWith<IllegalArgumentException> { RecordId.fromBytes(ByteArray(9)) }
        assertFailsWith<IllegalArgumentException> { RecordId.fromBytes(ByteArray(11)) }
        assertFailsWith<IllegalArgumentException> { RecordId.fromDigest(ByteArray(9)) }
    }

    @Test
    fun recordIdPropertyRoundTrip() {
        val rnd = Random(9)
        repeat(300) {
            val b = rnd.nextBytes(10)
            val id = RecordId.fromBytes(b)
            assertEquals(RecordId.TEXT_LENGTH, id.text.length)
            assertContentEquals(b, id.toBytes())
            assertEquals(id, RecordId.parse(id.text))
            assertEquals(id, RecordId.parse(id.grouped()))
            assertEquals(id, RecordId.parse(id.text.lowercase()))
        }
    }

    @Test
    fun recordIdOrderingFollowsBytes() {
        val a = RecordId.fromBytes(ByteArray(10).also { it[9] = 1 })
        val b = RecordId.fromBytes(ByteArray(10).also { it[0] = 1 })
        assertTrue(a < b)
    }

    @Test
    fun keyIdRoundTripsAndShortens() {
        val bytes = emptySha256.copyOf(16)
        val id = KeyId.fromBytes(bytes)
        assertEquals(KeyId.TEXT_LENGTH, id.text.length)
        assertContentEquals(bytes, id.toBytes())
        assertEquals(id, KeyId.fromDigest(emptySha256))
        assertEquals(id, KeyId.parse(id.text.lowercase()))
        assertEquals(id.text.take(8), id.short())
        assertNull(KeyId.parseOrNull("short"))
        assertFailsWith<IllegalArgumentException> { KeyId.fromBytes(ByteArray(10)) }
        assertFailsWith<IllegalArgumentException> { KeyId.fromDigest(ByteArray(15)) }
    }

    @Test
    fun caseIdEncodesShard() {
        val random = ByteArray(15) { (it + 1).toByte() }
        for (shard in listOf(0, 1, 7, 128, 255)) {
            val id = CaseId.of(shard, random)
            assertEquals(shard, id.shard)
            assertEquals(CaseId.TEXT_LENGTH, id.text.length)
            assertContentEquals(random, id.toBytes().copyOfRange(1, 16))
            assertEquals(id, CaseId.parse(id.grouped()))
            assertEquals(id, CaseId.fromBytes(id.toBytes()))
        }
        assertFailsWith<IllegalArgumentException> { CaseId.of(256, random) }
        assertFailsWith<IllegalArgumentException> { CaseId.of(-1, random) }
        assertFailsWith<IllegalArgumentException> { CaseId.of(0, ByteArray(14)) }
        assertFailsWith<IllegalArgumentException> { CaseId.fromBytes(ByteArray(15)) }
        assertNull(CaseId.parseOrNull("not-a-case"))
    }

    @Test
    fun localCaseIdOrders() {
        assertTrue(LocalCaseId(1) < LocalCaseId(2))
        assertEquals(LocalCaseId(3), LocalCaseId(3))
    }
}
