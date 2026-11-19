package app.witness.feature.home

import androidx.compose.ui.graphics.Color
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class HighlightTest {
    @Test
    fun termsAreLowercasedDistinctAndLongestFirst() = assertEquals(listOf("7mnv", "ab"), Highlight.terms("ab 7MNV ab"))

    @Test
    fun blankQueryHasNoTerms() {
        assertTrue(Highlight.terms("").isEmpty())
        assertTrue(Highlight.terms("  · , ").isEmpty())
    }

    @Test
    fun prefixOfWordIsHighlighted() = assertEquals(listOf(6 until 10), Highlight.ranges("Plate 7MNV562", listOf("7mnv")))

    @Test
    fun matchIsCaseInsensitive() = assertEquals(listOf(0 until 5), Highlight.ranges("PLATE", listOf("plate")))

    @Test
    fun middleOfWordIsNotHighlighted() = assertTrue(Highlight.ranges("ab7MNV", listOf("7mnv")).isEmpty())

    @Test
    fun everyOccurrenceIsHighlighted() = assertEquals(listOf(0 until 3, 8 until 11), Highlight.ranges("oak and oak", listOf("oak")))

    @Test
    fun longestTermWins() = assertEquals(listOf(0 until 4), Highlight.ranges("7MNV562", Highlight.terms("7m 7mnv")))

    @Test
    fun termLongerThanWordDoesNotMatch() = assertTrue(Highlight.ranges("7MN", listOf("7mnv")).isEmpty())

    @Test
    fun emptyInputs() {
        assertTrue(Highlight.ranges("", listOf("a")).isEmpty())
        assertTrue(Highlight.ranges("abc", emptyList()).isEmpty())
        assertFalse(Highlight.matches(null, listOf("a")))
    }

    @Test
    fun punctuationSeparatesWords() = assertEquals(listOf(13 until 17), Highlight.ranges("Note: hit by 7MNV562, silver", listOf("7mnv")))

    @Test
    fun snippetKeepsShortTextWhole() = assertEquals("Oak St & 5th Ave · plate 7MNV562", Highlight.snippet("Oak St & 5th Ave · plate 7MNV562", listOf("7mnv"), lead = 40))

    @Test
    fun snippetTrimsLongLeadAtWordBoundary() {
        val text = "The other driver ran the light at Oak St and fled the scene, plate 7MNV562 was visible"
        val s = Highlight.snippet(text, listOf("7mnv"), lead = 10, max = 20)
        assertTrue(s.startsWith(Highlight.ELLIPSIS))
        assertTrue(s.contains("7MNV562"))
        assertFalse(s.substring(1).first().isWhitespace())
    }

    @Test
    fun snippetKeepsTextThatFitsOneLine() = assertEquals("Oak St & 5th Ave · plate 7MNV562", Highlight.snippet("Oak St & 5th Ave · plate 7MNV562", listOf("7mnv")))

    @Test
    fun snippetDefaultsTrimLongSummaries() {
        val text = "Two cars collided at the intersection after the light changed; the other driver, plate 7MNV562, drove off"
        assertTrue(Highlight.snippet(text, listOf("7mnv")).startsWith(Highlight.ELLIPSIS))
    }

    @Test
    fun snippetWithoutMatchReturnsText() = assertEquals("abc", Highlight.snippet("abc", listOf("zz")))

    @Test
    fun annotateAddsOneSpanPerRange() {
        val a = Highlight.annotate("oak and oak", listOf("oak"), Color.Blue)
        assertEquals("oak and oak", a.text)
        assertEquals(listOf(0 to 3, 8 to 11), a.spanStyles.map { it.start to it.end })
        assertEquals(Color.Blue, a.spanStyles.first().item.background)
    }

    @Test
    fun annotateWithoutMatchHasNoSpans() = assertTrue(Highlight.annotate("abc", listOf("x"), Color.Blue).spanStyles.isEmpty())

    @Test
    fun rangesScaleLinearly() {
        val text = "word ".repeat(20_000)
        assertEquals(20_000, Highlight.ranges(text, listOf("wo")).size)
    }
}
