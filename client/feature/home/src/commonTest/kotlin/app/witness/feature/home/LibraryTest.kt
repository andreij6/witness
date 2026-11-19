package app.witness.feature.home

import app.witness.core.domain.usecase.SearchHit
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.LocalCaseId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertSame
import kotlin.test.assertTrue

class LibraryTest {
    private val fender = LocalCaseId(1)

    @Test
    fun emptyLibrary() {
        val lib = Library.EMPTY
        assertTrue(lib.isEmpty)
        assertTrue(lib.feed.isEmpty())
        assertTrue(lib.cases.isEmpty())
        assertTrue(lib.photoNumbers.isEmpty())
        assertEquals(0, lib.index.size)
    }

    @Test
    fun libraryWithOnlyCasesIsNotEmpty() = assertFalse(Library(emptyList(), listOf(case(1, "Fender bender"))).isEmpty)

    @Test
    fun feedExcludesCaseMembers() {
        val feed = Library.feedOf(listOf(capture(1), capture(2, caseId = fender), capture(3)))
        assertEquals(listOf(rid(1), rid(3)), feed.map { it.recordId })
    }

    @Test
    fun feedShowsOnlyLatestVersionOfLineage() {
        val original = capture(1, createdAtMs = NOW - HOUR)
        val edit = capture(2, createdAtMs = NOW - MIN, origin = CaptureOrigin.DERIVATIVE, parent = rid(1))
        assertEquals(listOf(rid(2)), Library.feedOf(listOf(original, edit)).map { it.recordId })
    }

    @Test
    fun feedSortsNewestFirstRegardlessOfInputOrder() {
        val feed = Library.feedOf(listOf(capture(1, createdAtMs = 10), capture(2, createdAtMs = 30), capture(3, createdAtMs = 20)))
        assertEquals(listOf(rid(2), rid(3), rid(1)), feed.map { it.recordId })
    }

    @Test
    fun feedBreaksTimestampTiesDeterministically() {
        val a = Library.feedOf(listOf(capture(2, createdAtMs = 5), capture(1, createdAtMs = 5)))
        val b = Library.feedOf(listOf(capture(1, createdAtMs = 5), capture(2, createdAtMs = 5)))
        assertEquals(a, b)
    }

    @Test
    fun photoNumbersCountPerCaseInCaptureOrder() {
        val other = LocalCaseId(2)
        val numbers = Library.photoNumbersOf(
            listOf(
                capture(1, createdAtMs = 300, caseId = fender),
                capture(2, createdAtMs = 100, caseId = fender),
                capture(3, createdAtMs = 200, caseId = other),
                capture(4, createdAtMs = 200, caseId = fender),
                capture(5),
            ),
        )
        assertEquals(mapOf(rid(2) to 1, rid(4) to 2, rid(1) to 3, rid(3) to 1), numbers)
    }

    @Test
    fun photoNumbersTieBreakByRecordId() {
        val numbers = Library.photoNumbersOf(listOf(capture(9, createdAtMs = 1, caseId = fender), capture(3, createdAtMs = 1, caseId = fender)))
        assertEquals(1, numbers[rid(3)])
        assertEquals(2, numbers[rid(9)])
    }

    @Test
    fun caseSummariesAndTitles() {
        val lib = Library(
            listOf(capture(1, caseId = fender), capture(2, caseId = fender, status = AnchoringStatus.QUEUED)),
            listOf(case(1, "Fender bender")),
        )
        val s = lib.cases.single()
        assertEquals(2, s.itemCount)
        assertEquals(1, s.anchoredCount)
        assertEquals(1, s.queuedCount)
        assertEquals("Fender bender", lib.caseTitles[fender])
        assertSame(s, lib.casesById[fender])
    }

    @Test
    fun anchoringSummaryCounts() {
        val lib = Library(
            listOf(capture(1), capture(2, status = AnchoringStatus.QUEUED), capture(3, status = AnchoringStatus.SEALED), capture(4, origin = CaptureOrigin.IMPORT)),
            emptyList(),
        )
        assertEquals(1, lib.anchoring.anchored)
        assertEquals(1, lib.anchoring.queued)
        assertEquals(1, lib.anchoring.sealedOnly)
        assertEquals(3, lib.anchoring.sealed)
    }

    @Test
    fun searchIndexIsBuiltOnceAndFindsLabels() {
        val lib = Library(listOf(capture(1, label = "Plate 7MNV562")), emptyList())
        assertSame(lib.index, lib.index)
        assertEquals(rid(1), (lib.index.search("7mnv").single() as SearchHit.Item).capture.recordId)
    }
}
