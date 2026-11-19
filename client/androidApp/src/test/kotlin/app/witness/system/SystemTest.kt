package app.witness.system

import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Dims
import app.witness.core.model.LocationConfidence
import app.witness.core.model.RecordId
import app.witness.feature.check.PickedImage
import app.witness.feature.settings.ExportOutcome
import app.witness.feature.share.ExportOutput
import app.witness.feature.share.ExportSink
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.util.zip.ZipInputStream
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class SharedImagesTest {
    private fun image(n: Int) = PickedImage(ByteString.of(byteArrayOf(n.toByte())))

    @Test
    fun putReturnsDistinctIdsAndGetFindsThem() {
        val s = SharedImages()
        val a = image(1)
        val b = image(2)
        val ia = s.put(a)
        val ib = s.put(b)
        assertNotEquals(ia, ib)
        assertSame(a, s.get(ia))
        assertSame(b, s.get(ib))
    }

    @Test
    fun evictsTheOldestBeyondCapacity() {
        val s = SharedImages(capacity = 2)
        val first = s.put(image(1))
        s.put(image(2))
        s.put(image(3))
        assertNull(s.get(first))
        assertEquals(2, s.size)
    }

    @Test
    fun capacityMustBePositive() {
        assertFailsWith<IllegalArgumentException> { SharedImages(0) }
    }

    @Test
    fun unknownIdIsNull() {
        assertNull(SharedImages().get("nope"))
        assertNull(SharedImages().get(SharedImages.MISSING))
    }

    @Test
    fun stashStoresReadableImages() {
        val s = SharedImages()
        val img = image(4)
        assertSame(img, s.get(s.stash { img }))
    }

    @Test
    fun stashTurnsUnreadableImagesIntoTheMissingId() {
        val s = SharedImages()
        assertEquals(SharedImages.MISSING, s.stash { null })
        assertEquals(SharedImages.MISSING, s.stash { throw SecurityException("no grant") })
        assertEquals(SharedImages.MISSING, s.stash { throw IOException("gone") })
        assertEquals(SharedImages.MISSING, s.stash { throw IllegalArgumentException("bad uri") })
        assertEquals(0, s.size)
    }

    @Test
    fun stashDoesNotHideProgrammingErrors() {
        assertFailsWith<IllegalStateException> { SharedImages().stash { error("bug") } }
    }

    @Test
    fun manyPutsStayBounded() {
        val s = SharedImages(capacity = 3)
        val ids = (0 until 1000).map { s.put(image(it)) }
        assertEquals(1000, ids.toSet().size)
        assertEquals(3, s.size)
    }
}

class SafeNameTest {
    @Test
    fun keepsSafeCharacters() {
        assertEquals("proof-1_a.json", SystemActions.safeName("proof-1_a.json"))
    }

    @Test
    fun replacesPathAndControlCharacters() {
        assertEquals("_.._etc_passwd", SystemActions.safeName("/../etc/passwd"))
        assertTrue('/' !in SystemActions.safeName("a/b\\c\n"))
    }

    @Test
    fun neverStartsWithADot() {
        assertEquals("hidden", SystemActions.safeName("..hidden"))
    }

    @Test
    fun emptyBecomesFile() {
        assertEquals("file", SystemActions.safeName(""))
        assertEquals("file", SystemActions.safeName("..."))
    }

    @Test
    fun isBounded() {
        assertEquals(96, SystemActions.safeName("a".repeat(10_000)).length)
    }
}

class MemoryOutput : ExportOutput {
    val out = ByteArrayOutputStream()
    var committed = false
    var aborted = false
    var failAt: Int = Int.MAX_VALUE
    var cancel = false
    private var writes = 0

    override suspend fun write(bytes: ByteArray, offset: Int, length: Int) {
        if (cancel) throw CancellationException("stop")
        if (writes++ == failAt) throw IOException("disk full")
        out.write(bytes, offset, length)
    }

    override suspend fun commit(): String {
        committed = true
        return "mem"
    }

    override suspend fun abort() {
        aborted = true
    }
}

class CapturesExporterTest {
    private val store = InMemoryCaptureStore()
    private val output = MemoryOutput()
    private val opened = ArrayList<Pair<String, String>>()
    private val sink = ExportSink { name, mime -> opened += name to mime; output }
    private val exporter = CapturesExporter(store, sink, FakeClock(1_000))

    private suspend fun add(n: Int, withImage: Boolean = true): RecordId {
        val id = RecordId.fromDigest(ByteArray(32) { (it + n).toByte() })
        store.insert(Capture(id, null, CaptureOrigin.ORIGINAL, AnchoringStatus.ANCHORED, n.toLong(), null, LocationConfidence.LOW, Dims(1, 1)), null, null)
        if (withImage) store.putImage(id, ByteString.of(ByteArray(n + 1) { n.toByte() }))
        return id
    }

    private fun entries(): Map<String, ByteArray> {
        val out = LinkedHashMap<String, ByteArray>()
        ZipInputStream(ByteArrayInputStream(output.out.toByteArray())).use { z ->
            while (true) {
                val e = z.nextEntry ?: break
                out[e.name] = z.readBytes()
            }
        }
        return out
    }

    @Test
    fun emptyLibraryExportsNothingAndOpensNoFile() = runTest {
        assertEquals(ExportOutcome.Ready(0), exporter.export())
        assertTrue(opened.isEmpty())
    }

    @Test
    fun writesEverySealedImageIntoAValidZip() = runTest {
        val a = add(1)
        val b = add(2)
        assertEquals(ExportOutcome.Ready(2), exporter.export())
        assertTrue(output.committed)
        assertEquals(listOf(CapturesExporter.fileName(1_000) to CapturesExporter.MIME), opened)
        val zip = entries()
        assertEquals(setOf("${a.text}.jpg", "${b.text}.jpg"), zip.keys)
        assertContentEquals(ByteArray(3) { 2 }, zip["${b.text}.jpg"])
    }

    @Test
    fun capturesWithoutImagesAreSkipped() = runTest {
        add(1)
        add(2, withImage = false)
        assertEquals(ExportOutcome.Ready(1), exporter.export())
        assertEquals(1, entries().size)
    }

    @Test
    fun ioFailureAbortsAndReports() = runTest {
        add(1)
        output.failAt = 0
        val outcome = assertIs<ExportOutcome.Failed>(exporter.export())
        assertEquals("disk full", outcome.reason)
        assertTrue(output.aborted)
        assertTrue(!output.committed)
    }

    @Test
    fun cancellationAbortsAndPropagates() = runTest {
        add(1)
        output.cancel = true
        assertFailsWith<CancellationException> { exporter.export() }
        assertTrue(output.aborted)
    }

    @Test
    fun fileNameIsStable() {
        assertEquals("witness-export-42.zip", CapturesExporter.fileName(42))
    }
}
