package app.witness.feature.settings

enum class PlusPlan { MONTHLY, YEARLY }

data class PlanOffer(val plan: PlusPlan, val price: String, val priceMicros: Long) {
    companion object {
        val MONTHLY = PlanOffer(PlusPlan.MONTHLY, "$2.99", 2_990_000)
        val YEARLY = PlanOffer(PlusPlan.YEARLY, "$24.99", 24_990_000)
        val DEFAULTS: List<PlanOffer> = listOf(MONTHLY, YEARLY)
    }
}

sealed interface PurchaseOutcome {
    data object Purchased : PurchaseOutcome

    data object Cancelled : PurchaseOutcome

    data object NothingToRestore : PurchaseOutcome

    data class Failed(val reason: String) : PurchaseOutcome
}

interface PlusBilling {
    suspend fun offers(): List<PlanOffer>

    suspend fun purchase(plan: PlusPlan, account: Principal): PurchaseOutcome

    suspend fun restore(account: Principal?): PurchaseOutcome
}

object PlanMath {
    private const val MONTHS = 12L
    private const val MICROS_PER_CENT = 10_000L

    fun savingsPercent(monthlyMicros: Long, yearlyMicros: Long): Int {
        if (monthlyMicros <= 0 || yearlyMicros <= 0 || monthlyMicros > Long.MAX_VALUE / (MONTHS * 100)) return 0
        val full = monthlyMicros * MONTHS
        return if (yearlyMicros >= full) 0 else ((full - yearlyMicros) * 100 / full).toInt()
    }

    fun monthlyEquivalentMicros(yearlyMicros: Long): Long =
        if (yearlyMicros <= 0) 0 else (yearlyMicros / MONTHS + MICROS_PER_CENT / 2) / MICROS_PER_CENT * MICROS_PER_CENT

    fun rescale(formatted: String, micros: Long): String? {
        val first = formatted.indexOfFirst(Char::isDigit)
        if (first < 0 || micros < 0) return null
        val last = formatted.indexOfLast(Char::isDigit)
        val number = formatted.substring(first, last + 1)
        val separator = number.getOrNull(number.length - 3)?.takeIf { it == '.' || it == ',' }
        val cents = (micros + MICROS_PER_CENT / 2) / MICROS_PER_CENT
        val amount = if (separator == null) ((cents + 50) / 100).toString() else "${cents / 100}$separator${(cents % 100).toString().padStart(2, '0')}"
        return formatted.substring(0, first) + amount + formatted.substring(last + 1)
    }

    fun offer(offers: List<PlanOffer>, plan: PlusPlan): PlanOffer =
        offers.firstOrNull { it.plan == plan } ?: if (plan == PlusPlan.MONTHLY) PlanOffer.MONTHLY else PlanOffer.YEARLY
}
