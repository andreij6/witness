package app.witness.core.domain.usecase

import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.LocalCaseId
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged

sealed interface SearchHit {
    data class Item(val capture: Capture) : SearchHit

    data class CaseHit(val case: Case) : SearchHit
}

class SearchIndex private constructor(
    private val hits: List<SearchHit>,
    private val terms: Array<String>,
    private val postings: Array<IntArray>,
) {
    val size: Int get() = hits.size

    fun search(query: String, limit: Int = DEFAULT_LIMIT): List<SearchHit> {
        val words = Tokens.of(query).distinct()
        if (words.isEmpty() || limit <= 0 || hits.isEmpty()) return emptyList()
        var acc: LongArray? = null
        for (w in words) {
            val bits = LongArray((hits.size + 63) ushr 6)
            var any = false
            var i = lowerBound(w)
            while (i < terms.size && terms[i].startsWith(w)) {
                for (doc in postings[i]) bits[doc ushr 6] = bits[doc ushr 6] or (1L shl (doc and 63))
                any = true
                i++
            }
            if (!any) return emptyList()
            acc = acc?.also { a -> for (k in a.indices) a[k] = a[k] and bits[k] } ?: bits
        }
        val out = ArrayList<SearchHit>(minOf(limit, hits.size))
        val set = acc!!
        for (word in set.indices) {
            var w = set[word]
            while (w != 0L) {
                val doc = (word shl 6) + w.countTrailingZeroBits()
                out += hits[doc]
                if (out.size == limit) return out
                w = w and (w - 1)
            }
        }
        return out
    }

    private fun lowerBound(prefix: String): Int {
        var lo = 0
        var hi = terms.size
        while (lo < hi) {
            val mid = (lo + hi) ushr 1
            if (terms[mid] < prefix) lo = mid + 1 else hi = mid
        }
        return lo
    }

    companion object {
        const val DEFAULT_LIMIT: Int = 50
        val EMPTY: SearchIndex = SearchIndex(emptyList(), emptyArray(), emptyArray())

        fun build(captures: List<Capture>, cases: List<Case>): SearchIndex {
            val caseTitles = cases.associate { it.id to it.title }
            val hits = ArrayList<SearchHit>(captures.size + cases.size)
            cases.sortedByDescending { it.updatedAtMs }.forEach { hits += SearchHit.CaseHit(it) }
            captures.sortedByDescending { it.createdAtMs }.forEach { hits += SearchHit.Item(it) }
            val index = HashMap<String, IntList>()
            hits.forEachIndexed { doc, hit ->
                val seen = HashSet<String>()
                for (field in fields(hit, caseTitles)) for (t in Tokens.of(field)) if (seen.add(t)) index.getOrPut(t) { IntList() }.add(doc)
            }
            val terms = index.keys.toTypedArray().also { it.sort() }
            return SearchIndex(hits, terms, Array(terms.size) { index.getValue(terms[it]).toArray() })
        }

        private fun fields(hit: SearchHit, caseTitles: Map<LocalCaseId, String>): List<String> = when (hit) {
            is SearchHit.CaseHit -> listOfNotNull(hit.case.title, hit.case.summary, hit.case.templateId.value.replace('_', ' '))
            is SearchHit.Item -> with(hit.capture) {
                listOfNotNull(label, place, caption, altText, recordId.text, status.name, origin.name, caseId?.let(caseTitles::get))
            }
        }
    }

    private class IntList {
        private var data = IntArray(4)
        private var n = 0

        fun add(v: Int) {
            if (n == data.size) data = data.copyOf(n * 2)
            data[n++] = v
        }

        fun toArray(): IntArray = data.copyOf(n)
    }
}

class Search(private val captures: CaptureStore, private val cases: CaseStore) {
    fun index(): Flow<SearchIndex> = combine(captures.observeAll(), cases.observeAll(), SearchIndex::build)

    fun results(query: Flow<String>, limit: Int = SearchIndex.DEFAULT_LIMIT): Flow<List<SearchHit>> =
        combine(index(), query.distinctUntilChanged()) { idx, q -> idx.search(q, limit) }
}
