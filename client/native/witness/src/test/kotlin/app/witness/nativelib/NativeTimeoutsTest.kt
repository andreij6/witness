package app.witness.nativelib

import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.ProbeQuery
import app.witness.core.domain.port.ShardInfo
import app.witness.core.domain.port.StageSink
import app.witness.core.domain.port.VerifyOracle
import app.witness.core.model.AnchoredRecord
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.KeyId
import app.witness.core.model.Platform
import app.witness.core.model.RecordId
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.test.currentTime
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withTimeout
import uniffi.witness_android.WitnessException
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class NativeTimeoutsTest {
    private val h = Harness()
    private val registry = NativeRegistry(h.witness, h.keys, h.trust) { Vectors.claimView(ByteArray(16) { 5 }) }

    @Test
    fun answersPassThroughIncludingNull() = runTest {
        assertEquals(7, remote { 7 })
        assertNull(remote<String?> { null })
        assertEquals(0L, currentTime)
    }

    @Test
    fun aSilentReplicaBecomesOfflineAfterTheCallTimeout() = runTest {
        val e = assertFailsWith<OfflineException> { remote { awaitCancellation() } }
        assertEquals(NativeTimeouts.CALL_MS, currentTime)
        assertTrue("can't reach the Witness network" in e.message.orEmpty(), e.message)
    }

    @Test
    fun longCallsGetTheLongTimeout() = runTest {
        assertFailsWith<OfflineException> { remote(NativeTimeouts.LONG_MS) { awaitCancellation() } }
        assertEquals(NativeTimeouts.LONG_MS, currentTime)
    }

    @Test
    fun nativeErrorsAreStillMapped() = runTest {
        assertIs<OfflineException>(assertFailsWith<OfflineException> { remote { throw WitnessException.Network("refused") } })
        assertEquals("rejected", assertFailsWith<NativeException> { remote { throw WitnessException.Rejected("no") } }.code)
        assertFailsWith<IllegalStateException> { remote { error("boom") } }
    }

    @Test
    fun anOuterCancellationIsNotTurnedIntoOffline() = runTest {
        assertFailsWith<CancellationException> { withTimeout(1_000) { remote { awaitCancellation() } } }
        assertEquals(1_000L, currentTime)
    }

    @Test
    fun registryCallsFailOfflineInsteadOfHanging() = runTest {
        h.witness.hang = true
        assertFailsWith<OfflineException> { registry.challenge() }
        assertFailsWith<OfflineException> { registry.get(RecordId.fromDigest(ByteArray(32))) }
        assertFailsWith<OfflineException> { registry.shards() }
        assertFailsWith<OfflineException> { registry.byCapture(CaptureId.fromLong(5)!!) }
        assertEquals(4 * NativeTimeouts.CALL_MS, currentTime)
    }

    @Test
    fun checksAndProbesFailOfflineInsteadOfHanging() = runTest {
        h.witness.hang = true
        assertFailsWith<OfflineException> { NativeIndex(h.witness).probe(ShardInfo(0, 0), ProbeQuery()) }
        assertEquals(NativeTimeouts.CALL_MS, currentTime)
        assertFailsWith<OfflineException> {
            NativeVerifyEngine(h.witness).judge(ByteString.of(byteArrayOf(1)), NoOracle, JudgeOptions.forPlatform(Platform.ANDROID), StageSink { _, _, _, _, _ -> })
        }
        assertEquals(NativeTimeouts.CALL_MS + NativeTimeouts.LONG_MS, currentTime)
    }

    private object NoOracle : VerifyOracle {
        override suspend fun key(keyId: KeyId) = error("unused")
        override suspend fun record(recordId: RecordId) = error("unused")
        override suspend fun exact(assetHash: ByteString, pixelHash: ByteString?) = error("unused")
        override suspend fun byWatermark(captureId: CaptureId) = error("unused")
        override suspend fun pdq(hashes: List<ByteString>) = error("unused")
        override suspend fun derivatives(root: AnchoredRecord) = error("unused")
        override suspend fun reference(record: AnchoredRecord) = error("unused")
    }
}
