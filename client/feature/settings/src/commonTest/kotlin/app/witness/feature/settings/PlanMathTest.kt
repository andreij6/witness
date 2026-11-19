package app.witness.feature.settings

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class PlanMathTest {
    @Test
    fun designPricesSaveThirtyPercent() = assertEquals(30, PlanMath.savingsPercent(2_990_000, 24_990_000))

    @Test
    fun noSavingsWhenYearlyCostsMore() {
        assertEquals(0, PlanMath.savingsPercent(1_000_000, 12_000_000))
        assertEquals(0, PlanMath.savingsPercent(1_000_000, 20_000_000))
    }

    @Test
    fun savingsGuardsInvalidPrices() {
        assertEquals(0, PlanMath.savingsPercent(0, 1))
        assertEquals(0, PlanMath.savingsPercent(-1, 1))
        assertEquals(0, PlanMath.savingsPercent(1, 0))
        assertEquals(0, PlanMath.savingsPercent(Long.MAX_VALUE, 1))
    }

    @Test
    fun savingsRoundsDown() = assertEquals(99, PlanMath.savingsPercent(1_000_000, 1))

    @Test
    fun monthlyEquivalentRoundsToCents() {
        assertEquals(2_080_000, PlanMath.monthlyEquivalentMicros(24_990_000))
        assertEquals(2_500_000, PlanMath.monthlyEquivalentMicros(30_000_000))
        assertEquals(10_000, PlanMath.monthlyEquivalentMicros(60_000))
        assertEquals(0, PlanMath.monthlyEquivalentMicros(0))
        assertEquals(0, PlanMath.monthlyEquivalentMicros(-5))
    }

    @Test
    fun rescaleKeepsDollarFormat() = assertEquals("$2.08", PlanMath.rescale("$24.99", 2_080_000))

    @Test
    fun rescaleKeepsCommaDecimalAndSuffix() = assertEquals("2,08 €", PlanMath.rescale("24,99 €", 2_080_000))

    @Test
    fun rescaleHandlesCurrencyCodePrefix() = assertEquals("US$ 2.08", PlanMath.rescale("US$ 24.99", 2_080_000))

    @Test
    fun rescaleZeroDecimalCurrency() {
        assertEquals("¥250", PlanMath.rescale("¥3,000", 250_000_000))
        assertEquals("¥250", PlanMath.rescale("¥3000", 249_600_000))
    }

    @Test
    fun rescaleGroupedTwoDecimalPrice() = assertEquals("₹208.25", PlanMath.rescale("₹2,499.00", 208_250_000))

    @Test
    fun rescalePadsCents() = assertEquals("$0.05", PlanMath.rescale("$1.00", 50_000))

    @Test
    fun rescaleRejectsNoDigitsOrNegative() {
        assertNull(PlanMath.rescale("free", 1))
        assertNull(PlanMath.rescale("", 1))
        assertNull(PlanMath.rescale("$1.00", -1))
    }

    @Test
    fun offerFallsBackToDefaults() {
        assertEquals(PlanOffer.MONTHLY, PlanMath.offer(emptyList(), PlusPlan.MONTHLY))
        assertEquals(PlanOffer.YEARLY, PlanMath.offer(emptyList(), PlusPlan.YEARLY))
        val custom = PlanOffer(PlusPlan.YEARLY, "€20", 20_000_000)
        assertEquals(custom, PlanMath.offer(listOf(PlanOffer.MONTHLY, custom), PlusPlan.YEARLY))
    }

    @Test
    fun defaultsMatchDesign() {
        assertEquals(listOf(PlusPlan.MONTHLY, PlusPlan.YEARLY), PlanOffer.DEFAULTS.map { it.plan })
        assertEquals("$2.99", PlanOffer.MONTHLY.price)
        assertEquals("$24.99", PlanOffer.YEARLY.price)
    }
}
