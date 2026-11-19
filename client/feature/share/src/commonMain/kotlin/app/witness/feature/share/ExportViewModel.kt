package app.witness.feature.share

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.Clock
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.EntitlementSource
import app.witness.core.model.LocalCaseId
import app.witness.core.model.Tier
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

@Immutable
data class ExportUiState(
    val loading: Boolean = true,
    val missing: Boolean = false,
    val subtitle: String = "",
    val format: ExportFormat = ExportFormat.ZIP,
    val pdfLocked: Boolean = true,
    val exactLocation: Boolean = false,
    val exporting: Boolean = false,
    val error: String? = null,
) {
    val buttonLabel: String get() = if (format == ExportFormat.PDF) "Export PDF" else "Export ZIP"
    val locationHint: String get() = if (exactLocation) EXACT_HINT else ROUNDED_HINT
    val canExport: Boolean get() = !loading && !missing && !exporting && !(format == ExportFormat.PDF && pdfLocked)

    companion object {
        const val ROUNDED_HINT: String = "Off: location is rounded to the neighborhood."
        const val EXACT_HINT: String = "On: exact coordinates are included in the proof records."
    }
}

sealed interface ExportEvent {
    data class Exported(val result: ExportResult.Saved) : ExportEvent

    data object Upgrade : ExportEvent
}

internal data class ExportLocal(val format: ExportFormat = ExportFormat.ZIP, val exactLocation: Boolean = false, val exporting: Boolean = false, val error: String? = null)

class ExportViewModel(
    private val caseId: LocalCaseId,
    cases: CaseStore,
    captures: CaptureStore,
    entitlements: EntitlementSource,
    private val exporter: CaseExporter,
    private val clock: Clock,
) : ViewModel() {
    private val local = MutableStateFlow(ExportLocal())
    private val events = Channel<ExportEvent>(Channel.BUFFERED)
    val effects: Flow<ExportEvent> = events.receiveAsFlow()

    private val header = combine(cases.observeAll(), captures.observeAll()) { c, k ->
        val case = c.firstOrNull { it.id == caseId } ?: return@combine null
        ShareText.subtitle(case.title, k.count { it.caseId == caseId && it.isVerified })
    }

    private val plus = entitlements.entitlement().map { it.effectiveTier(clock.nowEpochMillis()) == Tier.PLUS }

    val state: StateFlow<ExportUiState> = combine(header, plus, local) { h, p, l ->
        if (h == null) {
            ExportUiState(loading = false, missing = true)
        } else {
            val format = if (l.format == ExportFormat.PDF && !p) ExportFormat.ZIP else l.format
            ExportUiState(false, false, h, format, !p, l.exactLocation, l.exporting, l.error)
        }
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(ShareCaseViewModel.STOP_TIMEOUT_MS), ExportUiState())

    fun select(format: ExportFormat) {
        if (format == ExportFormat.PDF && state.value.pdfLocked) {
            events.trySend(ExportEvent.Upgrade)
            return
        }
        local.update { it.copy(format = format, error = null) }
    }

    fun setExactLocation(on: Boolean) = local.update { it.copy(exactLocation = on) }

    fun export() {
        val s = state.value
        if (!s.canExport) return
        local.update { it.copy(exporting = true, error = null) }
        viewModelScope.launch {
            val message = try {
                when (val result = exporter.export(ExportRequest(caseId, s.format, s.exactLocation))) {
                    is ExportResult.Saved -> {
                        events.trySend(ExportEvent.Exported(result))
                        null
                    }
                    ExportResult.Unsupported -> UNSUPPORTED
                    ExportResult.NothingToExport -> NOTHING
                }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                FAILED
            }
            local.update { it.copy(exporting = false, error = message) }
        }
    }

    companion object {
        const val UNSUPPORTED: String = "This format isn't available on this phone yet."
        const val NOTHING: String = "There are no sealed photos to export."
        const val FAILED: String = "Couldn't save the export. Check storage and try again."
    }
}
