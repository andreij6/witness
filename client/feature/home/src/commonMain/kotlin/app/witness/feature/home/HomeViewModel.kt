package app.witness.feature.home

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.KeyVault
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.DeviceKey
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.ui.Formatters
import app.witness.core.ui.StateCount
import app.witness.core.ui.stateCounts
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

enum class HomeFilter(val label: String) { All("All"), Posts("Posts"), Cases("Cases") }

enum class KeyState { Unknown, Missing, Pending, Ready }

@Immutable
data class TrustLine(val text: String, val tone: Tone, val icon: WitnessIcon)

@Immutable
data class FeedItem(val recordId: RecordId, val title: String, val meta: String, val state: WitnessState)

@Immutable
data class CaseItem(val id: LocalCaseId, val title: String, val meta: String, val counts: List<StateCount>, val covers: List<RecordId>)

@Immutable
data class HomeUiState(
    val loading: Boolean = true,
    val firstRun: Boolean = false,
    val trust: TrustLine = HomeCopy.checking,
    val filter: HomeFilter = HomeFilter.All,
    val featured: FeedItem? = null,
    val tiles: List<FeedItem> = emptyList(),
    val cases: List<CaseItem> = emptyList(),
) {
    val showPosts: Boolean get() = filter != HomeFilter.Cases
    val showCases: Boolean get() = filter != HomeFilter.Posts

    companion object {
        fun of(library: Library, filter: HomeFilter, key: KeyState, time: TimeLabels): HomeUiState {
            val feed = if (filter == HomeFilter.Cases) emptyList() else library.feed
            return HomeUiState(
                loading = key == KeyState.Unknown,
                firstRun = library.isEmpty,
                trust = trustLine(library, key),
                filter = filter,
                featured = feed.firstOrNull()?.let { c -> FeedItem(c.recordId, c.headline(), Formatters.joinDot(time.moment(c.createdAtMs), c.place), c.witnessState()) },
                tiles = if (feed.size <= 1) emptyList() else feed.subList(1, feed.size).map { c ->
                    val meta = if (c.origin == CaptureOrigin.IMPORT) IMPORTED_FROM_GALLERY else Formatters.joinDot(time.day(c.createdAtMs), c.place)
                    FeedItem(c.recordId, c.shortTitle(), meta, c.witnessState())
                },
                cases = if (filter == HomeFilter.Posts) emptyList() else library.cases.map { s ->
                    CaseItem(
                        s.case.id,
                        s.case.title,
                        Formatters.joinDot(time.day(s.case.updatedAtMs), Formatters.plural(s.itemCount, "item")),
                        stateCounts(s.anchoredCount, s.queuedCount),
                        s.coverRecords,
                    )
                },
            )
        }

        fun trustLine(library: Library, key: KeyState): TrustLine {
            if (library.captures.isNotEmpty()) {
                val a = library.anchoring
                return TrustLine(Formatters.anchoringSummary(a.anchored, a.queued + a.sealedOnly, a.sealed), Tone.Ok, WitnessIcon.Verified)
            }
            return when (key) {
                KeyState.Ready -> HomeCopy.ready
                KeyState.Pending -> HomeCopy.pending
                KeyState.Missing -> HomeCopy.missing
                KeyState.Unknown -> HomeCopy.checking
            }
        }

        fun keyState(key: DeviceKey?): KeyState = when {
            key == null || key.retired -> KeyState.Missing
            key.isEnrolled -> KeyState.Ready
            else -> KeyState.Pending
        }
    }
}

object HomeCopy {
    val ready = TrustLine("Ready · hardware key active", Tone.Ok, WitnessIcon.Verified)
    val pending = TrustLine("Sealing works now · setup finishes online", Tone.Warn, WitnessIcon.HourglassTop)
    val missing = TrustLine("Not set up · this phone can still check photos", Tone.Muted, WitnessIcon.Info)
    val checking = TrustLine("", Tone.Muted, WitnessIcon.Verified)
    const val EMPTY_TITLE = "Take your first sealed photo"
    const val EMPTY_LEAD = "Sealed by your phone the moment you press the shutter, then anchored so anyone can check it."
    const val OPEN_CAMERA = "Open camera"
    const val CHECK_TITLE = "Got a photo to check?"
    const val CHECK_DETAIL = "Use the Check tab"
    const val CASES = "Cases"
    const val NEW_CASE = "+ New case"
    const val NO_POSTS = "No posts yet"
    const val NO_CASES = "No cases yet. Start one to group evidence."
}

class HomeViewModel(captures: CaptureStore, cases: CaseStore, private val vault: KeyVault, private val time: TimeLabels) : ViewModel() {
    private val filter = MutableStateFlow(HomeFilter.All)
    private val key = MutableStateFlow(KeyState.Unknown)

    val state: StateFlow<HomeUiState> = combine(Library.observe(captures, cases), filter, key) { lib, f, k -> HomeUiState.of(lib, f, k, time) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(STOP_TIMEOUT_MS), HomeUiState())

    init {
        refreshKey()
    }

    fun select(filter: HomeFilter) {
        this.filter.value = filter
    }

    fun refreshKey() {
        viewModelScope.launch { key.value = HomeUiState.keyState(vault.activeKey()) }
    }

    companion object {
        const val STOP_TIMEOUT_MS: Long = 5_000
    }
}
