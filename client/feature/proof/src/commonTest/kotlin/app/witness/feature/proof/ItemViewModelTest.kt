package app.witness.feature.proof

import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCaseStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.usecase.EraseMine
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.DeviceKey
import app.witness.core.model.Freshness
import app.witness.core.model.SecurityLevel
import app.witness.core.model.TemplateId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
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
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class ItemViewModelTest {
    private val clock = FakeClock(Fixtures.T0)
    private val captures = InMemoryCaptureStore()
    private val cases = InMemoryCaseStore()
    private val registry = FakeRegistry(clock)
    private val vault = FakeKeyVault(clock)
    private val deps = ItemDependencies(captures, cases, registry, EraseMine(captures, registry, vault, SeededRandom(1)))

    @BeforeTest
    fun setUp() = Dispatchers.setMain(UnconfinedTestDispatcher())

    @AfterTest
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun missingCaptureIsReported() {
        val s = ItemViewModel(Fixtures.recordId, deps).state.value
        assertFalse(s.loading)
        assertTrue(s.missing)
    }

    @Test
    fun anchoredCaptureLoadsEverything() = runTest {
        val case = cases.create("Fender bender", TemplateId.GENERAL, Fixtures.T0)
        captures.insert(Fixtures.capture(Fixtures.otherId, caseId = case.id, createdAtMs = Fixtures.T0 - 1000), null, null)
        captures.insert(Fixtures.capture(caseId = case.id), null, Fixtures.opening)
        captures.insert(Fixtures.capture(Fixtures.thirdId, caseId = case.id, createdAtMs = Fixtures.T0 + 1000), null, null)
        captures.putImage(Fixtures.recordId, ByteString.of(byteArrayOf(1, 2)))
        registry.putRecord(Fixtures.recordId, Fixtures.claim())
        registry.putKey(Fixtures.key())
        val s = ItemViewModel(Fixtures.recordId, deps).state.value
        assertFalse(s.loading)
        assertEquals("Photo 2 of 3", s.position)
        assertEquals("Fender bender", s.caseTitle)
        assertEquals("Now in Fender bender · 3 items", s.moveSubtitle)
        assertEquals("Fender bender · captured 2:12 PM", s.itemSubtitle)
        assertEquals(ByteString.of(byteArrayOf(1, 2)), s.image)
        assertTrue(s.certified)
        assertEquals(5, s.custody.size)
        assertEquals("https://witness.app/p/${Fixtures.recordId.text}", s.webUrl)
        assertEquals("39.7392° N, 104.9903° W · ±6 m", assertNotNull(s.location).coordinates)
        assertTrue(s.json.contains(Fixtures.recordId.text))
    }

    @Test
    fun offlineRegistryStillShowsLocalFacts() = runTest {
        registry.offline = true
        captures.insert(Fixtures.capture(), null, null)
        val s = ItemViewModel(Fixtures.recordId, deps).state.value
        assertFalse(s.certified)
        assertEquals(5, s.custody.size)
        assertNull(s.position)
        assertEquals("Not in a case", s.moveSubtitle)
        assertNull(s.location)
    }

    @Test
    fun importsNeverHitTheRegistry() = runTest {
        captures.insert(Fixtures.capture(origin = CaptureOrigin.IMPORT, status = AnchoringStatus.UNVERIFIED, anchor = null), null, null)
        val s = ItemViewModel(Fixtures.recordId, deps).state.value
        assertEquals(0, registry.getCalls)
        assertNull(s.webUrl)
        assertEquals("Unverified import", s.header?.title)
    }

    @Test
    fun outOfRangeOpeningIsIgnored() = runTest {
        captures.insert(Fixtures.capture(), null, Fixtures.opening.copy(latE7 = 1_000_000_000))
        assertNull(ItemViewModel(Fixtures.recordId, deps).state.value.location)
    }

    @Test
    fun eraseFlowConfirmsThenReports() = runTest {
        captures.insert(Fixtures.capture(), null, null)
        registry.putRecord(Fixtures.recordId, Fixtures.claim())
        vault.put(DeviceKey(Fixtures.keyId, SecurityLevel.STRONG_BOX, Freshness.FRESH, 0))
        val vm = ItemViewModel(Fixtures.recordId, deps)
        vm.requestErase()
        assertEquals(EraseState.Confirm, vm.state.value.erase)
        vm.dismissErase()
        assertEquals(EraseState.Idle, vm.state.value.erase)
        vm.requestErase()
        vm.confirmErase()
        val done = assertIs<EraseState.Finished>(vm.state.value.erase)
        assertEquals(listOf(Fixtures.recordId), done.report.erased)
        assertEquals(1, registry.erased.size)
        assertTrue(captures.get(Fixtures.recordId)!!.publicErased)
    }

    @Test
    fun eraseOfflineIsReportedNotThrown() = runTest {
        captures.insert(Fixtures.capture(), null, null)
        registry.putRecord(Fixtures.recordId, Fixtures.claim())
        vault.put(DeviceKey(Fixtures.keyId, SecurityLevel.STRONG_BOX, Freshness.FRESH, 0))
        val vm = ItemViewModel(Fixtures.recordId, deps)
        registry.offline = true
        vm.confirmErase()
        assertTrue(assertIs<EraseState.Finished>(vm.state.value.erase).report.offline)
    }

    @Test
    fun positionBreaksTiesByRecordId() {
        val a = Fixtures.capture(Fixtures.recordId, caseId = app.witness.core.model.LocalCaseId(1))
        val b = Fixtures.capture(Fixtures.otherId, caseId = app.witness.core.model.LocalCaseId(1))
        assertEquals(1 to 2, ItemViewModel.positionOf(a, listOf(a, b)))
        assertEquals(2 to 2, ItemViewModel.positionOf(b, listOf(a, b)))
        assertNull(ItemViewModel.positionOf(a, emptyList()))
        assertNull(ItemViewModel.positionOf(Fixtures.capture(), listOf(a)))
        assertNull(ItemViewModel.positionOf(a, listOf(b)))
    }

    @Test
    fun eraseMessagesCoverEveryOutcome() {
        val id = Fixtures.recordId
        assertTrue(ItemViewModel.eraseMessage(EraseReport(offline = true)).contains("offline"))
        assertTrue(ItemViewModel.eraseMessage(EraseReport()).contains("no public copies"))
        assertTrue(ItemViewModel.eraseMessage(EraseReport(erased = listOf(id))).contains("1 public copy"))
        assertTrue(ItemViewModel.eraseMessage(EraseReport(erased = listOf(id, id))).contains("2 public copies"))
        assertTrue(ItemViewModel.eraseMessage(EraseReport(rejected = listOf(id))).contains("couldn't be erased"))
    }
}
