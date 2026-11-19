package app.witness.core.domain.usecase

import app.witness.core.domain.fake.FakeAnchorScheduler
import app.witness.core.domain.fake.FakeAssistant
import app.witness.core.domain.fake.FakeBlurDetector
import app.witness.core.domain.fake.FakeCaptureSource
import app.witness.core.domain.fake.FakeCaseBundleCodec
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeCrypto
import app.witness.core.domain.fake.FakeEntitlementSource
import app.witness.core.domain.fake.FakeIndex
import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.domain.fake.FakeLocationSource
import app.witness.core.domain.fake.FakeMedia
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.FakeSealEngine
import app.witness.core.domain.fake.FakeTelemetry
import app.witness.core.domain.fake.FakeTemplateCatalog
import app.witness.core.domain.fake.FakeVerifyEngine
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCaseStore
import app.witness.core.domain.fake.InMemoryCounterStore
import app.witness.core.domain.fake.InMemoryOutboxStore
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.port.CaptureFrame
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.DeviceKey
import app.witness.core.model.Platform
import kotlin.test.assertIs

class TestGraph {
    val clock = FakeClock()
    val random = SeededRandom(7)
    val crypto = FakeCrypto()
    val telemetry = FakeTelemetry()
    val scheduler = FakeAnchorScheduler()
    val outbox = InMemoryOutboxStore()
    val captures = InMemoryCaptureStore(outbox)
    val cases = InMemoryCaseStore()
    val settings = InMemorySettingsStore()
    val entitlements = FakeEntitlementSource()
    val counters = InMemoryCounterStore()
    val vault = FakeKeyVault(clock)
    val engine = FakeSealEngine()
    val signer = SealSigner(engine, counters)
    val source = FakeCaptureSource()
    val location = FakeLocationSource()
    val detector = FakeBlurDetector()
    val assistant = FakeAssistant()
    val catalog = FakeTemplateCatalog()
    val registry = FakeRegistry(clock) { engine.claim(it) }
    val index = FakeIndex()
    val media = FakeMedia()
    val verify = FakeVerifyEngine()
    val codec = FakeCaseBundleCodec()

    val seal = SealCapture(source, location, engine, signer, vault, captures, settings, scheduler, clock, random, telemetry)
    val anchor = AnchorOutbox(registry, outbox, captures, vault, engine, signer, clock, telemetry)
    val enrollment = Enrollment(vault, registry, outbox, crypto, settings, random, scheduler, clock, telemetry)
    val rotation = KeyRotation(vault, enrollment, clock, telemetry)
    val post = BuildPost(captures, detector, assistant, settings, engine, signer, vault, scheduler, clock, telemetry = telemetry)
    val manageCases = ManageCases(cases, captures, settings, catalog, assistant, clock, telemetry)
    val share = ShareCase(cases, captures, codec, crypto, media, registry, entitlements, random, clock, telemetry = telemetry)
    val erase = EraseMine(captures, registry, vault, random, telemetry)

    fun check(platform: Platform = Platform.ANDROID) = CheckImage(verify, registry, index, media, platform, telemetry)

    suspend fun enrolledKey(): DeviceKey = assertIs<EnrollmentStatus.Enrolled>(enrollment()).key

    suspend fun sealOne(frame: CaptureFrame? = null): Capture = assertIs<SealOutcome.Sealed>(seal(frame)).capture

    suspend fun sealAnchored(): Capture {
        val c = sealOne()
        anchor()
        return captures.get(c.recordId)!!
    }

    fun frame(vararg bytes: Byte) = CaptureFrame(ByteString.of(bytes), 0, 5, 1)
}
