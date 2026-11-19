package app.witness.core.ui

object Formatters {
    const val Dot = " · "

    fun groupThousands(n: Long): String {
        if (n == Long.MIN_VALUE) return "-9,223,372,036,854,775,808"
        if (n < 0) return "-" + groupThousands(-n)
        val digits = n.toString()
        val firstGroup = digits.length % 3
        return buildString(digits.length + digits.length / 3) {
            digits.forEachIndexed { i, c ->
                if (i > 0 && (i - firstGroup) % 3 == 0) append(',')
                append(c)
            }
        }
    }

    fun groupThousands(n: Int): String = groupThousands(n.toLong())

    fun plural(count: Long, singular: String, plural: String = singular + "s"): String =
        "${groupThousands(count)} ${if (count == 1L) singular else plural}"

    fun plural(count: Int, singular: String, plural: String = singular + "s"): String = plural(count.toLong(), singular, plural)

    fun joinDot(vararg parts: String?): String = joinDot(parts.asList())

    fun joinDot(parts: List<String?>): String = parts.filterNot { it.isNullOrBlank() }.joinToString(Dot)

    fun checkedBadge(count: Long): String = "Checked ${groupThousands(count)}×"

    fun checkedPhrase(count: Long): String = when (count) {
        0L -> "not checked yet"
        1L -> "checked once"
        else -> "checked ${groupThousands(count)} times"
    }

    fun anchoringSummary(anchored: Int, queued: Int, sealed: Int? = null): String = joinDot(
        sealed?.let { "${groupThousands(it)} sealed" },
        "${groupThousands(anchored)} anchored",
        if (queued > 0) "${groupThousands(queued)} queued" else null,
    )

    fun verifiedSummary(verified: Int, total: Int): String = "${groupThousands(verified)} of ${groupThousands(total)} verified"

    fun regionsMatch(matched: Int, total: Int): String = "${groupThousands(matched)} of ${groupThousands(total)} regions match"

    fun declaredEdits(count: Int, noun: String, nounPlural: String = noun + "s"): String =
        "${groupThousands(count)} declared ${if (count == 1) noun else nounPlural}"

    fun dimensions(width: Int, height: Int): String = "${width} × ${height}"

    fun accuracy(meters: Int): String = "±${meters} m"

    fun abbreviateMiddle(text: String, head: Int = 4, tail: Int = 4): String {
        require(head >= 0 && tail >= 0) { "head and tail must be non-negative" }
        return if (text.length <= head + tail + 1) text else text.take(head) + "…" + text.takeLast(tail)
    }

    fun shortHash(algorithm: String, hex: String): String = "$algorithm ${abbreviateMiddle(hex)}"

    private val byteUnits = arrayOf("B", "KB", "MB", "GB", "TB")

    fun bytes(count: Long): String {
        require(count >= 0) { "byte count must be non-negative: $count" }
        if (count < 1000) return "$count B"
        var index = 1
        var unit = 1000L
        while (index < byteUnits.lastIndex && count / unit >= 1000) {
            unit *= 1000
            index++
        }
        var tenths = roundedTenths(count, unit)
        if ((tenths + 5) / 10 >= 1000 && index < byteUnits.lastIndex) {
            unit *= 1000
            index++
            tenths = roundedTenths(count, unit)
        }
        val text = if (tenths < 100) {
            if (tenths % 10 == 0L) "${tenths / 10}" else "${tenths / 10}.${tenths % 10}"
        } else {
            "${(tenths + 5) / 10}"
        }
        return "$text ${byteUnits[index]}"
    }

    private fun roundedTenths(count: Long, unit: Long): Long {
        val tenth = unit / 10
        return count / tenth + if (count % tenth >= tenth / 2) 1 else 0
    }
}
