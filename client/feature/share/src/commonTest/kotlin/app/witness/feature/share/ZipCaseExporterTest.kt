package app.witness.feature.share

import app.witness.core.model.Anchor
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Edit
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationOpening
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ZipCaseExporterTest {
    private val opening = LocationOpening(ByteString.of(ByteArray(32)), 397_392_000, -1_049_903_000, 450)

    private fun names(zip: ByteArray): List<String> {
        fun u16(at: Int) = (zip[at].toInt() and 0xff) or ((zip[at + 1].toInt() and 0xff) shl 8)
        fun u32(at: Int) = u16(at) or (u16(at + 2) shl 16)
        val end = zip.size - ZipWriter.END_RECORD
        var p = u32(end + 16)
        return List(u16(end + 10)) {
            val len = u16(p + 28)
            val name = zip.copyOfRange(p + ZipWriter.CENTRAL_HEADER, p + ZipWriter.CENTRAL_HEADER + len).decodeToString()
            p += ZipWriter.CENTRAL_HEADER + len
            name
        }
    }

    private suspend fun setup(f: ShareFixture): LocalCaseId {
        val case = f.cases.create("Fender bender — Oak & 5th", TemplateId.CAR_ACCIDENT, NOW)
        f.add(capture(2, case.id))
        f.add(capture(1, case.id))
        f.add(capture(3, case.id, origin = CaptureOrigin.IMPORT))
        f.add(capture(4, null))
        return case.id
    }

    @Test
    fun zipContainsPhotosProofsCaseAndReadme() = runTest {
        val f = ShareFixture()
        val id = setup(f)
        val sink = MemorySink()
        val result = assertIs<ExportResult.Saved>(ZipCaseExporter(f.cases, f.captures, sink, f.clock).export(ExportRequest(id, ExportFormat.ZIP, false)))
        assertEquals(listOf("witness-fender-bender-oak-5th.zip" to "application/zip"), sink.opened)
        assertEquals(2, result.items)
        assertEquals("memory://export", result.location)
        val bytes = sink.last!!.toByteArray()
        assertEquals(bytes.size.toLong(), result.bytes)
        assertTrue(sink.last!!.committed)
        assertEquals(
            listOf(
                "photos/01-${record(1).text}.jpg", "proof/${record(1).text}.json",
                "photos/02-${record(2).text}.jpg", "proof/${record(2).text}.json",
                "case.json", "README.txt",
            ),
            names(bytes),
        )
    }

    @Test
    fun caseJsonRecordsSummaryProvenance() = runTest {
        val f = ShareFixture()
        val id = setup(f)
        suspend fun exported(): String {
            val sink = MemorySink()
            ZipCaseExporter(f.cases, f.captures, sink, f.clock).export(ExportRequest(id, ExportFormat.ZIP, false))
            return sink.last!!.toByteArray().decodeToString()
        }
        assertFalse("summary_source" in exported())
        f.cases.update(f.cases.get(id)!!.copy(summary = "Rear-ended", summarySource = SummarySource.WRITTEN))
        assertTrue("\"summary_source\":\"written\"" in exported())
        f.cases.update(f.cases.get(id)!!.copy(summarySource = SummarySource.GENERATED))
        assertTrue("\"summary_source\":\"generated\"" in exported())
    }

    @Test
    fun pdfIsUnsupported() = runTest {
        val f = ShareFixture()
        val sink = MemorySink()
        assertEquals(ExportResult.Unsupported, ZipCaseExporter(f.cases, f.captures, sink, f.clock).export(ExportRequest(setup(f), ExportFormat.PDF, false)))
        assertTrue(sink.opened.isEmpty())
    }

    @Test
    fun unknownCaseHasNothingToExport() = runTest {
        val f = ShareFixture()
        assertEquals(ExportResult.NothingToExport, ZipCaseExporter(f.cases, f.captures, MemorySink(), f.clock).export(ExportRequest(LocalCaseId(9), ExportFormat.ZIP, false)))
    }

    @Test
    fun caseWithOnlyImportsHasNothingToExport() = runTest {
        val f = ShareFixture()
        val case = f.cases.create("x", TemplateId.GENERAL, NOW)
        f.add(capture(1, case.id, origin = CaptureOrigin.IMPORT))
        val sink = MemorySink()
        assertEquals(ExportResult.NothingToExport, ZipCaseExporter(f.cases, f.captures, sink, f.clock).export(ExportRequest(case.id, ExportFormat.ZIP, false)))
        assertTrue(sink.opened.isEmpty())
    }

    @Test
    fun itemsWithoutImagesAreSkippedAndAllMissingAborts() = runTest {
        val f = ShareFixture()
        val case = f.cases.create("x", TemplateId.GENERAL, NOW)
        f.add(capture(1, case.id), jpeg = null)
        val sink = MemorySink()
        assertEquals(ExportResult.NothingToExport, ZipCaseExporter(f.cases, f.captures, sink, f.clock).export(ExportRequest(case.id, ExportFormat.ZIP, false)))
        assertTrue(sink.last!!.aborted)
        assertFalse(sink.last!!.committed)
    }

    @Test
    fun sinkFailureAbortsAndRethrows() = runTest {
        val f = ShareFixture()
        val id = setup(f)
        val failing = object : ExportOutput {
            var aborted = false
            override suspend fun write(bytes: ByteArray, offset: Int, length: Int) = throw IllegalStateException("disk full")
            override suspend fun commit(): String = error("unreachable")
            override suspend fun abort() {
                aborted = true
            }
        }
        assertFailsWith<IllegalStateException> { ZipCaseExporter(f.cases, f.captures, { _, _ -> failing }, f.clock).export(ExportRequest(id, ExportFormat.ZIP, false)) }
        assertTrue(failing.aborted)
    }

    @Test
    fun proofRecordRoundsLocationByDefault() {
        val f = ShareFixture()
        val c = capture(1, LocalCaseId(1))
        val json = ZipCaseExporter(f.cases, f.captures, MemorySink(), f.clock).proofRecord(c, "photos/01.jpg", opening, exact = false)
        val location = json["location"]!!.jsonObject
        assertEquals(6, location["geohash"]!!.jsonPrimitive.content.length)
        assertNull(location["lat"])
        assertEquals("https://witness.app/p/${c.recordId.text}", json["proof_url"]!!.jsonPrimitive.content)
        assertEquals("anchored", json["status"]!!.jsonPrimitive.content)
    }

    @Test
    fun proofRecordExactLocation() {
        val f = ShareFixture()
        val json = ZipCaseExporter(f.cases, f.captures, MemorySink(), f.clock).proofRecord(capture(1, null), "p", opening, exact = true)
        val location = json["location"]!!.jsonObject
        assertEquals("39.7392", location["lat"]!!.jsonPrimitive.content)
        assertEquals("-104.9903", location["lon"]!!.jsonPrimitive.content)
        assertEquals("4.5", location["accuracy_m"]!!.jsonPrimitive.content)
        assertNull(location["geohash"])
    }

    @Test
    fun proofRecordWithoutOpeningHasConfidenceOnly() {
        val f = ShareFixture()
        val json = ZipCaseExporter(f.cases, f.captures, MemorySink(), f.clock).proofRecord(capture(1, null), "p", null, exact = true)
        assertEquals(setOf("confidence"), json["location"]!!.jsonObject.keys)
    }

    @Test
    fun proofRecordIgnoresOutOfRangeOpening() {
        val f = ShareFixture()
        val bad = opening.copy(latE7 = Int.MAX_VALUE)
        val json = ZipCaseExporter(f.cases, f.captures, MemorySink(), f.clock).proofRecord(capture(1, null), "p", bad, exact = true)
        assertEquals(setOf("confidence"), json["location"]!!.jsonObject.keys)
    }

    @Test
    fun proofRecordCarriesKeyAnchorParentAndEdits() {
        val f = ShareFixture()
        val key = KeyId.fromBytes(ByteArray(KeyId.SIZE) { 1 })
        val c = capture(1, null, origin = CaptureOrigin.DERIVATIVE).copy(
            keyRef = KeyRef(key, 41),
            anchor = Anchor(7, NOW),
            parent = record(9),
            edits = listOf(Edit.Resize(10, 10)),
        )
        val json = ZipCaseExporter(f.cases, f.captures, MemorySink(), f.clock).proofRecord(c, "p", null, false)
        assertEquals(key.text, json["key_id"]!!.jsonPrimitive.content)
        assertEquals("41", json["counter"]!!.jsonPrimitive.content)
        assertEquals("7", json["anchor"]!!.jsonObject["record_ix"]!!.jsonPrimitive.content)
        assertEquals(record(9).text, json["parent"]!!.jsonPrimitive.content)
        assertEquals("derivative", json["origin"]!!.jsonPrimitive.content)
        assertTrue("resize" in json["edits"].toString())
    }

    @Test
    fun fileNameSlugs() {
        assertEquals("witness-fender-bender-oak-5th.zip", ZipCaseExporter.fileName("Fender bender — Oak & 5th", ExportFormat.ZIP))
        assertEquals("witness-case.pdf", ZipCaseExporter.fileName("—!!—", ExportFormat.PDF))
        assertEquals("witness-case.zip", ZipCaseExporter.fileName("", ExportFormat.ZIP))
        assertEquals("witness-a-b.zip", ZipCaseExporter.fileName("  A   b  ", ExportFormat.ZIP))
        assertEquals(40, ZipCaseExporter.fileName("x".repeat(200), ExportFormat.ZIP).removePrefix("witness-").removeSuffix(".zip").length)
        assertEquals("witness-dir-evil.zip", ZipCaseExporter.fileName("../dir/evil", ExportFormat.ZIP))
    }

    @Test
    fun photoNamesSortNaturally() {
        assertEquals("photos/01-${record(1).text}.jpg", ZipCaseExporter.photoName(0, capture(1, null)))
        assertEquals("photos/12-${record(1).text}.jpg", ZipCaseExporter.photoName(11, capture(1, null)))
    }
}
