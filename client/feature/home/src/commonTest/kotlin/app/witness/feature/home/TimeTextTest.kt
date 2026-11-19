package app.witness.feature.home

import app.witness.core.domain.fake.FakeClock
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class TimeTextTest {
    private val utc: (Long) -> Int = { 0 }

    @Test
    fun civilEpochIsJanuaryFirst1970() = assertEquals(CivilTime(1970, 1, 1, 0), TimeText.civil(0, 0))

    @Test
    fun civilFixtureMidnightIsNovember19() = assertEquals(CivilTime(2025, 11, 19, 0), TimeText.civil(MIDNIGHT, 0))

    @Test
    fun civilHandlesLeapDay() = assertEquals(CivilTime(2024, 2, 29, 12 * 60), TimeText.civil(1_709_208_000_000, 0))

    @Test
    fun civilHandlesCenturyNonLeapYear() = assertEquals(CivilTime(2100, 3, 1, 0), TimeText.civil(4_107_542_400_000, 0))

    @Test
    fun civilHandlesNegativeEpoch() = assertEquals(CivilTime(1969, 12, 31, 23 * 60 + 59), TimeText.civil(-60_000, 0))

    @Test
    fun civilAppliesPositiveOffsetAcrossMidnight() = assertEquals(CivilTime(2025, 11, 19, 60), TimeText.civil(MIDNIGHT - HOUR, (2 * HOUR).toInt()))

    @Test
    fun civilAppliesNegativeOffsetAcrossMidnight() = assertEquals(CivilTime(2025, 11, 18, 23 * 60), TimeText.civil(MIDNIGHT, (-HOUR).toInt()))

    @Test
    fun clockFormatsMidnightNoonAndPadding() {
        assertEquals("12:00 AM", TimeText.clock(0))
        assertEquals("12:00 PM", TimeText.clock(12 * 60))
        assertEquals("4:12 PM", TimeText.clock(16 * 60 + 12))
        assertEquals("9:05 AM", TimeText.clock(9 * 60 + 5))
        assertEquals("11:59 PM", TimeText.clock(24 * 60 - 1))
    }

    @Test
    fun momentTodayShowsClock() = assertEquals("Today 4:12 PM", TimeText.moment(NOW, NOW, utc))

    @Test
    fun momentYesterdayShowsClock() = assertEquals("Yesterday 9:00 AM", TimeText.moment(MIDNIGHT - 15 * HOUR, NOW, utc))

    @Test
    fun momentOlderShowsDateWithoutYear() = assertEquals("Nov 16", TimeText.moment(MIDNIGHT - 3 * DAY, NOW, utc))

    @Test
    fun momentPreviousYearShowsYear() = assertEquals("Dec 31, 2024", TimeText.moment(1_735_646_400_000, NOW, utc))

    @Test
    fun momentFutureUsesDate() = assertEquals("Nov 21", TimeText.moment(NOW + 2 * DAY, NOW, utc))

    @Test
    fun dayLabels() {
        assertEquals("Today", TimeText.day(MIDNIGHT, NOW, utc))
        assertEquals("Yesterday", TimeText.day(MIDNIGHT - 1, NOW, utc))
        assertEquals("Nov 12", TimeText.day(MIDNIGHT - 7 * DAY, NOW, utc))
    }

    @Test
    fun dayRespectsLocalOffset() {
        val newYork: (Long) -> Int = { (-5 * HOUR).toInt() }
        assertEquals("Today", TimeText.day(MIDNIGHT - HOUR, MIDNIGHT + HOUR, newYork))
        assertEquals("Yesterday", TimeText.day(MIDNIGHT - HOUR, MIDNIGHT + 6 * HOUR, newYork))
    }

    @Test
    fun agoBuckets() {
        assertEquals("just now", TimeText.ago(NOW - 59_000, NOW, utc))
        assertEquals("1 min ago", TimeText.ago(NOW - MIN, NOW, utc))
        assertEquals("2 min ago", TimeText.ago(NOW - 2 * MIN - 30_000, NOW, utc))
        assertEquals("59 min ago", TimeText.ago(NOW - HOUR + 1, NOW, utc))
        assertEquals("3 h ago", TimeText.ago(NOW - 3 * HOUR, NOW, utc))
        assertEquals("Nov 16", TimeText.ago(NOW - 3 * DAY, NOW, utc))
    }

    @Test
    fun agoForFutureFallsBackToMoment() = assertEquals("Today 4:13 PM", TimeText.ago(NOW + MIN, NOW, utc))

    @Test
    fun epochDayFloorsNegativeTimes() {
        assertEquals(-1, TimeText.epochDay(-1, 0))
        assertEquals(0, TimeText.epochDay(0, 0))
    }

    @Test
    fun labelsReadInjectedClock() {
        val clock = FakeClock(NOW)
        val labels = utcTime(clock)
        assertEquals("Today", labels.day(NOW))
        clock.advance(DAY)
        assertEquals("Yesterday", labels.day(NOW))
        assertEquals("Yesterday", labels.ago(NOW))
        assertEquals("Yesterday 4:12 PM", labels.moment(NOW))
    }

    @Test
    fun allMonthsHaveAbbreviations() {
        val names = (1..12).map { m -> TimeText.date(CivilTime(2025, m, 1, 0), withYear = false) }
        assertEquals(listOf("Jan 1", "Feb 1", "Mar 1", "Apr 1", "May 1", "Jun 1", "Jul 1", "Aug 1", "Sep 1", "Oct 1", "Nov 1", "Dec 1"), names)
    }

    @Test
    fun civilIsDeterministicAcrossManyDays() {
        var prev = TimeText.civil(MIDNIGHT, 0)
        for (d in 1..800) {
            val next = TimeText.civil(MIDNIGHT + d * DAY, 0)
            val advanced = next.day == prev.day + 1 || (next.day == 1 && (next.month == prev.month % 12 + 1))
            assertTrue(advanced, "day $d: $prev -> $next")
            prev = next
        }
    }
}
