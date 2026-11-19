package app.witness.core.domain

fun interface Clock {
    fun nowEpochMillis(): Long
}

class FixedClock(private val epochMillis: Long) : Clock {
    override fun nowEpochMillis(): Long = epochMillis
}
