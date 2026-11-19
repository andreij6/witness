package app.witness.core.domain.fake

import app.cash.turbine.test
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Dims
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationOpening
import app.witness.core.model.OutboxEntry
import app.witness.core.model.RecordId
import app.witness.core.model.Submission
import app.witness.core.model.TemplateId
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class FakeStoresTest {
    private val key = KeyId.fromBytes(ByteArray(16))
    private fun id(n: Int) = RecordId.fromBytes(ByteArray(10) { n.toByte() })
    private fun cap(n: Int, parent: Int? = null, created: Long = n.toLong()) =
        Capture(id(n), null, CaptureOrigin.ORIGINAL, AnchoringStatus.QUEUED, created, KeyRef(key, n.toLong()), LocationConfidence.HIGH, Dims(1, 1), parent = parent?.let(::id))
    private fun entry(n: Int) = OutboxEntry(Submission(id(n), KeyRef(key, n.toLong()), true, 0, ByteString.EMPTY, ByteString.EMPTY, ByteString.EMPTY))

    @Test
    fun derivativesAreTransitive() = runTest {
        val s = InMemoryCaptureStore()
        listOf(cap(1), cap(2, 1), cap(3, 2), cap(4, 1), cap(5)).forEach { s.insert(it, null, null) }
        assertEquals(listOf(2, 3, 4), s.derivativesOf(id(1)).map { it.recordId.toBytes()[0].toInt() })
        assertEquals(emptyList(), s.derivativesOf(id(5)))
    }

    @Test
    fun replaceMovesOutboxImageAndOpening() = runTest {
        val outbox = InMemoryOutboxStore()
        val s = InMemoryCaptureStore(outbox)
        s.insert(cap(1), entry(1), null)
        s.putImage(id(1), ByteString.of(byteArrayOf(1)))
        s.replace(id(1), cap(2), entry(2))
        assertNull(s.get(id(1)))
        assertNull(outbox[id(1)])
        assertEquals(entry(2), outbox[id(2)])
        assertEquals(ByteString.of(byteArrayOf(1)), s.readImage(id(2)))
    }

    @Test
    fun deleteDropsCaptureImageOpeningAndOutbox() = runTest {
        val outbox = InMemoryOutboxStore()
        val s = InMemoryCaptureStore(outbox)
        s.insert(cap(1), entry(1), null, ByteString.of(byteArrayOf(1)))
        s.insert(cap(2, 1), entry(2), null)
        s.putOpening(id(1), LocationOpening(ByteString.of(ByteArray(32)), 1, 2, 3))
        s.delete(id(1))
        s.delete(id(9))
        assertNull(s.get(id(1)))
        assertNull(s.readImage(id(1)))
        assertNull(s.opening(id(1)))
        assertNull(outbox[id(1)])
        assertEquals(entry(2), outbox[id(2)])
        assertEquals(listOf(id(2)), s.observeAll().first().map { it.recordId })
    }

    @Test
    fun insertKeepsTheSealedImage() = runTest {
        val s = InMemoryCaptureStore()
        s.insert(cap(1), entry(1), null, ByteString.of(byteArrayOf(9, 8)))
        s.insert(cap(2), null, null)
        assertEquals(ByteString.of(byteArrayOf(9, 8)), s.readImage(id(1)))
        assertNull(s.readImage(id(2)))
    }

    @Test
    fun replaceWithNewImageDropsTheOldOne() = runTest {
        val s = InMemoryCaptureStore()
        s.insert(cap(1), entry(1), null, ByteString.of(byteArrayOf(1)))
        s.replace(id(1), cap(2), entry(2), ByteString.of(byteArrayOf(2)))
        assertNull(s.readImage(id(1)))
        assertEquals(ByteString.of(byteArrayOf(2)), s.readImage(id(2)))
    }

    @Test
    fun setStatusKeepsExistingAnchorWhenNull() = runTest {
        val s = InMemoryCaptureStore()
        s.insert(cap(1), null, null)
        s.setStatus(id(1), AnchoringStatus.ANCHORED, Anchor(3, 4))
        s.setStatus(id(1), AnchoringStatus.FAILED, null)
        assertEquals(Anchor(3, 4), s.get(id(1))!!.anchor)
        s.setStatus(id(9), AnchoringStatus.FAILED, null)
        s.update(cap(9))
        assertNull(s.get(id(9)))
    }

    @Test
    fun observeEmitsNewestFirst() = runTest {
        val s = InMemoryCaptureStore()
        s.observeAll().test {
            assertEquals(emptyList(), awaitItem())
            s.insert(cap(1, created = 5), null, null)
            s.insert(cap(2, created = 9), null, null)
            assertEquals(listOf(id(2), id(1)), expectMostRecentItem().map { it.recordId })
        }
    }

    @Test
    fun outboxOrdersByKeyThenCounter() = runTest {
        val o = InMemoryOutboxStore()
        listOf(3, 1, 2).forEach { o.update(entry(it)) }
        assertEquals(listOf(1L, 2L, 3L), o.all().map { it.counter })
        o.remove(id(2))
        assertEquals(2, o.size)
    }

    @Test
    fun countersNeverRepeatAndCanSkip() = runTest {
        val c = InMemoryCounterStore()
        assertEquals(0, c.reserve(key))
        c.skip(key, 5)
        assertEquals(6, c.reserve(key))
        assertEquals(0, c.reserve(KeyId.fromBytes(ByteArray(16) { 1 })))
    }

    @Test
    fun caseStoreAssignsIds() = runTest {
        val s = InMemoryCaseStore()
        val a = s.create("a", TemplateId.GENERAL, 1)
        val b = s.create("b", TemplateId.GENERAL, 2)
        assertEquals(listOf(b.id, a.id), s.observeAll().first().map { it.id })
        s.delete(a.id)
        assertNull(s.get(a.id))
    }

    @Test
    fun fakeSealEngineRejectsReusedPreparation() = runTest {
        val g = app.witness.core.domain.usecase.TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        val prepared = g.engine.reseal(c.recordId)
        val key = g.vault.activeKey()!!
        g.signer.seal(prepared, key)
        kotlin.test.assertFailsWith<IllegalStateException> { g.signer.seal(prepared, key) }
    }
}
