package app.witness.core.model

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EnumBitsTest {
    @Test
    fun verdictFlagBitsMatchRust() {
        assertEquals(1, EnumBits.of(VerdictFlag.KEY_REVOKED_AFTER_ANCHOR).bits)
        assertEquals(2, EnumBits.of(VerdictFlag.LATE_ANCHOR).bits)
        assertEquals(4, EnumBits.of(VerdictFlag.CLOCK_MISMATCH).bits)
        assertEquals(8, EnumBits.of(VerdictFlag.COULDNT_COMPARE).bits)
        assertEquals(16, EnumBits.of(VerdictFlag.NO_PUBLIC_REFERENCE).bits)
        assertEquals(32, EnumBits.of(VerdictFlag.DEV_ATTESTATION).bits)
        assertEquals(64, EnumBits.of(VerdictFlag.TRUNCATED_CANDIDATES).bits)
        assertEquals(0x7f, allBits<VerdictFlag>().bits)
    }

    @Test
    fun locationReasonBitsMatchRust() {
        assertEquals(1, EnumBits.of(LocationReason.RAW_GNSS_PASSED).bits)
        assertEquals(1 shl 8, EnumBits.of(LocationReason.MOCK_LOCATION).bits)
        assertEquals(1 shl 10, EnumBits.of(LocationReason.INDOOR).bits)
        assertEquals(1 shl 6, EnumBits.of(GnssCheck.GNSS_TIME).bits)
        assertEquals(15, VerdictReason.entries.size)
    }

    @Test
    fun setOperations() {
        var f = EnumBits.none<VerdictFlag>()
        assertTrue(f.isEmpty)
        f += VerdictFlag.LATE_ANCHOR
        f += EnumBits.of(VerdictFlag.DEV_ATTESTATION)
        assertTrue(f.containsAll(EnumBits.of(VerdictFlag.LATE_ANCHOR, VerdictFlag.DEV_ATTESTATION)))
        assertFalse(VerdictFlag.CLOCK_MISMATCH in f)
        assertTrue(f.intersects(EnumBits.of(VerdictFlag.DEV_ATTESTATION, VerdictFlag.CLOCK_MISMATCH)))
        assertFalse(f.intersects(EnumBits.of(VerdictFlag.CLOCK_MISMATCH)))
        assertEquals(2, f.size)
        f -= VerdictFlag.LATE_ANCHOR
        assertEquals(EnumBits.of(VerdictFlag.DEV_ATTESTATION), f)
        f = f.with(VerdictFlag.COULDNT_COMPARE, true).with(VerdictFlag.DEV_ATTESTATION, false)
        assertEquals(EnumBits.of(VerdictFlag.COULDNT_COMPARE), f)
        f -= VerdictFlag.COULDNT_COMPARE
        f -= VerdictFlag.COULDNT_COMPARE
        assertTrue(f.isEmpty)
    }

    @Test
    fun toListIsInDeclarationOrder() {
        val f = EnumBits.of(listOf(VerdictFlag.TRUNCATED_CANDIDATES, VerdictFlag.KEY_REVOKED_AFTER_ANCHOR))
        assertEquals(listOf(VerdictFlag.KEY_REVOKED_AFTER_ANCHOR, VerdictFlag.TRUNCATED_CANDIDATES), f.toList())
        assertEquals(VerdictFlag.entries, allBits<VerdictFlag>().toList())
    }

    @Test
    fun rejectsUnknownBits() {
        assertNull(enumBitsOrNull<VerdictFlag>(0x80))
        assertNull(enumBitsOrNull<VerdictFlag>(-1))
        assertEquals(allBits(), enumBitsOrNull<VerdictFlag>(0x7f))
        assertEquals(EnumBits.none(), enumBitsOrNull<VerdictFlag>(0))
    }

    @Test
    fun everySubsetRoundTrips() {
        for (bits in 0..0x7f) {
            val f = enumBitsOrNull<VerdictFlag>(bits)!!
            assertEquals(f, EnumBits.of(f.toList()))
        }
    }
}
