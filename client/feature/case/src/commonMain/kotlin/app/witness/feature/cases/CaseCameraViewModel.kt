package app.witness.feature.cases

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.witness.core.domain.Clock
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.port.SettingsStore
import app.witness.core.model.Capture
import app.witness.core.model.CaptureMode
import app.witness.core.model.Case
import app.witness.core.model.LocalCaseId
import app.witness.core.model.Settings
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

@Immutable
data class CaseCameraUiState(
    val visible: Boolean = false,
    val caseId: LocalCaseId? = null,
    val pillLabel: String = NO_CASE,
    val coach: CoachUi? = null,
) {
    companion object {
        const val NO_CASE: String = "Choose a case"
    }
}

class CaseCameraViewModel(
    settings: SettingsStore,
    cases: CaseStore,
    captures: CaptureStore,
    private val feedback: AiFeedback,
    private val clock: Clock,
) : ViewModel() {
    private val skips = MutableStateFlow<Map<LocalCaseId, Int>>(emptyMap())

    val state: StateFlow<CaseCameraUiState> = combine(settings.settings, cases.observeAll(), captures.observeAll(), skips, ::build)
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(CasesViewModel.STOP_TIMEOUT_MS), CaseCameraUiState())

    fun skip() {
        val id = state.value.caseId ?: return
        if (state.value.coach == null) return
        skips.update { it + (id to (it[id] ?: 0) + 1) }
    }

    fun flagCoach() {
        val current = state.value
        val id = current.caseId ?: return
        val coach = current.coach ?: return
        skip()
        viewModelScope.launch {
            try {
                feedback.flag(AiFlag(id, COACH_TARGET_PREFIX + coach.step, FlagReason.WRONG, null, clock.nowEpochMillis()))
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
            }
        }
    }

    companion object {
        const val COACH_TARGET_PREFIX: String = "coach_step_"

        fun build(settings: Settings, cases: List<Case>, captures: List<Capture>, skips: Map<LocalCaseId, Int>): CaseCameraUiState {
            val visible = settings.captureMode == CaptureMode.CASE
            val active = settings.activeCase?.let { id -> cases.firstOrNull { it.id == id } }
                ?: return CaseCameraUiState(visible = visible)
            val captured = captures.count { it.caseId == active.id && it.isVerified }
            return CaseCameraUiState(
                visible = visible,
                caseId = active.id,
                pillLabel = "Case: " + active.title,
                coach = EvidenceCoach.coach(active.templateId, captured, skips[active.id] ?: 0),
            )
        }
    }
}
