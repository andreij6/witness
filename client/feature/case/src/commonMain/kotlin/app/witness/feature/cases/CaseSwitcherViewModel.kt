package app.witness.feature.cases

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.usecase.ManageCases
import app.witness.core.model.CaseSummary
import app.witness.core.model.LocalCaseId
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

@Immutable
data class CaseOptionUi(val id: LocalCaseId, val title: String, val subtitle: String, val selected: Boolean)

@Immutable
data class CaseSwitcherUiState(
    val options: List<CaseOptionUi> = emptyList(),
    val loading: Boolean = true,
    val done: Boolean = false,
    val error: String? = null,
)

internal data class SwitcherLocal(val done: Boolean = false, val error: String? = null, val pending: LocalCaseId? = null)

class CaseSwitcherViewModel(
    cases: CaseStore,
    captures: CaptureStore,
    settings: SettingsStore,
    private val manageCases: ManageCases,
) : ViewModel() {
    private val local = MutableStateFlow(SwitcherLocal())

    val state: StateFlow<CaseSwitcherUiState> = combine(cases.observeAll(), captures.observeAll(), settings.settings, local) { c, k, s, l ->
        CaseSwitcherUiState(options(ManageCases.summarize(c, k), l.pending ?: s.activeCase), loading = false, done = l.done, error = l.error)
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(CasesViewModel.STOP_TIMEOUT_MS), CaseSwitcherUiState())

    fun select(id: LocalCaseId) {
        if (local.value.pending != null) return
        local.update { it.copy(pending = id, error = null) }
        viewModelScope.launch {
            try {
                manageCases.setActive(id)
                local.update { it.copy(done = true) }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                local.update { SwitcherLocal(error = e.message ?: CaseReportViewModel.GENERIC_ERROR) }
            }
        }
    }

    companion object {
        fun options(summaries: List<CaseSummary>, active: LocalCaseId?): List<CaseOptionUi> = summaries.map {
            CaseOptionUi(it.case.id, it.case.title, CasePresentation.optionSubtitle(it.case, it.itemCount), it.case.id == active)
        }
    }
}
