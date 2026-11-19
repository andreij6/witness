package app.witness.core.domain.usecase

import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal

object PostText {
    const val MAX_ALT: Int = 1_000
    const val MAX_CAPTION: Int = 2_200

    fun clean(text: String?, max: Int): String? =
        text?.trim()?.replace(WHITESPACE, " ")?.take(max)?.trimEnd()?.takeIf { it.isNotEmpty() }

    fun rulesAltText(blurs: List<BlurProposal>, place: String?): String {
        val sb = StringBuilder("Photo")
        place?.trim()?.takeIf { it.isNotEmpty() }?.let { sb.append(" taken in ").append(it) }
        sb.append('.')
        val counts = IntArray(BlurClass.entries.size)
        for (b in blurs) if (b.enabled) counts[b.blurClass.ordinal]++
        val parts = BlurClass.entries.mapNotNull { c -> counts[c.ordinal].takeIf { it > 0 }?.let { phrase(c, it) } }
        if (parts.isNotEmpty()) sb.append(' ').append(join(parts)).append(if (parts.size == 1 && counts.sum() == 1) " is blurred." else " are blurred.")
        return sb.toString()
    }

    private fun phrase(c: BlurClass, n: Int): String {
        val (one, many) = when (c) {
            BlurClass.FACE -> "face" to "faces"
            BlurClass.PLATE -> "license plate" to "license plates"
            BlurClass.HOUSE_NO -> "house number" to "house numbers"
            BlurClass.OTHER -> "area" to "areas"
        }
        return if (n == 1) "1 $one" else "$n $many"
    }

    private fun join(parts: List<String>): String = when (parts.size) {
        1 -> parts[0]
        2 -> "${parts[0]} and ${parts[1]}"
        else -> parts.dropLast(1).joinToString(", ") + " and " + parts.last()
    }

    private val WHITESPACE = Regex("\\s+")
}
