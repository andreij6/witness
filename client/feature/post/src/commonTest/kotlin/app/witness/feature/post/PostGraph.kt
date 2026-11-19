package app.witness.feature.post

import app.witness.core.domain.fake.FakeAnchorScheduler
import app.witness.core.domain.fake.FakeAssistant
import app.witness.core.domain.fake.FakeBlurDetector
import app.witness.core.domain.fake.FakeCaptureSource
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.domain.fake.FakeLocationSource
import app.witness.core.domain.fake.FakeSealEngine
import app.witness.core.domain.fake.FakeTelemetry
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCounterStore
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.port.Links
import app.witness.core.domain.usecase.BuildPost
import app.witness.core.domain.usecase.SealCapture
import app.witness.core.domain.usecase.SealOutcome
import app.witness.core.domain.usecase.SealSigner
import app.witness.core.model.BlurProposal
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.Freshness
import app.witness.core.model.LocationOpening
import app.witness.core.model.Settings
import kotlinx.coroutines.flow.MutableStateFlow

class PostGraph(settings: Settings = Settings(), val links: Links = Links()) {
    val clock = FakeClock(16 * 3_600_000L + 12 * 60_000 + 9_000)
    val vault = FakeKeyVault(clock)
    val engine = FakeSealEngine()
    val signer = SealSigner(engine, InMemoryCounterStore())
    val captures = InMemoryCaptureStore()
    val settings = InMemorySettingsStore(settings)
    val detector = FakeBlurDetector()
    val assistant = FakeAssistant()
    val scheduler = FakeAnchorScheduler()
    val telemetry = FakeTelemetry()
    val ticks = MutableStateFlow(clock.nowMs)
    val seal = SealCapture(FakeCaptureSource(), FakeLocationSource(), engine, signer, vault, captures, this.settings, scheduler, clock, SeededRandom(5), telemetry)
    val post = BuildPost(captures, detector, assistant, this.settings, engine, signer, vault, scheduler, clock, links, telemetry)

    suspend fun parent(blurs: List<BlurProposal> = emptyList(), place: String? = null, opening: Boolean = true, image: ByteArray? = null): Capture {
        if (vault.activeKey() == null) {
            val g = vault.generate(ByteString.of(byteArrayOf(1)), Freshness.FRESH)
            vault.markEnrolled(g.key.keyId, clock.nowMs, clock.nowMs + 1_000_000_000, "1a", ByteString.EMPTY)
        }
        val sealed = (seal() as SealOutcome.Sealed).capture
        val c = sealed.copy(place = place)
        captures.update(c)
        detector.proposals = detector.proposals + (c.recordId to blurs)
        if (opening) captures.putOpening(c.recordId, LocationOpening(ByteString.of(ByteArray(32)), 397_392_000, -1_049_903_000, 500))
        if (image != null) captures.putImage(c.recordId, ByteString.of(image))
        return c
    }

    fun composer(parent: Capture, offset: Long = 0L) = ComposerViewModel(parent.recordId, post, captures, links, { offset }, telemetry)
}
