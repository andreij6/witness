package app.witness.feature.home

import app.witness.core.domain.fake.FakeAnchorScheduler
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeCrypto
import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCaseStore
import app.witness.core.domain.fake.InMemoryOutboxStore
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.usecase.Enrollment
import app.witness.core.domain.usecase.KeyRotation
import app.witness.core.model.Anchor
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Case
import app.witness.core.model.Dims
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationConfidence
import app.witness.core.model.RecordId
import app.witness.core.model.TemplateId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import kotlin.test.AfterTest
import kotlin.test.BeforeTest

const val MIDNIGHT: Long = 1_763_510_400_000
const val NOW: Long = MIDNIGHT + 16 * TimeText.HOUR_MS + 12 * TimeText.MINUTE_MS
const val MIN: Long = TimeText.MINUTE_MS
const val HOUR: Long = TimeText.HOUR_MS
const val DAY: Long = TimeText.DAY_MS

fun rid(n: Int): RecordId = RecordId.fromBytes(ByteArray(RecordId.SIZE).also { it[0] = (n shr 8).toByte(); it[1] = n.toByte() })

fun capture(
    n: Int,
    createdAtMs: Long = NOW - n * MIN,
    status: AnchoringStatus = AnchoringStatus.ANCHORED,
    origin: CaptureOrigin = CaptureOrigin.ORIGINAL,
    caseId: LocalCaseId? = null,
    parent: RecordId? = null,
    label: String? = null,
    caption: String? = null,
    place: String? = null,
    altText: String? = null,
    anchoredAtMs: Long? = if (status == AnchoringStatus.ANCHORED) createdAtMs else null,
) = Capture(
    recordId = rid(n),
    captureId = null,
    origin = origin,
    status = if (origin == CaptureOrigin.IMPORT) AnchoringStatus.UNVERIFIED else status,
    createdAtMs = createdAtMs,
    keyRef = null,
    locationConfidence = LocationConfidence.HIGH,
    dims = Dims(4032, 3024),
    caseId = caseId,
    parent = parent,
    anchor = anchoredAtMs?.let { Anchor(n.toLong(), it) },
    label = label,
    caption = caption,
    place = place,
    altText = altText,
)

fun case(id: Long, title: String, updatedAtMs: Long = NOW, summary: String? = null) =
    Case(LocalCaseId(id), title, TemplateId.GENERAL, updatedAtMs, updatedAtMs, summary)

fun utcTime(clock: FakeClock = FakeClock(NOW)) = TimeLabels(clock) { 0 }

class World {
    val clock = FakeClock(NOW)
    val time = utcTime(clock)
    val outbox = InMemoryOutboxStore()
    val captures = InMemoryCaptureStore(outbox)
    val cases = InMemoryCaseStore()
    val vault = FakeKeyVault(clock)
    val registry = FakeRegistry(clock)
    val scheduler = FakeAnchorScheduler()
    val settings = InMemorySettingsStore()

    fun enrollment(vault: KeyVault = this.vault) = Enrollment(vault, registry, outbox, FakeCrypto(), settings, SeededRandom(3), scheduler, clock)

    fun rotation() = KeyRotation(vault, enrollment(), clock)

    suspend fun add(vararg items: Capture) = items.forEach { captures.insert(it, null, null) }

    suspend fun newCase(title: String, summary: String? = null): Case {
        val c = cases.create(title, TemplateId.GENERAL, clock.nowMs)
        if (summary == null) return c
        return c.copy(summary = summary).also { cases.update(it) }
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
abstract class MainDispatcherTest {
    @BeforeTest
    fun setMainDispatcher() = Dispatchers.setMain(UnconfinedTestDispatcher())

    @AfterTest
    fun resetMainDispatcher() = Dispatchers.resetMain()
}
