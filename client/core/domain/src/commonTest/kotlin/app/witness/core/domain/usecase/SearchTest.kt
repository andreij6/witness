package app.witness.core.domain.usecase

import app.cash.turbine.test
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Case
import app.witness.core.model.Dims
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationConfidence
import app.witness.core.model.RecordId
import app.witness.core.model.TemplateId
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

class SearchTest {
    private fun cap(n: Int, created: Long, caption: String? = null, place: String? = null, case: LocalCaseId? = null, status: AnchoringStatus = AnchoringStatus.ANCHORED) =
        Capture(RecordId.fromBytes(ByteArray(10) { n.toByte() }), null, CaptureOrigin.ORIGINAL, status, created, null, LocationConfidence.HIGH, Dims(1, 1), caseId = case, caption = caption, place = place)

    private val crash = Case(LocalCaseId(1), "Fender bender", TemplateId.CAR_ACCIDENT, 0, 50, summary = "Two cars at Oak St")
    private val rental = Case(LocalCaseId(2), "Move-out walkthrough", TemplateId.RENTAL_CONDITION, 0, 10)
    private val items = listOf(
        cap(1, 100, caption = "Hail on Main Street", place = "Denver area"),
        cap(2, 300, caption = "Bumper damage", case = LocalCaseId(1)),
        cap(3, 200, caption = "Kitchen sink", place = "Denver area", case = LocalCaseId(2), status = AnchoringStatus.QUEUED),
    )
    private val index = SearchIndex.build(items, listOf(rental, crash))

    private fun ids(hits: List<SearchHit>) = hits.map {
        when (it) {
            is SearchHit.CaseHit -> "case:${it.case.id.value}"
            is SearchHit.Item -> "item:${it.capture.recordId.toBytes()[0]}"
        }
    }

    @Test
    fun prefixMatchingIsCaseInsensitive() {
        assertEquals(listOf("item:1"), ids(index.search("hai")))
        assertEquals(listOf("item:1"), ids(index.search("HAIL")))
        assertEquals(listOf("item:3", "item:1"), ids(index.search("denver")))
    }

    @Test
    fun allQueryTokensMustMatch() {
        assertEquals(listOf("item:3"), ids(index.search("denver kitchen")))
        assertEquals(emptyList(), ids(index.search("denver bumper")))
        assertEquals(emptyList(), ids(index.search("zzz")))
    }

    @Test
    fun casesRankBeforeItemsThenRecency() {
        assertEquals(listOf("case:1", "item:2"), ids(index.search("fender")))
        assertEquals(listOf("case:2", "item:3"), ids(index.search("walkthrough")))
        assertEquals(listOf("case:1"), ids(index.search("oak")))
        assertEquals(listOf("case:1"), ids(index.search("car accident")))
    }

    @Test
    fun matchesStatusAndRecordId() {
        assertEquals(listOf("item:3"), ids(index.search("queued")))
        val text = items[0].recordId.text
        assertEquals(listOf("item:1"), ids(index.search(text.take(6))))
    }

    @Test
    fun emptyQueriesAndLimits() {
        assertEquals(emptyList(), index.search(""))
        assertEquals(emptyList(), index.search("  ,, "))
        assertEquals(1, index.search("denver", limit = 1).size)
        assertEquals(emptyList(), index.search("denver", limit = 0))
        assertEquals(emptyList(), SearchIndex.EMPTY.search("x"))
        assertEquals(5, index.size)
    }

    @Test
    fun largeIndexScalesAndCrossesBitsetWords() {
        val many = (0 until 1000).map { cap(it % 256, it.toLong(), caption = if (it % 7 == 0) "seven $it" else "other $it") }
        val idx = SearchIndex.build(many, emptyList())
        val hits = idx.search("seven", limit = 1000)
        assertEquals(143, hits.size)
        val created = hits.map { (it as SearchHit.Item).capture.createdAtMs }
        assertEquals(created.sortedDescending(), created)
        assertEquals(1, idx.search("seven 994").size)
    }

    @Test
    fun searchFlowReactsToQueryAndData() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val query = MutableStateFlow("")
        Search(g.captures, g.cases).results(query).test {
            assertEquals(emptyList(), awaitItem())
            g.manageCases.create("Fender bender")
            assertEquals(emptyList(), expectMostRecentItem())
            query.value = "fend"
            val hits = awaitItem()
            assertIs<SearchHit.CaseHit>(hits.single())
            val c = g.sealOne()
            val withItem = expectMostRecentItem()
            assertEquals(2, withItem.size)
            assertTrue(withItem.any { it is SearchHit.Item && it.capture.recordId == c.recordId })
            cancelAndIgnoreRemainingEvents()
        }
    }
}
