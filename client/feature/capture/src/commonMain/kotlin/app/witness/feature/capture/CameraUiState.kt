package app.witness.feature.capture

import androidx.compose.runtime.Immutable
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessState
import app.witness.core.model.BlurProposal
import app.witness.core.model.CaptureMode
import app.witness.core.model.Dims
import app.witness.core.model.RecordId
import app.witness.core.ui.NormRect

data class FrameDetections(val dims: Dims, val proposals: List<BlurProposal>) {
    companion object {
        val NONE: FrameDetections = FrameDetections(Dims(1, 1), emptyList())
    }
}

enum class Flash(val label: String) {
    OFF("Flash off"),
    ON("Flash on");

    fun next(): Flash = if (this == OFF) ON else OFF
}

enum class Lens {
    BACK,
    FRONT;

    fun flipped(): Lens = if (this == BACK) FRONT else BACK
}

@Immutable
data class Detection(val label: String, val box: NormRect)

@Immutable
data class CameraHint(val title: String, val subtitle: String)

@Immutable
data class SealPill(val text: String, val ready: Boolean)

@Immutable
data class LocationPillUi(val text: String, val icon: WitnessIcon, val tone: Tone?)

@Immutable
data class LastCapture(val recordId: RecordId, val state: WitnessState, val photo: Photo?)

@Immutable
data class SealedToast(val recordId: RecordId, val title: String, val status: String, val tone: Tone, val icon: WitnessIcon, val photo: Photo?)

@Immutable
data class CameraUiState(
    val mode: CaptureMode = CaptureMode.POST,
    val seal: SealPill = CameraCopy.sealPill(null),
    val location: LocationPillUi = CameraCopy.locationPill(null),
    val online: Boolean = true,
    val frameAspect: Float? = null,
    val detections: List<Detection> = emptyList(),
    val hint: CameraHint? = null,
    val last: LastCapture? = null,
    val toast: SealedToast? = null,
    val sealing: Boolean = false,
    val flash: Flash = Flash.OFF,
    val lens: Lens = Lens.BACK,
    val meta: String = "",
    val error: String? = null,
) {
    val modeIndex: Int get() = mode.ordinal
}

sealed interface CameraEffect {
    data class OpenComposer(val recordId: RecordId) : CameraEffect
}

interface CameraActions {
    fun setMode(mode: CaptureMode)

    fun cycleFlash()

    fun flipLens()

    fun shutter()

    fun dismissToast()

    fun dismissError()

    object None : CameraActions {
        override fun setMode(mode: CaptureMode) = Unit

        override fun cycleFlash() = Unit

        override fun flipLens() = Unit

        override fun shutter() = Unit

        override fun dismissToast() = Unit

        override fun dismissError() = Unit
    }
}
