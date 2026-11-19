package app.witness.feature.post

import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.take
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalCoroutinesApi::class)
class PostSharedViewModelTest {
    private val main = UnconfinedTestDispatcher()

    @BeforeTest
    fun setUp() = Dispatchers.setMain(main)

    @AfterTest
    fun tearDown() = Dispatchers.resetMain()

    private suspend fun published(g: PostGraph, place: String? = "Denver area"): app.witness.core.model.Post {
        val p = g.parent(place = place)
        val draft = g.post.draft(p.recordId)
        return g.post.publish(draft)
    }

    @Test
    fun showsLinkAndPendingAnchor() = runTest(main) {
        val g = PostGraph()
        val post = published(g)
        val vm = PostSharedViewModel(post.recordId, post.precision, g.captures, g.ticks, g.links)
        backgroundScope.launch(main) { vm.state.collect {} }
        val s = vm.state.value
        assertEquals(post.proofUrl, s.url)
        assertEquals("witness.app/p/${post.recordId.text}", s.linkText)
        assertEquals("Sealed 4:12:09 PM · anchoring…", s.subtitle)
        assertEquals("Denver area", s.shows)
        assertEquals("0", s.checked)
        assertEquals("just now", s.shared)
    }

    @Test
    fun anchoredTimeAppears() = runTest(main) {
        val g = PostGraph()
        val post = published(g)
        val vm = PostSharedViewModel(post.recordId, post.precision, g.captures, g.ticks, g.links)
        backgroundScope.launch(main) { vm.state.collect {} }
        g.captures.setStatus(post.recordId, AnchoringStatus.ANCHORED, Anchor(9, g.clock.nowMs + 32_000))
        assertEquals("Sealed 4:12:09 PM · anchored 4:12:41 PM", vm.state.value.subtitle)
    }

    @Test
    fun sealedTimeComesFromParent() = runTest(main) {
        val g = PostGraph()
        val p = g.parent()
        g.clock.advance(60_000)
        val post = g.post.publish(g.post.draft(p.recordId))
        val vm = PostSharedViewModel(post.recordId, post.precision, g.captures, g.ticks, g.links)
        backgroundScope.launch(main) { vm.state.collect {} }
        assertEquals("Sealed 4:12:09 PM · anchoring…", vm.state.value.subtitle)
    }

    @Test
    fun sharedAgoTicks() = runTest(main) {
        val g = PostGraph()
        val post = published(g)
        val vm = PostSharedViewModel(post.recordId, post.precision, g.captures, g.ticks, g.links)
        backgroundScope.launch(main) { vm.state.collect {} }
        g.ticks.value = g.clock.nowMs + 5 * 60_000
        assertEquals("5 min ago", vm.state.value.shared)
    }

    @Test
    fun hiddenPrecisionShowsNoLocation() = runTest(main) {
        val g = PostGraph()
        val post = published(g)
        val vm = PostSharedViewModel(post.recordId, LocationPrecision.HIDDEN, g.captures, g.ticks, g.links)
        backgroundScope.launch(main) { vm.state.collect {} }
        assertEquals("No location", vm.state.value.shows)
    }

    @Test
    fun unknownRecordKeepsInitialState() = runTest(main) {
        val id = RecordId.fromBytes(ByteArray(10) { 3 })
        val vm = PostSharedViewModel(id, LocationPrecision.AREA, InMemoryCaptureStore(), flowOf(0L))
        backgroundScope.launch(main) { vm.state.collect {} }
        assertEquals("", vm.state.value.subtitle)
        assertEquals("City or area", vm.state.value.shows)
        assertEquals("https://witness.app/p/${id.text}", vm.state.first().url)
    }

    @Test
    fun ticksEmitClockReadings() = runTest {
        val clock = FakeClock(42)
        assertEquals(listOf(42L, 42L), PostSharedViewModel.ticks(clock, 10).take(2).toList())
    }
}
