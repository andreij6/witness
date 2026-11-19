package app.witness.feature.home

import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.usecase.AnchorOutbox
import app.witness.core.domain.usecase.KeyRotation
import app.witness.core.domain.usecase.RotationStatus
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.OutboxEntry
import app.witness.core.model.OutboxState
import app.witness.core.model.SecurityLevel
import app.witness.core.model.Submission
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class AnchoringViewModelTest : MainDispatcherTest() {
    private fun vm(w: World) = AnchoringViewModel(w.captures, w.cases, w.outbox, w.rotation(), w.scheduler, w.time)

    @Test
    fun emptyLibrary() = runTest {
        val s = vm(World()).state.first { !it.loading }
        assertEquals(0, s.anchored)
        assertEquals(0, s.queued)
        assertEquals(AnchoringCopy.NOTHING_YET, s.lastAnchored)
        assertTrue(s.queue.isEmpty())
        assertTrue(s.recent.isEmpty())
        assertNull(s.key)
    }

    @Test
    fun designScenario() = runTest {
        val w = World()
        w.enrollment()()
        val fender = w.newCase("Fender bender")
        w.add(
            capture(1, createdAtMs = NOW - HOUR, caption = "Golf-ball hail out of nowhere", anchoredAtMs = NOW - 2 * MIN),
            capture(2, createdAtMs = NOW - 50 * MIN, caseId = fender.id, label = "Crash"),
            capture(3, createdAtMs = NOW - 40 * MIN, caseId = fender.id, label = "Rear bumper", anchoredAtMs = NOW - 11 * MIN),
            capture(4, createdAtMs = NOW - 30 * MIN, caseId = fender.id, label = "Plate 7MNV562", anchoredAtMs = NOW - 9 * MIN),
            capture(5, createdAtMs = NOW - 5 * MIN, caseId = fender.id, label = "Wide shot", status = AnchoringStatus.QUEUED),
        )
        val s = vm(w).state.first { !it.loading && it.key != null }
        assertEquals(4, s.anchored)
        assertEquals(1, s.queued)
        assertEquals("Last anchored 2 min ago", s.lastAnchored)
        val q = s.queue.single()
        assertEquals("Wide shot · Fender bender", q.title)
        assertEquals(WitnessState.Queued, q.state)
        assertEquals(AnchoringCopy.waitingNetwork, q.reason)
        assertEquals(
            listOf(
                "Golf-ball hail out of nowhere" to "Post · 2 min ago",
                "Plate 7MNV562" to "Fender bender · Photo 3 · 9 min ago",
                "Rear bumper" to "Fender bender · Photo 2 · 11 min ago",
            ),
            s.recent.take(3).map { it.title to it.detail },
        )
        assertEquals("Active · StrongBox · rotates in 30 days", s.key?.title)
        assertEquals(AnchoringCopy.KEY_DETAIL, s.key?.detail)
    }

    @Test
    fun retryRequestsAnchoring() = runTest {
        val w = World()
        val vm = vm(w)
        vm.retry()
        vm.retry()
        assertEquals(2, w.scheduler.requests)
    }

    @Test
    fun recentIsCappedAtDomainLimit() = runTest {
        val w = World()
        w.add(*Array(12) { capture(it + 1) })
        assertEquals(5, vm(w).state.first { !it.loading }.recent.size)
    }

    @Test
    fun sealedOnlyCountsAsQueuedAwaitingSetup() = runTest {
        val w = World()
        w.add(capture(1, status = AnchoringStatus.SEALED))
        val s = vm(w).state.first { !it.loading }
        assertEquals(1, s.queued)
        assertEquals(AnchoringCopy.awaitingSetup, s.queue.single().reason)
        assertEquals(WitnessState.Sealed, s.queue.single().state)
    }

    @Test
    fun outboxStateDrivesReason() = runTest {
        val w = World()
        w.add(capture(1, status = AnchoringStatus.QUEUED), capture(2, status = AnchoringStatus.QUEUED), capture(3, status = AnchoringStatus.QUEUED))
        w.outbox.update(entry(1, OutboxState.PENDING, AnchorOutbox.RETRY_AFTER))
        w.outbox.update(entry(2, OutboxState.AWAITING_RESEAL))
        w.outbox.update(entry(3, OutboxState.AWAITING_ENROLLMENT))
        val reasons = vm(w).state.first { !it.loading }.queue.associate { it.recordId to it.reason }
        assertEquals(AnchoringCopy.retrying, reasons[rid(1)])
        assertEquals(AnchoringCopy.awaitingKey, reasons[rid(2)])
        assertEquals(AnchoringCopy.awaitingSetup, reasons[rid(3)])
    }

    @Test
    fun reasonTable() {
        val c = capture(1, status = AnchoringStatus.QUEUED)
        assertEquals(AnchoringCopy.waitingNetwork, AnchoringUiState.reasonOf(c, null))
        assertEquals(AnchoringCopy.waitingNetwork, AnchoringUiState.reasonOf(c, entry(1, OutboxState.PENDING)))
        assertEquals(AnchoringCopy.failed, AnchoringUiState.reasonOf(c, entry(1, OutboxState.FAILED)))
    }

    @Test
    fun keyLineVariants() {
        val id = KeyId.fromBytes(ByteArray(KeyId.SIZE))
        fun status(days: Int, due: Boolean) = RotationStatus(id, SecurityLevel.TEE, days, due, 0, false)
        assertNull(AnchoringUiState.keyLine(null))
        assertEquals("Active · TEE · rotates in 12 days", AnchoringUiState.keyLine(status(12, false))?.title)
        assertEquals("Active · TEE · rotates tomorrow", AnchoringUiState.keyLine(status(1, false))?.title)
        assertEquals("Active · TEE · rotation due", AnchoringUiState.keyLine(status(0, true))?.title)
    }

    @Test
    fun keyLineReflectsAgedKey() = runTest {
        val w = World()
        w.enrollment()()
        w.clock.advance(KeyRotation.ROTATE_AFTER_MS - 12 * DAY)
        assertEquals("Active · StrongBox · rotates in 12 days", vm(w).state.first { it.key != null }.key?.title)
    }

    private fun entry(n: Int, state: OutboxState, error: String? = null): OutboxEntry {
        val key = KeyRef(KeyId.fromBytes(ByteArray(KeyId.SIZE)), n.toLong())
        return OutboxEntry(Submission(rid(n), key, true, NOW, ByteString.EMPTY, ByteString.EMPTY, ByteString.EMPTY), state = state, lastError = error)
    }
}
