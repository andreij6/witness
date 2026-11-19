package app.witness.feature.share

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.Clock
import app.witness.core.domain.port.BlurDetector
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.usecase.ShareCase
import app.witness.core.domain.usecase.ShareOutcome
import app.witness.core.model.BlurClass
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.CaseShare
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.model.ShareExpiry
import app.witness.core.ui.Formatters
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

@Immutable
data class ShareItemUi(val recordId: RecordId, val selected: Boolean, val edited: Boolean)

@Immutable
data class PrivacyChipUi(val option: PrivacyOption, val label: String, val on: Boolean)

@Immutable
data class ShareLinkUi(val url: String, val display: String, val meta: String, val stale: Boolean)

@Immutable
data class ShareCaseUiState(
    val loading: Boolean = true,
    val missing: Boolean = false,
    val subtitle: String = "",
    val items: List<ShareItemUi> = emptyList(),
    val selectedLabel: String = "",
    val privacy: List<PrivacyChipUi> = emptyList(),
    val expiry: ShareExpiry = ShareExpiry.WEEK,
    val includeSummary: Boolean = true,
    val summaryLabel: String = ShareText.INCLUDE_SUMMARY,
    val link: ShareLinkUi? = null,
    val recipients: String = "",
    val sharing: Boolean = false,
    val error: String? = null,
) {
    val selectedCount: Int get() = items.count { it.selected }
    val canShare: Boolean get() = !sharing && selectedCount > 0
}

sealed interface ShareEvent {
    data class SendLink(val url: String) : ShareEvent
}

internal data class ShareChoices(
    val deselected: Set<RecordId> = emptySet(),
    val privacy: Set<PrivacyOption> = emptySet(),
    val expiry: ShareExpiry = ShareExpiry.WEEK,
    val includeSummary: Boolean = true,
)

internal data class ShareLocal(
    val choices: ShareChoices = ShareChoices(),
    val detections: Map<RecordId, Set<BlurClass>> = emptyMap(),
    val share: CaseShare? = null,
    val shareChoices: ShareChoices? = null,
    val sharing: Boolean = false,
    val error: String? = null,
)

