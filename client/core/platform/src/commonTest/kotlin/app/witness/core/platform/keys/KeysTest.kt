package app.witness.core.platform.keys

import app.witness.core.model.BootState
import app.witness.core.model.DeviceKey
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.SecurityLevel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

private fun keyId(seed: Int) = KeyId.fromBytes(ByteArray(16) { (it + seed).toByte() })

class KeyRecordCodecTest {
    private val fresh = DeviceKey(keyId(1), SecurityLevel.STRONG_BOX, Freshness.FRESH, 100)

    @Test
    fun roundTripsMinimal() {
        assertEquals(fresh, KeyRecordCodec.decode(KeyRecordCodec.encode(fresh)))
    }

    @Test
    fun roundTripsFull() {
        val k = fresh.copy(security = SecurityLevel.TEE, freshness = Freshness.LATE, enrolledAtMs = 5, activeUntilMs = 9, certSerial = "0a1b", retired = true, boot = BootState.SELF_SIGNED, deviceLocked = false)
        assertEquals(k, KeyRecordCodec.decode(KeyRecordCodec.encode(k)))
    }

    @Test
    fun rejectsBadSerial() {
        assertFailsWith<IllegalArgumentException> { KeyRecordCodec.encode(fresh.copy(certSerial = "a|b")) }
        assertFailsWith<IllegalArgumentException> { KeyRecordCodec.encode(fresh.copy(certSerial = "")) }
    }

    @Test
    fun malformedDecodesToNull() {
        val good = KeyRecordCodec.encode(fresh)
        listOf("", "2" + good.drop(1), good.replace("STRONG_BOX", "HSM"), good.replace("FRESH", "OLD"), "$good|x", good.dropLast(1) + "2", good.replace("|100|", "|x|"))
            .forEach { assertNull(KeyRecordCodec.decode(it), it) }
    }
}

class StrongBoxPolicyTest {
    @Test
    fun p90NearestRank() {
        assertEquals(9, StrongBoxPolicy.p90(longArrayOf(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)))
        assertEquals(5, StrongBoxPolicy.p90(longArrayOf(5)))
        assertEquals(50, StrongBoxPolicy.p90(longArrayOf(50, 10, 20, 40, 30)))
    }

    @Test
    fun p90DoesNotMutateInput() {
        val s = longArrayOf(3, 1, 2)
        StrongBoxPolicy.p90(s)
        assertEquals(listOf(3L, 1L, 2L), s.toList())
    }

    @Test
    fun p90RequiresSamples() {
        assertFailsWith<IllegalArgumentException> { StrongBoxPolicy.p90(LongArray(0)) }
    }

    @Test
    fun budget() {
        assertTrue(StrongBoxPolicy.withinBudget(LongArray(5) { 90_000_000 }))
        assertFalse(StrongBoxPolicy.withinBudget(longArrayOf(90_000_000, 90_000_000, 90_000_000, 90_000_000, 130_000_000)))
    }

    @Test
    fun activeIsNewestNonRetired() {
        val a = DeviceKey(keyId(1), SecurityLevel.TEE, Freshness.FRESH, 1)
        val b = DeviceKey(keyId(2), SecurityLevel.TEE, Freshness.FRESH, 3, retired = true)
        val c = DeviceKey(keyId(3), SecurityLevel.TEE, Freshness.FRESH, 2)
        assertEquals(c, StrongBoxPolicy.active(listOf(a, b, c)))
        assertNull(StrongBoxPolicy.active(listOf(b)))
        assertNull(StrongBoxPolicy.active(emptyList()))
    }

    @Test
    fun rotationAfterThirtyDays() {
        val k = DeviceKey(keyId(1), SecurityLevel.TEE, Freshness.FRESH, 0, enrolledAtMs = 1_000)
        assertFalse(StrongBoxPolicy.needsRotation(k, 1_000 + StrongBoxPolicy.ROTATE_AFTER_MS - 1))
        assertTrue(StrongBoxPolicy.needsRotation(k, 1_000 + StrongBoxPolicy.ROTATE_AFTER_MS))
        assertTrue(StrongBoxPolicy.needsRotation(k.copy(retired = true), 0))
        assertTrue(StrongBoxPolicy.needsRotation(k.copy(enrolledAtMs = null), StrongBoxPolicy.ROTATE_AFTER_MS))
    }
}
