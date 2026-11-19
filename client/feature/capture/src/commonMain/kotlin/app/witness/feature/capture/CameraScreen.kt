package app.witness.feature.capture

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import app.witness.core.designsystem.GlassIconButton
import app.witness.core.designsystem.GlassPill
import app.witness.core.designsystem.GlassSegmentedControl
import app.witness.core.designsystem.SealBadge
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessCameraTheme
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.WitnessPalette
import app.witness.core.model.CaptureMode
import app.witness.core.model.RecordId
import kotlinx.coroutines.delay

@Composable
fun CameraScreen(
    state: CameraUiState,
    actions: CameraActions,
    onClose: () -> Unit,
    onOpenItem: (RecordId) -> Unit,
    modifier: Modifier = Modifier,
    topAccessory: @Composable () -> Unit = {},
    bottomAccessory: @Composable () -> Unit = {},
    viewfinder: @Composable (Modifier) -> Unit = {},
) {
    WitnessCameraTheme {
        Column(modifier.fillMaxSize().background(WitnessColors.Camera)) {
            Box(Modifier.weight(1f).fillMaxWidth()) {
                viewfinder(Modifier.fillMaxSize())
                DetectionOverlay(state.detections, state.frameAspect, Modifier.fillMaxSize())
                TopScrim()
                FrameGuides(Modifier.fillMaxSize())
                TopControls(state, actions, onClose, topAccessory)
                BottomOverlay(state, actions, onOpenItem, bottomAccessory, Modifier.align(Alignment.BottomCenter))
            }
            ShutterPanel(state, actions, onOpenItem)
        }
    }
}

@Composable
private fun TopControls(state: CameraUiState, actions: CameraActions, onClose: () -> Unit, accessory: @Composable () -> Unit) {
    Column(Modifier.fillMaxWidth().statusBarsPadding().padding(start = 16.dp, end = 16.dp, top = 12.dp)) {
        Box(Modifier.fillMaxWidth().height(40.dp)) {
            GlassIconButton(WitnessIcon.Close, "Close camera", onClose, Modifier.align(Alignment.CenterStart))
            GlassSegmentedControl(
                CameraCopy.MODES,
                state.modeIndex,
                { actions.setMode(CaptureMode.entries[it]) },
                Modifier.align(Alignment.Center),
            )
            GlassIconButton(WitnessIcon.FlashOff, state.flash.label, actions::cycleFlash, Modifier.align(Alignment.CenterEnd).alpha(if (state.flash == Flash.ON) 0.6f else 1f))
        }
        Spacer(Modifier.height(14.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            GlassPill(state.seal.text, WitnessIcon.Lock, iconTint = if (state.seal.ready) WitnessColors.ProvenDark else CameraTokens.Amber)
            val loc = state.location
            GlassPill(loc.text, loc.icon, iconTint = loc.tone?.let(WitnessPalette.Camera::tone) ?: WitnessColors.CameraInk2)
        }
        if (!state.online) {
            Spacer(Modifier.height(8.dp))
            OfflinePill()
        }
        Spacer(Modifier.height(8.dp))
        accessory()
    }
}

@Composable
private fun BottomOverlay(state: CameraUiState, actions: CameraActions, onOpenItem: (RecordId) -> Unit, accessory: @Composable () -> Unit, modifier: Modifier) {
    Column(modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, bottom = 26.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        val toast = state.toast
        when {
            state.error != null -> ErrorCard(state.error, actions::dismissError)
            toast != null -> {
                LaunchedEffect(toast.recordId) {
                    delay(TOAST_MS)
                    actions.dismissToast()
                }
                SealedToastCard(toast, { onOpenItem(toast.recordId) })
            }
            state.hint != null -> HintCard(state.hint)
        }
        accessory()
    }
}

@Composable
private fun ShutterPanel(state: CameraUiState, actions: CameraActions, onOpenItem: (RecordId) -> Unit) {
    Column(Modifier.fillMaxWidth().navigationBarsPadding().height(CameraTokens.PanelHeight), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(Modifier.fillMaxWidth().padding(top = 34.dp, start = 32.dp, end = 34.dp).height(76.dp)) {
            LastThumb(state.last, onOpenItem, Modifier.align(Alignment.CenterStart))
            Shutter(state.sealing, actions::shutter, Modifier.align(Alignment.Center))
            WitnessIconButton(
                WitnessIcon.Cameraswitch,
                "Switch camera",
                actions::flipLens,
                Modifier.align(Alignment.CenterEnd),
                tint = WitnessColors.White,
                container = WitnessColors.CameraSurface,
                size = 52.dp,
                iconSize = 26.dp,
            )
        }
        Spacer(Modifier.height(26.dp))
        Text(state.meta, Modifier.fillMaxWidth(), style = Witness.type.proofSmall, color = WitnessColors.CameraInk2, textAlign = TextAlign.Center, maxLines = 1)
        Spacer(Modifier.height(4.dp))
        Text(CameraCopy.FOOTNOTE, Modifier.fillMaxWidth(), style = Witness.type.meta, color = WitnessColors.CameraMuted, textAlign = TextAlign.Center)
    }
}

private val ThumbShape = RoundedCornerShape(14.dp)

@Composable
private fun LastThumb(last: LastCapture?, onOpenItem: (RecordId) -> Unit, modifier: Modifier) {
    Box(modifier.size(56.dp)) {
        if (last == null) {
            Box(Modifier.size(56.dp).clip(ThumbShape).background(WitnessColors.CameraSurface))
            return@Box
        }
        PhotoImage(
            last.photo,
            "Last capture",
            Modifier.size(56.dp).clip(ThumbShape).clickable(role = Role.Button) { onOpenItem(last.recordId) },
        )
        SealBadge(last.state, Modifier.offset(41.dp, (-7).dp), radius = 9.dp, ring = WitnessColors.Camera)
    }
}

@Composable
private fun Shutter(sealing: Boolean, onClick: () -> Unit, modifier: Modifier) {
    Box(
        modifier
            .size(76.dp)
            .border(4.dp, WitnessColors.White, CircleShape)
            .clip(CircleShape)
            .clickable(enabled = !sealing, role = Role.Button, onClick = onClick)
            .semantics { contentDescription = if (sealing) "Sealing photo" else "Take sealed photo" },
        contentAlignment = Alignment.Center,
    ) {
        Box(Modifier.size(60.dp).alpha(if (sealing) 0.5f else 1f).clip(CircleShape).background(WitnessColors.White))
    }
}

const val TOAST_MS: Long = 6_000
