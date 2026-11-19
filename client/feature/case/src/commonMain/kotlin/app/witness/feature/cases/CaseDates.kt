package app.witness.feature.cases

fun interface UtcOffset {
    fun millisAt(epochMs: Long): Long

    companion object {
        val UTC: UtcOffset = UtcOffset { 0L }
    }
}

data class CivilDate(val year: Long, val month: Int, val day: Int)

object CaseDates {
    const val DAY_MS: Long = 86_400_000L
    private const val MINUTE_MS: Long = 60_000L
    private val MONTHS = arrayOf("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

    fun epochDay(ms: Long, offset: UtcOffset): Long = (ms + offset.millisAt(ms)).floorDiv(DAY_MS)

    fun civil(epochDay: Long): CivilDate {
        val z = epochDay + 719_468
        val era = (if (z >= 0) z else z - 146_096) / 146_097
        val doe = z - era * 146_097
        val yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365
        val doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
        val mp = (5 * doy + 2) / 153
        val day = (doy - (153 * mp + 2) / 5 + 1).toInt()
        val month = (if (mp < 10) mp + 3 else mp - 9).toInt()
        val year = yoe + era * 400 + if (month <= 2) 1 else 0
        return CivilDate(year, month, day)
    }

    fun day(ms: Long, nowMs: Long, offset: UtcOffset): String {
        val d = epochDay(ms, offset)
        val today = epochDay(nowMs, offset)
        return when (today - d) {
            0L -> "Today"
            1L -> "Yesterday"
            else -> monthDay(d, civil(today).year)
        }
    }

    fun monthDay(ms: Long, nowMs: Long, offset: UtcOffset): String =
        monthDay(epochDay(ms, offset), civil(epochDay(nowMs, offset)).year)

    fun time(ms: Long, offset: UtcOffset): String {
        val minutes = (ms + offset.millisAt(ms)).mod(DAY_MS) / MINUTE_MS
        val hour24 = (minutes / 60).toInt()
        val minute = (minutes % 60).toInt()
        val hour12 = if (hour24 % 12 == 0) 12 else hour24 % 12
        return buildString(8) {
            append(hour12).append(':')
            if (minute < 10) append('0')
            append(minute).append(if (hour24 < 12) " AM" else " PM")
        }
    }

    private fun monthDay(epochDay: Long, currentYear: Long): String {
        val c = civil(epochDay)
        val base = MONTHS[c.month - 1] + " " + c.day
        return if (c.year == currentYear) base else "$base, ${c.year}"
    }
}
