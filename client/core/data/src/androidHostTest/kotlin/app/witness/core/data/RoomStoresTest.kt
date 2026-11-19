package app.witness.core.data

import androidx.room3.useWriterConnection
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.CaseId
import app.witness.core.model.CaseShare
import app.witness.core.model.Edit
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationOpening
import app.witness.core.model.OutboxState
import app.witness.core.model.Rect
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId
import app.witness.core.model.Verdict
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], manifest = Config.NONE)
class RoomStoresTest {
    private val db = inMemoryDb()
    private val images = tempImages()
    private val captures = RoomCaptureStore(db, testCipher(), images)
    private val outbox = RoomOutboxStore(db)
    private val cases = RoomCaseStore(db)
    private val counter = RoomCounterStore(db)
    private val history = CheckHistoryStore(db, keep = 3)
    private val salt = ByteString.of(ByteArray(32) { 9 })

    @After
    fun close() = db.close()

    @Test
    fun counterStartsAtZeroAndIncrementsPerKey() = runTest {
        assertEquals(listOf(0L, 1L, 2L), List(3) { counter.reserve(KEY1) })
        assertEquals(0L, counter.reserve(KEY2))
        assertEquals(3L, db.keyCounterDao().next(KEY1.text))
    }

    @Test
    fun concurrentReservationsNeverRepeat() = runTest {
        val values = List(64) { async { counter.reserve(KEY1) } }.awaitAll()
        assertEquals((0L until 64L).toSet(), values.toSet())
    }

    @Test
    fun counterResumesFromStoredValue() = runTest {
        db.keyCounterDao().upsert(KeyCounterEntity(KEY1.text, 41))
        assertEquals(41, counter.reserve(KEY1))
    }

    @Test
    fun insertRoundTripsCaptureOutboxAndOpening() = runTest {
        val c = capture(1).copy(label = "Rear bumper", place = "Seattle", caption = "c", altText = "a", anchor = null)
        val opening = LocationOpening(salt, 10, 20, 30)
        captures.insert(c, entry(1), opening)
        assertEquals(c, captures.get(rid(1)))
        assertEquals(listOf(entry(1)), outbox.all())
        assertEquals(opening, captures.opening(rid(1)))
        assertNull(captures.get(rid(2)))
        assertNull(captures.opening(rid(2)))
    }

    @Test
    fun outboxRoundTripsFingerprints() = runTest {
        val e = entry(1).let { it.copy(submission = it.submission.copy(fingerprints = ByteString.of(byteArrayOf(0xa1.toByte(), 2)))) }
        captures.insert(capture(1), e, null)
        assertEquals(listOf(e), outbox.all())
    }

    @Test
    fun insertPersistsTheSealedImageBeforeTheRow() = runTest {
        val jpeg = ByteString.of(byteArrayOf(-1, -40, 1))
        captures.insert(capture(1), entry(1), null, jpeg)
        assertEquals(jpeg, captures.readImage(rid(1)))
        assertTrue(images.file(rid(1)).isFile)
    }

    @Test
    fun replaceWithResealedImageWritesNewAndDropsOld() = runTest {
        captures.insert(capture(1), entry(1), null, ByteString.of(byteArrayOf(1)))
        captures.replace(rid(1), capture(2), entry(2), ByteString.of(byteArrayOf(2)))
        assertNull(captures.readImage(rid(1)))
        assertEquals(ByteString.of(byteArrayOf(2)), captures.readImage(rid(2)))
    }

    @Test
    fun replaceWithoutImageMovesTheExistingOne() = runTest {
        captures.insert(capture(1), entry(1), null, ByteString.of(byteArrayOf(1)))
        captures.replace(rid(1), capture(2), entry(2))
        assertNull(captures.readImage(rid(1)))
        assertEquals(ByteString.of(byteArrayOf(1)), captures.readImage(rid(2)))
    }

    @Test
    fun openingIsEncryptedAtRest() = runTest {
        val opening = LocationOpening(salt, 123_456_789, -98_765_432, 1)
        captures.insert(capture(1), null, opening)
        val raw = db.captureDao().opening(rid(1).text)!!
        assertEquals(1 + 12 + app.witness.core.data.codec.OpeningCodec.SIZE + 16, raw.size)
        assertTrue(raw.toList().windowed(32).none { it == salt.toByteArray().toList() })
    }

    @Test
    fun outboxMustMatchCapture() = runTest {
        assertFailsWith<IllegalArgumentException> { captures.insert(capture(1), entry(2), null) }
        assertNull(captures.get(rid(1)))
    }

    @Test
    fun failedInsertIsAtomic() = runTest {
        captures.insert(capture(1), entry(1), null)
        assertFailsWith<Exception> { captures.insert(capture(2, counter = 1), entry(2, counter = 1), null) }
        assertNull(captures.get(rid(2)))
        assertEquals(1, outbox.all().size)
    }

