package app.witness.feature.capture

import android.content.Context
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageCapture
import androidx.camera.core.Preview
import androidx.camera.core.UseCase
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.suspendCancellableCoroutine

private suspend fun cameraProvider(context: Context): ProcessCameraProvider = suspendCancellableCoroutine { cont ->
    val future = ProcessCameraProvider.getInstance(context)
    future.addListener({ cont.resumeWith(runCatching { future.get() }) }, ContextCompat.getMainExecutor(context))
}

fun Lens.selector(): CameraSelector = if (this == Lens.FRONT) CameraSelector.DEFAULT_FRONT_CAMERA else CameraSelector.DEFAULT_BACK_CAMERA

fun Flash.imageCaptureMode(): Int = if (this == Flash.ON) ImageCapture.FLASH_MODE_ON else ImageCapture.FLASH_MODE_OFF

@Composable
fun CameraXViewfinder(lens: Lens, flash: Flash, useCases: List<UseCase>, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val owner = LocalLifecycleOwner.current
    val view = remember(context) {
        PreviewView(context).apply {
            scaleType = PreviewView.ScaleType.FILL_CENTER
            implementationMode = PreviewView.ImplementationMode.PERFORMANCE
        }
    }
    val preview = remember { Preview.Builder().build() }
    val bound = remember(useCases) { (useCases + preview).toTypedArray() }
    SideEffect {
        val mode = flash.imageCaptureMode()
        for (u in useCases) if (u is ImageCapture) u.flashMode = mode
    }
    LaunchedEffect(owner, lens, bound) {
        val provider = cameraProvider(context)
        preview.surfaceProvider = view.surfaceProvider
        provider.unbind(*bound)
        provider.bindToLifecycle(owner, lens.selector(), *bound)
    }
    DisposableEffect(owner, bound) {
        onDispose { ProcessCameraProvider.getInstance(context).takeIf { it.isDone }?.get()?.unbind(*bound) }
    }
    AndroidView({ view }, modifier)
}
