package app.witness.core.model

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class SummarySourceTest {
    @Test
    fun userTextOnAnEmptySummaryIsWritten() {
        assertEquals(SummarySource.WRITTEN, SummarySource.afterUserEdit(null, null, "hit from behind"))
        assertEquals(SummarySource.WRITTEN, SummarySource.afterUserEdit(SummarySource.GENERATED, null, "x"))
    }

    @Test
    fun changingAGeneratedOrEditedDraftMakesItEdited() {
        assertEquals(SummarySource.EDITED, SummarySource.afterUserEdit(SummarySource.GENERATED, "a", "b"))
        assertEquals(SummarySource.EDITED, SummarySource.afterUserEdit(SummarySource.EDITED, "b", "c"))
    }

    @Test
    fun rewritingAWrittenSummaryStaysWritten() {
        assertEquals(SummarySource.WRITTEN, SummarySource.afterUserEdit(SummarySource.WRITTEN, "a", "b"))
    }

    @Test
    fun savingUnchangedTextKeepsProvenance() {
        assertEquals(SummarySource.GENERATED, SummarySource.afterUserEdit(SummarySource.GENERATED, "a", "a"))
        assertEquals(SummarySource.EDITED, SummarySource.afterUserEdit(SummarySource.EDITED, "a", "a"))
        assertEquals(SummarySource.WRITTEN, SummarySource.afterUserEdit(null, "legacy", "legacy"))
    }

    @Test
    fun clearingDropsProvenance() {
        SummarySource.entries.forEach { assertNull(SummarySource.afterUserEdit(it, "a", null)) }
        assertNull(SummarySource.afterUserEdit(null, null, null))
    }

    @Test
    fun parsesStoredNamesAndRejectsUnknown() {
        SummarySource.entries.forEach { assertEquals(it, SummarySource.parse(it.name)) }
        assertNull(SummarySource.parse(null))
        assertNull(SummarySource.parse(""))
        assertNull(SummarySource.parse("generated"))
        assertNull(SummarySource.parse("AI"))
    }
}
