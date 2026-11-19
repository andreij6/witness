package app.witness.feature.cases

import kotlin.test.Test
import kotlin.test.assertEquals

class CaseDatesTest {
    private val utc = UtcOffset.UTC
    private val day = CaseDates.DAY_MS

    @Test
    fun civilOfEpochIsJanuaryFirst1970() = assertEquals(CivilDate(1970, 1, 1), CaseDates.civil(0))

    @Test
    fun civilHandlesLeapDay() = assertEquals(CivilDate(2000, 2, 29), CaseDates.civil(11_016))

    @Test
    fun civilHandlesDayBeforeEpoch() = assertEquals(CivilDate(1969, 12, 31), CaseDates.civil(-1))

    @Test
    fun civilHandlesCenturyNonLeapYear() = assertEquals(CivilDate(2100, 3, 1), CaseDates.civil(47_541))

    @Test
    fun civilRoundTripsEveryDayAcrossFourHundredYearsInOrder() {
        var previous = CaseDates.civil(-1)
        for (d in 0L until 146_097L) {
            val c = CaseDates.civil(d)
            val advancedDay = c.day == previous.day + 1 && c.month == previous.month && c.year == previous.year
            val advancedMonth = c.day == 1 && (c.month == previous.month + 1 && c.year == previous.year || c.month == 1 && previous.month == 12 && c.year == previous.year + 1)
            kotlin.test.assertTrue(advancedDay || advancedMonth, "day $d: $previous -> $c")
            previous = c
        }
        assertEquals(CivilDate(2370, 1, 1), CaseDates.civil(146_097))
    }

    @Test
    fun epochDayFloorsNegativeTimes() {
        assertEquals(-1, CaseDates.epochDay(-1, utc))
        assertEquals(0, CaseDates.epochDay(day - 1, utc))
    }

    @Test
    fun epochDayAppliesOffset() {
        assertEquals(1, CaseDates.epochDay(day - 1, UtcOffset { 1 }))
        assertEquals(-1, CaseDates.epochDay(0, UtcOffset { -1 }))
    }

    @Test
    fun dayIsTodayForSameCalendarDay() = assertEquals("Today", CaseDates.day(NOW - HOUR_MS, NOW, utc))

    @Test
    fun dayIsYesterdayForPreviousCalendarDay() = assertEquals("Yesterday", CaseDates.day(NOW - day, NOW, utc))

    @Test
    fun dayIsMonthAndDayWithinYear() = assertEquals("Nov 12", CaseDates.day(NOW - 7 * day, NOW, utc))

    @Test
    fun dayIncludesYearWhenDifferent() = assertEquals("Nov 19, 2024", CaseDates.day(NOW - 365 * day, NOW, utc))

    @Test
    fun dayForFutureTimesUsesMonthDay() = assertEquals("Nov 25", CaseDates.day(NOW + 6 * day, NOW, utc))

    @Test
    fun dayRespectsOffsetAcrossMidnight() {
        val lateEvening = 1_763_510_400_000L - HOUR_MS
        assertEquals("Today", CaseDates.day(lateEvening, 1_763_510_400_000L + HOUR_MS, UtcOffset { -2 * HOUR_MS }))
    }

    @Test
    fun monthDayFormatsExpiry() = assertEquals("Nov 26", CaseDates.monthDay(NOW + 7 * day, NOW, utc))

    @Test
    fun timeFormatsAfternoon() = assertEquals("2:32 PM", CaseDates.time(NOW, utc))

    @Test
    fun timeFormatsMidnightAsTwelveAm() = assertEquals("12:00 AM", CaseDates.time(0, utc))

    @Test
    fun timeFormatsNoonAsTwelvePm() = assertEquals("12:00 PM", CaseDates.time(12 * HOUR_MS, utc))

    @Test
    fun timePadsMinutes() = assertEquals("9:05 AM", CaseDates.time(9 * HOUR_MS + 5 * 60_000, utc))

    @Test
    fun timeHandlesNegativeEpoch() = assertEquals("11:59 PM", CaseDates.time(-60_000, utc))

    @Test
    fun timeAppliesOffset() = assertEquals("5:00 AM", CaseDates.time(0, UtcOffset { 5 * HOUR_MS }))

    @Test
    fun formattingIsDeterministic() = assertEquals(CaseDates.day(NOW, NOW, utc) + CaseDates.time(NOW, utc), CaseDates.day(NOW, NOW, utc) + CaseDates.time(NOW, utc))
}
