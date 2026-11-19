package app.witness.core.domain.usecase

import app.cash.turbine.test
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Dims
import app.witness.core.model.LocationConfidence
import app.witness.core.model.RecordId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals

class ObserveAnchoringTest {
    private fun cap(n: Int, status: AnchoringStatus, created: Long, anchoredAt: Long? = null) = Capture(
        RecordId.fromBytes(ByteArray(10) { n.toByte() }), null,
        if (status == AnchoringStatus.UNVERIFIED) CaptureOrigin.IMPORT else CaptureOrigin.ORIGINAL,
        status, created, null, LocationConfidence.HIGH, Dims(1, 1),
        anchor = anchoredAt?.let { Anchor(n.toLong(), it) },
    )

    @Test
    fun summarizeCountsAndOrders() {
        val all = listOf(
            cap(1, AnchoringStatus.ANCHORED, 1, 10),
            cap(2, AnchoringStatus.ANCHORED, 2, 30),
            cap(3, AnchoringStatus.QUEUED, 50),
            cap(4, AnchoringStatus.SEALED, 40),
            cap(5, AnchoringStatus.UNVERIFIED, 5),
            cap(6, AnchoringStatus.FAILED, 6),
        ) + (10 until 16).map { cap(it, AnchoringStatus.ANCHORED, it.toLong(), it.toLong()) }
        val s = ObserveAnchoring.summarize(all)
        assertEquals(12, s.total)
        assertEquals(11, s.sealed)
        assertEquals(8, s.anchored)
        assertEquals(1, s.queued)
        assertEquals(1, s.sealedOnly)
        assertEquals(1, s.failed)
        assertEquals(1, s.unverified)
        assertEquals(listOf(4, 3), s.queue.map { it.recordId.toBytes()[0].toInt() })
        assertEquals(listOf(2, 15, 14, 13, 12), s.recent.map { it.recordId.toBytes()[0].toInt() })
        assertEquals(ObserveAnchoring.RECENT, s.recent.size)
    }

    @Test
    fun emptySummary() {
        val s = ObserveAnchoring.summarize(emptyList())
        assertEquals(0, s.total)
        assertEquals(emptyList(), s.queue)
        assertEquals(emptyList(), s.recent)
    }

    @Test
    fun flowTracksAnchoring() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        ObserveAnchoring(g.captures)().test {
            assertEquals(0, awaitItem().total)
            g.sealOne()
            g.sealOne()
            val queued = expectMostRecentItem()
            assertEquals(2, queued.queued)
            g.anchor()
            val done = expectMostRecentItem()
            assertEquals(2, done.anchored)
            assertEquals(0, done.queued)
            assertEquals(2, done.recent.size)
            cancelAndIgnoreRemainingEvents()
        }
    }
}
