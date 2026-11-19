package app.witness.feature.share

import app.witness.core.domain.fake.FakeBlurDetector
import app.witness.core.domain.fake.FakeCaseBundleCodec
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeCrypto
import app.witness.core.domain.fake.FakeEntitlementSource
import app.witness.core.domain.fake.FakeMedia
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCaseStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.usecase.ShareCase
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
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

fun record(n: Int): RecordId = RecordId.fromBytes(ByteArray(RecordId.SIZE) { (n * 7 + it).toByte() })

fun capture(
    n: Int,
    caseId: LocalCaseId?,
    createdAtMs: Long = NOW - (100 - n) * 1_000L,
    origin: CaptureOrigin = CaptureOrigin.ORIGINAL,
    status: AnchoringStatus = AnchoringStatus.ANCHORED,
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
)

class MemoryOutput : ExportOutput {
    val bytes = ArrayList<Byte>()
    var committed = false
    var aborted = false

    override suspend fun write(bytes: ByteArray, offset: Int, length: Int) {
        for (i in offset until offset + length) this.bytes += bytes[i]
    }

    override suspend fun commit(): String {
        committed = true
        return "memory://export"
    }

    override suspend fun abort() {
        aborted = true
    }

    fun toByteArray(): ByteArray = bytes.toByteArray()
}

class MemorySink : ExportSink {
    val opened = ArrayList<Pair<String, String>>()
    var last: MemoryOutput? = null

    override suspend fun open(name: String, mimeType: String): ExportOutput {
        opened += name to mimeType
        return MemoryOutput().also { last = it }
    }
}

class ShareFixture {
    val clock = FakeClock(NOW)
    val cases = InMemoryCaseStore()
    val captures = InMemoryCaptureStore()
    val media = FakeMedia()
    val registry = FakeRegistry(clock)
    val entitlements = FakeEntitlementSource()
    val detector = FakeBlurDetector()
    val shareCase = ShareCase(cases, captures, FakeCaseBundleCodec(), FakeCrypto(), media, registry, entitlements, SeededRandom(3), clock)

    suspend fun add(capture: Capture, jpeg: ByteArray? = byteArrayOf(-1, -40, capture.recordId.toBytes()[0])) {
        captures.insert(capture, null, null)
        if (jpeg != null) captures.putImage(capture.recordId, ByteString.of(jpeg))
    }
}
