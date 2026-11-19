package app.witness.core.domain.usecase

import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.LocationReading
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureMode
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.EnumBits
import app.witness.core.model.Freshness
import app.witness.core.model.GnssCheck
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationConfidence
import app.witness.core.model.LocationEvidence
import app.witness.core.model.LocationOpening
import app.witness.core.model.LocationReason
import app.witness.core.model.OutboxState
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SealCaptureTest {
    private val opening = LocationOpening(ByteString.of(ByteArray(32) { 3 }), 397_392_000, -1_049_903_000, 500)

    @Test
    fun noKeyMeansNoSeal() = runTest {
        val g = TestGraph()
        assertIs<SealOutcome.NoKey>(g.seal())
        assertEquals(0, g.source.shots)
        assertEquals(0, g.engine.prepareCalls)
    }

    @Test
    fun retiredKeyCannotSeal() = runTest {
        val g = TestGraph()
        val key = g.enrolledKey()
        g.vault.retire(key.keyId)
        assertIs<SealOutcome.NoKey>(g.seal())
    }

    @Test
    fun sealedJpegIsPersistedWithTheCapture() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne(g.frame(-1, -40, 7, 7))
        assertEquals(ByteString.of(byteArrayOf(-1, -40, 7, 7)), g.captures.readImage(c.recordId))
    }

    @Test
    fun enrolledSealQueuesForAnchoring() = runTest {
        val g = TestGraph()
        val key = g.enrolledKey()
        val requestsBefore = g.scheduler.requests
        val c = g.sealOne()
        assertEquals(AnchoringStatus.QUEUED, c.status)
        assertEquals(CaptureOrigin.ORIGINAL, c.origin)
        assertEquals(key.keyId, c.keyRef?.keyId)
        assertEquals(0, c.keyRef?.counter)
        assertEquals(g.clock.nowMs, c.createdAtMs)
        val entry = assertNotNull(g.outbox[c.recordId])
        assertEquals(OutboxState.PENDING, entry.state)
        assertTrue(entry.submission.isOriginal)
        assertEquals(requestsBefore + 1, g.scheduler.requests)
        assertEquals(c, g.captures.get(c.recordId))
    }

    @Test
    fun unenrolledKeySealsButWaitsForEnrollment() = runTest {
        val g = TestGraph()
        g.registry.offline = true
        assertIs<EnrollmentStatus.Pending>(g.enrollment())
        val requestsBefore = g.scheduler.requests
        val c = g.sealOne()
        assertEquals(AnchoringStatus.SEALED, c.status)
        assertEquals(OutboxState.AWAITING_ENROLLMENT, g.outbox[c.recordId]?.state)
        assertEquals(requestsBefore, g.scheduler.requests)
        assertEquals(Freshness.LATE, g.vault.activeKey()?.freshness)
    }

    @Test
    fun countersAreUniqueUnderConcurrency() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val captures = (1..20).map { async { g.sealOne() } }.awaitAll()
        val counters = captures.map { it.keyRef!!.counter }
        assertEquals(20, counters.toSet().size)
        assertEquals((0L until 20L).toSet(), counters.toSet())
        assertEquals(20, captures.map { it.recordId }.toSet().size)
        assertEquals(20, g.outbox.size)
    }

    @Test
    fun captureIdsAreRandomAndValid() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val ids = List(10) { g.sealOne().captureId!! }
        assertEquals(10, ids.toSet().size)
        ids.forEach { assertEquals(SealCapture.CAPTURE_SCHEMA, it.schema) }
    }

    @Test
    fun caseModeFilesIntoActiveCase() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        g.settings.update { it.copy(captureMode = CaptureMode.CASE, activeCase = LocalCaseId(9)) }
        assertEquals(LocalCaseId(9), g.sealOne().caseId)
        g.settings.update { it.copy(captureMode = CaptureMode.POST) }
        assertNull(g.sealOne().caseId)
    }

    @Test
    fun locationEvidenceFlowsIntoClaimAndOpeningIsStoredLocally() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val evidence = LocationEvidence(ByteString.of(ByteArray(32)), LocationConfidence.HIGH, EnumBits.of(LocationReason.RAW_GNSS_PASSED))
        g.location.reading = LocationReading(evidence, opening, g.clock.nowMs + 10, EnumBits.of(GnssCheck.RAW_AVAILABLE, GnssCheck.GNSS_TIME))
        val c = g.sealOne()
        assertEquals(LocationConfidence.HIGH, c.locationConfidence)
        assertEquals(opening, g.captures.opening(c.recordId))
        val claim = assertNotNull(g.engine.claim(c.recordId))
        assertEquals(g.clock.nowMs + 10, claim.time.gnssMs)
        assertTrue(GnssCheck.GNSS_TIME in claim.checks.gnss)
        assertFalse(claim.checks.mockLocation)
    }

    @Test
    fun mockLocationIsRecordedInChecks() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        g.location.reading = LocationReading(LocationEvidence(null, LocationConfidence.LOW, EnumBits.of(LocationReason.MOCK_LOCATION)), null, null, EnumBits.none())
        val c = g.sealOne()
        assertTrue(g.engine.claim(c.recordId)!!.checks.mockLocation)
    }

    @Test
    fun locationFailureFallsBackToNoFix() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        g.location.failure = IllegalStateException("gnss off")
        val c = g.sealOne()
        assertEquals(LocationConfidence.LOW, c.locationConfidence)
        assertTrue(LocationReason.NO_FIX in g.engine.claim(c.recordId)!!.location.reasons)
        assertEquals(1, g.telemetry.errors.size)
        assertNull(g.captures.opening(c.recordId))
    }

    @Test
    fun explicitFrameSkipsCameraSource() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        g.sealOne(g.frame(1, 2, 3))
        assertEquals(0, g.source.shots)
    }

    @Test
    fun telemetryEventCarriesNoIdentifiers() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val c = g.sealOne()
        val (event, metrics) = g.telemetry.events.last()
        assertEquals(AnalyticsEvent.CAPTURE_SEALED, event)
        assertEquals(setOf("location_confidence"), metrics.keys)
        assertFalse(metrics.values.any { it.toString().contains(c.recordId.text) })
    }

    @Test
    fun deterministicGivenSameInputs() = runTest {
        val a = TestGraph().apply { enrolledKey() }.sealOne(TestGraph().frame(9, 9))
        val b = TestGraph().apply { enrolledKey() }.sealOne(TestGraph().frame(9, 9))
        assertEquals(a.recordId, b.recordId)
        assertEquals(a.captureId, b.captureId)
    }
}
