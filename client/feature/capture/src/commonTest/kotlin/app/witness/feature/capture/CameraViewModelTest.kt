package app.witness.feature.capture

import app.witness.core.designsystem.WitnessState
import app.witness.core.domain.fake.FakeAnchorScheduler
import app.witness.core.domain.fake.FakeCaptureSource
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.domain.fake.FakeLocationSource
import app.witness.core.domain.fake.FakeSealEngine
import app.witness.core.domain.fake.FakeTelemetry
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCounterStore
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.port.CaptureFrame
import app.witness.core.domain.port.CaptureSource
import app.witness.core.domain.port.LocationReading
import app.witness.core.domain.usecase.SealCapture
import app.witness.core.domain.usecase.SealSigner
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureMode
import app.witness.core.model.Dims
import app.witness.core.model.EnumBits
import app.witness.core.model.Freshness
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationPrecision
import app.witness.core.model.LocationReason
import app.witness.core.model.Rect
import app.witness.core.model.Settings
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class CameraViewModelTest {
    private val main = UnconfinedTestDispatcher()

    @BeforeTest
    fun setUp() = Dispatchers.setMain(main)

    @AfterTest
    fun tearDown() = Dispatchers.resetMain()

    private class Graph(source: CaptureSource? = null, initial: Settings = Settings()) {
        val clock = FakeClock(16 * 3_600_000L + 12 * 60_000 + 9_000)
        val vault = FakeKeyVault(clock)
        val engine = FakeSealEngine()
        val captures = InMemoryCaptureStore()
        val settings = InMemorySettingsStore(initial)
        val location = FakeLocationSource()
        val telemetry = FakeTelemetry()
        val scheduler = FakeAnchorScheduler()
        val online = MutableStateFlow(true)
        val detections = MutableStateFlow(FrameDetections.NONE)
        val source = source ?: FakeCaptureSource()
        val seal = SealCapture(this.source, location, engine, SealSigner(engine, InMemoryCounterStore()), vault, captures, settings, scheduler, clock, SeededRandom(3), telemetry)

        suspend fun key(enrolled: Boolean = true) {
            val g = vault.generate(ByteString.of(byteArrayOf(1)), Freshness.FRESH)
            if (enrolled) vault.markEnrolled(g.key.keyId, clock.nowMs, clock.nowMs + 1_000_000, "1a", ByteString.EMPTY)
        }

        fun vm(offset: Long = 0L) = CameraViewModel(seal, captures, settings, vault, location, online, detections, flowOf(clock.nowMs), { offset }, telemetry)
    }

    private fun TestScope.started(vm: CameraViewModel): CameraViewModel {
        backgroundScope.launch(main) { vm.state.collect {} }
        return vm
    }

    private val high = LocationReading(LocationEvidence(null, LocationConfidence.HIGH, EnumBits.of(LocationReason.RAW_GNSS_PASSED)), null, null, EnumBits.none())

    @Test
    fun sealPillReflectsActiveKey() = runTest(main) {
        val g = Graph().apply { key() }
        val vm = started(g.vm())
        assertEquals(SealPill("StrongBox seal", true), vm.state.value.seal)
    }

    @Test
    fun sealPillWithoutKey() = runTest(main) {
        val vm = started(Graph().vm())
        assertFalse(vm.state.value.seal.ready)
    }

    @Test
    fun retiredKeyIsNotReady() = runTest(main) {
        val g = Graph().apply { key() }
        g.vault.retire(g.vault.activeKey()!!.keyId)
        assertFalse(started(g.vm()).state.value.seal.ready)
    }

    @Test
    fun locationPillFromReading() = runTest(main) {
        val g = Graph().apply { location.reading = high }
        val vm = started(g.vm())
        assertEquals("Location · High", vm.state.value.location.text)
    }

    @Test
    fun locationFailureIsReportedAndShownOff() = runTest(main) {
        val g = Graph().apply { location.failure = IllegalStateException("gnss") }
        val vm = started(g.vm())
        assertEquals("Location off", vm.state.value.location.text)
        assertEquals(1, g.telemetry.errors.size)
    }

    @Test
    fun metaUsesTicksAndOffset() = runTest(main) {
        val g = Graph().apply { location.reading = high }
        val vm = started(g.vm(offset = 3_600_000))
        assertEquals("5:12:09 PM · spoof check passed", vm.state.value.meta)
    }

    @Test
    fun metaOffline() = runTest(main) {
        val g = Graph()
        val vm = started(g.vm())
        g.online.value = false
        assertEquals("4:12:09 PM · no network · sealed on-device", vm.state.value.meta)
        assertFalse(vm.state.value.online)
    }

    @Test
    fun shutterInPostModeOnlineOpensComposer() = runTest(main) {
        val g = Graph().apply { key() }
        val vm = started(g.vm())
        vm.shutter()
        val effect = assertIs<CameraEffect.OpenComposer>(vm.effects.first())
        assertNotNull(g.captures.get(effect.recordId))
        assertNull(vm.state.value.toast)
        assertFalse(vm.state.value.sealing)
    }

    @Test
    fun shutterOfflineShowsQueuedToast() = runTest(main) {
        val g = Graph().apply { key() }
        val vm = started(g.vm())
        g.online.value = false
        vm.shutter()
        val toast = assertNotNull(vm.state.value.toast)
        assertEquals("Sealed · StrongBox", toast.title)
        assertEquals("Queued for anchoring", toast.status)
        assertNull(vm.state.value.hint)
    }

    @Test
    fun unenrolledKeyToastExplainsSetup() = runTest(main) {
        val g = Graph().apply { key(enrolled = false) }
        val vm = started(g.vm())
        g.online.value = false
        vm.shutter()
        assertEquals("Anchors after device setup", vm.state.value.toast?.status)
    }

    @Test
    fun caseModeStaysOnCameraWithToast() = runTest(main) {
        val g = Graph(initial = Settings(captureMode = CaptureMode.CASE)).apply { key() }
        val vm = started(g.vm())
        vm.shutter()
        assertEquals("Anchoring…", vm.state.value.toast?.status)
    }

    @Test
    fun toastFollowsStatusChanges() = runTest(main) {
        val g = Graph().apply { key() }
        val vm = started(g.vm())
        g.online.value = false
        vm.shutter()
        val id = vm.state.value.toast!!.recordId
        g.captures.setStatus(id, AnchoringStatus.ANCHORED, null)
        assertEquals("Anchored", vm.state.value.toast?.status)
        assertEquals(WitnessState.Anchored, vm.state.value.last?.state)
    }

    @Test
    fun dismissToast() = runTest(main) {
        val g = Graph().apply { key() }
        val vm = started(g.vm())
        g.online.value = false
        vm.shutter()
        vm.dismissToast()
        assertNull(vm.state.value.toast)
    }

    @Test
    fun noKeyShowsError() = runTest(main) {
        val vm = started(Graph().vm())
        vm.shutter()
        assertEquals(CameraCopy.NO_KEY, vm.state.value.error)
        vm.dismissError()
        assertNull(vm.state.value.error)
    }

    @Test
    fun sealFailureIsReported() = runTest(main) {
        val g = Graph(source = CaptureSource { throw IllegalStateException("camera") }).apply { key() }
        val vm = started(g.vm())
        vm.shutter()
        assertEquals(CameraCopy.SEAL_FAILED, vm.state.value.error)
        assertEquals(1, g.telemetry.errors.size)
        assertFalse(vm.state.value.sealing)
    }

    @Test
    fun shutterIgnoredWhileSealing() = runTest(main) {
        val gate = CompletableDeferred<Unit>()
        var shots = 0
        val source = CaptureSource {
            shots++
            gate.await()
            CaptureFrame(ByteString.of(byteArrayOf(-1, -40, shots.toByte())), 0, 1, 1)
        }
        val g = Graph(source = source).apply { key() }
        val vm = started(g.vm())
        vm.shutter()
        assertTrue(vm.state.value.sealing)
        vm.shutter()
        gate.complete(Unit)
        assertEquals(1, shots)
        assertFalse(vm.state.value.sealing)
    }

    @Test
    fun lastCaptureShowsPhotoAndBadge() = runTest(main) {
        val g = Graph().apply { key() }
        val vm = started(g.vm())
        g.online.value = false
        vm.shutter()
        val id = vm.state.value.toast!!.recordId
        val jpeg = byteArrayOf(-1, -40, 9)
        g.captures.putImage(id, ByteString.of(jpeg))
        g.captures.setStatus(id, AnchoringStatus.QUEUED, null)
        val last = assertNotNull(vm.state.value.last)
        assertEquals(id, last.recordId)
        assertEquals(WitnessState.Queued, last.state)
    }

    @Test
    fun photoIsLoadedOnceAndCached() = runTest(main) {
        val g = Graph().apply { key() }
        val vm = started(g.vm())
        g.online.value = false
        vm.shutter()
        val id = vm.state.value.toast!!.recordId
        val first = assertNotNull(vm.state.value.last?.photo)
        assertEquals(g.captures.readImage(id)!!.size, first.bytes.size)
        assertSame(first, vm.state.value.toast?.photo)
        g.captures.putImage(id, ByteString.of(byteArrayOf(-1, -40, 7)))
        g.captures.setStatus(id, AnchoringStatus.ANCHORED, null)
        assertSame(first, vm.state.value.last?.photo)
        g.captures.setStatus(id, AnchoringStatus.QUEUED, null)
        assertSame(first, vm.state.value.last?.photo)
    }

    @Test
    fun setModeUpdatesSettings() = runTest(main) {
        val g = Graph()
        val vm = started(g.vm())
        vm.setMode(CaptureMode.CASE)
        assertEquals(CaptureMode.CASE, g.settings.value.captureMode)
        assertEquals(1, vm.state.value.modeIndex)
    }

    @Test
    fun flashAndLensCycle() = runTest(main) {
        val vm = started(Graph().vm())
        vm.cycleFlash()
        vm.flipLens()
        assertEquals(Flash.ON, vm.state.value.flash)
        assertEquals(Lens.FRONT, vm.state.value.lens)
    }

    @Test
    fun detectionsDriveHint() = runTest(main) {
        val g = Graph()
        val vm = started(g.vm())
        g.detections.value = FrameDetections(
            Dims(400, 300),
            listOf(BlurProposal(Rect(10, 10, 20, 20), BlurClass.HOUSE_NO), BlurProposal(Rect(100, 100, 40, 20), BlurClass.PLATE)),
        )
        val s = vm.state.value
        assertEquals(2, s.detections.size)
        assertEquals(400f / 300f, s.frameAspect)
        assertEquals("2 things will be blurred when you post", s.hint?.title)
    }

    @Test
    fun detectionsHiddenInCaseMode() = runTest(main) {
        val g = Graph(initial = Settings(captureMode = CaptureMode.CASE))
        val vm = started(g.vm())
        g.detections.value = FrameDetections(Dims(400, 300), listOf(BlurProposal(Rect(10, 10, 20, 20), BlurClass.FACE)))
        assertTrue(vm.state.value.detections.isEmpty())
        assertNull(vm.state.value.hint)
    }

    @Test
    fun exactPrecisionWarnsWithoutDetections() = runTest(main) {
        val g = Graph(initial = Settings(defaultPrecision = LocationPrecision.EXACT))
        val vm = started(g.vm())
        assertNotNull(vm.state.value.hint)
    }

    @Test
    fun locationRefreshesAfterSeal() = runTest(main) {
        val g = Graph().apply { key() }
        val vm = started(g.vm())
        assertEquals("Location off", vm.state.value.location.text)
        g.location.reading = high
        g.online.value = false
        vm.shutter()
        assertEquals("Location · High", vm.state.value.location.text)
    }
}
