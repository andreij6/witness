package app.witness.feature.capture

import app.witness.core.domain.fake.FakeClock
import kotlinx.coroutines.flow.take
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals

class ClockTextTest {
    private val hour = 3_600_000L

    @Test
    fun epochIsMidnightAm() = assertEquals("12:00:00 AM", ClockText.format(0))

    @Test
    fun noonIsPm() = assertEquals("12:00:00 PM", ClockText.format(12 * hour))

    @Test
    fun afternoonWithSeconds() = assertEquals("4:12:09 PM", ClockText.format(16 * hour + 12 * 60_000 + 9_000))

    @Test
    fun morningSingleDigitHour() = assertEquals("9:05:03 AM", ClockText.format(9 * hour + 5 * 60_000 + 3_000))

    @Test
    fun twoDigitHours() {
        assertEquals("10:00:00 AM", ClockText.format(10 * hour))
        assertEquals("11:59:59 PM", ClockText.format(24 * hour - 1_000))
    }

    @Test
    fun oneAmAndOnePm() {
        assertEquals("1:00:00 AM", ClockText.format(hour))
        assertEquals("1:00:00 PM", ClockText.format(13 * hour))
    }

    @Test
    fun millisecondsAreTruncated() = assertEquals("12:00:00 AM", ClockText.format(999))

    @Test
    fun positiveOffsetShiftsForward() = assertEquals("2:00:00 AM", ClockText.format(0, 2 * hour))

    @Test
    fun negativeOffsetWrapsToPreviousDay() = assertEquals("5:00:00 PM", ClockText.format(0, -7 * hour))

    @Test
    fun negativeEpochWraps() = assertEquals("11:59:59 PM", ClockText.format(-1_000))

    @Test
    fun multiDayEpochUsesTimeOfDay() = assertEquals("3:30:00 AM", ClockText.format(400 * ClockText.DAY_MS + 3 * hour + 30 * 60_000))

    @Test
    fun extremeValuesDoNotThrow() {
        ClockText.format(Long.MAX_VALUE / 2)
        ClockText.format(Long.MIN_VALUE / 2)
    }

    @Test
    fun lengthIsTenOrEleven() {
        for (h in 0 until 24) {
            val s = ClockText.format(h * hour)
            assertEquals(if (h % 12 in 1..9) 10 else 11, s.length, s)
        }
    }

    @Test
    fun deterministic() = assertEquals(ClockText.format(1_763_510_400_123, 3_600_000), ClockText.format(1_763_510_400_123, 3_600_000))

    @Test
    fun everySecondEmitsClockReadings() = runTest {
        val clock = FakeClock(5_000)
        val values = ClockText.everySecond(clock).take(2).toList()
        assertEquals(listOf(5_000L, 5_000L), values)
    }
}
