package app.witness.feature.capture

import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageInfo
import androidx.camera.core.ImageProxy
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.Dims
import app.witness.core.model.Rect
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.lang.reflect.Proxy
import java.util.concurrent.Executor
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class LiveBlurAnalysisTest {
    private var clock = 0L
    private var calls = 0
    private val errors = ArrayList<Exception>()
    private var result: () -> List<BlurProposal> = { listOf(BlurProposal(Rect(10, 20, 30, 40), BlurClass.FACE)) }
    private val direct = Executor { it.run() }
    private val analysis = LiveBlurAnalysis({ calls++; result() }, direct, { clock }, { errors += it })

    private class Frame(val proxy: ImageProxy, val closed: IntArray)

    private fun frame(width: Int = 640, height: Int = 480, rotation: Int = 90): Frame {
        val closed = IntArray(1)
        val loader = javaClass.classLoader
        val info = Proxy.newProxyInstance(loader, arrayOf(ImageInfo::class.java)) { _, m, _ ->
            when (m.name) {
                "getRotationDegrees" -> rotation
                "getTimestamp" -> 0L
                else -> null
            }
        } as ImageInfo
        val proxy = Proxy.newProxyInstance(loader, arrayOf(ImageProxy::class.java)) { _, m, _ ->
            when (m.name) {
                "getWidth" -> width
                "getHeight" -> height
                "getImageInfo" -> info
                "close" -> { closed[0]++; null }
                else -> null
            }
        } as ImageProxy
        return Frame(proxy, closed)
    }

    @Test
    fun inactiveFramesAreClosedWithoutDetection() {
        val f = frame()
        analysis.analyze(f.proxy)
        assertEquals(0, calls)
        assertEquals(1, f.closed[0])
        assertSame(FrameDetections.NONE, analysis.detections.value)
    }

    @Test
    fun activeFramesPublishUprightDetectionsAndClose() {
        analysis.active = true
        val f = frame()
        analysis.analyze(f.proxy)
        assertEquals(1, calls)
        assertEquals(1, f.closed[0])
        val d = analysis.detections.value
        assertEquals(Dims(480, 640), d.dims)
        assertEquals(Rect(10, 20, 30, 40), d.proposals.single().rect)
    }

    @Test
    fun framesInsideTheIntervalAreDroppedButClosed() {
        analysis.active = true
        val frames = List(4) { frame() }
        analysis.analyze(frames[0].proxy)
        clock = LiveFrames.MIN_INTERVAL_MS - 1
        analysis.analyze(frames[1].proxy)
        clock = LiveFrames.MIN_INTERVAL_MS
        analysis.analyze(frames[2].proxy)
        analysis.analyze(frames[3].proxy)
        assertEquals(2, calls)
        assertTrue(frames.all { it.closed[0] == 1 })
    }

    @Test
    fun detectorFailureIsReportedAndClearsDetections() {
        analysis.active = true
        analysis.analyze(frame().proxy)
        result = { throw IllegalStateException("ml kit") }
        clock = 1_000
        val f = frame()
        analysis.analyze(f.proxy)
        assertEquals(1, errors.size)
        assertEquals(1, f.closed[0])
        assertTrue(analysis.detections.value.proposals.isEmpty())
    }

    @Test
    fun frontLensDetectionsAreMirrored() {
        analysis.active = true
        analysis.mirrored = true
        analysis.analyze(frame(rotation = 0).proxy)
        assertEquals(Rect(600, 20, 30, 40), analysis.detections.value.proposals.single().rect)
    }

    @Test
    fun deactivatingClearsDetections() {
        analysis.active = true
        analysis.analyze(frame().proxy)
        assertEquals(1, analysis.detections.value.proposals.size)
        analysis.active = false
        assertSame(FrameDetections.NONE, analysis.detections.value)
    }

    @Test
    fun useCaseKeepsOnlyTheLatestFrame() {
        assertEquals(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST, analysis.useCase.backpressureStrategy)
        assertSame(analysis.useCase, analysis.useCase)
    }
}
