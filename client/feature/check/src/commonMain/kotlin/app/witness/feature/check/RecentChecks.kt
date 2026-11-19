package app.witness.feature.check

import androidx.compose.runtime.Immutable
import app.witness.core.model.RecordId
import app.witness.core.model.Verdict
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.update

@Immutable
data class RecentCheck(val label: String, val checkedAtMs: Long, val verdict: Verdict, val recordId: RecordId?, val note: String? = null)

interface RecentChecks {
    fun observe(limit: Int): Flow<List<RecentCheck>>

    suspend fun record(check: RecentCheck)
}

class InMemoryRecentChecks(private val capacity: Int = DEFAULT_CAPACITY) : RecentChecks {
    private val state = MutableStateFlow<List<RecentCheck>>(emptyList())

    override fun observe(limit: Int): Flow<List<RecentCheck>> = state.map { it.take(limit) }

    override suspend fun record(check: RecentCheck) = state.update { current ->
        val next = ArrayList<RecentCheck>(minOf(current.size + 1, capacity))
        next += check
        for (c in current) {
            if (next.size == capacity) break
            next += c
        }
        next
    }

    companion object {
        const val DEFAULT_CAPACITY: Int = 50
    }
}
