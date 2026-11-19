package app.witness.core.domain.usecase

import app.witness.core.domain.port.CaptureStore
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

data class AnchoringSummary(
    val total: Int,
    val anchored: Int,
    val queued: Int,
    val sealedOnly: Int,
    val failed: Int,
    val unverified: Int,
    val queue: List<Capture>,
    val recent: List<Capture>,
) {
    val sealed: Int get() = total - unverified
}

class ObserveAnchoring(private val captures: CaptureStore) {
    operator fun invoke(): Flow<AnchoringSummary> = captures.observeAll().map(::summarize)

    companion object {
        const val RECENT: Int = 5

        fun summarize(all: List<Capture>): AnchoringSummary {
            val counts = IntArray(AnchoringStatus.entries.size)
            val queue = ArrayList<Capture>()
            val anchored = ArrayList<Capture>()
            for (c in all) {
                counts[c.status.ordinal]++
                when (c.status) {
                    AnchoringStatus.QUEUED, AnchoringStatus.SEALED -> queue += c
                    AnchoringStatus.ANCHORED -> anchored += c
                    else -> Unit
                }
            }
            queue.sortWith(compareBy({ it.createdAtMs }, { it.keyRef?.counter ?: 0 }))
            anchored.sortWith(compareByDescending<Capture> { it.anchor?.anchoredAtMs ?: 0 }.thenByDescending { it.anchor?.recordIx ?: 0 })
            return AnchoringSummary(
                total = all.size,
                anchored = counts[AnchoringStatus.ANCHORED.ordinal],
                queued = counts[AnchoringStatus.QUEUED.ordinal],
                sealedOnly = counts[AnchoringStatus.SEALED.ordinal],
                failed = counts[AnchoringStatus.FAILED.ordinal],
                unverified = counts[AnchoringStatus.UNVERIFIED.ordinal],
                queue = queue,
                recent = anchored.take(RECENT),
            )
        }
    }
}
