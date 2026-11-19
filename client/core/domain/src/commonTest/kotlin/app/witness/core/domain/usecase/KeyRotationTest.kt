package app.witness.core.domain.usecase

import app.witness.core.model.DeviceKey
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.SecurityLevel
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class KeyRotationTest {
    private val day = KeyRotation.DAY_MS
    private val key = DeviceKey(KeyId.fromBytes(ByteArray(16)), SecurityLevel.STRONG_BOX, Freshness.FRESH, 0, enrolledAtMs = 0, activeUntilMs = 60 * day)

    @Test
    fun statusCountsDownToRotation() {
        assertEquals(30, KeyRotation.status(key, 0).rotatesInDays)
        assertEquals(12, KeyRotation.status(key, 18 * day).rotatesInDays)
        assertEquals(13, KeyRotation.status(key, 17 * day + 1).rotatesInDays)
        assertEquals(1, KeyRotation.status(key, 30 * day - 1).rotatesInDays)
        assertFalse(KeyRotation.status(key, 30 * day - 1).rotationDue)
        val due = KeyRotation.status(key, 30 * day)
        assertTrue(due.rotationDue)
        assertEquals(0, due.rotatesInDays)
        assertFalse(due.windowExpired)
        assertTrue(KeyRotation.status(key, 60 * day + 1).windowExpired)
        assertEquals(SecurityLevel.STRONG_BOX, due.security)
    }

    @Test
    fun statusFallsBackToServerWindow() {
        val s = KeyRotation.status(key.copy(activeUntilMs = null), 0)
        assertEquals(60 * day, s.windowEndsAtMs)
        assertFailsWith<IllegalArgumentException> { KeyRotation.status(key.copy(enrolledAtMs = null), 0) }
    }

    @Test
    fun noKeyOrNotDue() = runTest {
        val g = TestGraph()
        assertIs<RotationOutcome.NoKey>(g.rotation())
        assertNull(g.rotation.status())
        g.enrolledKey()
        assertIs<RotationOutcome.NotDue>(g.rotation())
        assertEquals(30, g.rotation.status()!!.rotatesInDays)
    }

    @Test
    fun rotatesWhenDueAndRetiresOldKey() = runTest {
        val g = TestGraph()
        val old = g.enrolledKey()
        g.clock.advance(30 * day)
        val r = assertIs<RotationOutcome.Rotated>(g.rotation())
        assertEquals(old.keyId, r.retired)
        assertTrue(g.vault.key(old.keyId)!!.retired)
        assertEquals(r.key.keyId, g.vault.activeKey()!!.keyId)
        assertEquals(Freshness.FRESH, r.key.freshness)
        assertEquals(30L, g.telemetry.events.last().second["age_days"])
        val c = g.sealOne()
        assertEquals(r.key.keyId, c.keyRef!!.keyId)
    }

    @Test
    fun retiredKeyStillSignsDerivativesOfItsCaptures() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        g.clock.advance(31 * day)
        g.rotation()
        val post = g.post.publish(g.post.draft(c.recordId))
        assertEquals(c.keyRef!!.keyId, g.captures.get(post.recordId)!!.keyRef!!.keyId)
    }

    @Test
    fun forcedRotationIgnoresAge() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        assertIs<RotationOutcome.Rotated>(g.rotation(force = true))
    }

    @Test
    fun offlineRotationKeepsCurrentKey() = runTest {
        val g = TestGraph()
        val old = g.enrolledKey()
        g.clock.advance(40 * day)
        g.registry.offline = true
        assertIs<RotationOutcome.Offline>(g.rotation())
        assertEquals(old.keyId, g.vault.activeKey()!!.keyId)
        assertFalse(g.vault.key(old.keyId)!!.retired)
    }

    @Test
    fun rejectedRotationFails() = runTest {
        val g = TestGraph()
        val old = g.enrolledKey()
        g.registry.rejectEnrollment = "quota"
        val r = assertIs<RotationOutcome.Failed>(g.rotation(force = true))
        assertEquals("quota", r.reason)
        assertEquals(old.keyId, g.vault.activeKey()!!.keyId)
    }
}
