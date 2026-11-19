package app.witness.feature.home

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.port.AnchorScheduler
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.OutboxStore
import app.witness.core.domain.usecase.AnchorOutbox
import app.witness.core.domain.usecase.KeyRotation
import app.witness.core.domain.usecase.RotationStatus
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.OutboxEntry
import app.witness.core.model.OutboxState
import app.witness.core.model.RecordId
import app.witness.core.ui.Formatters
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

@Immutable
data class QueueReason(val text: String, val icon: WitnessIcon)

@Immutable
data class AnchorRow(val recordId: RecordId, val title: String, val detail: String, val state: WitnessState, val reason: QueueReason? = null)

@Immutable
data class KeyLine(val title: String, val detail: String)

@Immutable
data class AnchoringUiState(
    val loading: Boolean = true,
    val anchored: Int = 0,
    val queued: Int = 0,
    val lastAnchored: String = AnchoringCopy.NOTHING_YET,
    val queue: List<AnchorRow> = emptyList(),
    val recent: List<AnchorRow> = emptyList(),
    val key: KeyLine? = null,
) {
    companion object {
        fun of(library: Library, outbox: List<OutboxEntry>, key: KeyLine?, time: TimeLabels): AnchoringUiState {
            val a = library.anchoring
            val entries = HashMap<RecordId, OutboxEntry>(outbox.size)
            for (e in outbox) entries[e.recordId] = e
            return AnchoringUiState(
                loading = false,
                anchored = a.anchored,
                queued = a.queued + a.sealedOnly,
                lastAnchored = a.recent.firstOrNull()?.anchor?.let { "${AnchoringCopy.LAST_ANCHORED} ${time.ago(it.anchoredAtMs)}" } ?: AnchoringCopy.NOTHING_YET,
                queue = a.queue.map { c ->
                    AnchorRow(c.recordId, Formatters.joinDot(c.shortTitle(), c.caseId?.let(library.caseTitles::get)), "", c.witnessState(), reasonOf(c, entries[c.recordId]))
                },
                recent = a.recent.map { c ->
                    val where = c.caseId?.let(library.caseTitles::get)
                    val detail = Formatters.joinDot(
                        where ?: AnchoringCopy.POST,
                        if (where != null) photoLabel(library.photoNumbers[c.recordId]) else null,
                        c.anchor?.let { time.ago(it.anchoredAtMs) },
                    )
                    AnchorRow(c.recordId, c.headline(), detail, WitnessState.Anchored)
                },
                key = key,
            )
        }

        fun reasonOf(capture: Capture, entry: OutboxEntry?): QueueReason = when {
            entry == null && capture.status == AnchoringStatus.SEALED -> AnchoringCopy.awaitingSetup
            entry == null -> AnchoringCopy.waitingNetwork
            entry.state == OutboxState.AWAITING_ENROLLMENT -> AnchoringCopy.awaitingSetup
            entry.state == OutboxState.AWAITING_RESEAL -> AnchoringCopy.awaitingKey
            entry.state == OutboxState.FAILED -> AnchoringCopy.failed
            entry.lastError == AnchorOutbox.RETRY_AFTER -> AnchoringCopy.retrying
            else -> AnchoringCopy.waitingNetwork
        }

        fun keyLine(status: RotationStatus?): KeyLine? = status?.let {
            val cadence = when {
                it.rotationDue -> AnchoringCopy.ROTATION_DUE
                it.rotatesInDays == 1 -> "rotates tomorrow"
                else -> "rotates in ${it.rotatesInDays} days"
            }
            KeyLine(Formatters.joinDot(AnchoringCopy.ACTIVE, it.security.label(), cadence), AnchoringCopy.KEY_DETAIL)
        }
    }
}

object AnchoringCopy {
    const val TITLE = "Anchoring"
    const val ANCHORED = "Anchored"
    const val QUEUED = "Queued"
    const val LAST_ANCHORED = "Last anchored"
    const val NOTHING_YET = "Nothing anchored yet"
    const val CERTIFIED = "Certified by the Internet Computer"
    const val QUEUE = "Queue"
    const val RETRY = "Retry now"
    const val RECENT = "Recently anchored"
    const val YOUR_KEY = "Your key"
    const val POST = "Post"
    const val ACTIVE = "Active"
    const val ROTATION_DUE = "rotation due"
    const val KEY_DETAIL = "Previous keys keep signing edits of the photos they sealed"
    const val ALL_ANCHORED = "Everything you've sealed is anchored"
    val waitingNetwork = QueueReason("Waiting for network", WitnessIcon.WifiOff)
    val retrying = QueueReason("Registry busy · retrying soon", WitnessIcon.Schedule)
    val awaitingSetup = QueueReason("Waiting for device setup", WitnessIcon.HourglassTop)
    val awaitingKey = QueueReason("Waiting for a new key", WitnessIcon.VpnKey)
    val failed = QueueReason("Couldn't anchor", WitnessIcon.Error)
}

class AnchoringViewModel(
    captures: CaptureStore,
    cases: CaseStore,
    outbox: OutboxStore,
    private val rotation: KeyRotation,
    private val scheduler: AnchorScheduler,
    private val time: TimeLabels,
) : ViewModel() {
    private val key = MutableStateFlow<KeyLine?>(null)

    val state: StateFlow<AnchoringUiState> = combine(Library.observe(captures, cases), outbox.observe(), key) { lib, entries, k -> AnchoringUiState.of(lib, entries, k, time) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(HomeViewModel.STOP_TIMEOUT_MS), AnchoringUiState())

    init {
        viewModelScope.launch { key.value = AnchoringUiState.keyLine(rotation.status()) }
    }

    fun retry() = scheduler.requestAnchoring()
}
