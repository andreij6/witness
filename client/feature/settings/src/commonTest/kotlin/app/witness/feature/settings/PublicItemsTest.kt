package app.witness.feature.settings

import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Dims
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.LocationConfidence
import app.witness.core.model.RecordId
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

internal fun rid(n: Int): RecordId = RecordId.fromDigest(ByteArray(32) { (n * 31 + it).toByte() })

internal fun capture(
    n: Int,
    at: Long = n.toLong(),
    parent: Int? = null,
    status: AnchoringStatus = AnchoringStatus.ANCHORED,
    origin: CaptureOrigin = if (parent == null) CaptureOrigin.ORIGINAL else CaptureOrigin.DERIVATIVE,
    erased: Boolean = false,
    label: String? = null,
    place: String? = null,
    key: Boolean = true,
) = Capture(
    recordId = rid(n),
    captureId = null,
    origin = origin,
    status = status,
    createdAtMs = at,
    keyRef = if (key) KeyRef(KeyId.fromDigest(ByteArray(32) { 7 }), n.toLong()) else null,
    locationConfidence = LocationConfidence.HIGH,
    dims = Dims(4, 3),
    parent = parent?.let(::rid),
    label = label,
    place = place,
    publicErased = erased,
)

class PublicItemsTest {
    @Test
    fun emptyAndNothingPublicGiveNoItems() {
        assertEquals(emptyList(), PublicItems.of(emptyList()))
        val private = listOf(
            capture(1, status = AnchoringStatus.QUEUED),
            capture(2, origin = CaptureOrigin.IMPORT),
            capture(3, erased = true),
            capture(4, key = false),
            capture(5, status = AnchoringStatus.FAILED),
        )
        assertEquals(emptyList(), PublicItems.of(private))
    }

    @Test
    fun rootsAreListedNewestFirstWithTheirPublicCopies() {
        val items = PublicItems.of(
            listOf(
                capture(1, at = 10, label = "Front door"),
                capture(2, at = 30, place = "Denver area"),
                capture(3, at = 31, parent = 1),
                capture(4, at = 32, parent = 1),
                capture(5, at = 20),
            ),
        )
        assertEquals(listOf(rid(2), rid(5), rid(1)), items.map { it.recordId })
        assertEquals(listOf("Denver area", PublicItems.UNTITLED, "Front door"), items.map { it.title })
        assertEquals("${rid(1).grouped()} · 3 public copies", items[2].subtitle)
        assertEquals("${rid(2).grouped()} · 1 public copy", items[0].subtitle)
    }

    @Test
    fun aRootWhoseOnlyPublicCopyIsAPostIsStillListed() {
        val items = PublicItems.of(listOf(capture(1, status = AnchoringStatus.QUEUED), capture(2, parent = 1), capture(3, parent = 1, erased = true)))
        assertEquals(listOf(rid(1)), items.map { it.recordId })
        assertEquals("${rid(1).grouped()} · 1 public copy", items.single().subtitle)
    }

    @Test
    fun erasedFamiliesAndOrphanPostsDropOut() {
        assertEquals(emptyList(), PublicItems.of(listOf(capture(1, erased = true), capture(2, parent = 1, erased = true))))
        assertEquals(emptyList(), PublicItems.of(listOf(capture(2, parent = 9))))
    }

    @Test
    fun blankLabelsFallBack() {
        assertEquals("Kitchen", PublicItems.of(listOf(capture(1, label = " ", place = "Kitchen"))).single().title)
        assertEquals(PublicItems.UNTITLED, PublicItems.of(listOf(capture(1, label = "", place = ""))).single().title)
    }

    @Test
    fun confirmationNamesTheItemAndWhatStays() {
        val item = PublicItems.of(listOf(capture(1, label = "Front door"))).single()
        val text = PrivacyDataCopy.eraseItemText(item)
        assertTrue(text.startsWith("Front door · ${rid(1).grouped()} · 1 public copy.\n\n"), text)
        assertTrue(text.endsWith(PrivacyDataCopy.ERASE_ITEM_TEXT))
    }