    @Test
    fun observeAllNewestFirst() = runTest {
        captures.insert(capture(1, createdAtMs = 5), null, null)
        captures.insert(capture(2, createdAtMs = 9), null, null)
        assertEquals(listOf(rid(2), rid(1)), captures.observeAll().first().map { it.recordId })
    }

    @Test
    fun setStatusKeepsAnchorWhenNull() = runTest {
        captures.insert(capture(1), null, null)
        captures.setStatus(rid(1), AnchoringStatus.ANCHORED, Anchor(7, 99))
        captures.setStatus(rid(1), AnchoringStatus.ANCHORED, null)
        val c = captures.get(rid(1))!!
        assertEquals(AnchoringStatus.ANCHORED, c.status)
        assertEquals(Anchor(7, 99), c.anchor)
        captures.setStatus(rid(9), AnchoringStatus.FAILED, null)
    }

    @Test
    fun updateChangesFieldsCaseAndIgnoresUnknown() = runTest {
        val case = cases.create("Fender bender", TemplateId.CAR_ACCIDENT, 1)
        captures.insert(capture(1), null, null)
        captures.update(capture(1).copy(caption = "new", caseId = case.id, publicErased = true))
        val c = captures.get(rid(1))!!
        assertEquals("new", c.caption)
        assertEquals(case.id, c.caseId)
        assertTrue(c.publicErased)
        captures.update(c.copy(caseId = null))
        assertNull(captures.get(rid(1))!!.caseId)
        captures.update(capture(5))
        assertNull(captures.get(rid(5)))
    }

    @Test
    fun derivativesAreTransitiveAndKeepEdits() = runTest {
        val edits = listOf(Edit.Crop(Rect(0, 0, 10, 10)), Edit.Blur(Rect(1, 1, 2, 2), BlurClass.FACE, 300))
        captures.insert(capture(1, createdAtMs = 1), null, null)
        captures.insert(capture(2, createdAtMs = 2).copy(origin = CaptureOrigin.DERIVATIVE, parent = rid(1), edits = edits), entry(2), null)
        captures.insert(capture(3, createdAtMs = 3).copy(origin = CaptureOrigin.DERIVATIVE, parent = rid(2)), null, null)
        captures.insert(capture(4, createdAtMs = 4), null, null)
        val d = captures.derivativesOf(rid(1))
        assertEquals(listOf(rid(2), rid(3)), d.map { it.recordId })
        assertEquals(edits, d[0].edits)
        assertEquals(rid(1), d[0].parent)
        assertTrue(captures.derivativesOf(rid(4)).isEmpty())
    }

    @Test
    fun replaceMovesEverythingToNewRecord() = runTest {
        val case = cases.create("C", TemplateId.GENERAL, 1)
        val opening = LocationOpening(salt, 1, 2, 3)
        captures.insert(capture(1).copy(caseId = case.id), entry(1), opening)
        captures.insert(capture(2).copy(origin = CaptureOrigin.DERIVATIVE, parent = rid(1)), null, null)
        captures.writeImage(rid(1), ByteString.of(byteArrayOf(-1, -40)))
        val fresh = capture(1).copy(recordId = rid(50), keyRef = app.witness.core.model.KeyRef(KEY2, 0), caseId = case.id)
        captures.replace(rid(1), fresh, entry(50, counter = 0, key = KEY2))
        assertNull(captures.get(rid(1)))
        assertEquals(fresh, captures.get(rid(50)))
        assertEquals(opening, captures.opening(rid(50)))
        assertEquals(listOf(rid(50)), outbox.all().map { it.recordId })
        assertEquals(rid(50), captures.derivativesOf(rid(50)).single().parent)
        assertEquals(2, captures.readImage(rid(50))!!.size)
        assertNull(captures.readImage(rid(1)))
    }

    @Test
    fun replaceUnknownFails() = runTest {
        assertFailsWith<IllegalStateException> { captures.replace(rid(1), capture(2), entry(2)) }
    }

    @Test
    fun replaceSameIdUpdatesOutbox() = runTest {
        captures.insert(capture(1), entry(1), null)
        captures.replace(rid(1), capture(1).copy(status = AnchoringStatus.QUEUED), entry(1).copy(state = OutboxState.PENDING, attempts = 0))
        assertEquals(1, outbox.all().size)
    }

    @Test
    fun outboxOrderedByKeyThenCounterAndUpdatable() = runTest {
        captures.insert(capture(1, key = KEY2, counter = 0), entry(1, counter = 0, key = KEY2), null)
        captures.insert(capture(2, counter = 5), entry(2, counter = 5), null)
        captures.insert(capture(3, counter = 1), entry(3, counter = 1), null)
        assertEquals(listOf(rid(3), rid(2), rid(1)), outbox.all().map { it.recordId })
        outbox.update(entry(3, counter = 1).copy(attempts = 2, notBeforeMs = 77, state = OutboxState.AWAITING_RESEAL, lastError = "key_expired"))
        val e = outbox.all().first()
        assertEquals(2, e.attempts)
        assertEquals(OutboxState.AWAITING_RESEAL, e.state)
        assertEquals("key_expired", e.lastError)
        outbox.remove(rid(3))
        assertEquals(listOf(rid(2), rid(1)), outbox.observe().first().map { it.recordId })
    }

