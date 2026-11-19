package app.witness.core.model

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class KeyHistoryTest {
    private fun h(vararg c: Pair<KeyStatus, Long>) = KeyHistory.of(c.map { StatusChange(it.first, it.second) })

    @Test
    fun statusClassification() {
        assertTrue(KeyStatus.ACTIVE.isTrusted)
        assertTrue(KeyStatus.RETIRED.isTrusted)
        assertTrue(KeyStatus.SUSPENDED.isDistrusted)
        assertTrue(KeyStatus.REVOKED.isDistrusted)
        KeyStatus.entries.forEach { assertTrue(it.isTrusted != it.isDistrusted) }
    }

    @Test
    fun enrolledHistory() {
        val k = KeyHistory.enrolled(100)
        assertEquals(KeyStatus.ACTIVE, k.current)
        assertNull(k.statusAt(99))
        assertEquals(KeyStatus.ACTIVE, k.statusAt(100))
        assertFalse(k.activeAt(99))
        assertTrue(k.activeAt(1_000))
        assertFalse(k.revokedAfter(0))
    }

    @Test
    fun statusAtUsesBinarySearchBoundaries() {
        val k = assertNotNull(h(KeyStatus.ACTIVE to 10, KeyStatus.RETIRED to 20, KeyStatus.SUSPENDED to 30, KeyStatus.ACTIVE to 40, KeyStatus.REVOKED to 50))
        assertNull(k.statusAt(9))
        assertEquals(KeyStatus.ACTIVE, k.statusAt(19))
        assertEquals(KeyStatus.RETIRED, k.statusAt(20))
        assertEquals(KeyStatus.SUSPENDED, k.statusAt(35))
        assertEquals(KeyStatus.ACTIVE, k.statusAt(45))
        assertEquals(KeyStatus.REVOKED, k.statusAt(Long.MAX_VALUE))
        assertTrue(k.activeAt(25))
        assertFalse(k.activeAt(30))
        assertTrue(k.revokedAfter(25))
        assertEquals(StatusChange(KeyStatus.SUSPENDED, 30), k.firstDistrustedAfter(25))
        assertEquals(StatusChange(KeyStatus.REVOKED, 50), k.firstDistrustedAfter(30))
        assertFalse(k.revokedAfter(50))
        assertEquals(KeyStatus.REVOKED, k.current)
    }

    @Test
    fun rejectsMalformedHistories() {
        assertNull(KeyHistory.of(emptyList()))
        assertNull(h(KeyStatus.ACTIVE to 10, KeyStatus.RETIRED to 10))
        assertNull(h(KeyStatus.ACTIVE to 10, KeyStatus.RETIRED to 5))
        assertNull(h(KeyStatus.ACTIVE to 10, KeyStatus.ACTIVE to 20))
        assertNull(h(KeyStatus.REVOKED to 10, KeyStatus.ACTIVE to 20))
        assertNull(KeyHistory.of(List(65) { StatusChange(if (it % 2 == 0) KeyStatus.ACTIVE else KeyStatus.RETIRED, it.toLong()) }))
        assertNotNull(KeyHistory.of(List(64) { StatusChange(if (it % 2 == 0) KeyStatus.ACTIVE else KeyStatus.RETIRED, it.toLong()) }))
    }

    @Test
    fun plusValidates() {
        val k = KeyHistory.enrolled(10)
        assertEquals(KeyStatus.RETIRED, k.plus(StatusChange(KeyStatus.RETIRED, 11))?.current)
        assertNull(k.plus(StatusChange(KeyStatus.ACTIVE, 11)))
        assertNull(k.plus(StatusChange(KeyStatus.RETIRED, 10)))
        val revoked = assertNotNull(k.plus(StatusChange(KeyStatus.REVOKED, 20)))
        assertNull(revoked.plus(StatusChange(KeyStatus.ACTIVE, 30)))
    }

    @Test
    fun equalityStructural() {
        assertEquals(KeyHistory.enrolled(1), KeyHistory.enrolled(1))
        assertEquals(KeyHistory.enrolled(1).hashCode(), KeyHistory.enrolled(1).hashCode())
    }

    @Test
    fun keyInfoAcceptsOriginalsInsideWindow() {
        val id = KeyId.fromBytes(ByteArray(16))
        val info = KeyInfo(id, SecurityLevel.STRONG_BOX, Provisioning.RKP, Freshness.FRESH, 0, 100, KeyHistory.enrolled(0))
        assertTrue(info.acceptsOriginalAt(100))
        assertFalse(info.acceptsOriginalAt(101))
        assertFalse(info.isDev)
        val revoked = info.copy(history = KeyHistory.enrolled(0).plus(StatusChange(KeyStatus.REVOKED, 5))!!)
        assertFalse(revoked.acceptsOriginalAt(50))
        assertTrue(info.copy(provisioning = Provisioning.DEV).isDev)
        assertTrue(info.copy(security = SecurityLevel.DEV).isDev)
    }

    @Test
    fun deviceKeyEnrollment() {
        val id = KeyId.fromBytes(ByteArray(16))
        val k = DeviceKey(id, SecurityLevel.TEE, Freshness.LATE, 0)
        assertFalse(k.isEnrolled)
        assertTrue(k.copy(enrolledAtMs = 1).isEnrolled)
    }
}
