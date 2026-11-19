package app.witness.feature.capture

import android.os.SystemClock
import android.util.Size
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import app.witness.core.model.BlurProposal
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.runBlocking
import java.util.concurrent.Executor
import java.util.concurrent.Executors

fun interface FrameBlurDetector {
    suspend fun detect(frame: ImageProxy): List<BlurProposal>
}

class LiveBlurAnalysis(
    private val detector: FrameBlurDetector,
    private val executor: Executor = Executors.newSingleThreadExecutor { r -> Thread(r, "witness-live-blur").apply { isDaemon = true } },
    private val now: () -> Long = SystemClock::elapsedRealtime,
    private val onError: (Exception) -> Unit = {},
) {
    private val throttle = FrameThrottle(LiveFrames.MIN_INTERVAL_MS)
    private val out = MutableStateFlow(FrameDetections.NONE)

    val detections: StateFlow<FrameDetections> = out.asStateFlow()

    @Volatile
    var active: Boolean = false
        set(value) {
            field = value
            if (!value) out.value = FrameDetections.NONE
        }

    @Volatile
    var mirrored: Boolean = false

    val useCase: ImageAnalysis by lazy {
        ImageAnalysis.Builder()
            .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
            .setResolutionSelector(
                ResolutionSelector.Builder()
                    .setResolutionStrategy(ResolutionStrategy(ANALYSIS_SIZE, ResolutionStrategy.FALLBACK_RULE_CLOSEST_LOWER_THEN_HIGHER))
                    .build(),
            )
            .build()
            .also { it.setAnalyzer(executor, ::analyze) }
    }

    fun analyze(frame: ImageProxy) {
        try {
            if (!active || !throttle.tryAcquire(now())) return
            val dims = LiveFrames.upright(frame.width, frame.height, frame.imageInfo.rotationDegrees)
            val mirror = mirrored
            val proposals = try {
                runBlocking { detector.detect(frame) }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                onError(e)
                emptyList()
            }
            if (active) out.value = LiveFrames.detections(dims, proposals, mirror)
        } finally {
            frame.close()
        }
    }

    companion object {
        val ANALYSIS_SIZE: Size = Size(640, 480)
    }
}
