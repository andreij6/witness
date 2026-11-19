package app.witness.core.domain

import kotlin.test.Test
import kotlin.test.assertEquals

class ClockTest {
    @Test
    fun fixedClockIsDeterministic() {
        val clock = FixedClock(1_700_000_000_000)
        assertEquals(clock.nowEpochMillis(), clock.nowEpochMillis())
        assertEquals(1_700_000_000_000, clock.nowEpochMillis())
    }

    @Test
    fun lambdaClockDelegates() {
        var ticks = 0L
        val clock = Clock { ++ticks }
        assertEquals(1, clock.nowEpochMillis())
        assertEquals(2, clock.nowEpochMillis())
    }
}
