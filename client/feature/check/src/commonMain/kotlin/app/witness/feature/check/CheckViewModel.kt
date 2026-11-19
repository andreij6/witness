package app.witness.feature.check

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.Clock
import app.witness.core.domain.port.Links
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.domain.usecase.CheckProgress
import app.witness.core.model.ByteString
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.ui.CheckStage
import app.witness.core.ui.title
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.toImmutableList
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

class CheckDependencies(
    val checkImage: CheckImage,
    val media: MediaGateway,
    val clock: Clock,
    val recent: RecentChecks = InMemoryRecentChecks(),
    val links: Links = Links(),
    val time: TimeText = TimeText.UTC,
    val shared: (String) -> PickedImage? = { null },
)

enum class CheckPhase { Idle, Checking, Done, Failed }

@Immutable
data class CheckUiState(
    val phase: CheckPhase = CheckPhase.Idle,
    val image: PickedImage? = null,
    val stages: ImmutableList<CheckStage> = persistentListOf(),
    val outcome: CheckOutcome? = null,
    val reference: ByteString? = null,
    val error: String? = null,
    val link: String = "",
    val linkError: String? = null,
    val recent: ImmutableList<RecentCheck> = persistentListOf(),
    val notifyWhenAnchored: Boolean = false,
    val shareText: String? = null,
)

class CheckViewModel(
    private val deps: CheckDependencies,
    private val surface: CheckSurface,
    private val source: String? = null,
    private val maxBytes: Int = PickedImage.MAX_BYTES,
) : ViewModel() {
    private val mutable = MutableStateFlow(CheckUiState(stages = StageText.map(CheckProgress.INITIAL.stages, surface)))
    val state: StateFlow<CheckUiState> = mutable.asStateFlow()
    private var job: Job? = null

    init {
        viewModelScope.launch {
            deps.recent.observe(RECENT_LIMIT).collect { list -> mutable.update { it.copy(recent = list.toImmutableList()) } }
        }
    }

    fun check(image: PickedImage) {
        job?.cancel()
        if (image.size > maxBytes) {
            mutable.update { it.copy(phase = CheckPhase.Failed, image = null, outcome = null, reference = null, error = TOO_LARGE) }
            return
        }
        mutable.update {
            it.copy(
                phase = CheckPhase.Checking,
                image = image,
                stages = StageText.map(CheckProgress.INITIAL.stages, surface),
                outcome = null,
                reference = null,
                error = null,
                linkError = null,
                shareText = null,
            )
        }
        job = viewModelScope.launch {
            try {
                deps.checkImage(image.bytes).collect { progress -> onProgress(image, progress) }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                mutable.update { it.copy(phase = CheckPhase.Failed, error = FAILED) }
            }
        }
    }

    private suspend fun onProgress(image: PickedImage, progress: CheckProgress) {
        val stages = StageText.map(progress.stages, surface)
        val result = progress.result
        when {
            progress.error != null -> mutable.update { it.copy(phase = CheckPhase.Failed, stages = stages, error = FAILED) }
            result == null -> mutable.update { it.copy(stages = stages) }
            else -> {
                val outcome = CheckOutcome.of(result, image.dims, deps.time)
                val reference = outcome.recordId
                    ?.takeIf { outcome.hasReference && VerdictFlag.NO_PUBLIC_REFERENCE !in result.flags }
                    ?.let { id -> runCatchingReference(id) }
                val share = outcome.recordId?.let { "Witness check: ${result.verdict.title()} — ${deps.links.proof(it)}" }
                mutable.update { it.copy(phase = CheckPhase.Done, stages = stages, outcome = outcome, reference = reference, shareText = share) }
                if (!outcome.registryUnverified) deps.recent.record(RecentCheck(image.name ?: DEFAULT_LABEL, deps.clock.nowEpochMillis(), result.verdict, outcome.recordId, source))
            }
        }
    }

    private suspend fun runCatchingReference(id: app.witness.core.model.RecordId): ByteString? =
        try {
            deps.media.reference(id)
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            null
        }

    fun checkShared(id: String) {
        val image = deps.shared(id)
        if (image != null) check(image) else mutable.update { it.copy(phase = CheckPhase.Failed, error = MISSING) }
    }

    fun cancel() {
        job?.cancel()
        job = null
        mutable.update { it.copy(phase = CheckPhase.Idle, image = null, outcome = null, reference = null, error = null, stages = StageText.map(CheckProgress.INITIAL.stages, surface)) }
    }

    fun reset() = cancel()

    fun onLinkChange(text: String) = mutable.update { it.copy(link = text.take(PastedLink.MAX_LENGTH), linkError = null) }

    fun submitLink(text: String = state.value.link): PastedLink? {
        val parsed = PastedLink.parse(text, deps.links)
        if (parsed is PastedLink.Problem) {
            mutable.update { it.copy(link = text.take(PastedLink.MAX_LENGTH), linkError = parsed.message) }
            return null
        }
        mutable.update { it.copy(link = "", linkError = null) }
        return parsed
    }

    fun setNotifyWhenAnchored(on: Boolean) = mutable.update { it.copy(notifyWhenAnchored = on && it.outcome?.verdict == Verdict.SEALED_UNANCHORED) }

    companion object {
        const val RECENT_LIMIT: Int = 3
        const val DEFAULT_LABEL: String = "Photo"
        const val FAILED: String = "We couldn't finish this check. Check your connection and try again."
        const val TOO_LARGE: String = "This file is too large to check."
        const val MISSING: String = "The shared photo is no longer available. Share it again."
    }
}