    @Test
    fun outboxUpdateNeverStealsAnotherCounter() = runTest {
        captures.insert(capture(1), entry(1), null)
        captures.insert(capture(2), null, null)
        assertFailsWith<Exception> { outbox.update(entry(2, counter = 1)) }
        assertEquals(listOf(rid(1)), outbox.all().map { it.recordId })
        outbox.update(entry(2, counter = 2))
        assertEquals(listOf(rid(1), rid(2)), outbox.all().map { it.recordId })
    }

    @Test
    fun deletingCaptureCascades() = runTest {
        captures.insert(capture(1), entry(1), LocationOpening(salt, 0, 0, 0))
        db.useWriterConnectionDelete(rid(1).text)
        assertTrue(outbox.all().isEmpty())
        assertNull(captures.opening(rid(1)))
    }

    @Test
    fun deleteRemovesRowImageOutboxOpeningAndCaseItem() = runTest {
        val case = cases.create("One", TemplateId.GENERAL, 1)
        captures.insert(capture(1).copy(caseId = case.id), entry(1), LocationOpening(salt, 0, 0, 0), ByteString.of(byteArrayOf(1)))
        captures.insert(capture(2), entry(2, counter = 2), null, ByteString.of(byteArrayOf(2)))
        captures.delete(rid(1))
        assertNull(captures.get(rid(1)))
        assertNull(captures.readImage(rid(1)))
        assertNull(captures.opening(rid(1)))
        assertFalse(images.file(rid(1)).exists())
        assertEquals(listOf(rid(2)), outbox.all().map { it.recordId })
        assertEquals(listOf(rid(2)), captures.observeAll().first().map { it.recordId })
        assertEquals(ByteString.of(byteArrayOf(2)), captures.readImage(rid(2)))
        assertEquals(case, cases.get(case.id))
    }

    @Test
    fun deleteParentKeepsDerivatives() = runTest {
        captures.insert(capture(1), null, null)
        captures.insert(capture(2).copy(parent = rid(1), origin = CaptureOrigin.DERIVATIVE), null, null)
        captures.delete(rid(1))
        assertEquals(rid(1), captures.get(rid(2))!!.parent)
    }

    @Test
    fun deleteUnknownIsANoOp() = runTest {
        captures.insert(capture(1), entry(1), null)
        captures.delete(rid(9))
        assertEquals(listOf(rid(1)), captures.observeAll().first().map { it.recordId })
    }

    @Test
    fun caseLifecycle() = runTest {
        val a = cases.create("One", TemplateId.GENERAL, 1)
        val b = cases.create("Two", TemplateId.CAR_ACCIDENT, 2)
        assertTrue(a.id != b.id)
        assertEquals(listOf(b, a), cases.observeAll().first())
        val share = CaseShare(CaseId.of(1, ByteArray(15)), "https://case.witness.app/v/x#k=y", 3, null, 2)
        val updated = a.copy(title = "Renamed", updatedAtMs = 10, summary = "sum", shares = listOf(share))
        cases.update(updated)
        assertEquals(updated, cases.get(a.id))
        assertEquals(listOf(updated, b), cases.observeAll().first())
        captures.insert(capture(1).copy(caseId = a.id), null, null)
        cases.delete(a.id)
        assertNull(cases.get(a.id))
        assertNull(captures.get(rid(1))!!.caseId)
        assertNull(cases.get(LocalCaseId(999)))
    }

    @Test
    fun caseSummaryProvenanceRoundTrips() = runTest {
        val a = cases.create("One", TemplateId.GENERAL, 1)
        assertNull(cases.get(a.id)!!.summarySource)
        for (source in SummarySource.entries) {
            val next = a.copy(summary = "s", summarySource = source)
            cases.update(next)
            assertEquals(next, cases.get(a.id))
        }
        cases.update(a.copy(summary = null, summarySource = null))
        assertNull(cases.get(a.id)!!.summarySource)
    }

    @Test
    fun checkHistoryBoundedNewestFirst() = runTest {
        (1..5).forEach { history.record(CheckRecord(it.toLong(), Verdict.ORIGINAL, rid(it), "c$it")) }
        assertEquals(listOf("c5", "c4", "c3"), history.observeRecent(10).first().map { it.label })
        assertEquals(rid(5), history.observeRecent(1).first().single().recordId)
        assertTrue(history.observeRecent(-1).first().isEmpty())
        history.clear()
        assertTrue(history.observeRecent(10).first().isEmpty())
    }

    @Test
    fun checkHistoryKeepMustBePositive() {
        assertFailsWith<IllegalArgumentException> { CheckHistoryStore(db, 0) }
    }
}

private suspend fun WitnessDatabase.useWriterConnectionDelete(recordId: String) {
    useWriterConnection { c ->
        c.usePrepared("DELETE FROM capture WHERE recordId = ?") { it.bindText(1, recordId); it.step() }
    }
}
