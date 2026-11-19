package app.witness.core.data

import app.witness.core.domain.fake.FakeAnchorScheduler
import app.witness.core.domain.fake.FakeCaptureSource
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.domain.fake.FakeLocationSource
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.FakeSealEngine
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.port.EnrollmentRequest
import app.witness.core.domain.port.SubmitResult
import app.witness.core.domain.usecase.AnchorOutbox
import app.witness.core.domain.usecase.SealCapture
import app.witness.core.domain.usecase.SealOutcome
import app.witness.core.domain.usecase.SealSigner
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Freshness
import app.witness.core.model.OutboxState
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], manifest = Config.NONE)
class AnchorIntegrationTest {
    private val db = inMemoryDb()
    private val clock = FakeClock()
    private val captures = RoomCaptureStore(db, testCipher(), tempImages())
    private val outbox = RoomOutboxStore(db)
    private val engine = FakeSealEngine()
    private val signer = SealSigner(engine, RoomCounterStore(db))
    private val vault = FakeKeyVault(clock)
    private val registry = FakeRegistry(clock) { engine.claim(it) }
    private val scheduler = FakeAnchorScheduler()
    private val seal = SealCapture(FakeCaptureSource(), FakeLocationSource(), engine, signer, vault, captures, InMemorySettingsStore(), scheduler, clock, SeededRandom())
    private val anchor = AnchorOutbox(registry, outbox, captures, vault, engine, signer, clock)

    @After
    fun close() = db.close()

    private suspend fun enrolledKey() {
        val generated = vault.generate(ByteString.of(byteArrayOf(1)), Freshness.FRESH)
        val key = generated.key
        registry.enroll(EnrollmentRequest(key.keyId, generated.attestationChain, Freshness.FRESH, generated.challenge))
        vault.markEnrolled(key.keyId, clock.nowEpochMillis(), clock.nowEpochMillis() + 1_000_000, "01", ByteString.EMPTY)
    }

    @Test
    fun sealedCapturesAnchorInCounterOrder() = runTest {
        enrolledKey()
        val sealed = List(5) { (seal() as SealOutcome.Sealed).capture }
        assertEquals(listOf(0L, 1L, 2L, 3L, 4L), sealed.map { it.keyRef!!.counter })
        assertEquals(5, outbox.all().size)
        val report = anchor()
        assertEquals(5, report.anchored + report.duplicates)
        assertFalse(report.shouldRetry)
        assertTrue(outbox.all().isEmpty())
        assertEquals(listOf(0L, 1L, 2L, 3L, 4L), registry.submitted.flatten().map { it.keyRef.counter })
        sealed.forEach { assertEquals(AnchoringStatus.ANCHORED, captures.get(it.recordId)!!.status) }
    }

    @Test
    fun largeRoomBacklogIsSubmittedInBatchesOfAtMost64InCounterOrder() = runTest {
        enrolledKey()
        repeat(130) { seal() }
        val report = anchor()
        assertEquals(130, report.anchored)
        assertEquals(listOf(64, 64, 2), registry.submitted.map { it.size })
        assertEquals((0L until 130L).toList(), registry.submitted.flatten().map { it.keyRef.counter })
        assertTrue(outbox.all().isEmpty())
    }

    @Test
    fun countersSurviveAReopenedStore() = runTest {
        enrolledKey()
        repeat(3) { seal() }
        val reopened = SealSigner(engine, RoomCounterStore(db))
        val again = SealCapture(FakeCaptureSource(), FakeLocationSource(), engine, reopened, vault, captures, InMemorySettingsStore(), scheduler, clock, SeededRandom(9))
        assertEquals(3L, (again() as SealOutcome.Sealed).capture.keyRef!!.counter)
    }

    @Test
    fun sealedImageLandsInAppPrivateFiles() = runTest {
        enrolledKey()
        val c = (seal() as SealOutcome.Sealed).capture
        assertTrue(captures.readImage(c.recordId) != null)
    }

    @Test
    fun offlineKeepsQueue() = runTest {
        enrolledKey()
        seal()
        registry.offline = true
        val report = anchor()
        assertTrue(report.offline)
        assertTrue(report.shouldRetry)
        assertEquals(1, outbox.all().size)
    }

    @Test
    fun retryAfterAndRejectionPersist() = runTest {
        enrolledKey()
        val a = (seal() as SealOutcome.Sealed).capture
        val b = (seal() as SealOutcome.Sealed).capture
        registry.scripted += SubmitResult.RetryAfter(5_000)
        registry.scripted += SubmitResult.Rejected("bad_sig")
        anchor()
        val rows = outbox.all().associateBy { it.recordId }
        assertEquals(clock.nowEpochMillis() + 5_000, rows.getValue(a.recordId).notBeforeMs)
        assertEquals(OutboxState.FAILED, rows.getValue(b.recordId).state)
        assertEquals(AnchoringStatus.FAILED, captures.get(b.recordId)!!.status)
    }

    @Test
    fun unenrolledKeySealsButWaits() = runTest {
        vault.generate(ByteString.of(byteArrayOf(1)), Freshness.LATE)
        val c = (seal() as SealOutcome.Sealed).capture
        assertEquals(AnchoringStatus.SEALED, captures.get(c.recordId)!!.status)
        assertEquals(OutboxState.AWAITING_ENROLLMENT, outbox.all().single().state)
        assertEquals(0, anchor().anchored)
        assertTrue(registry.submitted.isEmpty())
    }

    @Test
    fun noKeyNoSeal() = runTest {
        assertIs<SealOutcome.NoKey>(seal())
        assertTrue(captures.derivativesOf(app.witness.core.model.RecordId.fromBytes(ByteArray(10))).isEmpty())
    }
}
