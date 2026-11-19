package app.witness.feature.check

import kotlin.test.Test
import kotlin.test.assertEquals

class TimeTextTest {
    private val t = TimeText.UTC

    @Test
    fun epochZeroIsJan1st1970Midnight() {
        val c = TimeText.civilUtc(0)
        assertEquals(listOf(1970, 1, 1, 0, 0, 0), listOf(c.year, c.month, c.day, c.hour, c.minute, c.second))
        assertEquals("12:00 AM", t.time(0))
    }

    @Test
    fun knownInstantFormatsInEveryStyle() {
        val ms = Fixtures.NOV_18_2026_1412_UTC
        assertEquals("Nov 18", t.date(ms))
        assertEquals("2:12 PM", t.time(ms))
        assertEquals("2:12:00 PM", t.timeSeconds(ms))
        assertEquals("Nov 18, 2:12 PM", t.dateTime(ms))
        assertEquals("Nov 18, 2026", t.dateYear(ms))
        assertEquals("Nov 18, 2026 · 2:12 PM", t.dateYearTime(ms))
        assertEquals("2026-11-18 14:12:00 UTC", TimeText.utc(ms))
    }

    @Test
    fun leapDayAndCenturyRules() {
        assertEquals("Feb 29, 2024", t.dateYear(1_709_164_800_000))
        assertEquals("Mar 1, 2100", t.dateYear(4_107_542_400_000))
        assertEquals("Feb 29, 2000", t.dateYear(951_782_400_000))
    }

    @Test
    fun negativeEpochRollsBackToPreviousDay() {
        assertEquals("1969-12-31 23:59:59 UTC", TimeText.utc(-1_000))
        assertEquals("Dec 31, 1969", t.dateYear(-1))
    }

    @Test
    fun noonAndMidnightUseTwelve() {
        assertEquals("12:00 PM", t.time(12 * TimeText.HOUR))
        assertEquals("11:59 PM", t.time(TimeText.DAY - TimeText.MINUTE))
    }

    @Test
    fun offsetShiftsLocalTimeButNotUtc() {
        val denver = TimeText { -7 * TimeText.HOUR }
        val ms = Fixtures.NOV_18_2026_1412_UTC
        assertEquals("7:12 AM", denver.time(ms))
        assertEquals("2026-11-18 14:12:00 UTC", TimeText.utc(ms))
        assertEquals("Nov 17", TimeText { -15 * TimeText.HOUR }.date(ms))
    }

    @Test
    fun relativeBuckets() {
        val now = Fixtures.NOV_18_2026_1412_UTC
        assertEquals("Just now", t.relative(now - 5_000, now))
        assertEquals("2 min ago", t.relative(now - 2 * TimeText.MINUTE, now))
        assertEquals("3 h ago", t.relative(now - 3 * TimeText.HOUR, now))
        assertEquals("Yesterday", t.relative(now - 20 * TimeText.HOUR, now))
        assertEquals("Nov 15", t.relative(now - 3 * TimeText.DAY, now))
        assertEquals("Just now", t.relative(now + 10_000, now))
    }

    @Test
    fun formattingIsDeterministic() {
        val ms = 1_234_567_890_123
        assertEquals(t.dateYearTime(ms), TimeText.UTC.dateYearTime(ms))
        assertEquals("2009-02-13 23:31:30 UTC", TimeText.utc(ms))
    }
}
