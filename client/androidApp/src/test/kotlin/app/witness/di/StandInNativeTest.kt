package app.witness.di

import app.witness.core.data.ImageFiles
import app.witness.core.domain.fake.FakeAnchorScheduler
import app.witness.core.domain.fake.FakeCaptureSource
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeCrypto
import app.witness.core.domain.fake.FakeIndex
import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.domain.fake.FakeLocationSource
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.FakeSealEngine
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCounterStore
import app.witness.core.domain.fake.InMemoryOutboxStore
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.port.CaptureFrame
import app.witness.core.domain.port.EnrollOutcome
import app.witness.core.domain.port.EnrollmentRequest
import app.witness.core.domain.port.PrepareRequest
import app.witness.core.domain.port.SubmitResult
import app.witness.core.domain.usecase.AnchorOutbox
import app.witness.core.domain.usecase.CheckImage
import app.witness.core.domain.usecase.Enrollment
import app.witness.core.domain.usecase.EnrollmentStatus
import app.witness.core.domain.usecase.SealCapture
import app.witness.core.domain.usecase.SealOutcome
import app.witness.core.domain.usecase.SealSigner
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.BootState
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.Checks
import app.witness.core.model.DeviceKey
import app.witness.core.model.Edit
import app.witness.core.model.EnumBits
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.LocationEvidence
import app.witness.core.model.Platform
import app.witness.core.model.Provisioning
import app.witness.core.model.RecordId
import app.witness.core.model.Rect
import app.witness.core.model.SecurityLevel
import app.witness.core.model.TimeEvidence
import app.witness.core.model.Verdict
import app.witness.di.StandInNative.toDevInfo
import kotlinx.coroutines.flow.last
import kotlinx.coroutines.test.runTest
import java.nio.file.Files
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

fun env(clock: FakeClock, images: ImageFiles, vault: FakeKeyVault = FakeKeyVault(clock)) =
    NativeEnvironment(clock, images, vault, Files.createTempDirectory("witness-files").toFile(), SeededRandom(1))

val CHECKS = Checks(SecurityLevel.TEE, BootState.VERIFIED, true, false, EnumBits.none(), 0)

class StandInNativeTest {
    private val clock = FakeClock()
    private val images = ImageFiles(Files.createTempDirectory("witness-images").toFile())
    private val ports = StandInNative.create(env(clock, images))

    private inner class World(val ports: NativePorts) {
        val vault = FakeKeyVault(clock)
        val outbox = InMemoryOutboxStore()
        val captures = InMemoryCaptureStore(outbox)
        val settings = InMemorySettingsStore()
        val scheduler = FakeAnchorScheduler()
        val random = SeededRandom(3)
        val signer = SealSigner(ports.seal, InMemoryCounterStore())
        val source = FakeCaptureSource(CaptureFrame(ByteString.of(byteArrayOf(-1, -40, -1, 7, 7, 7)), 0, 10, 1))
        val enrollment = Enrollment(vault, ports.registry, outbox, FakeCrypto(), settings, random, scheduler, clock)
        val seal = SealCapture(source, FakeLocationSource(), ports.seal, signer, vault, captures, settings, scheduler, clock, random)
        val anchor = AnchorOutbox(ports.registry, outbox, captures, vault, ports.seal, signer, clock)
        val check = CheckImage(ports.verify, ports.registry, ports.index, ports.media, Platform.ANDROID)
    }

    @Test
    fun standInIsNotLinkedAndHasEveryPort() {
        assertFalse(ports.linked)
        assertIs<ImageKeepingSealEngine>(ports.seal)
        assertIs<LoopbackRegistry>(ports.registry)
        assertIs<FakeIndex>(ports.index)
    }

    @Test
    fun factoryIsTheSingleSwitch() {
        assertSame(StandInNative, NativeAvailability.factory)
    }

    @Test
    fun sealedPhotoIsStoredAnchoredAndCheckableEndToEnd() = runTest {
        val w = World(ports)
        assertIs<EnrollmentStatus.Enrolled>(w.enrollment())
        val capture = assertIs<SealOutcome.Sealed>(w.seal()).capture
        val stored = assertNotNull(images.read(capture.recordId))
        w.anchor()
        assertEquals(AnchoringStatus.ANCHORED, w.captures.get(capture.recordId)?.status)
        assertEquals(Verdict.ORIGINAL, w.check(stored).last().result?.verdict)
    }

    @Test
    fun unknownBytesHaveNoRecord() = runTest {
        val w = World(ports)
        w.enrollment()
        w.seal()
        w.anchor()
        assertEquals(Verdict.NO_RECORD, w.check(ByteString.of(byteArrayOf(-1, -40, -1, 1))).last().result?.verdict)
    }

