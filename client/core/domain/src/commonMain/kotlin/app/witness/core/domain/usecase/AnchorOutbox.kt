package app.witness.core.domain.usecase

import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.OutboxStore
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.SealEngine
import app.witness.core.domain.port.SubmitResult
import app.witness.core.domain.port.Telemetry
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.DeviceKey
import app.witness.core.model.KeyId
import app.witness.core.model.OutboxEntry
import app.witness.core.model.OutboxState
import app.witness.core.model.RecordId

data class AnchorReport(
    val anchored: Int = 0,
    val duplicates: Int = 0,
    val deferred: Int = 0,
    val resealed: Int = 0,
    val failed: Int = 0,
    val offline: Boolean = false,
    val needsRotation: Boolean = false,
    val previewsPending: Int = 0,
) {
    val shouldRetry: Boolean get() = offline || deferred > 0 || previewsPending > 0

    operator fun plus(o: AnchorReport) = AnchorReport(
        anchored + o.anchored,
        duplicates + o.duplicates,
        deferred + o.deferred,
        resealed + o.resealed,
        failed + o.failed,
        offline || o.offline,
        needsRotation || o.needsRotation,
        previewsPending + o.previewsPending,
    )
}

class AnchorOutbox(
    private val registry: RegistryGateway,
    private val outbox: OutboxStore,
    private val captures: CaptureStore,
    private val vault: KeyVault,
    private val engine: SealEngine,
    private val signer: SealSigner,
    private val clock: Clock,
    private val telemetry: Telemetry = Telemetry.None,
    private val previews: PublishPreviews? = null,
) {
    suspend operator fun invoke(): AnchorReport {
        val total = submitAll()
        val pending = if (total.offline || previews == null) 0 else previews()
        return finish(total.copy(previewsPending = pending))
    }

    private suspend fun submitAll(): AnchorReport {
        var total = AnchorReport()
        repeat(MAX_ROUNDS) {
            val round = pass()
            total += round
            if (round.resealed == 0 || round.offline) return total
        }
        return total
    }

    private fun finish(report: AnchorReport): AnchorReport {
        telemetry.event(
            AnalyticsEvent.ANCHOR_RUN,
            mapOf("anchored" to (report.anchored + report.duplicates).toLong(), "deferred" to report.deferred.toLong(), "failed" to report.failed.toLong()),
        )
        return report
    }

    private suspend fun pass(): AnchorReport {
        val now = clock.nowEpochMillis()
        val keys = HashMap<KeyId, DeviceKey?>()
        suspend fun keyOf(id: KeyId) = keys.getOrPut(id) { vault.key(id) }

        val submit = ArrayList<OutboxEntry>()
        val reseal = ArrayList<OutboxEntry>()
        val all = outbox.all()
        val queued = all.filter { it.state != OutboxState.FAILED }.mapTo(HashSet()) { it.recordId }
        for (e in all) {
            when (e.state) {
                OutboxState.PENDING -> if (e.notBeforeMs <= now) submit += e
                OutboxState.AWAITING_ENROLLMENT -> if (keyOf(e.keyId)?.isEnrolled == true) submit += e
                OutboxState.AWAITING_RESEAL -> reseal += e
                OutboxState.FAILED -> Unit
            }
        }
        var report = AnchorReport()
        for (batch in batches(submit)) {
            val results = try {
                registry.submit(batch.map { it.submission })
            } catch (_: OfflineException) {
                return report.copy(offline = true)
            }
            report += apply(batch, results, now, reseal, queued)
        }
        return report + resealAll(reseal)
    }

    private suspend fun apply(batch: List<OutboxEntry>, results: List<SubmitResult>, now: Long, reseal: MutableList<OutboxEntry>, queued: Set<RecordId>): AnchorReport {
        var anchored = 0
        var dup = 0
        var deferred = 0
        var failed = 0
        batch.forEachIndexed { i, e ->
            when (val r = results.getOrNull(i) ?: SubmitResult.RetryAfter(DEFAULT_RETRY_MS)) {
                is SubmitResult.Anchored -> {
                    captures.setStatus(e.recordId, AnchoringStatus.ANCHORED, r.anchor)
                    outbox.remove(e.recordId)
                    anchored++
                }
                is SubmitResult.Duplicate -> {
                    captures.setStatus(e.recordId, AnchoringStatus.ANCHORED, r.anchor)
                    outbox.remove(e.recordId)
                    dup++
                }
                is SubmitResult.RetryAfter -> {
                    outbox.update(e.copy(attempts = e.attempts + 1, notBeforeMs = now + r.delayMs.coerceAtLeast(0), state = OutboxState.PENDING, lastError = RETRY_AFTER))
                    deferred++
                }
                SubmitResult.KeyExpired -> {
                    val waiting = e.copy(state = OutboxState.AWAITING_RESEAL, lastError = KEY_EXPIRED)
                    outbox.update(waiting)
                    reseal += waiting
                }
                is SubmitResult.Rejected -> if (r.reason == PARENT_UNKNOWN && captures.get(e.recordId)?.parent in queued) {
                    outbox.update(e.copy(attempts = e.attempts + 1, notBeforeMs = now + DEFAULT_RETRY_MS, state = OutboxState.PENDING, lastError = PARENT_UNKNOWN))
                    deferred++
                } else {
                    outbox.update(e.copy(attempts = e.attempts + 1, state = OutboxState.FAILED, lastError = r.reason))
                    captures.setStatus(e.recordId, AnchoringStatus.FAILED, null)
                    failed++
                }
            }
        }
        return AnchorReport(anchored = anchored, duplicates = dup, deferred = deferred, failed = failed)
    }

    private suspend fun resealAll(entries: List<OutboxEntry>): AnchorReport {
        if (entries.isEmpty()) return AnchorReport()
        val key = vault.activeKey()?.takeIf { it.isEnrolled && !it.retired }
        var resealed = 0
        var needsRotation = false
        for (e in entries) {
            if (key == null || key.keyId == e.keyId) {
                needsRotation = true
                continue
            }
            val capture = captures.get(e.recordId) ?: run {
                outbox.remove(e.recordId)
                null
            } ?: continue
            val sealed = signer.seal(engine.reseal(e.recordId), key)
            captures.replace(
                e.recordId,
                capture.copy(recordId = sealed.recordId, keyRef = sealed.claim.keyRef, status = AnchoringStatus.QUEUED, anchor = null),
                OutboxEntry(sealed.submission),
                sealed.jpeg,
            )
            resealed++
        }
        return AnchorReport(resealed = resealed, needsRotation = needsRotation)
    }

    companion object {
        const val MAX_ROUNDS: Int = 3
        const val DEFAULT_RETRY_MS: Long = 30_000
        const val RETRY_AFTER: String = "retry_after"
        const val KEY_EXPIRED: String = "key_expired"
        const val PARENT_UNKNOWN: String = "parent_unknown"

        fun batches(entries: List<OutboxEntry>, max: Int = RegistryGateway.MAX_BATCH): List<List<OutboxEntry>> {
            val byKey = LinkedHashMap<KeyId, MutableList<OutboxEntry>>()
            for (e in entries) byKey.getOrPut(e.keyId) { ArrayList() }.add(e)
            val out = ArrayList<List<OutboxEntry>>()
            for (group in byKey.values) {
                group.sortBy { it.counter }
                group.chunkedTo(max, out)
            }
            return out
        }

        private fun <T> List<T>.chunkedTo(size: Int, out: MutableList<List<T>>) {
            var i = 0
            while (i < this.size) {
                out += subList(i, minOf(i + size, this.size))
                i += size
            }
        }
    }
}
