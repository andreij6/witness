package app.witness.feature.home

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import app.witness.core.domain.usecase.Tokens

object Highlight {
    const val SNIPPET_LEAD: Int = 24
    const val SNIPPET_MAX: Int = 48
    const val ELLIPSIS: String = "…"

    fun terms(query: String): List<String> = Tokens.of(query).distinct().sortedByDescending { it.length }

    fun ranges(text: String, terms: List<String>): List<IntRange> {
        if (terms.isEmpty() || text.isEmpty()) return emptyList()
        val out = ArrayList<IntRange>()
        var i = 0
        while (i < text.length) {
            if (!text[i].isLetterOrDigit()) {
                i++
                continue
            }
            var end = i
            while (end < text.length && text[end].isLetterOrDigit()) end++
            val len = longestPrefix(text, i, end - i, terms)
            if (len > 0) out += i until i + len
            i = end
        }
        return out
    }

    fun matches(text: String?, terms: List<String>): Boolean = text != null && ranges(text, terms).isNotEmpty()

    fun snippet(text: String, terms: List<String>, lead: Int = SNIPPET_LEAD, max: Int = SNIPPET_MAX): String {
        if (text.length <= max) return text
        val first = ranges(text, terms).firstOrNull()?.first ?: return text
        if (first <= lead) return text
        var start = first - lead
        while (start < first && text[start - 1].isLetterOrDigit()) start++
        return ELLIPSIS + text.substring(start).trimStart()
    }

    fun annotate(text: String, terms: List<String>, background: Color): AnnotatedString {
        val ranges = ranges(text, terms)
        if (ranges.isEmpty()) return AnnotatedString(text)
        val style = SpanStyle(background = background)
        return buildAnnotatedString {
            append(text)
            for (r in ranges) addStyle(style, r.first, r.last + 1)
        }
    }

    private fun longestPrefix(text: String, start: Int, wordLength: Int, terms: List<String>): Int {
        for (t in terms) if (t.length <= wordLength && text.regionMatches(start, t, 0, t.length, ignoreCase = true)) return t.length
        return 0
    }
}
