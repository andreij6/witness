package app.witness.core.domain.usecase

import app.witness.core.domain.Clock
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.PreviewPublisher
import app.witness.core.domain.port.Telemetry
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.RecordId
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.first

class PublishPreviews(
    private val captures: CaptureStore,
    private val publisher: PreviewPublisher,
    private val clock: Clock,
    private val telemetry: Telemetry = Telemetry.None,
) {
    private val published = HashSet<RecordId>()

    suspend operator fun invoke(): Int {
        val since = clock.nowEpochMillis() - WINDOW_MS
        val due = captures.observeAll().first().filter { due(it, since) && it.recordId !in published }
        var pending = 0
        for ((i, c) in due.withIndex()) {
            try {
                if (!publisher.isPublished(c.recordId)) {
                    val jpeg = captures.readImage(c.recordId) ?: continue
                    publisher.publish(c.recordId, c.keyRef!!.keyId, jpeg)
                }
                published += c.recordId
            } catch (e: CancellationException) {
                throw e
            } catch (_: OfflineException) {
                return pending + due.size - i
            } catch (e: Exception) {
                telemetry.nonFatal(e)
                pending++
            }
        }
        return pending
    }

    companion object {
        const val WINDOW_MS: Long = 7L * 24 * 60 * 60 * 1000

        fun due(c: Capture, sinceMs: Long): Boolean =
            c.origin == CaptureOrigin.DERIVATIVE && c.status == AnchoringStatus.ANCHORED && !c.publicErased && c.keyRef != null && c.createdAtMs >= sinceMs
    }
}
