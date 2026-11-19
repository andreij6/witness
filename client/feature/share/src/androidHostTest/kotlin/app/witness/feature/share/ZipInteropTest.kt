package app.witness.feature.share

import app.witness.core.model.TemplateId
import java.io.ByteArrayInputStream
import java.io.File
import java.util.zip.ZipFile
import java.util.zip.ZipInputStream
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

class ZipInteropTest {
    private class Buffer : ByteSink {
        val out = java.io.ByteArrayOutputStream()
        override suspend fun write(bytes: ByteArray, offset: Int, length: Int) = out.write(bytes, offset, length)
    }

    @Test
    fun javaZipInputStreamReadsEntriesAndValidatesCrc() = runTest {
        val buf = Buffer()
        val payloads = linkedMapOf("photos/01.jpg" to ByteArray(70_000) { (it * 13).toByte() }, "proof/a.json" to "{\"a\":1}".encodeToByteArray(), "empty" to ByteArray(0))
        ZipWriter(buf).apply {
            payloads.forEach { (name, data) -> add(name, data) }
            finish()
        }
        val read = LinkedHashMap<String, ByteArray>()
        ZipInputStream(ByteArrayInputStream(buf.out.toByteArray())).use { zin ->
            while (true) {
                val entry = zin.nextEntry ?: break
                read[entry.name] = zin.readBytes()
            }
        }
        assertEquals(payloads.keys.toList(), read.keys.toList())
        payloads.forEach { (name, data) -> assertContentEquals(data, read[name]) }
    }

    @Test
    fun javaZipFileReadsCentralDirectory() = runTest {
        val f = ShareFixture()
        val case = f.cases.create("Fender bender", TemplateId.CAR_ACCIDENT, NOW)
        for (n in 1..3) f.add(capture(n, case.id))
        val sink = MemorySink()
        val saved = assertIs<ExportResult.Saved>(ZipCaseExporter(f.cases, f.captures, sink, f.clock).export(ExportRequest(case.id, ExportFormat.ZIP, false)))
        val file = File.createTempFile("witness", ".zip").apply { deleteOnExit() }
        file.writeBytes(sink.last!!.toByteArray())
        ZipFile(file).use { zip ->
            val names = zip.entries().toList().map { it.name }
            assertEquals(3 * 2 + 2, names.size)
            assertTrue(names.containsAll(listOf("case.json", "README.txt")))
            val case = zip.getInputStream(zip.getEntry("case.json")).readBytes().decodeToString()
            assertTrue("\"title\":\"Fender bender\"" in case)
            assertEquals(saved.bytes, file.length())
            zip.entries().toList().forEach { assertEquals(java.util.zip.ZipEntry.STORED, it.method) }
        }
    }
}