class ShareCaseViewModel(
    private val caseId: LocalCaseId,
    cases: CaseStore,
    captures: CaptureStore,
    private val shareCase: ShareCase,
    private val detector: BlurDetector?,
    private val redactor: ShareRedactor?,
    private val clock: Clock,
    private val offset: UtcOffset,
) : ViewModel() {
    private val local = MutableStateFlow(ShareLocal())
    private val events = Channel<ShareEvent>(Channel.BUFFERED)
    val effects: Flow<ShareEvent> = events.receiveAsFlow()

    private val case = cases.observeAll().map { all -> all.firstOrNull { it.id == caseId } }.distinctUntilChanged()
    private val items = captures.observeAll().map { all -> shareable(all, caseId) }.distinctUntilChanged()

    val state: StateFlow<ShareCaseUiState> = combine(case, items, local) { c, i, l -> build(c, i, l, redactor != null, clock.nowEpochMillis(), offset) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(STOP_TIMEOUT_MS), ShareCaseUiState())

    init {
        viewModelScope.launch {
            val c = case.first() ?: return@launch
            latestActive(c, clock.nowEpochMillis())?.let { s -> local.update { it.copy(share = s, shareChoices = it.choices) } }
            detect(items.first())
        }
    }

    fun toggleItem(id: RecordId) = choose { it.copy(deselected = if (id in it.deselected) it.deselected - id else it.deselected + id) }

    fun togglePrivacy(option: PrivacyOption) {
        if (redactor == null) return
        choose { it.copy(privacy = if (option in it.privacy) it.privacy - option else it.privacy + option) }
    }

    fun setExpiry(expiry: ShareExpiry) = choose { it.copy(expiry = expiry) }

    fun setIncludeSummary(include: Boolean) = choose { it.copy(includeSummary = include) }

    fun clearError() = local.update { it.copy(error = null) }

    fun shareLink() {
        val l = local.value
        if (l.sharing) return
        val s = l.share
        if (s != null && l.shareChoices == l.choices) {
            events.trySend(ShareEvent.SendLink(s.url))
            return
        }
        local.update { it.copy(sharing = true, error = null) }
        viewModelScope.launch {
            try {
                val snapshot = local.value.choices
                val selected = items.first().filter { it.recordId !in snapshot.deselected }
                val ids = ArrayList<RecordId>(selected.size)
                for (c in selected) {
                    val plan = RedactionPlan.forItem(local.value.detections[c.recordId].orEmpty(), snapshot.privacy)
                    ids += if (plan.isEmpty || redactor == null) c.recordId else redactor.redact(c.recordId, plan)
                }
                when (val outcome = shareCase(caseId, ids, snapshot.expiry, snapshot.includeSummary)) {
                    is ShareOutcome.Shared -> {
                        local.update { it.copy(share = outcome.share, shareChoices = snapshot, sharing = false) }
                        events.trySend(ShareEvent.SendLink(outcome.share.url))
                    }
                    is ShareOutcome.TooLarge -> fail(tooLarge(outcome.bytes, outcome.limit))
                    ShareOutcome.NothingToShare -> fail(NOTHING_TO_SHARE)
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                fail(UPLOAD_FAILED)
            }
        }
    }

    private fun fail(message: String) = local.update { it.copy(sharing = false, error = message) }

    private fun choose(transform: (ShareChoices) -> ShareChoices) = local.update { it.copy(choices = transform(it.choices), error = null) }

    private suspend fun detect(list: List<Capture>) {
        val d = detector ?: return
        val found = HashMap<RecordId, Set<BlurClass>>(list.size * 2)
        for (c in list) {
            val classes = try {
                d.detect(c.recordId).filter { it.enabled }.mapTo(HashSet()) { it.blurClass }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                emptySet()
            }
            if (classes.isNotEmpty()) found[c.recordId] = classes
        }
        val faces = found.values.any { BlurClass.FACE in it }
        local.update { l ->
            val privacy = if (faces && redactor != null) l.choices.privacy + PrivacyOption.BLUR_FACES else l.choices.privacy
            val adopted = l.shareChoices?.let { if (it == l.choices) it.copy(privacy = privacy) else it }
            l.copy(detections = found, choices = l.choices.copy(privacy = privacy), shareChoices = adopted)
        }
    }

    companion object {
        const val STOP_TIMEOUT_MS: Long = 5_000
        const val NOTHING_TO_SHARE: String = "Select at least one sealed photo."
        const val UPLOAD_FAILED: String = "Couldn't create the link. Check your connection and try again."

        fun tooLarge(bytes: Long, limit: Long): String =
            "This selection is ${Formatters.bytes(bytes)}. Links hold up to ${Formatters.bytes(limit)} — remove a few items or get Witness Plus."

        fun shareable(all: List<Capture>, caseId: LocalCaseId): List<Capture> =
            all.filter { it.caseId == caseId && it.isVerified }.sortedBy { it.createdAtMs }

        fun latestActive(case: Case, nowMs: Long): CaseShare? =
            case.shares.filter { !it.isExpired(nowMs) }.maxByOrNull { it.createdAtMs }

        fun isEdited(capture: Capture): Boolean = capture.origin == CaptureOrigin.DERIVATIVE || capture.edits.isNotEmpty()

        internal fun build(case: Case?, items: List<Capture>, local: ShareLocal, canRedact: Boolean, nowMs: Long, offset: UtcOffset): ShareCaseUiState {
            if (case == null) return ShareCaseUiState(loading = false, missing = true)
            val choices = local.choices
            val ui = ArrayList<ShareItemUi>(items.size)
            val selectedIds = ArrayList<RecordId>(items.size)
            var edits = 0
            for (c in items) {
                val selected = c.recordId !in choices.deselected
                val redacts = !RedactionPlan.forItem(local.detections[c.recordId].orEmpty(), choices.privacy).blur.isEmpty()
                val edited = isEdited(c) || redacts
                ui += ShareItemUi(c.recordId, selected, edited)
                if (selected) {
                    selectedIds += c.recordId
                    if (edited) edits++
                }
            }
            val counts = RedactionPlan.counts(local.detections, selectedIds)
            val privacy = if (!canRedact) emptyList() else PrivacyOption.entries.map { o ->
                val n = o.blurClass?.let { counts[it] } ?: 0
                PrivacyChipUi(o, if (n > 0) o.label + Formatters.Dot + n else o.label, o in choices.privacy)
            }
            val share = local.share
            return ShareCaseUiState(
                loading = false,
                subtitle = ShareText.subtitle(case.title, items.size),
                items = ui,
                selectedLabel = ShareText.selected(selectedIds.size, items.size),
                privacy = privacy,
                expiry = choices.expiry,
                includeSummary = choices.includeSummary,
                summaryLabel = ShareText.includeSummary(case.summarySource),
                link = share?.let { ShareLinkUi(it.url, ShareText.displayLink(it.url), ShareText.linkMeta(it.expiresAtMs, it.itemCount, nowMs, offset), local.shareChoices != choices) },
                recipients = ShareText.recipients(selectedIds.size, selectedIds.size, edits),
                sharing = local.sharing,
                error = local.error,
            )
        }
    }
}
