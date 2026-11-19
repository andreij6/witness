package app.witness.feature.check

import app.witness.core.model.Verdict
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals

class RecentChecksTest {
    private fun check(i: Int) = RecentCheck("p$i", i.toLong(), Verdict.NO_RECORD, null)

    @Test
    fun newestFirstAndLimited() = runTest {
        val store = InMemoryRecentChecks()
        repeat(5) { store.record(check(it)) }
        assertEquals(listOf("p4", "p3", "p2"), store.observe(3).first().map { it.label })
    }

    @Test
    fun capacityDropsOldest() = runTest {
        val store = InMemoryRecentChecks(capacity = 2)
        repeat(4) { store.record(check(it)) }
        assertEquals(listOf("p3", "p2"), store.observe(10).first().map { it.label })
    }

    @Test
    fun emptyStoreObservesEmpty() = runTest {
        assertEquals(emptyList(), InMemoryRecentChecks().observe(3).first())
    }
}
