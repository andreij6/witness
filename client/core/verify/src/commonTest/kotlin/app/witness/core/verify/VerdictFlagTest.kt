package app.witness.core.verify

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class VerdictFlagTest {
    @Test
    fun bitsMatchRustVerdictFlags() {
        assertEquals(1, VerdictFlag.KEY_REVOKED_AFTER_ANCHOR.mask)
        assertEquals(2, VerdictFlag.LATE_ANCHOR.mask)
        assertEquals(4, VerdictFlag.CLOCK_MISMATCH.mask)
        assertEquals(8, VerdictFlag.COULDNT_COMPARE.mask)
        assertEquals(16, VerdictFlag.NO_PUBLIC_REFERENCE.mask)
        assertEquals(32, VerdictFlag.DEV_ATTESTATION.mask)
        assertEquals(64, VerdictFlag.TRUNCATED_CANDIDATES.mask)
        assertEquals(0x7f, VerdictFlag.ALL_BITS)
    }

    @Test
    fun everySubsetRoundTrips() {
        for (bits in 0..VerdictFlag.ALL_BITS) {
            assertEquals(bits, VerdictFlag.encode(VerdictFlag.decode(bits)))
        }
    }

    @Test
    fun unknownBitsAreIgnored() {
        assertEquals(emptySet(), VerdictFlag.decode(0x80))
        assertEquals(VerdictFlag.entries.toSet(), VerdictFlag.decode(-1))
        assertEquals(setOf(VerdictFlag.LATE_ANCHOR), VerdictFlag.decode(0x102))
    }

    @Test
    fun decodeKeepsDeclarationOrder() {
        assertEquals(VerdictFlag.entries, VerdictFlag.decode(VerdictFlag.ALL_BITS).toList())
        assertTrue(VerdictFlag.decode(0).isEmpty())
    }
}