    @Test
    fun orderIsDeterministicForEqualTimes() {
        val all = listOf(capture(1, at = 5), capture(2, at = 5), capture(3, at = 5))
        assertEquals(PublicItems.of(all), PublicItems.of(all.reversed()))
    }
}

class PrivacyItemEraseTest : MainDispatcherTest() {
    private val exporter = app.witness.feature.settings.fake.FakeDataExporter(ExportOutcome.Ready(1))
    private val captures = MutableStateFlow(listOf(capture(1, at = 1), capture(2, at = 2), capture(3, parent = 1)))
    private val erased = mutableListOf<RecordId>()
    private var eraseItem: suspend (RecordId) -> EraseReport = { id ->
        erased += id
        captures.value = captures.value.map { if (it.recordId == id || it.parent == id) it.copy(publicErased = true) else it }
        EraseReport(erased = listOf(id))
    }

    private fun vm() = PrivacyDataViewModel(exporter, captures, { eraseItem(it) }) { EraseReport() }

    @Test
    fun listsPublicItemsAndFollowsTheStore() = runTest {
        val vm = vm()
        assertEquals(listOf(rid(2), rid(1)), vm.state.value.items.map { it.recordId })
        captures.value = captures.value + capture(4, at = 9)
        assertEquals(listOf(rid(4), rid(2), rid(1)), vm.state.value.items.map { it.recordId })
    }

    @Test
    fun itemEraseNeedsConfirmationAndUsesTheItemEraseUseCase() = runTest {
        val vm = vm()
        vm.confirmEraseItem()
        assertTrue(erased.isEmpty())
        vm.requestEraseItem(rid(1))
        assertEquals(rid(1), vm.state.value.confirmItem?.recordId)
        vm.cancelErase()
        assertNull(vm.state.value.confirmItem)
        vm.confirmEraseItem()
        assertTrue(erased.isEmpty())
        vm.requestEraseItem(rid(1))
        vm.confirmEraseItem()
        assertEquals(listOf(rid(1)), erased)
        assertEquals("1 public copy erased. Proofs stay.", vm.state.value.notice?.text)
        assertEquals(listOf(rid(2)), vm.state.value.items.map { it.recordId })
        assertNull(vm.state.value.busy)
        assertNull(vm.state.value.erasingItem)
    }

    @Test
    fun unknownItemsCannotBeErased() = runTest {
        val vm = vm()
        vm.requestEraseItem(rid(99))
        assertNull(vm.state.value.confirmItem)
        vm.confirmEraseItem()
        assertTrue(erased.isEmpty())
    }

    @Test
    fun itemEraseOfflineIsReported() = runTest {
        eraseItem = { throw IllegalStateException("no network") }
        val vm = vm()
        vm.requestEraseItem(rid(2))
        vm.confirmEraseItem()
        assertTrue(vm.state.value.notice!!.text.startsWith("You're offline."))
        assertNull(vm.state.value.busy)
    }

    @Test
    fun itemEraseIsExclusiveWithOtherWork() = runTest {
        val gate = CompletableDeferred<Unit>()
        eraseItem = { gate.await(); EraseReport(erased = listOf(it)) }
        val vm = vm()
        vm.requestEraseItem(rid(2))
        vm.confirmEraseItem()
        assertEquals(PrivacyBusy.Erasing, vm.state.value.busy)
        assertEquals(rid(2), vm.state.value.erasingItem)
        vm.requestEraseItem(rid(1))
        vm.requestErase()
        vm.export()
        assertNull(vm.state.value.confirmItem)
        assertEquals(false, vm.state.value.confirmErase)
        gate.complete(Unit)
        assertNull(vm.state.value.busy)
    }

    @Test
    fun wholeEraseAndItemEraseConfirmationsAreExclusive() = runTest {
        val vm = vm()
        vm.requestEraseItem(rid(1))
        vm.requestErase()
        assertNull(vm.state.value.confirmItem)
        assertTrue(vm.state.value.confirmErase)
        vm.requestEraseItem(rid(1))
        assertEquals(false, vm.state.value.confirmErase)
    }
}
