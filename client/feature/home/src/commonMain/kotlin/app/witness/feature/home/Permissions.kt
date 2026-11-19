package app.witness.feature.home

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import app.witness.core.designsystem.Hairline
import app.witness.core.designsystem.IconDisc
import app.witness.core.designsystem.Overline
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCard
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.WitnessTheme
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.flow.updateAndGet

enum class Grant { Unknown, Granted, Denied, Blocked }

enum class PermissionsAction { Request, OpenSettings, Proceed }

@Immutable
data class PermissionProbe(val granted: Boolean, val rationale: Boolean)

@Immutable
data class PermissionsUiState(val camera: Grant = Grant.Unknown, val location: Grant = Grant.Unknown) {
    val action: PermissionsAction
        get() = when (camera) {
            Grant.Granted -> PermissionsAction.Proceed
            Grant.Blocked -> PermissionsAction.OpenSettings
            Grant.Unknown, Grant.Denied -> PermissionsAction.Request
        }

    val buttonLabel: String get() = if (action == PermissionsAction.OpenSettings) PermissionsCopy.OPEN_SETTINGS else PermissionsCopy.CONTINUE

    val note: String
        get() = when (camera) {
            Grant.Denied -> PermissionsCopy.CAMERA_NEEDED
            Grant.Blocked -> PermissionsCopy.CAMERA_BLOCKED
            Grant.Unknown, Grant.Granted -> PermissionsCopy.CHANGE_LATER
        }

    val noteTone: Tone get() = if (camera == Grant.Denied || camera == Grant.Blocked) Tone.Warn else Tone.Muted
}

class PermissionsViewModel : ViewModel() {
    private val _state = MutableStateFlow(PermissionsUiState())
    val state: StateFlow<PermissionsUiState> = _state.asStateFlow()

    fun sync(camera: PermissionProbe, location: PermissionProbe) = _state.update { s ->
        PermissionsUiState(next(s.camera, camera, asked = false), next(s.location, location, asked = false))
    }

    fun onResult(camera: PermissionProbe, location: PermissionProbe): Boolean {
        val s = _state.updateAndGet { PermissionsUiState(next(it.camera, camera, asked = true), next(it.location, location, asked = true)) }
        return s.camera == Grant.Granted
    }

    companion object {
        fun next(previous: Grant, probe: PermissionProbe, asked: Boolean): Grant = when {
            probe.granted -> Grant.Granted
            probe.rationale -> Grant.Denied
            asked || previous == Grant.Denied || previous == Grant.Blocked -> Grant.Blocked
            else -> Grant.Unknown
        }
    }
}

object PermissionsCopy {
    const val TITLE = "Two permissions"
    const val LEAD = "Witness asks only for what makes a photo provable, and only while the camera is open."
    const val CAMERA = "Camera"
    const val CAMERA_DETAIL = "Required to take sealed photos"
    const val LOCATION = "Location while using"
    const val LOCATION_DETAIL = "Optional — makes location provable; never in the background"
    const val NEVER = "Witness never asks for"
    val never = listOf("Access to your photo library", "Location in the background", "Contacts, microphone or your accounts")
    const val GALLERY = "Gallery imports use the system picker — no access to your library"
    const val CONTINUE = "Continue"
    const val OPEN_SETTINGS = "Open settings"
    const val CHANGE_LATER = "You can change these anytime in Android settings"
    const val CAMERA_NEEDED = "Witness needs the camera to take sealed photos"
    const val CAMERA_BLOCKED = "Camera is off for Witness. Turn it on in Android settings."
    const val GRANTED = "Allowed"
}

@Composable
fun PermissionsScreen(state: PermissionsUiState, onBack: () -> Unit, onContinue: () -> Unit, modifier: Modifier = Modifier) {
    val palette = Witness.colors
    OnboardingLayout(
        modifier,
        footer = {
            PageDots(3, 1)
            VSpace(20.dp)
            WitnessButton(state.buttonLabel, onContinue, Modifier.fillMaxWidth(), height = LargeButtonHeight, textStyle = Witness.type.button.merge(LargeButtonText))
            VSpace(18.dp)
            Footnote(state.note, color = palette.toneText(state.noteTone))
        },
    ) {
        VSpace(6.dp)
        WitnessIconButton(WitnessIcon.ArrowBack, "Back", onBack, Modifier.offset(x = (-8).dp))
        VSpace(20.dp)
        ScreenTitle(PermissionsCopy.TITLE)
        VSpace(10.dp)
        Lead(PermissionsCopy.LEAD)
        VSpace(24.dp)
        WitnessCard(Modifier.fillMaxWidth(), contentPadding = PaddingValues(0.dp)) {
            PermissionRow(WitnessIcon.PhotoCamera, PermissionsCopy.CAMERA, PermissionsCopy.CAMERA_DETAIL, state.camera)
            Hairline(Modifier.padding(horizontal = 16.dp))
            PermissionRow(WitnessIcon.NearMe, PermissionsCopy.LOCATION, PermissionsCopy.LOCATION_DETAIL, state.location)
        }
        VSpace(16.dp)
        WitnessCard(Modifier.fillMaxWidth()) {
            Overline(PermissionsCopy.NEVER)
            PermissionsCopy.never.forEach { line ->
                VSpace(10.dp)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    WIcon(WitnessIcon.Block, palette.muted, size = 18.dp)
                    HSpace(10.dp)
                    Text(line, style = Witness.type.bodySmall, color = palette.ink)
                }
            }
        }
        VSpace(16.dp)
        Row(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).background(palette.surface2).padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            WIcon(WitnessIcon.PhotoLibrary, palette.ink2, size = 22.dp)
            HSpace(14.dp)
            Text(PermissionsCopy.GALLERY, style = Witness.type.subtitle, color = palette.ink2)
        }
        VSpace(16.dp)
    }
}

@Composable
private fun PermissionRow(icon: WitnessIcon, title: String, detail: String, grant: Grant) {
    val palette = Witness.colors
    Row(
        Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 24.dp, bottom = 22.dp).semantics(mergeDescendants = true) {
            if (grant == Grant.Granted) stateDescription = PermissionsCopy.GRANTED
        },
        verticalAlignment = Alignment.Top,
    ) {
        IconDisc(icon, palette.ink, size = 44.dp, iconSize = 24.dp, fill = palette.surface2)
        HSpace(16.dp)
        Column(Modifier.weight(1f).padding(top = 2.dp)) {
            Text(title, style = Witness.type.cardTitle, color = palette.ink)
            VSpace(4.dp)
            Text(detail, style = Witness.type.bodyCompact, color = palette.ink2)
        }
        if (grant == Grant.Granted) {
            HSpace(8.dp)
            WIcon(WitnessIcon.CheckCircle, palette.ok, size = 22.dp)
        }
    }
}

@Preview
@Composable
private fun PermissionsPreview() = WitnessTheme { PermissionsScreen(PermissionsUiState(), {}, {}) }

@Preview
@Composable
private fun PermissionsBlockedPreview() = WitnessTheme { PermissionsScreen(PermissionsUiState(Grant.Blocked, Grant.Granted), {}, {}) }
