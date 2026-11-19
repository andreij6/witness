package app.witness.feature.home

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.WitnessTheme
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.usecase.Enrollment
import app.witness.core.domain.usecase.EnrollmentStatus
import app.witness.core.model.SecurityLevel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

enum class StepStatus { Done, Busy, Todo, Waiting, Failed }

@Immutable
data class SetupStep(val title: String, val detail: String, val status: StepStatus)

sealed interface SetupPhase {
    data object CreatingKey : SetupPhase

    data object Verifying : SetupPhase

    data object Ready : SetupPhase

    data object Offline : SetupPhase

    data class Unsupported(val reason: String) : SetupPhase

    data object Failed : SetupPhase
}

@Immutable
data class DeviceSetupUiState(val phase: SetupPhase, val steps: List<SetupStep>) {
    val canOpenCamera: Boolean get() = phase == SetupPhase.Ready || phase == SetupPhase.Offline
    val canRetry: Boolean get() = phase == SetupPhase.Failed
    val isWorking: Boolean get() = phase == SetupPhase.CreatingKey || phase == SetupPhase.Verifying
    val banner: String get() = if (phase == SetupPhase.Offline) DeviceSetupCopy.UNREACHABLE else DeviceSetupCopy.OFFLINE

    val footnote: String?
        get() = when (phase) {
            SetupPhase.CreatingKey, SetupPhase.Verifying -> DeviceSetupCopy.USUALLY_FAST
            SetupPhase.Offline -> DeviceSetupCopy.FINISHES_LATER
            SetupPhase.Failed -> DeviceSetupCopy.FAILED
            SetupPhase.Ready, is SetupPhase.Unsupported -> null
        }

    companion object {
        fun of(phase: SetupPhase, security: SecurityLevel?): DeviceSetupUiState = DeviceSetupUiState(phase, steps(phase, security))

        fun steps(phase: SetupPhase, security: SecurityLevel?): List<SetupStep> {
            val keyTitle = security?.let { "${DeviceSetupCopy.KEY} · ${it.label()}" } ?: DeviceSetupCopy.KEY
            val statuses = when (phase) {
                SetupPhase.CreatingKey -> listOf(StepStatus.Busy, StepStatus.Todo, StepStatus.Todo, StepStatus.Todo)
                SetupPhase.Verifying -> listOf(StepStatus.Done, StepStatus.Busy, StepStatus.Todo, StepStatus.Todo)
                SetupPhase.Ready -> listOf(StepStatus.Done, StepStatus.Done, StepStatus.Done, StepStatus.Done)
                SetupPhase.Offline -> listOf(StepStatus.Done, StepStatus.Waiting, StepStatus.Waiting, StepStatus.Done)
                SetupPhase.Failed -> listOf(if (security != null) StepStatus.Done else StepStatus.Failed, StepStatus.Failed, StepStatus.Todo, StepStatus.Todo)
                is SetupPhase.Unsupported -> listOf(StepStatus.Done, StepStatus.Failed, StepStatus.Todo, StepStatus.Todo)
            }
            return listOf(
                SetupStep(keyTitle, DeviceSetupCopy.KEY_DETAIL, statuses[0]),
                SetupStep(DeviceSetupCopy.ATTESTATION, if (security == SecurityLevel.DEV) DeviceSetupCopy.ATTESTATION_DEV else DeviceSetupCopy.ATTESTATION_DETAIL, statuses[1]),
                SetupStep(DeviceSetupCopy.CERTIFICATE, certificateDetail(statuses[2]), statuses[2]),
                SetupStep(DeviceSetupCopy.READY, if (statuses[3] == StepStatus.Done) DeviceSetupCopy.READY_NOW else DeviceSetupCopy.READY_DETAIL, statuses[3]),
            )
        }

        private fun certificateDetail(status: StepStatus): String = when (status) {
            StepStatus.Done -> DeviceSetupCopy.CERTIFICATE_DONE
            StepStatus.Waiting -> DeviceSetupCopy.CERTIFICATE_LATER
            else -> DeviceSetupCopy.CERTIFICATE_DETAIL
        }

        fun phaseOf(status: EnrollmentStatus): SetupPhase = when (status) {
            is EnrollmentStatus.Enrolled -> SetupPhase.Ready
            is EnrollmentStatus.Pending, EnrollmentStatus.Offline -> SetupPhase.Offline
            is EnrollmentStatus.Unsupported -> SetupPhase.Unsupported(status.reason)
        }

        fun securityOf(status: EnrollmentStatus): SecurityLevel? = when (status) {
            is EnrollmentStatus.Enrolled -> status.key.security
            is EnrollmentStatus.Pending -> status.key.security
            else -> null
        }
    }
}