    @Test
    fun unanchoredPhotoIsNotYetCheckable() = runTest {
        val w = World(ports)
        w.enrollment()
        val capture = assertIs<SealOutcome.Sealed>(w.seal()).capture
        val stored = assertNotNull(images.read(capture.recordId))
        assertEquals(Verdict.NO_RECORD, w.check(stored).last().result?.verdict)
    }

    @Test
    fun restoreSeedsTheEnrolledKeyAsDev() = runTest {
        val w = World(ports)
        val key = assertIs<EnrollmentStatus.Enrolled>(w.enrollment()).key
        val restarted = StandInNative.create(env(clock, images))
        assertNull(restarted.registry.getKey(key.keyId))
        restarted.restore(w.vault)
        assertEquals(Provisioning.DEV, restarted.registry.getKey(key.keyId)?.provisioning)
    }

    @Test
    fun inProcessEnrollmentIsDevWithTheVaultSecurity() = runTest {
        val vault = FakeKeyVault(clock, SecurityLevel.TEE)
        val standIn = StandInNative.create(env(clock, images, vault))
        val generated = vault.generate(ByteString.of(byteArrayOf(1)), Freshness.FRESH)
        val outcome = assertIs<EnrollOutcome.Enrolled>(
            standIn.registry.enroll(EnrollmentRequest(generated.key.keyId, generated.attestationChain, Freshness.FRESH, generated.challenge)),
        )
        assertEquals(Provisioning.DEV, outcome.info.provisioning)
        assertEquals(SecurityLevel.TEE, outcome.info.security)
        val stored = standIn.registry.getKey(generated.key.keyId)!!
        assertEquals(Provisioning.DEV, stored.provisioning)
        assertTrue(stored.isDev)
    }

    @Test
    fun restoreWithoutKeyIsANoOp() = runTest {
        ports.restore(FakeKeyVault(clock))
    }

    @Test
    fun devInfoNeedsAnEnrolment() {
        val key = DeviceKey(KeyId.fromDigest(ByteArray(32)), SecurityLevel.TEE, Freshness.FRESH, 1)
        assertNull(key.toDevInfo())
        val enrolled = key.copy(enrolledAtMs = 5, activeUntilMs = 50).toDevInfo()!!
        assertEquals(5, enrolled.enrolledAtMs)
        assertEquals(50, enrolled.activeUntilMs)
        assertEquals(Provisioning.DEV, enrolled.provisioning)
        assertEquals(5, key.copy(enrolledAtMs = 5).toDevInfo()!!.activeUntilMs)
    }
}

class ImageKeepingSealEngineTest {
    private val clock = FakeClock()
    private val images = ImageFiles(Files.createTempDirectory("witness-keep").toFile())
    private val fake = FakeSealEngine()
    private val rotations = ArrayList<Int>()
    private val engine = ImageKeepingSealEngine(fake, images, upright = { jpeg, degrees ->
        rotations += degrees
        if (degrees == 0) jpeg else ByteString.of(jpeg.toByteArray() + degrees.toByte())
    })
    private val key = DeviceKey(KeyId.fromDigest(ByteArray(32) { 1 }), SecurityLevel.TEE, Freshness.FRESH, 0, enrolledAtMs = 0, activeUntilMs = Long.MAX_VALUE)
    private val signer = SealSigner(engine, InMemoryCounterStore())

    private fun request(bytes: ByteArray, rotation: Int = 0) = PrepareRequest(
        CaptureFrame(ByteString.of(bytes), rotation, 1, 1),
        CaptureId.of(42, 1)!!,
        TimeEvidence(clock.nowEpochMillis(), null, 1, 1),
        LocationEvidence.NONE,
        CHECKS,
    )

    @Test
    fun sealWritesTheUprightFrameUnderTheRecordId() = runTest {
        val sealed = signer.seal(engine.prepare(request(byteArrayOf(1, 2, 3), 90)), key)
        assertContentEquals(byteArrayOf(1, 2, 3, 90), images.read(sealed.recordId)!!.toByteArray())
        assertEquals(listOf(90), rotations)
        assertEquals(0, engine.pending)
    }

    @Test
    fun assetHashCoversTheStoredBytes() = runTest {
        val sealed = signer.seal(engine.prepare(request(byteArrayOf(9, 9), 180)), key)
        val stored = images.read(sealed.recordId)!!.toByteArray()
        val digest = java.security.MessageDigest.getInstance("SHA-256").digest(stored)
        assertContentEquals(digest, sealed.claim.assetHash.toByteArray())
    }

