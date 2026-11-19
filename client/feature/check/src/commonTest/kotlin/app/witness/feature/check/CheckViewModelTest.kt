package app.witness.feature.check

import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeIndex
import app.witness.core.domain.fake.FakeMedia
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.FakeVerifyEngine
import app.witness.core.domain.fake.SoftwareSha256
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.model.ByteString
import app.witness.core.model.Platform
import app.witness.core.model.Verdict
import app.witness.core.ui.StageStatus
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlinx.coroutines.awaitCancellation
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class CheckViewModelTest {
    private val clock = FakeClock(Fixtures.NOV_18_2026_1412_UTC)
    private val registry = FakeRegistry(clock)
    private val index = FakeIndex()
    private val media = FakeMedia()
    private val engine = FakeVerifyEngine()
    private val recent = InMemoryRecentChecks()
    private val shared = HashMap<String, PickedImage>()
    private val deps = CheckDependencies(CheckImage(engine, registry, index, media, Platform.ANDROID), media, clock, recent, shared = { shared[it] })

    @BeforeTest
    fun setUp() = Dispatchers.setMain(UnconfinedTestDispatcher())

    @AfterTest
    fun tearDown() = Dispatchers.resetMain()

    private fun vm(maxBytes: Int = PickedImage.MAX_BYTES) = CheckViewModel(deps, CheckSurface.App, maxBytes = maxBytes)

    private fun digest(image: PickedImage) = ByteString.of(SoftwareSha256.digest(image.bytes.toByteArray()))

    @Test
    fun startsIdleWithPendingStages() {
        val s = vm().state.value
        assertEquals(CheckPhase.Idle, s.phase)
        assertEquals(5, s.stages.size)
        assertTrue(s.stages.all { it.status == StageStatus.Pending })
    }

    @Test
    fun exactHashMatchEndsOriginalAndRecordsRecent() = runTest {
        val image = Fixtures.image(1, 2, 3)
        val rec = registry.putRecord(Fixtures.recordId, Fixtures.claim())
        index.addExact(0, digest(image), rec.recordIx)
        media.references[Fixtures.recordId] = ByteString.of(byteArrayOf(9))
        val vm = vm()
        vm.check(image)
        val s = vm.state.value
        assertEquals(CheckPhase.Done, s.phase)
        assertEquals(Verdict.ORIGINAL, s.outcome?.verdict)
        assertEquals(ByteString.of(byteArrayOf(9)), s.reference)
        assertTrue(s.shareText!!.endsWith("https://witness.app/p/${Fixtures.recordId.text}"))
        assertEquals(listOf("IMG_1.jpg"), s.recent.map { it.label })
        assertEquals(Fixtures.recordId, s.recent.single().recordId)
        assertTrue(s.stages.none { it.status == StageStatus.Running || it.status == StageStatus.Pending })
    }

    @Test
    fun fingerprintMatchIsWithChanges() = runTest {
        val image = Fixtures.image(4, 5, name = null)
        val rec = registry.putRecord(Fixtures.recordId, Fixtures.claim())
        index.addPdq(0, digest(image), rec.recordIx, 4)
        media.references[Fixtures.recordId] = ByteString.of(byteArrayOf(1))
        val vm = vm()
        vm.check(image)
        assertEquals(Verdict.MATCHES_WITH_CHANGES, vm.state.value.outcome?.verdict)
        assertEquals(CheckViewModel.DEFAULT_LABEL, vm.state.value.recent.single().label)
    }

    @Test
    fun noPublicReferenceSkipsReferenceFetch() = runTest {
        val image = Fixtures.image(6)
        val rec = registry.putRecord(Fixtures.recordId, Fixtures.claim())
        index.addPdq(0, digest(image), rec.recordIx, 4)
        val vm = vm()
        vm.check(image)
        assertEquals("Matches a sealed photo", vm.state.value.outcome?.headline)
        assertNull(vm.state.value.reference)
    }

    @Test
    fun unknownImageIsNoRecordWithoutShareText() = runTest {
        val vm = vm()
        vm.check(Fixtures.image(7))
        assertEquals(Verdict.NO_RECORD, vm.state.value.outcome?.verdict)
        assertNull(vm.state.value.shareText)
        assertNull(vm.state.value.recent.single().recordId)
    }

    @Test
    fun emptyFileIsNoRecord() = runTest {
        val vm = vm()
        vm.check(PickedImage(ByteString.EMPTY))
        assertEquals(Verdict.NO_RECORD, vm.state.value.outcome?.verdict)
    }

    @Test
    fun engineFailureEndsFailed() = runTest {
        engine.failure = IllegalStateException("boom")
        val vm = vm()
        vm.check(Fixtures.image(1))
        assertEquals(CheckPhase.Failed, vm.state.value.phase)
        assertEquals(CheckViewModel.FAILED, vm.state.value.error)
        assertTrue(vm.state.value.recent.isEmpty())
    }

    @Test
    fun oversizedFileIsRejectedBeforeChecking() = runTest {
        val vm = vm(maxBytes = 4)
        vm.check(Fixtures.image(1, 2))
        assertEquals(CheckPhase.Failed, vm.state.value.phase)
        assertEquals(CheckViewModel.TOO_LARGE, vm.state.value.error)
        assertNull(vm.state.value.image)
    }

    @Test
    fun cancelReturnsToIdle() = runTest {
        val vm = vm()
        vm.check(Fixtures.image(1))
        vm.cancel()
        val s = vm.state.value
        assertEquals(CheckPhase.Idle, s.phase)
        assertNull(s.outcome)
        assertNull(s.image)
        assertTrue(s.stages.all { it.status == StageStatus.Pending })
    }

    @Test
    fun cancelStopsARunningCheck() = runTest {
        val slow = CheckDependencies(CheckImage({ _, _, _, _ -> awaitCancellation() }, registry, index, media, Platform.WEB), media, clock)
        val vm = CheckViewModel(slow, CheckSurface.Web)
        vm.check(Fixtures.image(1))
        assertEquals(CheckPhase.Checking, vm.state.value.phase)
        vm.cancel()
        assertEquals(CheckPhase.Idle, vm.state.value.phase)
    }

    @Test
    fun sharedImageIsLookedUp() = runTest {
        shared["s1"] = Fixtures.image(3)
        val vm = vm()
        vm.checkShared("s1")
        assertEquals(CheckPhase.Done, vm.state.value.phase)
        vm.checkShared("missing")
        assertEquals(CheckPhase.Failed, vm.state.value.phase)
        assertEquals(CheckViewModel.MISSING, vm.state.value.error)
    }

    @Test
    fun submitLinkReturnsTargetOrSetsError() {
        val vm = vm()
        assertNull(vm.submitLink("nope"))
        assertEquals(PastedLink.Problem.Foreign.message, vm.state.value.linkError)
        vm.onLinkChange("x")
        assertNull(vm.state.value.linkError)
        val target = assertIs<PastedLink.Proof>(vm.submitLink("https://witness.app/p/${Fixtures.recordId.text}"))
        assertEquals(Fixtures.recordId, target.recordId)
        assertEquals("", vm.state.value.link)
    }

    @Test
    fun linkInputIsCapped() {
        val vm = vm()
        vm.onLinkChange("a".repeat(PastedLink.MAX_LENGTH * 2))
        assertEquals(PastedLink.MAX_LENGTH, vm.state.value.link.length)
    }

    @Test
    fun notifyOnlyAppliesToUnanchoredResults() = runTest {
        val vm = vm()
        vm.setNotifyWhenAnchored(true)
        assertEquals(false, vm.state.value.notifyWhenAnchored)
    }

    @Test
    fun newCheckReplacesPreviousResult() = runTest {
        val vm = vm()
        vm.check(Fixtures.image(1))
        val first = vm.state.value.outcome
        assertNotNull(first)
        vm.check(Fixtures.image(2))
        assertEquals(2, vm.state.value.recent.size)
        assertEquals(CheckPhase.Done, vm.state.value.phase)
    }

    @Test
    fun recentFlowCapsAtThree() = runTest {
        val vm = vm()
        repeat(5) { vm.check(Fixtures.image(it.toByte())) }
        assertEquals(CheckViewModel.RECENT_LIMIT, vm.state.value.recent.size)
    }

    @Suppress("unused")
    private fun never(): Flow<Unit> = flow { awaitCancellation() }

    @Suppress("unused")
    private fun none(): Flow<Unit> = emptyFlow()
}
