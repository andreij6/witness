package app.witness.core.domain.usecase

import app.witness.core.domain.Clock
import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.AnchorScheduler
import app.witness.core.domain.port.Assistant
import app.witness.core.domain.port.AssistantContext
import app.witness.core.domain.port.BlurDetector
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.Links
import app.witness.core.domain.port.SealEngine
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.port.Telemetry
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurProposal
import app.witness.core.model.Capture
import app.witness.core.model.CaptureMode
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Edit
import app.witness.core.model.EditKind
import app.witness.core.model.LocationPrecision
import app.witness.core.model.OutboxEntry
import app.witness.core.model.Post
import app.witness.core.model.PostDraft
import app.witness.core.model.RecordId
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.first

class PostException(message: String) : Exception(message)

class BuildPost(
    private val captures: CaptureStore,
    private val detector: BlurDetector,
    private val assistant: Assistant,
    private val settings: SettingsStore,
    private val engine: SealEngine,
    private val signer: SealSigner,
    private val vault: KeyVault,
    private val scheduler: AnchorScheduler,
    private val clock: Clock,
    private val links: Links = Links(),
    private val telemetry: Telemetry = Telemetry.None,
) {
    suspend fun draft(parent: RecordId): PostDraft {
        val capture = sourceOf(captures.get(parent) ?: throw PostException("unknown capture"))
        if (!capture.isVerified) throw PostException("imports cannot be posted as sealed")
        val prefs = settings.settings.first()
        val blurs = detectBlurs(capture)
        val precision = prefs.defaultPrecision
        val coarse = coarseFor(capture.recordId, precision)
        val fallback = PostText.rulesAltText(blurs, capture.place)
        val ai = if (prefs.aiAssistant) suggestText(capture, blurs) else null
        val alt = PostText.clean(ai?.first, PostText.MAX_ALT)
        return PostDraft(
            parent = capture.recordId,
            dims = capture.dims,
            blurs = blurs,
            precision = precision,
            coarse = coarse,
            altText = alt ?: fallback,
            caption = PostText.clean(ai?.second, PostText.MAX_CAPTION).orEmpty(),
            aiGenerated = alt != null,
            locationWarning = precision.revealsPrecise,
        )
    }

    private suspend fun sourceOf(start: Capture): Capture {
        var c = start
        repeat(MAX_CHAIN) {
            if (c.origin != CaptureOrigin.DERIVATIVE) return c
            c = c.parent?.let { captures.get(it) } ?: return c
        }
        return c
    }

    suspend fun withPrecision(draft: PostDraft, precision: LocationPrecision): PostDraft =
        draft.copy(precision = precision, coarse = coarseFor(draft.parent, precision), locationWarning = precision.revealsPrecise)

    fun toggleBlur(draft: PostDraft, index: Int): PostDraft {
        if (index !in draft.blurs.indices) return draft
        val blurs = draft.blurs.toMutableList()
        blurs[index] = blurs[index].copy(enabled = !blurs[index].enabled)
        return draft.copy(blurs = blurs)
    }

    suspend fun publish(draft: PostDraft): Post {
        val parent = captures.get(draft.parent) ?: throw PostException("unknown capture")
        val keyRef = parent.keyRef ?: throw PostException("capture has no key")
        val key = vault.key(keyRef.keyId) ?: throw PostException("parent key unavailable")
        val edits = derivativeEdits(draft)
        val alt = PostText.clean(draft.altText, PostText.MAX_ALT) ?: PostText.rulesAltText(draft.blurs, parent.place)
        val caption = PostText.clean(draft.caption, PostText.MAX_CAPTION).orEmpty()
        val sealed = signer.seal(engine.derive(parent.recordId, edits, draft.coarse), key)
        val capture = Capture(
            recordId = sealed.recordId,
            captureId = parent.captureId,
            origin = CaptureOrigin.DERIVATIVE,
            status = if (key.isEnrolled) AnchoringStatus.QUEUED else AnchoringStatus.SEALED,
            createdAtMs = clock.nowEpochMillis(),
            keyRef = sealed.claim.keyRef,
            locationConfidence = parent.locationConfidence,
            dims = sealed.claim.dims,
            caseId = parent.caseId,
            parent = parent.recordId,
            edits = edits,
            label = parent.label,
            place = parent.place.takeIf { draft.precision != LocationPrecision.HIDDEN },
            caption = caption,
            altText = alt,
        )
        captures.insert(capture, OutboxEntry(sealed.submission), null, sealed.jpeg)
        if (key.isEnrolled) scheduler.requestAnchoring()
        telemetry.event(AnalyticsEvent.POST_PUBLISHED, mapOf("blurs" to edits.count { it.kind == EditKind.BLUR }.toLong(), "precision" to draft.precision.ordinal.toLong()))
        return Post(sealed.recordId, parent.recordId, edits, draft.precision, draft.coarse, alt, caption, links.proof(sealed.recordId))
    }

    private suspend fun detectBlurs(capture: Capture): List<BlurProposal> =
        try {
            detector.detect(capture.recordId)
                .mapNotNull { b -> b.rect.clippedTo(capture.dims.width, capture.dims.height)?.let { b.copy(rect = it) } }
                .sortedWith(compareBy({ it.rect.y }, { it.rect.x }, { it.blurClass.ordinal }))
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            emptyList()
        }

    private suspend fun suggestText(capture: Capture, blurs: List<BlurProposal>): Pair<String?, String?>? =
        try {
            if (!assistant.isAvailable()) null
            else assistant.suggest(AssistantContext(CaptureMode.POST, place = capture.place, detected = blurs.map { it.blurClass }, records = listOf(capture.recordId)), emptyList())
                ?.let { it.altText to it.caption }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            telemetry.nonFatal(e)
            null
        }

    private suspend fun coarseFor(parent: RecordId, precision: LocationPrecision): String? {
        val chars = precision.geohashChars ?: return null
        val opening = captures.opening(parent)?.takeIf { it.inRange } ?: return null
        return Geohash.of(opening, chars)
    }

    companion object {
        const val MAX_CHAIN: Int = 16
        const val MIN_SIGMA_E2: Int = 400
        const val MAX_SIGMA_E2: Int = 6_400
        val APPLY_ORDER: List<EditKind> = listOf(
            EditKind.BLUR, EditKind.CROP, EditKind.ROTATE, EditKind.TONE, EditKind.FILTER, EditKind.ORIENT, EditKind.RESIZE, EditKind.RECOMPRESS,
        )

        fun derivativeEdits(draft: PostDraft): List<Edit> {
            val out = ArrayList<Edit>(draft.blurs.size + draft.adjustments.size)
            for (b in draft.blurs) {
                if (!b.enabled) continue
                val rect = b.rect.clippedTo(draft.dims.width, draft.dims.height) ?: continue
                out += Edit.Blur(rect, b.blurClass, sigmaFor(rect.w, rect.h))
            }
            draft.adjustments.filter { it.kind != EditKind.BLUR }.sortedBy { APPLY_ORDER.indexOf(it.kind) }.forEach { out += it }
            if (out.size > Edit.MAX_EDITS) throw PostException("too many edits")
            out.firstOrNull { !it.isValid }?.let { throw PostException("invalid edit ${it.kind}") }
            return out
        }

        fun sigmaFor(w: Int, h: Int): Int = (minOf(w, h) * 100 / 8).coerceIn(MIN_SIGMA_E2, MAX_SIGMA_E2)
    }
}
