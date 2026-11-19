package app.witness.feature.proof

import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCaseStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.port.CaptureStore
import app.witness.core.domain.port.CaseStore
import app.witness.core.domain.usecase.EraseMine
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.Case
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.model.TemplateId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class ItemMoveDeleteTest {
    private val clock = FakeClock(Fixtures.T0)
    private val captures = InMemoryCaptureStore()
    private val cases = InMemoryCaseStore()
    private val registry = FakeRegistry(clock)
    private val vault = FakeKeyVault(clock)

    private fun deps(c: CaptureStore = captures, k: CaseStore = cases) = ItemDependencies(c, k, registry, EraseMine(c, registry, vault, SeededRandom(1)))

    private fun vm(c: CaptureStore = captures, k: CaseStore = cases) = ItemViewModel(Fixtures.recordId, deps(c, k))

    @BeforeTest
    fun setUp() = Dispatchers.setMain(UnconfinedTestDispatcher())

    @AfterTest
    fun tearDown() = Dispatchers.resetMain()

    private suspend fun twoCases(): Pair<Case, Case> {
        val a = cases.create("Fender bender", TemplateId.GENERAL, 1)
        val b = cases.create("Hail damage", TemplateId.GENERAL, 2)
        return a to b
    }

    @Test
    fun pickerListsCasesWithCurrentMarkedAndNoCaseLast() = runTest {
        val (a, b) = twoCases()
        captures.insert(Fixtures.capture(caseId = a.id), null, null)
        val vm = vm()
        vm.requestMove()
        val picking = assertIs<MoveState.Picking>(vm.state.value.move)
        assertEquals(listOf(b.title, a.title, ItemViewModel.NO_CASE), picking.options.map { it.title })
        assertEquals(listOf(false, true, false), picking.options.map { it.current })
        assertNull(picking.options.last().id)
    }

    @Test
    fun noCaseIsCurrentWhenUnfiled() = runTest {
        captures.insert(Fixtures.capture(), null, null)
        val vm = vm()
        vm.requestMove()
        val picking = assertIs<MoveState.Picking>(vm.state.value.move)
        assertEquals(listOf(true), picking.options.map { it.current })
    }

    @Test
    fun pickingTheCurrentCaseClosesWithoutChange() = runTest {
        val (a, _) = twoCases()
        captures.insert(Fixtures.capture(caseId = a.id), null, null)
        val vm = vm()
        vm.requestMove()
        vm.pickCase(assertIs<MoveState.Picking>(vm.state.value.move).options.first { it.current })
        assertEquals(MoveState.Idle, vm.state.value.move)
        assertEquals(a.id, captures.get(Fixtures.recordId)!!.caseId)
    }

    @Test
    fun moveNeedsConfirmationThenUpdatesTheCaptureAndReloads() = runTest {
        val (a, b) = twoCases()
        captures.insert(Fixtures.capture(caseId = a.id), null, null)
        val vm = vm()
        assertEquals("Fender bender", vm.state.value.caseTitle)
        vm.requestMove()
        vm.pickCase(assertIs<MoveState.Picking>(vm.state.value.move).options.first { it.id == b.id })
        val confirm = assertIs<MoveState.Confirm>(vm.state.value.move)
        assertEquals("Move to Hail damage?", ItemViewModel.moveTitle(confirm))
        assertEquals("Fender bender", confirm.from)
        assertEquals(a.id, captures.get(Fixtures.recordId)!!.caseId)
        vm.confirmMove()
        assertEquals(b.id, captures.get(Fixtures.recordId)!!.caseId)
        assertEquals(MoveState.Idle, vm.state.value.move)
        assertEquals("Hail damage", vm.state.value.caseTitle)
        assertEquals("Photo 1 of 1", vm.state.value.position)
    }

    @Test
    fun removingFromACaseClearsTheCaseId() = runTest {
        val (a, _) = twoCases()
        captures.insert(Fixtures.capture(caseId = a.id), null, null)
        val vm = vm()
        vm.requestMove()
        vm.pickCase(assertIs<MoveState.Picking>(vm.state.value.move).options.last())
        val confirm = assertIs<MoveState.Confirm>(vm.state.value.move)
        assertEquals("Remove from Fender bender?", ItemViewModel.moveTitle(confirm))
        assertTrue(ItemViewModel.moveText(confirm).contains("leaves the case"))
        vm.confirmMove()
        assertNull(captures.get(Fixtures.recordId)!!.caseId)
        assertNull(vm.state.value.caseTitle)
        assertEquals("Not in a case", vm.state.value.moveSubtitle)
    }

    @Test
    fun dismissingTheConfirmationChangesNothing() = runTest {
        val (a, b) = twoCases()
        captures.insert(Fixtures.capture(caseId = a.id), null, null)
        val vm = vm()
        vm.requestMove()
        vm.pickCase(CaseOption(b.id, b.title, false))
        vm.dismissMove()
        assertEquals(MoveState.Idle, vm.state.value.move)
        vm.confirmMove()
        assertEquals(a.id, captures.get(Fixtures.recordId)!!.caseId)
    }

    @Test
    fun pickOutsideThePickerIsIgnored() = runTest {
        captures.insert(Fixtures.capture(), null, null)
        val vm = vm()
        vm.pickCase(CaseOption(LocalCaseId(3), "x", false))
        assertEquals(MoveState.Idle, vm.state.value.move)
    }

    @Test
    fun requestMoveIsIgnoredWhileAFlowIsOpen() = runTest {
        val (a, b) = twoCases()
        captures.insert(Fixtures.capture(caseId = a.id), null, null)
        val vm = vm()
        vm.requestMove()
        vm.pickCase(CaseOption(b.id, b.title, false))
        vm.requestMove()
        assertIs<MoveState.Confirm>(vm.state.value.move)
    }

    @Test
    fun failedMoveShowsFailureAndCanBeDismissed() = runTest {
        val (a, b) = twoCases()
        captures.insert(Fixtures.capture(caseId = a.id), null, null)
        val broken = object : CaptureStore by captures {
            override suspend fun update(capture: Capture) = throw IllegalStateException("disk")
        }
        val vm = vm(broken)
        vm.requestMove()
        vm.pickCase(CaseOption(b.id, b.title, false))
        vm.confirmMove()
        assertEquals(MoveState.Failed, vm.state.value.move)
        assertEquals(a.id, captures.get(Fixtures.recordId)!!.caseId)
        vm.dismissMove()
        assertEquals(MoveState.Idle, vm.state.value.move)
    }

    @Test
    fun moveOfAVanishedCaptureFails() = runTest {
        val (a, b) = twoCases()
        captures.insert(Fixtures.capture(caseId = a.id), null, null)
        val vm = vm()
        vm.requestMove()
        vm.pickCase(CaseOption(b.id, b.title, false))
        captures.delete(Fixtures.recordId)
        vm.confirmMove()
        assertEquals(MoveState.Failed, vm.state.value.move)
    }

    @Test
    fun unreadableCasesFailThePicker() = runTest {
        captures.insert(Fixtures.capture(), null, null)
        val broken = object : CaseStore by cases {
            override fun observeAll(): Flow<List<Case>> = flow { throw IllegalStateException("db") }
        }
        val vm = vm(k = broken)
        vm.requestMove()
        assertEquals(MoveState.Failed, vm.state.value.move)
    }

    @Test
    fun deleteAsksWithAnchoredCopyThenRemovesEverything() = runTest {
        captures.insert(Fixtures.capture(), null, Fixtures.opening, ByteString.of(byteArrayOf(1)))
        val vm = vm()
        vm.requestDelete()
        val confirm = assertIs<DeleteState.Confirm>(vm.state.value.delete)
        assertTrue(confirm.anchored)
        assertNotNull(captures.get(Fixtures.recordId))
        vm.confirmDelete()
        assertEquals(DeleteState.Deleted, vm.state.value.delete)
        assertNull(captures.get(Fixtures.recordId))
        assertNull(captures.readImage(Fixtures.recordId))
        assertNull(captures.opening(Fixtures.recordId))
        assertTrue(captures.observeAll().first().isEmpty())
    }

    @Test
    fun unanchoredDeleteWarnsAboutLosingTheProof() = runTest {
        captures.insert(Fixtures.capture(status = AnchoringStatus.QUEUED, anchor = null), null, null)
        val vm = vm()
        vm.requestDelete()
        val confirm = assertIs<DeleteState.Confirm>(vm.state.value.delete)
        assertEquals(false, confirm.anchored)
        assertTrue(ItemViewModel.deleteText(false).contains("never get a public proof"))
        assertTrue(ItemViewModel.deleteText(true).contains("anchored proof stays"))
    }

    @Test
    fun dismissingDeleteKeepsTheCapture() = runTest {
        captures.insert(Fixtures.capture(), null, null)
        val vm = vm()
        vm.requestDelete()
        vm.dismissDelete()
        assertEquals(DeleteState.Idle, vm.state.value.delete)
        vm.confirmDelete()
        assertNotNull(captures.get(Fixtures.recordId))
    }

    @Test
    fun deleteOfAMissingItemIsNotOffered() = runTest {
        val vm = vm()
        vm.requestDelete()
        assertEquals(DeleteState.Idle, vm.state.value.delete)
    }

    @Test
    fun failedDeleteReportsAndKeepsTheCapture() = runTest {
        captures.insert(Fixtures.capture(), null, null)
        val broken = object : CaptureStore by captures {
            override suspend fun delete(recordId: RecordId) = throw IllegalStateException("disk")
        }
        val vm = vm(broken)
        vm.requestDelete()
        vm.confirmDelete()
        assertEquals(DeleteState.Failed, vm.state.value.delete)
        assertNotNull(captures.get(Fixtures.recordId))
        vm.dismissDelete()
        assertEquals(DeleteState.Idle, vm.state.value.delete)
    }

    @Test
    fun deletedStateIsTerminal() = runTest {
        captures.insert(Fixtures.capture(), null, null)
        val vm = vm()
        vm.requestDelete()
        vm.confirmDelete()
        vm.dismissDelete()
        vm.requestDelete()
        assertEquals(DeleteState.Deleted, vm.state.value.delete)
    }
}
