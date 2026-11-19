package app.witness.feature.capture

import androidx.camera.core.UseCase
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.EntryProviderScope
import androidx.navigation3.runtime.NavKey
import app.witness.core.model.CaptureMode
import app.witness.core.model.RecordId

fun EntryProviderScope<NavKey>.captureEntries(
    newCameraViewModel: () -> CameraViewModel,
    cameraUseCases: List<UseCase>,
    onClose: () -> Unit,
    onCompose: (RecordId) -> Unit,
    onOpenItem: (RecordId) -> Unit,
    analysis: LiveBlurAnalysis? = null,
) {
    entry<CameraKey> {
        CameraRoute(viewModel { newCameraViewModel() }, cameraUseCases, onClose, onCompose, onOpenItem, analysis)
    }
}

@Composable
fun CameraRoute(
    vm: CameraViewModel,
    cameraUseCases: List<UseCase>,
    onClose: () -> Unit,
    onCompose: (RecordId) -> Unit,
    onOpenItem: (RecordId) -> Unit,
    analysis: LiveBlurAnalysis? = null,
    topAccessory: @Composable () -> Unit = {},
    bottomAccessory: @Composable () -> Unit = {},
) {
    val state by vm.state.collectAsStateWithLifecycle()
    val bound = remember(cameraUseCases, analysis) { cameraUseCases + listOfNotNull(analysis?.useCase) }
    if (analysis != null) {
        SideEffect {
            analysis.mirrored = state.lens == Lens.FRONT
            analysis.active = state.mode == CaptureMode.POST
        }
        DisposableEffect(analysis) { onDispose { analysis.active = false } }
    }
    LaunchedEffect(vm) {
        vm.effects.collect { effect ->
            when (effect) {
                is CameraEffect.OpenComposer -> onCompose(effect.recordId)
            }
        }
    }
    CameraScreen(state, vm, onClose, onOpenItem, topAccessory = topAccessory, bottomAccessory = bottomAccessory) { modifier ->
        CameraXViewfinder(state.lens, state.flash, bound, modifier)
    }
}
