package app.witness.feature.share

import app.witness.core.model.ShareExpiry
import app.witness.core.model.SummarySource
import app.witness.core.ui.Formatters

fun interface UtcOffset {
    fun millisAt(epochMs: Long): Long

    companion object {
        val UTC: UtcOffset = UtcOffset { 0L }
    }
}

object ShareText {
    const val DAY_MS: Long = 86_400_000L
    const val KEY_MASK: String = "••••"
    const val INCLUDE_SUMMARY: String = "Include summary"
    const val INCLUDE_AI_SUMMARY: String = "Include AI summary"

    fun includeSummary(source: SummarySource?): String = if (source == SummarySource.GENERATED) INCLUDE_AI_SUMMARY else INCLUDE_SUMMARY
    private const val SHORT_ID: Int = 8
    private val MONTHS = arrayOf("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

    fun expiryLabel(expiry: ShareExpiry): String = when (expiry) {
        ShareExpiry.DAY -> "24 h"
        ShareExpiry.WEEK -> "7 days"
        ShareExpiry.MONTH -> "30 days"
        ShareExpiry.NEVER -> "Never"
    }

    fun civil(epochDay: Long): Triple<Long, Int, Int> {
        val z = epochDay + 719_468
        val era = (if (z >= 0) z else z - 146_096) / 146_097
        val doe = z - era * 146_097
        val yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365
        val doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
        val mp = (5 * doy + 2) / 153
        val day = (doy - (153 * mp + 2) / 5 + 1).toInt()
        val month = (if (mp < 10) mp + 3 else mp - 9).toInt()
        return Triple(yoe + era * 400 + if (month <= 2) 1 else 0, month, day)
    }

    fun monthDay(ms: Long, nowMs: Long, offset: UtcOffset): String {
        val (year, month, day) = civil((ms + offset.millisAt(ms)).floorDiv(DAY_MS))
        val currentYear = civil((nowMs + offset.millisAt(nowMs)).floorDiv(DAY_MS)).first
        val base = MONTHS[month - 1] + " " + day
        return if (year == currentYear) base else "$base, $year"
    }

    fun linkMeta(expiresAtMs: Long?, items: Int, nowMs: Long, offset: UtcOffset): String = Formatters.joinDot(
        if (expiresAtMs == null) "Never expires" else "Expires " + monthDay(expiresAtMs, nowMs, offset),
        Formatters.plural(items, "item"),
    )

    fun displayLink(url: String): String {
        val bare = url.substringAfter("://")
        val marker = bare.indexOf("/v/")
        if (marker < 0) return maskKey(bare)
        val head = bare.substring(0, marker + 3)
        val id = bare.substring(marker + 3).substringBefore('#')
        val short = id.take(SHORT_ID)
        val grouped = if (short.length > 4) short.substring(0, 4) + "-" + short.substring(4) else short
        return head + "\n" + grouped + "#k=" + KEY_MASK
    }

    fun maskKey(url: String): String {
        val at = url.indexOf("#k=")
        return if (at < 0) url else url.substring(0, at + 3) + KEY_MASK
    }

    fun recipients(verified: Int, total: Int, edits: Int): String =
        Formatters.joinDot(Formatters.verifiedSummary(verified, total), if (edits > 0) Formatters.declaredEdits(edits, "edit") else null)

    fun selected(selected: Int, total: Int): String = "${Formatters.groupThousands(selected)} of ${Formatters.groupThousands(total)} selected"

    fun subtitle(title: String, items: Int): String = Formatters.joinDot(title, Formatters.plural(items, "item"))
}
