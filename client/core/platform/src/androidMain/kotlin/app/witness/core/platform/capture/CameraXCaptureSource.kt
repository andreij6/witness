package app.witness.core.platform.capture

import android.content.ContentResolver
import android.os.SystemClock
import android.provider.Settings
import android.util.Size
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.core.ImageProxy
import androidx.camera.core.resolutionselector.AspectRatioStrategy
import androidx.camera.core.resolutionselector.ResolutionFilter
import androidx.camera.core.resolutionselector.ResolutionSelector
import app.witness.core.domain.port.CaptureFrame
import app.witness.core.domain.port.CaptureSource
import app.witness.core.model.ByteString
import kotlinx.coroutines.suspendCancellableCoroutine
import java.util.concurrent.Executor
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

class CameraXCaptureSource(
    private val executor: Executor,
    private val bootCount: () -> Int,
    val imageCapture: ImageCapture = buildImageCapture(),
) : CaptureSource {
    override suspend fun capture(): CaptureFrame = suspendCancellableCoroutine { cont ->
        imageCapture.takePicture(
            executor,
            object : ImageCapture.OnImageCapturedCallback() {
                override fun onCaptureSuccess(image: ImageProxy) {
                    val frame = try {
                        image.toFrame(SystemClock.elapsedRealtimeNanos(), bootCount())
                    } catch (e: Exception) {
                        cont.resumeWithException(e)
                        return
                    } finally {
                        image.close()
                    }
                    cont.resume(frame)
                }

                override fun onError(exception: ImageCaptureException) = cont.resumeWithException(exception)
            },
        )
    }

    companion object {
        const val MAX_PIXELS = 4032 * 3024
        const val JPEG_QUALITY = 100

        fun buildImageCapture(): ImageCapture = ImageCapture.Builder()
            .setCaptureMode(ImageCapture.CAPTURE_MODE_MINIMIZE_LATENCY)
            .setJpegQuality(JPEG_QUALITY)
            .setResolutionSelector(resolutionSelector())
            .build()

        fun resolutionSelector(maxPixels: Int = MAX_PIXELS): ResolutionSelector = ResolutionSelector.Builder()
            .setAspectRatioStrategy(AspectRatioStrategy.RATIO_4_3_FALLBACK_AUTO_STRATEGY)
            .setResolutionFilter(MaxPixelsFilter(maxPixels))
            .build()

        fun systemBootCount(resolver: ContentResolver): () -> Int = {
            Settings.Global.getInt(resolver, Settings.Global.BOOT_COUNT, 0)
        }
    }
}

class MaxPixelsFilter(private val maxPixels: Int) : ResolutionFilter {
    override fun filter(supportedSizes: List<Size>, rotationDegrees: Int): List<Size> =
        PixelBudget.select(supportedSizes, maxPixels, Size::getWidth, Size::getHeight)
}

internal fun ImageProxy.toFrame(elapsedNs: Long, bootCount: Int): CaptureFrame {
    val buffer = planes[0].buffer
    buffer.rewind()
    val jpeg = ByteArray(buffer.remaining())
    buffer.get(jpeg)
    return CaptureFrame(ByteString.wrap(jpeg), imageInfo.rotationDegrees, elapsedNs, bootCount)
}
