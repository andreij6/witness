package app.witness.core.model

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class VerdictTest {
    @Test
    fun onlySealedUnanchoredIsTransient() {
        assertTrue(Verdict.SEALED_UNANCHORED.isTransient)
        Verdict.entries.filter { it != Verdict.SEALED_UNANCHORED }.forEach { assertFalse(it.isTransient) }
    }

    @Test
    fun hasFourSpecVerdictsPlusTransientState() {
        assertEquals(5, Verdict.entries.size)
    }

    @Test
    fun outcomeCodesMatchRust() {
        assertEquals(listOf(0, 1, 2, 3, 4), listOf(
            Verdict.ORIGINAL, Verdict.MATCHES_WITH_CHANGES, Verdict.ALTERED, Verdict.NO_RECORD, Verdict.SEALED_UNANCHORED,
        ).map { it.ordinal })
    }

    @Test
    fun sealedMatchClassification() {
        assertTrue(Verdict.ORIGINAL.isSealedMatch)
        assertTrue(Verdict.MATCHES_WITH_CHANGES.isSealedMatch)
        assertTrue(Verdict.SEALED_UNANCHORED.isSealedMatch)
        assertFalse(Verdict.ALTERED.isSealedMatch)
        assertFalse(Verdict.NO_RECORD.isSealedMatch)
    }

    @Test
    fun defaultCheckResultIsEmpty() {
        val r = CheckResult.NO_RECORD
        assertEquals(Verdict.NO_RECORD, r.verdict)
        assertTrue(r.flags.isEmpty)
        assertTrue(r.reasons.isEmpty)
        assertEquals(CheckResult.VERDICT_VERSION, r.version)
    }
}
