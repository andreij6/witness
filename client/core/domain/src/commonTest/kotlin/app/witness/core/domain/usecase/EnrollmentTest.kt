package app.witness.core.domain.usecase

import app.witness.core.domain.fake.SoftwareSha256
import app.witness.core.model.ByteString
import app.witness.core.model.Freshness
import app.witness.core.model.OutboxState
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EnrollmentTest {
    @Test
    fun onlineEnrollmentIsFresh() = runTest {
        val g = TestGraph()
        val key = assertIs<EnrollmentStatus.Enrolled>(g.enrollment()).key
        assertEquals(Freshness.FRESH, key.freshness)
        assertTrue(key.isEnrolled)
        assertEquals(g.clock.nowMs + 60L * 86_400_000, key.activeUntilMs)
        assertNotNull(key.certSerial)
        val req = g.registry.enrollments.single()
        assertTrue(req.challenge.toByteArray().decodeToString().startsWith("challenge-"))
        assertEquals(2, req.attestationChain.size)
        assertEquals(1, g.scheduler.requests)
    }

    @Test
    fun alreadyEnrolledIsIdempotent() = runTest {
        val g = TestGraph()
        val a = assertIs<EnrollmentStatus.Enrolled>(g.enrollment()).key
        val b = assertIs<EnrollmentStatus.Enrolled>(g.enrollment()).key
        assertEquals(a, b)
        assertEquals(1, g.registry.enrollments.size)
    }

    @Test
    fun offlineGeneratesLateKeyWithDerivedChallenge() = runTest {
        val g = TestGraph()
        g.registry.offline = true
        val pending = assertIs<EnrollmentStatus.Pending>(g.enrollment()).key
        assertEquals(Freshness.LATE, pending.freshness)
        val nonce = assertNotNull(g.settings.value.installNonce)
        assertEquals(32, nonce.size)
        val wall = g.clock.nowMs
        val expected = SoftwareSha256.digest("witness-offline".encodeToByteArray() + nonce.toByteArray() + ByteArray(8) { i -> (wall ushr (56 - 8 * i)).toByte() })
        assertEquals(ByteString.of(expected), g.vault.attestation(pending.keyId)!!.challenge)
    }

    @Test
    fun installNonceIsStable() = runTest {
        val g = TestGraph()
        g.registry.offline = true
        g.enrollment()
        val first = g.settings.value.installNonce
        g.clock.advance(8L * 86_400_000)
        g.enrollment()
        assertEquals(first, g.settings.value.installNonce)
    }

    @Test
    fun pendingLateKeyEnrollsWhenOnlineWithinWindow() = runTest {
        val g = TestGraph()
        g.registry.offline = true
        val pending = assertIs<EnrollmentStatus.Pending>(g.enrollment()).key
        g.registry.offline = false
        g.clock.advance(6L * 86_400_000)
        val enrolled = assertIs<EnrollmentStatus.Enrolled>(g.enrollment()).key
        assertEquals(pending.keyId, enrolled.keyId)
        assertEquals(Freshness.LATE, g.registry.enrollments.single().freshness)
    }

    @Test
    fun staleLateKeyIsAbandonedAndItsCapturesResealed() = runTest {
        val g = TestGraph()
        g.registry.offline = true
        val stale = assertIs<EnrollmentStatus.Pending>(g.enrollment()).key
        val c = g.sealOne()
        assertEquals(OutboxState.AWAITING_ENROLLMENT, g.outbox[c.recordId]!!.state)
        g.registry.offline = false
        g.clock.advance(8L * 86_400_000)
        val fresh = assertIs<EnrollmentStatus.Enrolled>(g.enrollment()).key
        assertNotEquals(stale.keyId, fresh.keyId)
        assertEquals(Freshness.FRESH, fresh.freshness)
        assertTrue(g.vault.key(stale.keyId)!!.retired)
        assertEquals(OutboxState.AWAITING_RESEAL, g.outbox[c.recordId]!!.state)
        val r = g.anchor()
        assertEquals(1, r.resealed)
        assertEquals(1, r.anchored)
    }

    @Test
    fun rejectionMeansUnsupportedDevice() = runTest {
        val g = TestGraph()
        g.registry.rejectEnrollment = "boot_not_verified"
        val r = assertIs<EnrollmentStatus.Unsupported>(g.enrollment())
        assertEquals("boot_not_verified", r.reason)
        assertNull(g.vault.activeKey())
        assertEquals(1, g.vault.deleted.size)
    }

    @Test
    fun enrollNewWithoutLateFallbackReportsOffline() = runTest {
        val g = TestGraph()
        g.registry.offline = true
        assertIs<EnrollmentStatus.Offline>(g.enrollment.enrollNew(allowLate = false))
        assertNull(g.vault.activeKey())
    }

    @Test
    fun telemetryRecordsFreshnessOnly() = runTest {
        val g = TestGraph()
        g.enrollment()
        assertEquals(mapOf("fresh" to 1L), g.telemetry.events.single().second)
    }
}
