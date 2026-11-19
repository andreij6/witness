package app.witness.feature.home

import app.witness.core.designsystem.WitnessState
import app.witness.core.model.CaptureOrigin
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class SearchViewModelTest : MainDispatcherTest() {
    private suspend fun designWorld(): World {
        val w = World()
        val fender = w.newCase("Fender bender", summary = "Oak St & 5th Ave · plate 7MNV562")
        w.add(
            capture(1, createdAtMs = MIDNIGHT + 14 * HOUR + 9 * MIN - 2, caseId = fender.id, label = "Crash"),
            capture(2, createdAtMs = MIDNIGHT + 14 * HOUR + 9 * MIN - 1, caseId = fender.id, label = "Rear damage", caption = "hit by 7MNV562, silver sedan"),
            capture(3, createdAtMs = MIDNIGHT + 14 * HOUR + 9 * MIN, caseId = fender.id, label = "Plate 7MNV562"),
            capture(4, createdAtMs = NOW - DAY, caption = "Golf-ball hail", place = "Denver area"),
        )
        return w
    }

    private suspend fun SearchViewModel.settle(predicate: (SearchUiState) -> Boolean = { true }) = state.first { it.searched && predicate(it) }

    @Test
    fun emptyQueryShowsNothing() = runTest {
        val vm = SearchViewModel(designWorld().captures, designWorld().cases, utcTime())
        val s = vm.state.first()
        assertFalse(s.searched)
        assertFalse(s.noResults)
        assertTrue(s.items.isEmpty())
    }

    @Test
    fun plateQueryMatchesDesign() = runTest {
        val w = designWorld()
        val vm = SearchViewModel(w.captures, w.cases, w.time)
        vm.onQuery("7MNV")
        val s = vm.settle()
        assertEquals(listOf("7mnv"), s.terms)
        assertEquals(
            listOf(
                SearchItemRow(rid(3), "Plate 7MNV562 · Photo 3", "Fender bender · Today 2:09 PM", WitnessState.Anchored),
                SearchItemRow(rid(2), "Rear damage · Photo 2", "Note: hit by 7MNV562, silver sedan", WitnessState.Anchored),
            ),
            s.items,
        )
        val c = s.cases.single()
        assertEquals("Fender bender", c.title)
        assertEquals("Oak St & 5th Ave · plate 7MNV562", c.subtitle)
        assertEquals(listOf(rid(3), rid(2), rid(1)), c.covers)
    }

    @Test
    fun queryIsExposedImmediately() {
        val w = World()
        val vm = SearchViewModel(w.captures, w.cases, w.time)
        vm.onQuery("abc")
        assertEquals("abc", vm.query.value)
    }

    @Test
    fun queryIsBounded() {
        val w = World()
        val vm = SearchViewModel(w.captures, w.cases, w.time)
        vm.onQuery("x".repeat(10_000))
        assertEquals(SearchViewModel.MAX_QUERY, vm.query.value.length)
    }

    @Test
    fun noMatchesReportsNoResults() = runTest {
        val w = designWorld()
        val vm = SearchViewModel(w.captures, w.cases, w.time)
        vm.onQuery("zebra")
        assertTrue(vm.settle().noResults)
        assertEquals("No matches for “zebra”", SearchCopy.noResults(" zebra "))
    }

    @Test
    fun casesFilterDropsItems() = runTest {
        val w = designWorld()
        val vm = SearchViewModel(w.captures, w.cases, w.time)
        vm.onQuery("7mnv")
        vm.select(SearchFilter.Cases)
        val s = vm.settle { it.filter == SearchFilter.Cases }
        assertTrue(s.items.isEmpty())
        assertEquals(1, s.cases.size)
    }

    @Test
    fun itemsFilterKeepsCaseMembersOnly() = runTest {
        val w = designWorld()
        val vm = SearchViewModel(w.captures, w.cases, w.time)
        vm.onQuery("fender")
        vm.select(SearchFilter.Items)
        val s = vm.settle { it.filter == SearchFilter.Items }
        assertTrue(s.cases.isEmpty())
        assertEquals(setOf(rid(1), rid(2), rid(3)), s.items.map { it.recordId }.toSet())
    }

    @Test
    fun postsFilterKeepsLooseCaptures() = runTest {
        val w = designWorld()
        val vm = SearchViewModel(w.captures, w.cases, w.time)
        vm.onQuery("denver")
        vm.select(SearchFilter.Posts)
        val s = vm.settle { it.filter == SearchFilter.Posts }
        val row = s.items.single()
        assertEquals("Golf-ball hail", row.title)
        assertEquals("Denver area · Yesterday 4:12 PM", row.subtitle)
    }

    @Test
    fun caseWithoutSummaryMatchShowsMeta() = runTest {
        val w = designWorld()
        val vm = SearchViewModel(w.captures, w.cases, w.time)
        vm.onQuery("fender")
        assertEquals("Today · 3 items", vm.settle().cases.single().subtitle)
    }

    @Test
    fun clearingQueryResets() = runTest {
        val w = designWorld()
        val vm = SearchViewModel(w.captures, w.cases, w.time)
        vm.onQuery("7mnv")
        vm.settle()
        vm.onQuery("")
        assertFalse(vm.state.first { !it.searched }.searched)
    }

    @Test
    fun acceptsTable() {
        val loose = capture(1)
        val member = capture(2, caseId = app.witness.core.model.LocalCaseId(1))
        assertTrue(SearchUiState.accepts(SearchFilter.All, loose))
        assertTrue(SearchUiState.accepts(SearchFilter.Posts, loose))
        assertFalse(SearchUiState.accepts(SearchFilter.Posts, member))
        assertTrue(SearchUiState.accepts(SearchFilter.Items, member))
        assertFalse(SearchUiState.accepts(SearchFilter.Items, loose))
        assertFalse(SearchUiState.accepts(SearchFilter.Cases, member))
    }

    @Test
    fun altTextMatchBecomesNote() {
        val lib = Library(listOf(capture(1, label = "Porch", altText = "Hailstones on a wooden railing")), emptyList())
        val row = SearchUiState.itemRow(lib.captures.single(), lib, Highlight.terms("railing"), utcTime())
        assertEquals("Note: Hailstones on a wooden railing", row.subtitle)
    }

    @Test
    fun importsShowUnverified() {
        val lib = Library(listOf(capture(1, origin = CaptureOrigin.IMPORT, label = "Moose")), emptyList())
        assertEquals(WitnessState.Unverified, SearchUiState.of(lib, "moose", SearchFilter.All, utcTime()).items.single().state)
    }

    @Test
    fun largeLibrarySearchIsBoundedByIndexLimit() {
        val lib = Library(List(2_000) { capture(it + 1, label = "Shot $it") }, emptyList())
        assertEquals(50, SearchUiState.of(lib, "shot", SearchFilter.All, utcTime()).items.size)
    }
}
