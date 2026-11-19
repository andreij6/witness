package app.witness.feature.cases

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.Clock
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.usecase.ManageCases
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn

@Immutable
data class CasesUiState(
    val filter: CaseFilter = CaseFilter.ALL,
    val cases: List<CaseCardUi> = emptyList(),
    val loading: Boolean = true,
) {
    val isEmpty: Boolean get() = !loading && cases.isEmpty()
}

class CasesViewModel(cases: CaseStore, captures: CaptureStore, private val clock: Clock, private val offset: UtcOffset) : ViewModel() {
    private val filter = MutableStateFlow(CaseFilter.ALL)

    private val cards = combine(cases.observeAll(), captures.observeAll()) { c, k ->
        val now = clock.nowEpochMillis()
        val places = CasePresentation.places(k)
        ManageCases.summarize(c, k).map { CasePresentation.card(it, places[it.case.id], now, offset) }
    }

    val state: StateFlow<CasesUiState> = combine(cards, filter) { all, f -> CasesUiState(f, CasePresentation.filter(all, f), loading = false) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(STOP_TIMEOUT_MS), CasesUiState())

    fun setFilter(value: CaseFilter) {
        filter.value = value
    }

    companion object {
        const val STOP_TIMEOUT_MS: Long = 5_000
    }
}
