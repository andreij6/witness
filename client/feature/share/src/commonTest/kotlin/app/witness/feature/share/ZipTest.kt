package app.witness.feature.share

import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class Crc32Test {
    @Test
    fun emptyIsZero() = assertEquals(0, Crc32.of(ByteArray(0)))

    @Test
    fun checkVector() = assertEquals(0xCBF43926.toInt(), Crc32.of("123456789".encodeToByteArray()))

    @Test
    fun singleByte() = assertEquals(0xE8B7BE43.toInt(), Crc32.of("a".encodeToByteArray()))

    @Test
    fun incrementalEqualsOneShot() {
        val data = ByteArray(1_000) { (it * 31).toByte() }
        assertEquals(Crc32.of(data), Crc32.update(Crc32.update(0, data, 0, 400), data, 400, 600))
    }

    @Test
    fun rangeIsChecked() {
        assertFailsWith<IllegalArgumentException> { Crc32.update(0, ByteArray(4), 2, 3) }
        assertFailsWith<IllegalArgumentException> { Crc32.update(0, ByteArray(4), -1, 1) }
    }

    @Test
    fun sensitiveToEveryBit() {
        val a = ByteArray(64)
        val base = Crc32.of(a)
        for (i in 0 until 64 * 8) {
            val b = a.copyOf()
            b[i / 8] = (b[i / 8].toInt() xor (1 shl (i % 8))).toByte()
            assertTrue(Crc32.of(b) != base, "bit $i")
        }
    }
}

class ZipWriterTest {
    private class Buffer : ByteSink {
        val out = ArrayList<Byte>()
        override suspend fun write(bytes: ByteArray, offset: Int, length: Int) {
            for (i in offset until offset + length) out += bytes[i]
        }
        fun bytes() = out.toByteArray()
    }

    private fun u16(b: ByteArray, at: Int) = (b[at].toInt() and 0xff) or ((b[at + 1].toInt() and 0xff) shl 8)
    private fun u32(b: ByteArray, at: Int) = u16(b, at) or (u16(b, at + 2) shl 16)

    @Test
    fun emptyArchiveIsJustEndRecord() = runTest {
        val buf = Buffer()
        ZipWriter(buf).finish()
        val b = buf.bytes()
        assertEquals(ZipWriter.END_RECORD, b.size)
        assertEquals(ZipWriter.END_SIG, u32(b, 0))
    }

    @Test
    fun layoutOfSingleStoredEntry() = runTest {
        val buf = Buffer()
        val zip = ZipWriter(buf)
        val data = "hello".encodeToByteArray()
        zip.add("a.txt", data)
        zip.finish()
        val b = buf.bytes()
        assertEquals(ZipWriter.LOCAL_SIG, u32(b, 0))
        assertEquals(ZipWriter.METHOD_STORE, u16(b, 8))
        assertEquals(Crc32.of(data), u32(b, 14))
        assertEquals(5, u32(b, 18))
        assertEquals(5, u32(b, 22))
        assertEquals(5, u16(b, 26))
        assertContentEquals("a.txt".encodeToByteArray(), b.copyOfRange(30, 35))
        assertContentEquals(data, b.copyOfRange(35, 40))
        val central = 40
        assertEquals(ZipWriter.CENTRAL_SIG, u32(b, central))
        assertEquals(0, u32(b, central + 42))
        val end = b.size - ZipWriter.END_RECORD
        assertEquals(ZipWriter.END_SIG, u32(b, end))
        assertEquals(1, u16(b, end + 10))
        assertEquals(central, u32(b, end + 16))
        assertEquals(end - central, u32(b, end + 12))
        assertEquals(b.size.toLong(), zip.bytesWritten)
    }

    @Test
    fun centralDirectoryPointsAtEachLocalHeader() = runTest {
        val buf = Buffer()
        val zip = ZipWriter(buf)
        zip.add("one", ByteArray(3))
        zip.add("dir/two", ByteArray(7))
        zip.finish()
        val b = buf.bytes()
        val end = b.size - ZipWriter.END_RECORD
        var p = u32(b, end + 16)
        val offsets = ArrayList<Int>()
        repeat(u16(b, end + 10)) {
            assertEquals(ZipWriter.CENTRAL_SIG, u32(b, p))
            offsets += u32(b, p + 42)
            p += ZipWriter.CENTRAL_HEADER + u16(b, p + 28)
        }
        assertEquals(listOf(0, 30 + 3 + 3), offsets)
        offsets.forEach { assertEquals(ZipWriter.LOCAL_SIG, u32(b, it)) }
    }

    @Test
    fun outputIsDeterministic() = runTest {
        suspend fun build(): ByteArray {
            val buf = Buffer()
            ZipWriter(buf, 123, 456).apply {
                add("x", byteArrayOf(1, 2, 3))
                finish()
            }
            return buf.bytes()
        }
        assertContentEquals(build(), build())
    }

    @Test
    fun rejectsDuplicateNames() = runTest {
        val zip = ZipWriter(Buffer())
        zip.add("a", ByteArray(0))
        assertFailsWith<IllegalArgumentException> { zip.add("a", ByteArray(0)) }
    }

    @Test
    fun rejectsAddAfterFinishAndDoubleFinish() = runTest {
        val zip = ZipWriter(Buffer())
        zip.finish()
        assertFailsWith<IllegalStateException> { zip.add("a", ByteArray(0)) }
        assertFailsWith<IllegalStateException> { zip.finish() }
    }

    @Test
    fun rejectsPathTraversal() = runTest {
        val zip = ZipWriter(Buffer())
        listOf("", "/abs", "../up", "a/../b", "a//b", "a\\b", "./a", "a/", "bad\u0000name").forEach { name ->
            assertFailsWith<IllegalArgumentException>(name) { zip.add(name, ByteArray(0)) }
        }
    }

    @Test
    fun safeNames() {
        assertTrue(ZipWriter.isSafeName("photos/01-ABC.jpg"))
        assertTrue(ZipWriter.isSafeName("README.txt"))
        assertFalse(ZipWriter.isSafeName("x".repeat(256)))
    }

    @Test
    fun utf8NamesSetFlag() = runTest {
        val buf = Buffer()
        ZipWriter(buf).apply {
            add("café.txt", ByteArray(0))
            finish()
        }
        val b = buf.bytes()
        assertEquals(ZipWriter.FLAG_UTF8, u16(b, 6))
        assertEquals("café.txt".encodeToByteArray().size, u16(b, 26))
    }

    @Test
    fun dosDateTimeEncodesFields() {
        val (time, date) = ZipWriter.dosDateTime(2025, 11, 19, 14, 32, 7)
        assertEquals((14 shl 11) or (32 shl 5) or 3, time)
        assertEquals((45 shl 9) or (11 shl 5) or 19, date)
    }

    @Test
    fun dosDateTimeClampsBefore1980() =
        assertEquals(ZipWriter.DOS_EPOCH_TIME to ZipWriter.DOS_EPOCH_DATE, ZipWriter.dosDateTime(1979, 12, 31, 23, 59, 59))

    @Test
    fun dosStampFromEpochMillis() =
        assertEquals(ZipWriter.dosDateTime(2025, 11, 19, 14, 32, 0), ZipCaseExporter.dosStamp(NOW))
}
