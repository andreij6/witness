package app.witness.core.ui

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class FormattersTest {
    @Test
    fun groupsThousands() {
        assertEquals("0", Formatters.groupThousands(0))
        assertEquals("7", Formatters.groupThousands(7))
        assertEquals("999", Formatters.groupThousands(999))
        assertEquals("1,000", Formatters.groupThousands(1000))
        assertEquals("1,284", Formatters.groupThousands(1284))
        assertEquals("12,345", Formatters.groupThousands(12345))
        assertEquals("123,456", Formatters.groupThousands(123456))
        assertEquals("2,108,733", Formatters.groupThousands(2108733))
        assertEquals("-1,284", Formatters.groupThousands(-1284))
        assertEquals("9,223,372,036,854,775,807", Formatters.groupThousands(Long.MAX_VALUE))
        assertEquals("-9,223,372,036,854,775,808", Formatters.groupThousands(Long.MIN_VALUE))
    }

    @Test
    fun pluralizes() {
        assertEquals("1 item", Formatters.plural(1, "item"))
        assertEquals("4 items", Formatters.plural(4, "item"))
        assertEquals("0 items", Formatters.plural(0, "item"))
        assertEquals("2 copies", Formatters.plural(2, "copy", "copies"))
        assertEquals("1,200 items", Formatters.plural(1200L, "item"))
    }

    @Test
    fun joinsWithMiddleDotSkippingBlanks() {
        assertEquals("Today · 4 items", Formatters.joinDot("Today", "4 items"))
        assertEquals("Denver area", Formatters.joinDot("Denver area", null, " ", ""))
        assertEquals("", Formatters.joinDot())
        assertEquals("a · b · c", Formatters.joinDot(listOf("a", null, "b", "c")))
    }

    @Test
    fun formatsCheckCounts() {
        assertEquals("Checked 1,284×", Formatters.checkedBadge(1284))
        assertEquals("checked 1,284 times", Formatters.checkedPhrase(1284))
        assertEquals("checked once", Formatters.checkedPhrase(1))
        assertEquals("not checked yet", Formatters.checkedPhrase(0))
    }

    @Test
    fun summarizesAnchoring() {
        assertEquals("31 sealed · 30 anchored · 1 queued", Formatters.anchoringSummary(30, 1, 31))
        assertEquals("3 anchored · 1 queued", Formatters.anchoringSummary(3, 1))
        assertEquals("6 anchored", Formatters.anchoringSummary(6, 0))
        assertEquals("1,000 anchored", Formatters.anchoringSummary(1000, 0))
    }

    @Test
    fun summarizesVerification() {
        assertEquals("4 of 4 verified", Formatters.verifiedSummary(4, 4))
        assertEquals("62 of 64 regions match", Formatters.regionsMatch(62, 64))
    }

    @Test
    fun describesDeclaredEdits() {
        assertEquals("2 declared blurs", Formatters.declaredEdits(2, "blur"))
        assertEquals("1 declared blur", Formatters.declaredEdits(1, "blur"))
        assertEquals("3 declared crops", Formatters.declaredEdits(3, "crop"))
    }

    @Test
    fun formatsDimensionsAndAccuracy() {
        assertEquals("1080 × 1080", Formatters.dimensions(1080, 1080))
        assertEquals("±6 m", Formatters.accuracy(6))
    }

    @Test
    fun abbreviatesMiddle() {
        assertEquals("9f3a…c21e", Formatters.abbreviateMiddle("9f3a00112233445566778899c21e"))
        assertEquals("short", Formatters.abbreviateMiddle("short"))
        assertEquals("123456789", Formatters.abbreviateMiddle("123456789"))
        assertEquals("1234…7890", Formatters.abbreviateMiddle("1234567890"))
        assertEquals("ab…z", Formatters.abbreviateMiddle("abcdefghz", 2, 1))
        assertEquals("sha256 9f3a…c21e", Formatters.shortHash("sha256", "9f3a00112233445566778899c21e"))
        assertFailsWith<IllegalArgumentException> { Formatters.abbreviateMiddle("x", -1, 2) }
    }

    @Test
    fun formatsBytesWithSiUnits() {
        assertEquals("0 B", Formatters.bytes(0))
        assertEquals("999 B", Formatters.bytes(999))
        assertEquals("1 KB", Formatters.bytes(1000))
        assertEquals("1.5 KB", Formatters.bytes(1500))
        assertEquals("9.9 KB", Formatters.bytes(9949))
        assertEquals("10 KB", Formatters.bytes(9950))
        assertEquals("214 KB", Formatters.bytes(214_000))
        assertEquals("214 KB", Formatters.bytes(214_400))
        assertEquals("1 MB", Formatters.bytes(999_600))
        assertEquals("1.6 MB", Formatters.bytes(1_600_000))
        assertEquals("3.1 MB", Formatters.bytes(3_140_000))
        assertEquals("47 MB", Formatters.bytes(47_000_000))
        assertEquals("25 GB", Formatters.bytes(25_000_000_000))
        assertEquals("1,000 TB".replace(",", ""), Formatters.bytes(1_000_000_000_000_000))
    }

    @Test
    fun bytesHandlesExtremesWithoutOverflow() {
        val huge = Formatters.bytes(Long.MAX_VALUE)
        kotlin.test.assertTrue(huge.endsWith(" TB"), huge)
        assertFailsWith<IllegalArgumentException> { Formatters.bytes(-1) }
    }
}
