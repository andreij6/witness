package app.witness.core.platform.capture

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class PixelBudgetTest {
    private data class S(val w: Int, val h: Int)

    private fun select(sizes: List<S>, max: Int) = PixelBudget.select(sizes, max, S::w, S::h)

    @Test
    fun dropsOversizedAndSortsLargestFirst() {
        val sizes = listOf(S(1920, 1080), S(8160, 6120), S(4032, 3024), S(4080, 3072), S(640, 480))
        assertEquals(listOf(S(4032, 3024), S(1920, 1080), S(640, 480)), select(sizes, 4032 * 3024))
    }

    @Test
    fun emptyWhenNothingFits() {
        assertTrue(select(listOf(S(8000, 6000)), 100).isEmpty())
    }

    @Test
    fun noOverflowOnHugeSizes() {
        assertTrue(select(listOf(S(Int.MAX_VALUE, Int.MAX_VALUE)), Int.MAX_VALUE).isEmpty())
    }

    @Test
    fun rotationValidation() {
        listOf(0, 90, 180, 270).forEach { assertTrue(PixelBudget.rotationIsValid(it)) }
        listOf(-90, 45, 360).forEach { assertFalse(PixelBudget.rotationIsValid(it)) }
    }
}
