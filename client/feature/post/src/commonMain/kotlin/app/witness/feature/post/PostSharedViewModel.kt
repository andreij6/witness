package app.witness.feature.post

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.Links
import app.witness.core.domain.Clock
import app.witness.core.model.Capture
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn

@Immutable
data class PostSharedUiState(
    val url: String,
    val linkText: String,
    val subtitle: String = "",
    val checked: String = "0",
    val shared: String = "just now",
    val shows: String,
)

class PostSharedViewModel(
    recordId: RecordId,
    precision: LocationPrecision,
    captures: CaptureStore,
    ticks: Flow<Long>,
    links: Links = Links(),
    private val utcOffset: (Long) -> Long = { 0L },
) : ViewModel() {
    private val url = links.proof(recordId)
    private val initial = PostSharedUiState(url, PostCopy.linkDisplay(url), shows = PostCopy.shows(precision, null))

    val state: StateFlow<PostSharedUiState> = combine(captures.observeAll(), ticks) { list, now ->
        var post: Capture? = null
        for (c in list) if (c.recordId == recordId) post = c
        val p = post ?: return@combine initial
        var sealedAt = p.createdAtMs
        val parentId = p.parent
        if (parentId != null) for (c in list) if (c.recordId == parentId) sealedAt = c.createdAtMs
        initial.copy(
            subtitle = PostCopy.sharedSubtitle(sealedAt, p, utcOffset(sealedAt)),
            shared = PostCopy.sharedAgo(now, p.createdAtMs),
            shows = PostCopy.shows(precision, p.place),
        )
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(STOP_TIMEOUT_MS), initial)

    companion object {
        const val STOP_TIMEOUT_MS: Long = 5_000
        const val TICK_MS: Long = 15_000

        fun ticks(clock: Clock, periodMs: Long = TICK_MS): Flow<Long> = flow {
            while (true) {
                emit(clock.nowEpochMillis())
                delay(periodMs)
            }
        }
    }
}