class DeviceSetupViewModel(private val vault: KeyVault, private val enrollment: Enrollment) : ViewModel() {
    private val _state = MutableStateFlow(DeviceSetupUiState.of(SetupPhase.CreatingKey, null))
    val state: StateFlow<DeviceSetupUiState> = _state.asStateFlow()
    private var job: Job? = null

    init {
        start()
    }

    fun retry() {
        if (_state.value.canRetry) start()
    }

    private fun start() {
        if (job?.isActive == true) return
        job = viewModelScope.launch {
            var security: SecurityLevel? = null
            try {
                val existing = vault.activeKey()?.takeUnless { it.retired }
                security = existing?.security
                _state.value = DeviceSetupUiState.of(if (existing != null) SetupPhase.Verifying else SetupPhase.CreatingKey, security)
                val result = enrollment()
                _state.value = DeviceSetupUiState.of(DeviceSetupUiState.phaseOf(result), DeviceSetupUiState.securityOf(result) ?: security)
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                _state.value = DeviceSetupUiState.of(SetupPhase.Failed, security)
            }
        }
    }
}

object DeviceSetupCopy {
    const val TITLE = "Setting up your phone"
    const val LEAD = "Witness creates a signing key inside your phone's security chip. It can never be copied out, not even by us."
    const val KEY = "Hardware key created"
    const val KEY_DETAIL = "P-256 key · can't be exported"
    const val ATTESTATION = "Verified by Google key attestation"
    const val ATTESTATION_DETAIL = "Locked bootloader · verified OS · real app"
    const val ATTESTATION_DEV = "Development device · proofs show a DEV badge"
    const val CERTIFICATE = "Witness certificate issued"
    const val CERTIFICATE_DETAIL = "Waiting for the registry…"
    const val CERTIFICATE_DONE = "Issued by the Witness registry"
    const val CERTIFICATE_LATER = "Finishes when you're back online"
    const val READY = "Ready to capture"
    const val READY_DETAIL = "Your first photo can be sealed right after"
    const val READY_NOW = "Your photos are sealed from the first shot"
    const val OFFLINE = "Offline? You can capture right away — setup finishes when you're back online."
    const val UNREACHABLE = "Can't reach the Witness network right now. You can capture — setup finishes once it answers."
    const val PRIVACY = "Only your key's attestation is sent to be verified. Photos, accounts and your phone number stay out of it."
    const val OPEN_CAMERA = "Open camera"
    const val TRY_AGAIN = "Try again"
    const val USUALLY_FAST = "Usually takes a few seconds"
    const val FINISHES_LATER = "Sealed photos anchor automatically once setup finishes"
    const val FAILED = "Setup didn't finish. Check your connection and try again."
}

@Composable
fun DeviceSetupScreen(state: DeviceSetupUiState, onOpenCamera: () -> Unit, onRetry: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    OnboardingLayout(
        modifier,
        footer = {
            when {
                state.canRetry -> WitnessButton(DeviceSetupCopy.TRY_AGAIN, onRetry, Modifier.fillMaxWidth(), height = LargeButtonHeight, textStyle = Witness.type.button.merge(LargeButtonText))
                state.canOpenCamera -> WitnessButton(
                    DeviceSetupCopy.OPEN_CAMERA,
                    onOpenCamera,
                    Modifier.fillMaxWidth(),
                    height = LargeButtonHeight,
                    textStyle = Witness.type.button.merge(LargeButtonText),
                )
                else -> PendingButton(DeviceSetupCopy.OPEN_CAMERA)
            }
            state.footnote?.let {
                VSpace(18.dp)
                Footnote(it)
            }
        },
    ) {
        VSpace(68.dp)
        ScreenTitle(DeviceSetupCopy.TITLE)
        VSpace(10.dp)
        Lead(DeviceSetupCopy.LEAD)
        VSpace(26.dp)
        WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(top = 16.dp, bottom = 8.dp)) {
            state.steps.forEachIndexed { i, step -> StepRow(step, state.steps.getOrNull(i + 1)) }
        }
        VSpace(16.dp)
        Row(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).background(palette.warn.copy(alpha = 0.1f)).padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            WIcon(WitnessIcon.CloudOff, palette.warn, size = 22.dp)
            HSpace(14.dp)
            Text(state.banner, style = Witness.type.bodyCompact, color = palette.warnText)
        }
        VSpace(16.dp)
        WitnessCard(Modifier.fillMaxWidth(), shape = WitnessShapes.Row, contentPadding = PaddingValues(horizontal = 16.dp, vertical = 14.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                WIcon(WitnessIcon.PrivacyTip, palette.ink2, size = 22.dp)
                HSpace(14.dp)
                Text(DeviceSetupCopy.PRIVACY, style = Witness.type.subtitle, color = palette.ink2)
            }
        }
        VSpace(16.dp)
    }
}

