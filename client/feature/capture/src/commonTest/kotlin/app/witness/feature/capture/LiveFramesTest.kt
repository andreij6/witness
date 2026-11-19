package app.witness.feature.capture

import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.Dims
import app.witness.core.model.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class LiveFramesTest {
    private fun p(x: Int, y: Int, w: Int, h: Int, c: BlurClass = BlurClass.FACE) = BlurProposal(Rect(x, y, w, h), c)

    @Test
    fun throttleAllowsAtMostOneFramePerInterval() {
        val t = FrameThrottle(200)
        assertTrue(t.tryAcquire(0))
        assertFalse(t.tryAcquire(1))
        assertFalse(t.tryAcquire(199))
        assertTrue(t.tryAcquire(200))
        assertFalse(t.tryAcquire(399))
        assertTrue(t.tryAcquire(1_000))
    }

    @Test
    fun throttleCapsAFastStreamAtFiveFps() {
        val t = FrameThrottle(LiveFrames.MIN_INTERVAL_MS)
        val accepted = (0 until 60).count { t.tryAcquire(it * 1_000L / 60) }
        assertEquals(LiveFrames.MAX_FPS, accepted)
    }

    @Test
    fun throttleAcceptsTheFirstFrameAtAnyClockValue() {
        assertTrue(FrameThrottle(200).tryAcquire(Long.MIN_VALUE))
        assertTrue(FrameThrottle(200).tryAcquire(-5))
    }

    @Test
    fun throttleResetLetsTheNextFrameThrough() {
        val t = FrameThrottle(200)
        assertTrue(t.tryAcquire(10))
        t.reset()
        assertTrue(t.tryAcquire(11))
    }

    @Test
    fun zeroIntervalNeverThrottlesAndNegativeIsRejected() {
        val t = FrameThrottle(0)
        assertTrue(t.tryAcquire(5))
        assertTrue(t.tryAcquire(5))
        assertFailsWith<IllegalArgumentException> { FrameThrottle(-1) }
    }

    @Test
    fun uprightSwapsForQuarterTurns() {
        assertEquals(Dims(480, 640), LiveFrames.upright(640, 480, 90))
        assertEquals(Dims(480, 640), LiveFrames.upright(640, 480, 270))
        assertEquals(Dims(480, 640), LiveFrames.upright(640, 480, -90))
        assertEquals(Dims(640, 480), LiveFrames.upright(640, 480, 0))
        assertEquals(Dims(640, 480), LiveFrames.upright(640, 480, 180))
        assertEquals(Dims(640, 480), LiveFrames.upright(640, 480, 360))
    }

    @Test
    fun detectionsClipToTheFrame() {
        val f = LiveFrames.detections(Dims(100, 50), listOf(p(90, 40, 30, 30), p(200, 0, 5, 5)), mirrored = false)
        assertEquals(Dims(100, 50), f.dims)
        assertEquals(listOf(Rect(90, 40, 10, 10)), f.proposals.map { it.rect })
    }

    @Test
    fun mirroringFlipsHorizontally() {
        val f = LiveFrames.detections(Dims(100, 50), listOf(p(10, 5, 20, 10, BlurClass.PLATE)), mirrored = true)
        assertEquals(Rect(70, 5, 20, 10), f.proposals.single().rect)
        assertEquals(BlurClass.PLATE, f.proposals.single().blurClass)
    }

    @Test
    fun detectionsAreCapped() {
        val many = List(40) { p(it, 0, 1, 1) }
        assertEquals(CameraCopy.MAX_DETECTIONS, LiveFrames.detections(Dims(100, 100), many, false).proposals.size)
    }

    @Test
    fun emptyOrInvalidFramesHaveNoProposals() {
        assertTrue(LiveFrames.detections(Dims(100, 100), emptyList(), false).proposals.isEmpty())
        assertTrue(LiveFrames.detections(Dims(0, 100), listOf(p(0, 0, 5, 5)), false).proposals.isEmpty())
    }

    @Test
    fun liveDetectionsFeedTheCameraChips() {
        val frame = LiveFrames.detections(Dims(480, 640), listOf(p(48, 64, 96, 128), p(0, 0, 48, 64, BlurClass.HOUSE_NO)), mirrored = false)
        val chips = CameraCopy.detections(frame, app.witness.core.model.CaptureMode.POST)
        assertEquals(listOf("Face · auto-blur", "House number · auto-blur"), chips.map { it.label })
        assertEquals(0.1f, chips.first().box.left)
        assertEquals(0.1f, chips.first().box.top)
    }
}
