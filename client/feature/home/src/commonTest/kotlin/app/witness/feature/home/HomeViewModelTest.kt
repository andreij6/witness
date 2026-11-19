package app.witness.feature.home

import app.cash.turbine.test
import app.witness.core.designsystem.Tone
import app.witness.core.designsystem.WitnessState
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.CaptureOrigin
import app.witness.core.ui.StateCount
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class HomeViewModelTest : MainDispatcherTest() {
    private fun vm(w: World) = HomeViewModel(w.captures, w.cases, w.vault, w.time)

    @Test
    fun startsLoading() {
        val s = HomeUiState()
        assertTrue(s.loading)
        assertEquals("", s.trust.text)
    }

    @Test
    fun firstRunWithEnrolledKeyShowsReady() = runTest {
        val w = World()
        w.enrollment()()
        val s = vm(w).state.first { !it.loading }
        assertTrue(s.firstRun)
        assertEquals(HomeCopy.ready, s.trust)
        assertNull(s.featured)
    }

    @Test
    fun firstRunWithPendingKeyExplainsSetup() = runTest {
        val w = World()
        w.registry.offline = true
        w.enrollment()()
        assertEquals(HomeCopy.pending, vm(w).state.first { !it.loading }.trust)
    }

    @Test
    fun firstRunWithoutKey() = runTest {
        assertEquals(HomeCopy.missing, vm(World()).state.first { !it.loading }.trust)
    }

    @Test
    fun populatedHomeMatchesDesign() = runTest {
        val w = World()
        val fender = w.newCase("Fender bender")
        w.add(
            capture(1, createdAtMs = NOW, caption = "Golf-ball hail out of nowhere.", place = "Denver area"),
            capture(2, createdAtMs = NOW - 3 * DAY, label = "Double rainbow"),
            capture(3, createdAtMs = NOW - 4 * DAY, label = "Moose on Route 9", origin = CaptureOrigin.IMPORT),
            capture(4, caseId = fender.id),
            capture(5, caseId = fender.id, status = AnchoringStatus.QUEUED),
        )
        val s = vm(w).state.first { !it.loading }
        assertFalse(s.firstRun)
        assertEquals("4 sealed · 3 anchored · 1 queued", s.trust.text)
        assertEquals(Tone.Ok, s.trust.tone)
        assertEquals(FeedItem(rid(1), "Golf-ball hail out of nowhere.", "Today 4:12 PM · Denver area", WitnessState.Anchored), s.featured)
        assertEquals(
            listOf(
                FeedItem(rid(2), "Double rainbow", "Nov 16", WitnessState.Anchored),
                FeedItem(rid(3), "Moose on Route 9", IMPORTED_FROM_GALLERY, WitnessState.Unverified),
            ),
            s.tiles,
        )
        val c = s.cases.single()
        assertEquals("Fender bender", c.title)
        assertEquals("Today · 2 items", c.meta)
        assertEquals(listOf(StateCount(WitnessState.Anchored, 1), StateCount(WitnessState.Queued, 1)), c.counts)
        assertEquals(listOf(rid(4), rid(5)), c.covers)
    }

    @Test
    fun singlePostHasNoTiles() = runTest {
        val w = World()
        w.add(capture(1))
        val s = vm(w).state.first { !it.loading }
        assertEquals(rid(1), s.featured?.recordId)
        assertTrue(s.tiles.isEmpty())
    }

    @Test
    fun postsFilterHidesCases() = runTest {
        val w = World()
        val c = w.newCase("Leak")
        w.add(capture(1), capture(2, caseId = c.id))
        val vm = vm(w)
        vm.select(HomeFilter.Posts)
        val s = vm.state.first { !it.loading && it.filter == HomeFilter.Posts }
        assertTrue(s.showPosts)
        assertFalse(s.showCases)
        assertTrue(s.cases.isEmpty())
        assertEquals(rid(1), s.featured?.recordId)
    }

    @Test
    fun casesFilterHidesPosts() = runTest {
        val w = World()
        val c = w.newCase("Leak")
        w.add(capture(1), capture(2, caseId = c.id))
        val vm = vm(w)
        vm.select(HomeFilter.Cases)
        val s = vm.state.first { !it.loading && it.filter == HomeFilter.Cases }
        assertFalse(s.showPosts)
        assertNull(s.featured)
        assertEquals(1, s.cases.size)
    }

    @Test
    fun caseOnlyLibraryIsNotFirstRun() = runTest {
        val w = World()
        w.newCase("Empty case")
        val s = vm(w).state.first { !it.loading }
        assertFalse(s.firstRun)
        assertNull(s.featured)
        assertEquals("Today · 0 items", s.cases.single().meta)
        assertTrue(s.cases.single().counts.isEmpty())
    }

    @Test
    fun newCaptureUpdatesState() = runTest {
        val w = World()
        w.enrollment()()
        vm(w).state.test {
            var s = awaitItem()
            while (s.loading) s = awaitItem()
            assertTrue(s.firstRun)
            w.add(capture(7, caption = "New"))
            s = awaitItem()
            assertEquals("New", s.featured?.title)
            assertEquals("1 sealed · 1 anchored", s.trust.text)
            cancelAndIgnoreRemainingEvents()
        }
    }

    @Test
    fun refreshKeyPicksUpEnrollment() = runTest {
        val w = World()
        val vm = vm(w)
        assertEquals(HomeCopy.missing, vm.state.first { !it.loading }.trust)
        w.enrollment()()
        vm.refreshKey()
        assertEquals(HomeCopy.ready, vm.state.first { it.trust == HomeCopy.ready }.trust)
    }

    @Test
    fun derivativeReplacesOriginalInFeed() = runTest {
        val w = World()
        w.add(capture(1, createdAtMs = NOW - HOUR), capture(2, origin = CaptureOrigin.DERIVATIVE, parent = rid(1), caption = "Blurred"))
        val s = vm(w).state.first { !it.loading }
        assertEquals("Blurred", s.featured?.title)
        assertTrue(s.tiles.isEmpty())
    }

    @Test
    fun keyStateMapping() {
        assertEquals(KeyState.Missing, HomeUiState.keyState(null))
    }
}
