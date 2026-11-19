package app.witness.feature.check

class TimeText(private val offsetMs: (Long) -> Long = { 0L }) {
    class Civil(val year: Int, val month: Int, val day: Int, val hour: Int, val minute: Int, val second: Int)

    fun civil(epochMs: Long): Civil = civilUtc(epochMs + offsetMs(epochMs))

    fun date(epochMs: Long): String = civil(epochMs).let { "${MONTHS[it.month - 1]} ${it.day}" }

    fun time(epochMs: Long): String = clock(civil(epochMs), seconds = false)

    fun timeSeconds(epochMs: Long): String = clock(civil(epochMs), seconds = true)

    fun dateTime(epochMs: Long): String = civil(epochMs).let { "${MONTHS[it.month - 1]} ${it.day}, ${clock(it, false)}" }

    fun dateYear(epochMs: Long): String = civil(epochMs).let { "${MONTHS[it.month - 1]} ${it.day}, ${it.year}" }

    fun dateYearTime(epochMs: Long): String = civil(epochMs).let { "${MONTHS[it.month - 1]} ${it.day}, ${it.year} · ${clock(it, false)}" }

    fun relative(epochMs: Long, nowMs: Long): String {
        val delta = nowMs - epochMs
        return when {
            delta < MINUTE -> "Just now"
            delta < HOUR -> "${delta / MINUTE} min ago"
            delta < DAY && civil(epochMs).day == civil(nowMs).day -> "${delta / HOUR} h ago"
            isYesterday(epochMs, nowMs) -> "Yesterday"
            else -> date(epochMs)
        }
    }

    private fun isYesterday(epochMs: Long, nowMs: Long): Boolean {
        val a = dayNumber(epochMs + offsetMs(epochMs))
        val b = dayNumber(nowMs + offsetMs(nowMs))
        return b - a == 1L
    }

    companion object {
        const val MINUTE: Long = 60_000
        const val HOUR: Long = 60 * MINUTE
        const val DAY: Long = 24 * HOUR
        val UTC: TimeText = TimeText()
        private val MONTHS = arrayOf("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

        fun utc(epochMs: Long): String = civilUtc(epochMs).let {
            "${it.year}-${pad(it.month)}-${pad(it.day)} ${pad(it.hour)}:${pad(it.minute)}:${pad(it.second)} UTC"
        }

        private fun dayNumber(ms: Long): Long = ms.floorDiv(DAY)

        fun civilUtc(ms: Long): Civil {
            val days = dayNumber(ms)
            val secOfDay = ((ms - days * DAY) / 1000).toInt()
            val z = days + 719_468
            val era = z.floorDiv(146_097L)
            val doe = z - era * 146_097
            val yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365
            val doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
            val mp = (5 * doy + 2) / 153
            val d = (doy - (153 * mp + 2) / 5 + 1).toInt()
            val m = (if (mp < 10) mp + 3 else mp - 9).toInt()
            val y = (yoe + era * 400 + if (m <= 2) 1 else 0).toInt()
            return Civil(y, m, d, secOfDay / 3600, secOfDay / 60 % 60, secOfDay % 60)
        }

        private fun clock(c: Civil, seconds: Boolean): String {
            val h12 = if (c.hour % 12 == 0) 12 else c.hour % 12
            val suffix = if (c.hour < 12) "AM" else "PM"
            return if (seconds) "$h12:${pad(c.minute)}:${pad(c.second)} $suffix" else "$h12:${pad(c.minute)} $suffix"
        }

        private fun pad(v: Int): String = if (v < 10) "0$v" else v.toString()
    }
}
