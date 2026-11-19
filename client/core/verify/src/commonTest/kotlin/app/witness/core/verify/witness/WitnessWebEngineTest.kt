package app.witness.core.verify.witness

import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.StageSink
import app.witness.core.domain.port.VerifyOracle
import app.witness.core.icp.witness.OpenedBundle
import app.witness.core.icp.witness.StageListener
import app.witness.core.icp.witness.WitnessBridge
import app.witness.core.model.ByteString
import app.witness.core.model.CheckStage
import app.witness.core.model.Platform
import app.witness.core.model.StageState
import app.witness.core.model.Verdict
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class WitnessWebEngineTest {
    private class Bridge(val json: String, val stages: List<IntArray>, val fail: Exception? = null) : WitnessBridge {
        var input: ByteArray? = null

        override suspend fun check(input: ByteArray, listener: StageListener): String {
            this.input = input
            stages.forEach { listener.onStage(it[0], it[1], it[2], it[3]) }
            fail?.let { throw it }
            return json
        }

        override suspend fun proof(recordId: String): String? = null

        override suspend fun key(keyIdHex: String): String? = null

        override suspend fun preview(recordId: String): ByteArray? = null

        override suspend fun openCase(caseId: String, fragment: String): OpenedBundle = OpenedBundle(OpenedBundle.NOT_FOUND, null, null)
    }

    private val unusedOracle = object : VerifyOracle {
        override suspend fun key(keyId: app.witness.core.model.KeyId) = error("unused")
        override suspend fun record(recordId: app.witness.core.model.RecordId) = error("unused")
        override suspend fun exact(assetHash: ByteString, pixelHash: ByteString?) = error("unused")
        override suspend fun byWatermark(captureId: app.witness.core.model.CaptureId) = error("unused")
        override suspend fun pdq(hashes: List<ByteString>) = error("unused")
        override suspend fun derivatives(root: app.witness.core.model.AnchoredRecord) = error("unused")
        override suspend fun reference(record: app.witness.core.model.AnchoredRecord) = error("unused")
    }

    private val options = JudgeOptions.forPlatform(Platform.WEB)

    @Test
    fun forwardsStagesAndParsesTheVerdict() = runTest {
        val bridge = Bridge(Verdicts.WITH_CHANGES, listOf(intArrayOf(0, 1, 0, 0), intArrayOf(2, 1, 1, 2), intArrayOf(9, 1, 0, 0), intArrayOf(2, 2, 2, 2)))
        val seen = mutableListOf<String>()
        val sink = StageSink { stage, state, done, total, detail -> seen += "$stage/$state/$done/$total/$detail" }
        val result = WitnessWebEngine(bridge).judge(ByteString.of(byteArrayOf(1, 2, 3)), unusedOracle, options, sink)
        assertEquals(Verdict.MATCHES_WITH_CHANGES, result.verdict)
        assertEquals(listOf<Byte>(1, 2, 3), bridge.input?.toList())
        assertEquals(listOf("READ_FILE/RUNNING/0/0/null", "FINGERPRINTS/RUNNING/1/2/null", "FINGERPRINTS/DONE/2/2/null"), seen)
    }

    @Test
    fun bridgeFailurePropagatesAfterStages() = runTest {
        val seen = mutableListOf<CheckStage>()
        val bridge = Bridge("", listOf(intArrayOf(0, 1, 0, 0)), IllegalStateException("offline"))
        assertFailsWith<IllegalStateException> {
            WitnessWebEngine(bridge).judge(ByteString.of(byteArrayOf(1)), unusedOracle, options) { s, _, _, _, _ -> seen += s }
        }
    }

    @Test
    fun stageUpdatesAreValidated() {
        assertEquals(StageState.SKIPPED, StageUpdate.decode(3, 3, 0, 0)?.state)
        assertEquals(CheckStage.COMPARE, StageUpdate.decode(4, 4, 0, 0)?.stage)
        assertNull(StageUpdate.decode(5, 0, 0, 0))
        assertNull(StageUpdate.decode(-1, 0, 0, 0))
        assertNull(StageUpdate.decode(0, 5, 0, 0))
        assertNull(StageUpdate.decode(2, 1, 3, 2))
        assertNull(StageUpdate.decode(2, 1, -1, 2))
    }
}
