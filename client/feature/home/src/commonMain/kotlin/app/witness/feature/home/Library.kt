package app.witness.feature.home

import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.usecase.AnchoringSummary
import app.witness.core.domain.usecase.ManageCases
import app.witness.core.domain.usecase.ObserveAnchoring
import app.witness.core.domain.usecase.SearchIndex
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.CaseSummary
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.combine

class Library(val captures: List<Capture>, val caseList: List<Case>) {
    val cases: List<CaseSummary> by lazy { ManageCases.summarize(caseList, captures) }
    val anchoring: AnchoringSummary by lazy { ObserveAnchoring.summarize(captures) }
    val index: SearchIndex by lazy { SearchIndex.build(captures, caseList) }
    val caseTitles: Map<LocalCaseId, String> by lazy { caseList.associate { it.id to it.title } }
    val casesById: Map<LocalCaseId, CaseSummary> by lazy { cases.associateBy { it.case.id } }
    val feed: List<Capture> by lazy { feedOf(captures) }
    val photoNumbers: Map<RecordId, Int> by lazy { photoNumbersOf(captures) }

    val isEmpty: Boolean get() = captures.isEmpty() && caseList.isEmpty()

    companion object {
        val EMPTY: Library = Library(emptyList(), emptyList())

        fun observe(captures: CaptureStore, cases: CaseStore): Flow<Library> =
            combine(captures.observeAll(), cases.observeAll(), ::Library)

        fun feedOf(captures: List<Capture>): List<Capture> {
            val parents = HashSet<RecordId>()
            for (c in captures) c.parent?.let(parents::add)
            val out = ArrayList<Capture>()
            for (c in captures) if (c.caseId == null && c.recordId !in parents) out += c
            out.sortWith(newestFirst)
            return out
        }

        fun photoNumbersOf(captures: List<Capture>): Map<RecordId, Int> {
            val byCase = HashMap<LocalCaseId, MutableList<Capture>>()
            for (c in captures) c.caseId?.let { byCase.getOrPut(it) { ArrayList() }.add(c) }
            val out = HashMap<RecordId, Int>()
            for (members in byCase.values) {
                members.sortWith(oldestFirst)
                members.forEachIndexed { i, c -> out[c.recordId] = i + 1 }
            }
            return out
        }

        private val oldestFirst = compareBy<Capture>({ it.createdAtMs }, { it.recordId })
        private val newestFirst = compareByDescending<Capture> { it.createdAtMs }.thenBy { it.recordId }
    }
}
