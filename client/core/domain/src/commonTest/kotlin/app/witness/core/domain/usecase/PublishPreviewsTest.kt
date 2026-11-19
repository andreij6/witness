package app.witness.core.domain.usecase

import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.PreviewPublisher
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.KeyId
import app.witness.core.model.Post
import app.witness.core.model.RecordId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class PublishPreviewsTest {
    private class Recording : PreviewPublisher {
        val published = LinkedHashMap<RecordId, Pair<KeyId, ByteString>>()
        val asked = ArrayList<RecordId>()
        var failure: Exception? = null

        override suspend fun isPublished(recordId: RecordId): Boolean {
            asked += recordId
            return recordId in published
        }

        override suspend fun publish(recordId: RecordId, keyId: KeyId, jpeg: ByteString) {
            failure?.let { throw it }
            published[recordId] = keyId to jpeg
        }
    }

    private suspend fun posted(g: TestGraph): Post {
        g.enrolledKey()
        val c = g.sealOne(g.frame(1, 2, 3))
        val post = g.post.publish(g.post.draft(c.recordId))
        g.captures.replace(post.recordId, g.captures.get(post.recordId)!!, g.outbox[post.recordId]!!, postJpeg)
        return post
    }

    private val postJpeg = ByteString.of(byteArrayOf(-1, -40, 7))

    @Test
    fun anchoredPostsArePublishedOnceWithTheirKeyAndImage() = runTest {
        val g = TestGraph()
        val p = Recording()
        val post = posted(g)
        val previews = PublishPreviews(g.captures, p, g.clock, g.telemetry)
        assertEquals(0, previews())
        assertTrue(p.published.isEmpty())
        g.anchor()
        assertEquals(0, previews())
        val stored = g.captures.get(post.recordId)!!
        assertEquals(listOf(post.recordId), p.published.keys.toList())
        assertEquals(stored.keyRef!!.keyId, p.published.getValue(post.recordId).first)
        assertEquals(postJpeg, p.published.getValue(post.recordId).second)
        assertEquals(0, previews())
        assertEquals(1, p.published.size)
        assertFalse(g.captures.get(post.parent)!!.recordId in p.asked)
    }

    @Test
    fun failuresStayPendingAndAreRetriedNextRun() = runTest {
        val g = TestGraph()
        val p = Recording()
        val post = posted(g)
        g.anchor()
        val previews = PublishPreviews(g.captures, p, g.clock, g.telemetry)
        p.failure = OfflineException()
        assertEquals(1, previews())
        p.failure = IllegalStateException("media.unauthorized")
        assertEquals(1, previews())
        assertTrue(g.telemetry.errors.isNotEmpty())
        p.failure = null
        assertEquals(0, previews())
        assertEquals(listOf(post.recordId), p.published.keys.toList())
    }

    @Test
    fun erasedOldAndNonDerivativeCapturesAreSkipped() = runTest {
        val g = TestGraph()
        val p = Recording()
        val post = posted(g)
        g.anchor()
        val stored = g.captures.get(post.recordId)!!
        val since = g.clock.nowEpochMillis() - PublishPreviews.WINDOW_MS
        assertTrue(PublishPreviews.due(stored, since))
        assertFalse(PublishPreviews.due(stored.copy(publicErased = true), since))
        assertFalse(PublishPreviews.due(stored.copy(origin = CaptureOrigin.ORIGINAL), since))
        assertFalse(PublishPreviews.due(stored.copy(keyRef = null), since))
        assertFalse(PublishPreviews.due(stored.copy(createdAtMs = since - 1), since))
        g.clock.advance(PublishPreviews.WINDOW_MS + 1)
        assertEquals(0, PublishPreviews(g.captures, p, g.clock, g.telemetry)())
        assertTrue(p.asked.isEmpty())
    }

    @Test
    fun anchorRunPublishesPreviewsAndRetriesWhileAnyArePending() = runTest {
        val g = TestGraph()
        val p = Recording()
        val post = posted(g)
        val anchor = AnchorOutbox(g.registry, g.outbox, g.captures, g.vault, g.engine, g.signer, g.clock, g.telemetry, PublishPreviews(g.captures, p, g.clock, g.telemetry))
        p.failure = IllegalStateException("media.unauthorized")
        val first = anchor()
        assertEquals(1, first.previewsPending)
        assertTrue(first.shouldRetry)
        p.failure = null
        val second = anchor()
        assertEquals(0, second.previewsPending)
        assertFalse(second.shouldRetry)
        assertEquals(listOf(post.recordId), p.published.keys.toList())
    }

    @Test
    fun offlineAnchorRunSkipsPreviews() = runTest {
        val g = TestGraph()
        val p = Recording()
        posted(g)
        g.registry.offline = true
        val anchor = AnchorOutbox(g.registry, g.outbox, g.captures, g.vault, g.engine, g.signer, g.clock, g.telemetry, PublishPreviews(g.captures, p, g.clock, g.telemetry))
        val r = anchor()
        assertTrue(r.offline)
        assertEquals(0, r.previewsPending)
        assertTrue(p.asked.isEmpty())
    }

    @Test
    fun publishedPostsAreNotAskedAboutAgainOnLaterRuns() = runTest {
        val g = TestGraph()
        val p = Recording()
        val post = posted(g)
        g.anchor()
        val previews = PublishPreviews(g.captures, p, g.clock, g.telemetry)
        repeat(3) { assertEquals(0, previews()) }
        assertEquals(listOf(post.recordId), p.asked)
        assertEquals(1, p.published.size)
    }

    @Test
    fun offlineStopsTheRunAndCountsEveryRemainingPostAsPending() = runTest {
        val g = TestGraph()
        val p = Recording()
        g.enrolledKey()
        repeat(3) { i ->
            val c = g.sealOne(g.frame(i.toByte(), 2, 3))
            val post = g.post.publish(g.post.draft(c.recordId))
            g.captures.replace(post.recordId, g.captures.get(post.recordId)!!, g.outbox[post.recordId]!!, postJpeg)
        }
        g.anchor()
        val previews = PublishPreviews(g.captures, p, g.clock, g.telemetry)
        p.failure = OfflineException()
        assertEquals(3, previews())
        assertEquals(1, p.asked.size)
        p.failure = null
        assertEquals(0, previews())
        assertEquals(3, p.published.size)
    }

    @Test
    fun noneIsAlwaysPublished() = runTest {
        assertTrue(PreviewPublisher.None.isPublished(RecordId.fromBytes(ByteArray(10))))
    }
}
