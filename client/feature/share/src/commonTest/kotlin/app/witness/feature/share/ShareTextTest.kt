package app.witness.feature.share

import app.witness.core.model.BlurClass
import app.witness.core.model.ShareExpiry
import app.witness.core.model.SummarySource
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ShareTextTest {
    private val day = ShareText.DAY_MS

    @Test
    fun expiryLabelsMatchDesign() =
        assertEquals(listOf("24 h", "7 days", "30 days", "Never"), ShareExpiry.entries.map(ShareText::expiryLabel))

    @Test
    fun civilEpoch() = assertEquals(Triple(1970L, 1, 1), ShareText.civil(0))

    @Test
    fun civilLeapDay() = assertEquals(Triple(2024L, 2, 29), ShareText.civil(19_782))

    @Test
    fun civilBeforeEpoch() = assertEquals(Triple(1969L, 12, 31), ShareText.civil(-1))

    @Test
    fun monthDaySameYear() = assertEquals("Nov 26", ShareText.monthDay(NOW + 7 * day, NOW, UtcOffset.UTC))

    @Test
    fun monthDayNextYearShowsYear() = assertEquals("Jan 2, 2026", ShareText.monthDay(NOW + 44 * day, NOW, UtcOffset.UTC))

    @Test
    fun monthDayHonoursOffset() = assertEquals("Nov 20", ShareText.monthDay(NOW, NOW, UtcOffset { 10 * HOUR_MS }))

    @Test
    fun linkMetaWithExpiry() = assertEquals("Expires Nov 25 · 4 items", ShareText.linkMeta(NOW + 6 * day, 4, NOW, UtcOffset.UTC))

    @Test
    fun linkMetaNeverExpires() = assertEquals("Never expires · 1 item", ShareText.linkMeta(null, 1, NOW, UtcOffset.UTC))

    @Test
    fun displayLinkMatchesDesign() =
        assertEquals("case.witness.app/v/\n8F2K-QM7D#k=••••", ShareText.displayLink("https://case.witness.app/v/8F2KQM7DXXXXXXXXXXXXXXXXXX#k=c2VjcmV0a2V5"))

    @Test
    fun displayLinkNeverRevealsKey() {
        val shown = ShareText.displayLink("https://case.witness.app/v/ABCDEFGH#k=SECRETKEY")
        assertTrue("SECRET" !in shown)
    }

    @Test
    fun displayLinkWithoutCasePathMasksKey() = assertEquals("example.com/x#k=••••", ShareText.displayLink("https://example.com/x#k=SECRET"))

    @Test
    fun displayLinkShortId() = assertEquals("h/v/\nABC#k=••••", ShareText.displayLink("h/v/ABC#k=1"))

    @Test
    fun maskKeyWithoutKeyIsUnchanged() = assertEquals("a/b", ShareText.maskKey("a/b"))

    @Test
    fun recipientsWithEdits() = assertEquals("4 of 4 verified · 1 declared edit", ShareText.recipients(4, 4, 1))

    @Test
    fun recipientsWithoutEdits() = assertEquals("2 of 2 verified", ShareText.recipients(2, 2, 0))

    @Test
    fun recipientsPluralEdits() = assertEquals("3 of 3 verified · 2 declared edits", ShareText.recipients(3, 3, 2))

    @Test
    fun selectedLabel() = assertEquals("4 of 4 selected", ShareText.selected(4, 4))

    @Test
    fun subtitle() = assertEquals("Fender bender · 4 items", ShareText.subtitle("Fender bender", 4))
}

class RedactionPlanTest {
    @Test
    fun blursOnlyDetectedClassesThatAreOn() {
        val plan = RedactionPlan.forItem(setOf(BlurClass.FACE, BlurClass.PLATE), setOf(PrivacyOption.BLUR_FACES))
        assertEquals(Redaction(setOf(BlurClass.FACE), false), plan)
    }

    @Test
    fun nothingDetectedIsEmpty() = assertTrue(RedactionPlan.forItem(emptySet(), setOf(PrivacyOption.BLUR_FACES, PrivacyOption.BLUR_PLATES)).isEmpty)

    @Test
    fun hideLocationAppliesToEveryItem() = assertEquals(Redaction(emptySet(), true), RedactionPlan.forItem(emptySet(), setOf(PrivacyOption.HIDE_LOCATION)))

    @Test
    fun noOptionsIsEmpty() = assertTrue(RedactionPlan.forItem(setOf(BlurClass.FACE), emptySet()).isEmpty)

    @Test
    fun allOptions() = assertEquals(
        Redaction(setOf(BlurClass.FACE, BlurClass.PLATE), true),
        RedactionPlan.forItem(setOf(BlurClass.FACE, BlurClass.PLATE, BlurClass.HOUSE_NO), PrivacyOption.entries.toSet()),
    )

    @Test
    fun countsOnlySelectedItems() {
        val d = mapOf(record(1) to setOf(BlurClass.FACE), record(2) to setOf(BlurClass.FACE, BlurClass.PLATE))
        assertEquals(mapOf(BlurClass.FACE to 2, BlurClass.PLATE to 1), RedactionPlan.counts(d, listOf(record(1), record(2), record(3))))
        assertEquals(mapOf(BlurClass.FACE to 1), RedactionPlan.counts(d, listOf(record(1))))
        assertEquals(emptyMap(), RedactionPlan.counts(d, emptyList()))
    }

    @Test
    fun summaryToggleOnlySaysAiForGeneratedSummaries() {
        assertEquals("Include AI summary", ShareText.includeSummary(SummarySource.GENERATED))
        assertEquals("Include summary", ShareText.includeSummary(SummarySource.EDITED))
        assertEquals("Include summary", ShareText.includeSummary(SummarySource.WRITTEN))
        assertEquals("Include summary", ShareText.includeSummary(null))
    }
}
