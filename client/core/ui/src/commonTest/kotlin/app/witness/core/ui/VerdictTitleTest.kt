package app.witness.core.ui

import app.witness.core.model.Verdict
import kotlin.test.Test
import kotlin.test.assertEquals

class VerdictTitleTest {
    @Test
    fun titlesFollowSpecCopy() {
        assertEquals("Original", Verdict.ORIGINAL.title())
        assertEquals("No Witness record", Verdict.NO_RECORD.title())
        assertEquals("Sealed · not yet anchored", Verdict.SEALED_UNANCHORED.title())
    }

    @Test
    fun everyVerdictHasDistinctTitle() {
        assertEquals(Verdict.entries.size, Verdict.entries.map { it.title() }.toSet().size)
    }
}
