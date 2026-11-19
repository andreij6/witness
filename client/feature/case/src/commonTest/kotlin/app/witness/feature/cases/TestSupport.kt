package app.witness.feature.cases

import app.witness.core.domain.fake.FakeAssistant
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeTemplateCatalog
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCaseStore
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.usecase.ManageCases
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Dims
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationConfidence
import app.witness.core.model.RecordId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestResult
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain

const val HOUR_MS: Long = 3_600_000L
const val NOW: Long = 1_763_510_400_000L + 14 * HOUR_MS + 32 * 60_000L

@OptIn(ExperimentalCoroutinesApi::class)
fun vmTest(block: suspend TestScope.() -> Unit): TestResult {
    Dispatchers.setMain(UnconfinedTestDispatcher())
    try {
        return runTest { block() }
    } finally {
        Dispatchers.resetMain()
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
fun TestScope.keep(flow: Flow<*>) {
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) { flow.collect {} }
}

fun record(n: Int): RecordId = RecordId.fromBytes(ByteArray(RecordId.SIZE) { (n + it).toByte() })

fun capture(
    n: Int,
    caseId: LocalCaseId?,
    status: AnchoringStatus = AnchoringStatus.ANCHORED,
    createdAtMs: Long = NOW - n * HOUR_MS,
    origin: CaptureOrigin = CaptureOrigin.ORIGINAL,
    place: String? = null,
) = Capture(
    recordId = record(n),
    captureId = null,
    origin = origin,
    status = status,
    createdAtMs = createdAtMs,
    keyRef = null,
    locationConfidence = LocationConfidence.HIGH,
    dims = Dims(4, 3),
    caseId = caseId,
    place = place,
)

class Fixture {
    val clock = FakeClock(NOW)
    val cases = InMemoryCaseStore()
    val captures = InMemoryCaptureStore()
    val settings = InMemorySettingsStore()
    val catalog = FakeTemplateCatalog()
    val assistant = FakeAssistant()
    val feedback = InMemoryAiFeedback()
    val manage = ManageCases(cases, captures, settings, catalog, assistant, clock)

    suspend fun add(vararg items: Capture) = items.forEach { captures.insert(it, null, null) }
}
