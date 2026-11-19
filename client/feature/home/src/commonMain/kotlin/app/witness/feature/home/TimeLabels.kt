package app.witness.feature.home

import app.witness.core.domain.Clock

data class CivilTime(val year: Int, val month: Int, val day: Int, val minuteOfDay: Int)

object TimeText {
    const val MINUTE_MS: Long = 60_000
    const val HOUR_MS: Long = 60 * MINUTE_MS
    const val DAY_MS: Long = 24 * HOUR_MS
    private val months = arrayOf("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

    fun epochDay(epochMs: Long, offsetMs: Int): Long = (epochMs + offsetMs).floorDiv(DAY_MS)

    fun civil(epochMs: Long, offsetMs: Int): CivilTime {
        val local = epochMs + offsetMs
        val days = local.floorDiv(DAY_MS)
        val minute = (local.mod(DAY_MS) / MINUTE_MS).toInt()
        val z = days + 719_468
        val era = z.floorDiv(146_097L)
        val doe = z - era * 146_097
        val yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365
        val doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
        val mp = (5 * doy + 2) / 153
        val day = (doy - (153 * mp + 2) / 5 + 1).toInt()
        val month = (if (mp < 10) mp + 3 else mp - 9).toInt()
        val year = (yoe + era * 400 + if (month <= 2) 1 else 0).toInt()
        return CivilTime(year, month, day, minute)
    }

    fun clock(minuteOfDay: Int): String {
        val h = minuteOfDay / 60
        val m = minuteOfDay % 60
        val h12 = if (h % 12 == 0) 12 else h % 12
        return "$h12:${if (m < 10) "0" else ""}$m ${if (h < 12) "AM" else "PM"}"
    }

    fun date(t: CivilTime, withYear: Boolean): String =
        if (withYear) "${months[t.month - 1]} ${t.day}, ${t.year}" else "${months[t.month - 1]} ${t.day}"

    fun day(atMs: Long, nowMs: Long, offset: (Long) -> Int): String {
        val at = offset(atMs)
        return when (epochDay(nowMs, offset(nowMs)) - epochDay(atMs, at)) {
            0L -> "Today"
            1L -> "Yesterday"
            else -> calendar(atMs, at, nowMs, offset)
        }
    }

    fun moment(atMs: Long, nowMs: Long, offset: (Long) -> Int): String {
        val at = offset(atMs)
        return when (epochDay(nowMs, offset(nowMs)) - epochDay(atMs, at)) {
            0L -> "Today ${clock(civil(atMs, at).minuteOfDay)}"
            1L -> "Yesterday ${clock(civil(atMs, at).minuteOfDay)}"
            else -> calendar(atMs, at, nowMs, offset)
        }
    }

    fun ago(atMs: Long, nowMs: Long, offset: (Long) -> Int): String {
        val delta = nowMs - atMs
        return when {
            delta < 0 -> moment(atMs, nowMs, offset)
            delta < MINUTE_MS -> "just now"
            delta < HOUR_MS -> "${delta / MINUTE_MS} min ago"
            delta < DAY_MS -> "${delta / HOUR_MS} h ago"
            else -> day(atMs, nowMs, offset)
        }
    }

    private fun calendar(atMs: Long, at: Int, nowMs: Long, offset: (Long) -> Int): String {
        val t = civil(atMs, at)
        return date(t, t.year != civil(nowMs, offset(nowMs)).year)
    }
}

class TimeLabels(private val clock: Clock, private val offset: (Long) -> Int) {
    fun now(): Long = clock.nowEpochMillis()

    fun day(atMs: Long): String = TimeText.day(atMs, now(), offset)

    fun moment(atMs: Long): String = TimeText.moment(atMs, now(), offset)

    fun ago(atMs: Long): String = TimeText.ago(atMs, now(), offset)
}
