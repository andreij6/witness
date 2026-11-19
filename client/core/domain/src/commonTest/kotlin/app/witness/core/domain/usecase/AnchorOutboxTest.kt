package app.witness.core.domain.usecase

import app.witness.core.domain.port.SubmitResult
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.OutboxEntry
import app.witness.core.model.OutboxState
import app.witness.core.model.RecordId
import app.witness.core.model.Submission
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class AnchorOutboxTest {
    private fun key(n: Int) = KeyId.fromBytes(ByteArray(16) { n.toByte() })

    private fun entry(k: Int, counter: Long): OutboxEntry {
        val rid = RecordId.fromBytes(ByteArray(10).also { it[0] = k.toByte(); it[1] = (counter ushr 8).toByte(); it[2] = counter.toByte() })
        return OutboxEntry(Submission(rid, KeyRef(key(k), counter), true, 0, ByteString.EMPTY, ByteString.EMPTY, ByteString.EMPTY))
    }

    @Test
    fun batchesGroupByKeyInCounterOrder() {
        val input = listOf(entry(1, 5), entry(2, 1), entry(1, 2), entry(2, 0), entry(1, 9))
        val out = AnchorOutbox.batches(input)
        assertEquals(2, out.size)
        assertEquals(listOf(2L, 5L, 9L), out[0].map { it.counter })
        assertEquals(listOf(0L, 1L), out[1].map { it.counter })
        out.forEach { b -> assertEquals(1, b.map { it.keyId }.toSet().size) }
    }

    @Test
    fun batchesCapAt64() {
        val input = (0L until 130L).reversed().map { entry(1, it) }
        val out = AnchorOutbox.batches(input)
        assertEquals(listOf(64, 64, 2), out.map { it.size })
        assertEquals((0L until 130L).toList(), out.flatten().map { it.counter })
        assertEquals(emptyList(), AnchorOutbox.batches(emptyList()))
        assertEquals(listOf(1, 1, 1), AnchorOutbox.batches(input.take(3), max = 1).map { it.size })
    }

    @Test
    fun anchorsQueuedCaptures() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val a = g.sealOne()
        val b = g.sealOne()
        val report = g.anchor()
        assertEquals(2, report.anchored)
        assertFalse(report.shouldRetry)
        assertEquals(0, g.outbox.size)
        listOf(a, b).forEach { c ->
            val stored = g.captures.get(c.recordId)!!
            assertEquals(AnchoringStatus.ANCHORED, stored.status)
            assertNotNull(stored.anchor)
        }
        assertEquals(listOf(0L, 1L), g.registry.submitted.single().map { it.keyRef.counter })
    }

    @Test
    fun duplicateCountsAsSuccess() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.anchor()
        g.outbox.update(OutboxEntry(g.registry.submitted.single().single()))
        val report = g.anchor()
        assertEquals(1, report.duplicates)
        assertEquals(0, g.outbox.size)
        assertEquals(AnchoringStatus.ANCHORED, g.captures.get(c.recordId)!!.status)
    }

    @Test
    fun retryAfterDefersWithBackoff() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.registry.scripted += SubmitResult.RetryAfter(60_000)
        val r = g.anchor()
        assertEquals(1, r.deferred)
        assertTrue(r.shouldRetry)
        val e = g.outbox[c.recordId]!!
        assertEquals(1, e.attempts)
        assertEquals(g.clock.nowMs + 60_000, e.notBeforeMs)
        assertEquals(AnchorOutbox.RETRY_AFTER, e.lastError)
        assertEquals(0, g.anchor().anchored)
        assertEquals(1, g.registry.submitted.size)
        g.clock.advance(60_000)
        assertEquals(1, g.anchor().anchored)
    }

    @Test
    fun negativeRetryAfterIsClamped() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.registry.scripted += SubmitResult.RetryAfter(-5)
        g.anchor()
        assertEquals(g.clock.nowMs, g.outbox[c.recordId]!!.notBeforeMs)
    }

    @Test
    fun missingResultsAreRetried() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        val shortRegistry = object : app.witness.core.domain.port.RegistryGateway by g.registry {
            override suspend fun submit(batch: List<Submission>): List<SubmitResult> = emptyList()
        }
        val r = AnchorOutbox(shortRegistry, g.outbox, g.captures, g.vault, g.engine, g.signer, g.clock)()
        assertEquals(1, r.deferred)
        assertEquals(g.clock.nowMs + AnchorOutbox.DEFAULT_RETRY_MS, g.outbox[c.recordId]!!.notBeforeMs)
    }

    @Test
    fun rejectedMarksFailed() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.registry.scripted += SubmitResult.Rejected("bad_signature")
        val r = g.anchor()
        assertEquals(1, r.failed)
        assertEquals(OutboxState.FAILED, g.outbox[c.recordId]!!.state)
        assertEquals("bad_signature", g.outbox[c.recordId]!!.lastError)
        assertEquals(AnchoringStatus.FAILED, g.captures.get(c.recordId)!!.status)
        assertEquals(0, g.anchor().failed)
        assertEquals(1, g.registry.submitted.size)
    }

    @Test
    fun derivativeWaitsWhileItsParentIsStillQueued() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        val post = g.post.publish(g.post.draft(c.recordId))
        g.registry.scripted += SubmitResult.RetryAfter(60_000)
        g.registry.scripted += SubmitResult.Rejected(AnchorOutbox.PARENT_UNKNOWN)
        val r = g.anchor()
        assertEquals(2, r.deferred)
        assertEquals(0, r.failed)
        val e = g.outbox[post.recordId]!!
        assertEquals(OutboxState.PENDING, e.state)
        assertEquals(AnchorOutbox.PARENT_UNKNOWN, e.lastError)
        assertNotEquals(AnchoringStatus.FAILED, g.captures.get(post.recordId)!!.status)
        g.clock.advance(AnchorOutbox.DEFAULT_RETRY_MS.coerceAtLeast(60_000))
        assertEquals(2, g.anchor().anchored)
        assertEquals(AnchoringStatus.ANCHORED, g.captures.get(post.recordId)!!.status)
    }

    @Test
    fun parentUnknownFailsWhenTheParentIsNotQueued() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealAnchored()
        val post = g.post.publish(g.post.draft(c.recordId))
        g.registry.scripted += SubmitResult.Rejected(AnchorOutbox.PARENT_UNKNOWN)
        assertEquals(1, g.anchor().failed)
        assertEquals(OutboxState.FAILED, g.outbox[post.recordId]!!.state)
    }

    @Test
    fun offlineLeavesQueueUntouched() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.registry.offline = true
        val r = g.anchor()
        assertTrue(r.offline)
        assertTrue(r.shouldRetry)
        assertEquals(OutboxState.PENDING, g.outbox[c.recordId]!!.state)
        assertEquals(0, g.outbox[c.recordId]!!.attempts)
    }

    @Test
    fun keyExpiredWithoutNewKeyNeedsRotation() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.registry.scripted += SubmitResult.KeyExpired
        val r = g.anchor()
        assertTrue(r.needsRotation)
        assertEquals(0, r.resealed)
        assertEquals(OutboxState.AWAITING_RESEAL, g.outbox[c.recordId]!!.state)
        assertEquals(AnchorOutbox.KEY_EXPIRED, g.outbox[c.recordId]!!.lastError)
    }

    @Test
    fun keyExpiredResealsWithNewKeyAndAnchors() = runTest {
        val g = TestGraph()
        val old = g.enrolledKey()
        val c = g.sealOne()
        g.registry.scripted += SubmitResult.KeyExpired
        g.anchor()
        g.clock.advance(31L * 86_400_000)
        val rotated = assertIs<RotationOutcome.Rotated>(g.rotation())
        val r = g.anchor()
        assertEquals(1, r.resealed)
        assertEquals(1, r.anchored)
        assertNull(g.captures.get(c.recordId))
        assertNull(g.outbox[c.recordId])
        val resealed = g.captures.snapshot.values.single()
        assertNotEquals(c.recordId, resealed.recordId)
        assertEquals(rotated.key.keyId, resealed.keyRef!!.keyId)
        assertNotEquals(old.keyId, resealed.keyRef!!.keyId)
        assertEquals(AnchoringStatus.ANCHORED, resealed.status)
        assertEquals(c.captureId, resealed.captureId)
        assertEquals(0, g.outbox.size)
    }

    @Test
    fun awaitingEnrollmentSubmitsAfterEnrollment() = runTest {
        val g = TestGraph()
        g.registry.offline = true
        g.enrollment()
        val c = g.sealOne()
        g.registry.offline = false
        val before = g.anchor()
        assertEquals(0, before.anchored)
        assertEquals(OutboxState.AWAITING_ENROLLMENT, g.outbox[c.recordId]!!.state)
        assertIs<EnrollmentStatus.Enrolled>(g.enrollment())
        assertEquals(1, g.anchor().anchored)
        assertEquals(AnchoringStatus.ANCHORED, g.captures.get(c.recordId)!!.status)
    }

    @Test
    fun largeQueueSubmitsInBoundedBatches() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        repeat(70) { g.sealOne() }
        val r = g.anchor()
        assertEquals(70, r.anchored)
        assertEquals(listOf(64, 6), g.registry.submitted.map { it.size })
        g.registry.submitted.forEach { b -> assertEquals(b.map { it.keyRef.counter }.sorted(), b.map { it.keyRef.counter }) }
    }

    @Test
    fun reportsTelemetryWithoutIds() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        g.sealOne()
        g.anchor()
        val metrics = g.telemetry.events.last().second
        assertEquals(setOf("anchored", "deferred", "failed"), metrics.keys)
        assertEquals(1L, metrics["anchored"])
    }

    @Test
    fun reportArithmetic() {
        val a = AnchorReport(anchored = 1, offline = true)
        val b = AnchorReport(deferred = 2, needsRotation = true)
        val s = a + b
        assertEquals(1, s.anchored)
        assertEquals(2, s.deferred)
        assertTrue(s.offline && s.needsRotation && s.shouldRetry)
        assertFalse(AnchorReport().shouldRetry)
    }
}