    @Test
    fun derivativeReusesTheParentImage() = runTest {
        val parent = signer.seal(engine.prepare(request(byteArrayOf(4, 5, 6))), key)
        val child = signer.seal(engine.derive(parent.recordId, listOf<Edit>(), null), key)
        assertContentEquals(byteArrayOf(4, 5, 6), images.read(child.recordId)!!.toByteArray())
    }

    @Test
    fun derivativeStoresTheRenderedEdits() = runTest {
        val seen = ArrayList<List<Edit>>()
        val rendering = ImageKeepingSealEngine(fake, images, render = { jpeg, edits -> seen += edits; ByteString.of(jpeg.toByteArray() + 7) })
        val signing = SealSigner(rendering, InMemoryCounterStore())
        val parent = signing.seal(rendering.prepare(request(byteArrayOf(4, 5))), key)
        val blur = Edit.Blur(Rect(0, 0, 1, 1), BlurClass.FACE, 400)
        val child = signing.seal(rendering.derive(parent.recordId, listOf(blur), null), key)
        assertContentEquals(byteArrayOf(4, 5, 7), images.read(child.recordId)!!.toByteArray())
        assertEquals(listOf(listOf<Edit>(blur)), seen)
        assertEquals(0, rendering.pending)
    }

    @Test
    fun resealDoesNotCopyImages() = runTest {
        val parent = signer.seal(engine.prepare(request(byteArrayOf(4))), key)
        val again = signer.seal(engine.reseal(parent.recordId), key)
        if (again.recordId != parent.recordId) assertNull(images.read(again.recordId))
        assertEquals(0, engine.pending)
    }

    @Test
    fun failedSealLeavesNothingPending() = runTest {
        val prepared = engine.prepare(request(byteArrayOf(1)))
        val first = signer.seal(prepared, key)
        assertFailsWith<IllegalStateException> { engine.seal(prepared, first.claim) }
        assertEquals(0, engine.pending)
    }

    @Test
    fun unknownParentFails() = runTest {
        assertFailsWith<IllegalStateException> { engine.derive(RecordId.fromDigest(ByteArray(32) { 7 }), emptyList(), null) }
        assertEquals(0, engine.pending)
    }
}

class LoopbackRegistryTest {
    private val clock = FakeClock()
    private val fake = FakeSealEngine()
    private val index = FakeIndex()
    private val inner = FakeRegistry(clock, fake::claim)
    private val registry = LoopbackRegistry(inner, index, fake::claim)
    private val key = DeviceKey(KeyId.fromDigest(ByteArray(32) { 2 }), SecurityLevel.TEE, Freshness.FRESH, 0, enrolledAtMs = 0, activeUntilMs = Long.MAX_VALUE)
    private val signer = SealSigner(fake, InMemoryCounterStore())

    private suspend fun sealed(bytes: ByteArray) = signer.seal(
        fake.prepare(
            PrepareRequest(
                CaptureFrame(ByteString.of(bytes), 0, 1, 1), CaptureId.of(bytes.size.toLong(), 1)!!,
                TimeEvidence(0, null, 1, 1), LocationEvidence.NONE, CHECKS,
            ),
        ),
        key,
    )

    @Test
    fun anchoredRecordsAreIndexedByExactHash() = runTest {
        inner.putKey(key.toDevInfo()!!)
        val s = sealed(byteArrayOf(1, 2))
        val result = assertIs<SubmitResult.Anchored>(registry.submit(listOf(s.submission)).single())
        assertEquals<List<Long>?>(listOf(result.anchor.recordIx), index.exact[StandInNative.SHARD]!![s.claim.assetHash])
    }

    @Test
    fun duplicatesAreNotIndexedTwice() = runTest {
        inner.putKey(key.toDevInfo()!!)
        val s = sealed(byteArrayOf(3))
        registry.submit(listOf(s.submission))
        assertIs<SubmitResult.Duplicate>(registry.submit(listOf(s.submission)).single())
        assertEquals(1, index.exact[StandInNative.SHARD]!![s.claim.assetHash]!!.size)
    }

    @Test
    fun rejectedSubmissionsAreNotIndexed() = runTest {
        val s = sealed(byteArrayOf(5))
        assertIs<SubmitResult.Rejected>(registry.submit(listOf(s.submission)).single())
        assertNull(index.exact[StandInNative.SHARD])
    }

    @Test
    fun emptyBatchIsEmpty() = runTest {
        assertEquals(emptyList(), registry.submit(emptyList()))
    }

    @Test
    fun readsDelegate() = runTest {
        inner.putKey(key.toDevInfo()!!)
        val s = sealed(byteArrayOf(6))
        registry.submit(listOf(s.submission))
        assertEquals(s.recordId, registry.get(s.recordId)?.recordId)
        assertNotNull(registry.getKey(key.keyId))
    }
}
