package app.witness.feature.home

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.usecase.SearchHit
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.ui.Formatters
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn

enum class SearchFilter(val label: String) { All("All"), Posts("Posts"), Cases("Cases"), Items("Items") }

@Immutable
data class SearchItemRow(val recordId: RecordId, val title: String, val subtitle: String, val state: WitnessState)

@Immutable
data class SearchCaseRow(val id: LocalCaseId, val title: String, val subtitle: String, val covers: List<RecordId>)

@Immutable
data class SearchUiState(
    val filter: SearchFilter = SearchFilter.All,
    val terms: List<String> = emptyList(),
    val items: List<SearchItemRow> = emptyList(),
    val cases: List<SearchCaseRow> = emptyList(),
) {
    val searched: Boolean get() = terms.isNotEmpty()
    val noResults: Boolean get() = searched && items.isEmpty() && cases.isEmpty()

    companion object {
        fun of(library: Library, query: String, filter: SearchFilter, time: TimeLabels): SearchUiState {
            val terms = Highlight.terms(query)
            if (terms.isEmpty()) return SearchUiState(filter)
            val items = ArrayList<SearchItemRow>()
            val cases = ArrayList<SearchCaseRow>()
            for (hit in library.index.search(query)) {
                when (hit) {
                    is SearchHit.CaseHit -> if (filter == SearchFilter.All || filter == SearchFilter.Cases) cases += caseRow(hit.case, library, terms, time)
                    is SearchHit.Item -> if (accepts(filter, hit.capture)) items += itemRow(hit.capture, library, terms, time)
                }
            }
            return SearchUiState(filter, terms, items, cases)
        }

        fun accepts(filter: SearchFilter, capture: Capture): Boolean = when (filter) {
            SearchFilter.All -> true
            SearchFilter.Posts -> capture.caseId == null
            SearchFilter.Items -> capture.caseId != null
            SearchFilter.Cases -> false
        }

        fun itemRow(c: Capture, library: Library, terms: List<String>, time: TimeLabels): SearchItemRow {
            val title = Formatters.joinDot(c.shortTitle(), photoLabel(library.photoNumbers[c.recordId]))
            val caseTitle = c.caseId?.let(library.caseTitles::get)
            val subtitle = when {
                Highlight.matches(title, terms) -> Formatters.joinDot(caseTitle, time.moment(c.createdAtMs))
                Highlight.matches(c.caption, terms) -> NOTE_PREFIX + c.caption!!.trim()
                Highlight.matches(c.altText, terms) -> NOTE_PREFIX + c.altText!!.trim()
                Highlight.matches(c.place, terms) -> Formatters.joinDot(c.place, time.moment(c.createdAtMs))
                else -> Formatters.joinDot(caseTitle, time.moment(c.createdAtMs))
            }
            return SearchItemRow(c.recordId, title, subtitle, c.witnessState())
        }

        fun caseRow(case: Case, library: Library, terms: List<String>, time: TimeLabels): SearchCaseRow {
            val summary = library.casesById[case.id]
            val text = case.summary?.takeIf { Highlight.matches(it, terms) }?.let { Highlight.snippet(it, terms) }
                ?: Formatters.joinDot(time.day(case.updatedAtMs), Formatters.plural(summary?.itemCount ?: 0, "item"))
            return SearchCaseRow(case.id, case.title, text, summary?.coverRecords.orEmpty())
        }

        const val NOTE_PREFIX = "Note: "
    }
}

class SearchViewModel(captures: CaptureStore, cases: CaseStore, private val time: TimeLabels) : ViewModel() {
    private val _query = MutableStateFlow("")
    val query: StateFlow<String> = _query.asStateFlow()
    private val filter = MutableStateFlow(SearchFilter.All)

    val state: StateFlow<SearchUiState> = combine(Library.observe(captures, cases), _query, filter) { lib, q, f -> SearchUiState.of(lib, q, f, time) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(HomeViewModel.STOP_TIMEOUT_MS), SearchUiState())

    fun onQuery(text: String) {
        _query.value = text.take(MAX_QUERY)
    }

    fun select(filter: SearchFilter) {
        this.filter.value = filter
    }

    companion object {
        const val MAX_QUERY: Int = 100
    }
}

object SearchCopy {
    const val PLACEHOLDER = "Search items, cases, plates…"
    const val ITEMS = "Items"
    const val CASES = "Cases"
    const val ON_DEVICE = "Search runs on your phone"
    fun noResults(query: String) = "No matches for “${query.trim()}”"
}
