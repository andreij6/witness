package app.witness.feature.cases

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.Clock
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.TemplateCatalog
import app.witness.core.domain.usecase.ManageCases
import app.witness.core.domain.usecase.PostText
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.LocalCaseId
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId
import app.witness.core.ui.Formatters
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

sealed interface CaseSheet {
    data class ReportTypes(val options: List<ReportType>, val selected: TemplateId, val suggested: TemplateId?) : CaseSheet

    data class Flag(val output: GeneratedOutput, val reason: FlagReason = FlagReason.WRONG, val note: String = "", val busy: Boolean = false) : CaseSheet

    data class Summary(val text: String) : CaseSheet

    data object ConfirmDelete : CaseSheet
}

@Immutable
data class CaseReportUiState(
    val caseId: LocalCaseId,
    val loading: Boolean = true,
    val missing: Boolean = false,
    val title: String = "",
    val subtitle: String = "",
    val templateId: TemplateId = TemplateId.GENERAL,
    val reportLabel: String = "",
    val evidenceTitle: String = "",
    val anchoringLine: String = "",
    val items: List<EvidenceItemUi> = emptyList(),
    val summary: String? = null,
    val summarySource: SummarySource? = null,
    val sheet: CaseSheet? = null,
    val error: String? = null,
)

internal data class CaseReportLocal(val sheet: CaseSheet? = null, val error: String? = null)

class CaseReportViewModel(
    private val caseId: LocalCaseId,
    cases: CaseStore,
    captures: CaptureStore,
    private val manageCases: ManageCases,
    private val catalog: TemplateCatalog,
    private val feedback: AiFeedback,
    private val clock: Clock,
    private val offset: UtcOffset,
) : ViewModel() {
    private val local = MutableStateFlow(CaseReportLocal())

    private val case = cases.observeAll().map { all -> all.firstOrNull { it.id == caseId } }.distinctUntilChanged()
    private val items = captures.observeAll().map { all -> evidenceOf(all, caseId) }.distinctUntilChanged()

    val state: StateFlow<CaseReportUiState> = combine(case, items, local) { c, i, l -> build(caseId, c, i, l, clock.nowEpochMillis(), offset) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(CasesViewModel.STOP_TIMEOUT_MS), CaseReportUiState(caseId))

    fun openReportTypes() = launchGuarded {
        val current = state.value
        val schemas = catalog.templates()
        val suggested = runCatchingNonCancel { manageCases.suggestTemplate(current.title) }
        local.update { it.copy(sheet = CaseSheet.ReportTypes(ReportType.options(schemas), current.templateId, suggested)) }
    }

    fun chooseReportType(type: ReportType) {
        if (!type.available) return
        if (type.templateId == state.value.templateId) return dismissSheet()
        launchGuarded {
            manageCases.changeTemplate(caseId, type.templateId)
            dismissSheet()
        }
    }

    fun openFlag(output: GeneratedOutput) = local.update { it.copy(sheet = CaseSheet.Flag(output)) }

    fun setFlagReason(reason: FlagReason) = updateFlag { it.copy(reason = reason) }

    fun setFlagNote(note: String) = updateFlag { it.copy(note = note.take(MAX_NOTE)) }

    fun sendFlag() {
        val sheet = local.value.sheet as? CaseSheet.Flag ?: return
        if (sheet.busy) return
        updateFlag { it.copy(busy = true) }
        launchGuarded {
            feedback.flag(AiFlag(caseId, sheet.output.target, sheet.reason, PostText.clean(sheet.note, MAX_NOTE), clock.nowEpochMillis()))
            dismissSheet()
        }
    }

    fun removeFlagged() {
        val sheet = local.value.sheet as? CaseSheet.Flag ?: return
        if (sheet.busy) return
        updateFlag { it.copy(busy = true) }
        launchGuarded {
            feedback.remove(caseId, sheet.output.target)
            if (sheet.output.target == SUMMARY_TARGET) manageCases.updateSummary(caseId, null)
            dismissSheet()
        }
    }

    fun editSummary() = local.update { it.copy(sheet = CaseSheet.Summary(state.value.summary.orEmpty())) }

    fun setSummaryDraft(text: String) = local.update { l ->
        val s = l.sheet as? CaseSheet.Summary ?: return@update l
        l.copy(sheet = s.copy(text = text.take(ManageCases.MAX_SUMMARY)))
    }

    fun saveSummary() {
        val sheet = local.value.sheet as? CaseSheet.Summary ?: return
        launchGuarded {
            manageCases.updateSummary(caseId, sheet.text)
            dismissSheet()
        }
    }

    fun requestDelete() = local.update { it.copy(sheet = CaseSheet.ConfirmDelete) }

    fun confirmDelete() = launchGuarded {
        manageCases.delete(caseId)
        dismissSheet()
    }

    fun dismissSheet() = local.update { it.copy(sheet = null) }

    fun clearError() = local.update { it.copy(error = null) }

    private fun updateFlag(transform: (CaseSheet.Flag) -> CaseSheet.Flag) = local.update { l ->
        val s = l.sheet as? CaseSheet.Flag ?: return@update l
        l.copy(sheet = transform(s))
    }

    private fun launchGuarded(block: suspend () -> Unit) {
        viewModelScope.launch {
            try {
                block()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                local.update { l -> l.copy(sheet = (l.sheet as? CaseSheet.Flag)?.copy(busy = false) ?: l.sheet, error = e.message ?: GENERIC_ERROR) }
            }
        }
    }

    companion object {
        const val SUMMARY_TARGET: String = "summary"
        const val MAX_NOTE: Int = 500
        const val GENERIC_ERROR: String = "Something went wrong. Try again."

        fun evidenceOf(all: List<Capture>, caseId: LocalCaseId): List<Capture> =
            all.filter { it.caseId == caseId }.sortedBy { it.createdAtMs }

        internal fun build(caseId: LocalCaseId, case: Case?, items: List<Capture>, local: CaseReportLocal, nowMs: Long, offset: UtcOffset): CaseReportUiState {
            if (case == null) return CaseReportUiState(caseId, loading = false, missing = true)
            var anchored = 0
            var queued = 0
            var sealed = 0
            var place: Capture? = null
            val evidence = ArrayList<EvidenceItemUi>(items.size)
            for (c in items) {
                val s = CasePresentation.evidenceState(c)
                evidence += EvidenceItemUi(c.recordId, s)
                if (c.isVerified) sealed++
                if (s == WitnessState.Anchored) anchored++
                if (s == WitnessState.Queued) queued++
                if (!c.place.isNullOrBlank() && (place == null || c.createdAtMs > place.createdAtMs)) place = c
            }
            return CaseReportUiState(
                caseId = caseId,
                loading = false,
                title = case.title,
                subtitle = CasePresentation.caseSubtitle(case, place?.place, nowMs, offset),
                templateId = case.templateId,
                reportLabel = "Report built on-device · " + ReportType.label(case.templateId),
                evidenceTitle = "Evidence · $sealed sealed",
                anchoringLine = Formatters.anchoringSummary(anchored, queued),
                items = evidence,
                summary = case.summary,
                summarySource = case.summarySource?.takeIf { case.summary != null },
                sheet = local.sheet,
                error = local.error,
            )
        }

        private suspend fun <T> runCatchingNonCancel(block: suspend () -> T): T? = try {
            block()
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            null
        }
    }
}