internal fun connectorDone(step: SetupStep, next: SetupStep?): Boolean = step.status == StepStatus.Done && next != null && next.status != StepStatus.Todo

@Composable
private fun StepRow(step: SetupStep, next: SetupStep?) {
    val palette = Witness.colors
    val line = if (connectorDone(step, next)) palette.ok else palette.line
    val todo = step.status == StepStatus.Todo
    Row(
        Modifier
            .fillMaxWidth()
            .height(72.dp)
            .drawBehind {
                if (next != null) {
                    val x = 36.dp.toPx()
                    drawLine(line, Offset(x, 43.dp.toPx()), Offset(x, 81.dp.toPx()), 2.dp.toPx())
                }
            }
            .semantics(mergeDescendants = true) { stateDescription = step.status.name },
    ) {
        Box(Modifier.padding(start = 22.dp, top = 12.dp).size(28.dp), contentAlignment = Alignment.Center) { StepIndicator(step.status) }
        HSpace(14.dp)
        Column(Modifier.weight(1f).padding(top = 10.dp, end = 16.dp)) {
            Text(step.title, style = Witness.type.rowTitle, color = if (todo) palette.muted else palette.ink, maxLines = 1)
            VSpace(4.dp)
            Text(step.detail, style = Witness.type.subtitle, color = if (todo) palette.muted else palette.ink2, maxLines = 1)
        }
    }
}

@Composable
private fun StepIndicator(status: StepStatus) {
    val palette = Witness.colors
    when (status) {
        StepStatus.Done -> Box(Modifier.size(26.dp).clip(CircleShape).background(palette.ok), contentAlignment = Alignment.Center) {
            WIcon(WitnessIcon.Check, palette.onPrimary, size = 16.dp)
        }
        StepStatus.Failed -> Box(Modifier.size(26.dp).clip(CircleShape).background(palette.crit), contentAlignment = Alignment.Center) {
            WIcon(WitnessIcon.Close, palette.onPrimary, size = 16.dp)
        }
        StepStatus.Waiting -> Box(Modifier.size(24.dp).border(1.6.dp, palette.warn, CircleShape), contentAlignment = Alignment.Center) {
            WIcon(WitnessIcon.HourglassTop, palette.warn, size = 14.dp)
        }
        StepStatus.Todo -> Box(Modifier.size(24.dp).border(1.6.dp, palette.muted.copy(alpha = 0.6f), CircleShape))
        StepStatus.Busy -> Spinner()
    }
}

@Composable
private fun Spinner() {
    val primary = Witness.colors.primary
    val angle by rememberInfiniteTransition().animateFloat(0f, 360f, infiniteRepeatable(tween(900, easing = LinearEasing), RepeatMode.Restart))
    Box(
        Modifier.size(24.dp).rotate(angle).drawBehind {
            val w = 3.dp.toPx()
            val stroke = Stroke(w, cap = StrokeCap.Round)
            val inset = Offset(w / 2, w / 2)
            val arc = Size(size.width - w, size.height - w)
            drawArc(primary.copy(alpha = 0.18f), 0f, 360f, false, inset, arc, style = Stroke(w))
            drawArc(primary, -90f, 240f, false, inset, arc, style = stroke)
        },
    )
}

@Composable
private fun PendingButton(text: String) {
    val palette = Witness.colors
    Box(Modifier.fillMaxWidth().height(LargeButtonHeight).clip(WitnessShapes.Pill).background(palette.surface3), contentAlignment = Alignment.Center) {
        Text(text, style = Witness.type.button.merge(LargeButtonText), color = palette.muted)
    }
}

@Preview
@Composable
private fun DeviceSetupBusyPreview() = WitnessTheme {
    DeviceSetupScreen(DeviceSetupUiState(SetupPhase.Verifying, DeviceSetupUiState.steps(SetupPhase.Verifying, SecurityLevel.STRONG_BOX)), {}, {})
}

@Preview
@Composable
private fun DeviceSetupOfflinePreview() = WitnessTheme { DeviceSetupScreen(DeviceSetupUiState.of(SetupPhase.Offline, SecurityLevel.TEE), {}, {}) }

@Composable
fun DeviceSetupRoute(viewModel: DeviceSetupViewModel, navigator: HomeNavigator) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LaunchedEffect(state.phase) { if (state.phase is SetupPhase.Unsupported) navigator.resetTo(HomeKey.Unsupported) }
    DeviceSetupScreen(state, onOpenCamera = { navigator.finishOnboarding(openCamera = true) }, onRetry = viewModel::retry)
}
