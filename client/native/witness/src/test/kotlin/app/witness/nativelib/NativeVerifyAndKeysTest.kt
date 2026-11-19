package app.witness.nativelib

import app.witness.core.domain.fake.FakeKeyVault
import app.witness.core.domain.port.JudgeOptions
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.StageSink
import app.witness.core.domain.port.VerifyOracle
import app.witness.core.model.ByteString
import app.witness.core.model.CheckResult
import app.witness.core.model.CheckStage
import app.witness.core.model.EnumBits
import app.witness.core.model.KeyId
import app.witness.core.model.Platform
import app.witness.core.model.StageState
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictReason
import kotlinx.coroutines.test.runTest
import uniffi.witness_android.Outcome
import uniffi.witness_android.Reason
import uniffi.witness_android.VerdictView
import uniffi.witness_android.WitnessException
import java.io.File
import java.math.BigInteger
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class NativeVerifyAndKeysTest {
    private val h = Harness()
    private val unusedOracle = object : VerifyOracle {
        override suspend fun key(keyId: KeyId) = error("native check owns the oracle")
        override suspend fun record(recordId: app.witness.core.model.RecordId) = error("unused")
        override suspend fun exact(assetHash: ByteString, pixelHash: ByteString?) = error("unused")
        override suspend fun byWatermark(captureId: app.witness.core.model.CaptureId) = error("unused")
        override suspend fun pdq(hashes: List<ByteString>) = error("unused")
        override suspend fun derivatives(root: app.witness.core.model.AnchoredRecord) = error("unused")
        override suspend fun reference(record: app.witness.core.model.AnchoredRecord) = error("unused")
    }

    private fun view(outcome: Outcome, vararg reasons: Reason) = VerdictView(1u, outcome, emptyList(), reasons.toList(), emptyList(), emptyList(), null)

    @Test
    fun checkRunsNativeAndReportsStages() = runTest {
        h.witness.checkAnswer = { view(Outcome.SEALED_UNANCHORED, Reason.SEAL_VERIFIED, Reason.WATERMARK_UNAVAILABLE) }
        val updates = ArrayList<Pair<CheckStage, StageState>>()
        val r = NativeVerifyEngine(h.witness).judge(ByteString.of(byteArrayOf(1)), unusedOracle, JudgeOptions.forPlatform(Platform.ANDROID), StageSink { s, st, _, _, _ -> updates += s to st })
        assertEquals(Verdict.SEALED_UNANCHORED, r.verdict)
        assertEquals(CheckStage.READ_FILE to StageState.RUNNING, updates.first())
        assertEquals(StageState.SKIPPED, updates.last { it.first == CheckStage.WATERMARK }.second)
        assertEquals(StageState.SKIPPED, updates.last { it.first == CheckStage.COMPARE }.second)
    }

    @Test
    fun checkNetworkFailureIsOffline() = runTest {
        h.witness.failure = WitnessException.Network("down")
        assertFailsWith<OfflineException> {
            NativeVerifyEngine(h.witness).judge(ByteString.of(byteArrayOf(1)), unusedOracle, JudgeOptions.forPlatform(Platform.ANDROID), StageSink { _, _, _, _, _ -> })
        }
    }

    @Test
    fun undecodableInputFailsTheReadStage() {
        val stages = NativeVerifyEngine.stagesOf(CheckResult(Verdict.NO_RECORD, reasons = EnumBits.of(VerdictReason.UNDECODABLE))).toMap()
        assertEquals(StageState.FAILED, stages[CheckStage.READ_FILE])
        assertTrue(stages.filterKeys { it != CheckStage.READ_FILE }.values.all { it == StageState.SKIPPED })
        assertEquals(CheckStage.entries.toSet(), stages.keys)
    }

    @Test
    fun trustFetchesOncePersistsAndServesOffline() = runTest {
        assertContentEquals(Vectors.CA, h.trust.caRoot())
        assertContentEquals(Vectors.CA, h.trust.caRoot())
        assertEquals(1, h.witness.caRootCalls)
        h.witness.failure = WitnessException.Network("down")
        val reopened = NativeTrust(h.witness, File(h.dir, "ca.der"))
        assertContentEquals(Vectors.CA, reopened.caRoot())
        reopened.remember(byteArrayOf(1))
        assertContentEquals(byteArrayOf(1), NativeTrust(h.witness, File(h.dir, "ca.der")).caRoot())
    }

    @Test
    fun aPinnedAnchorIsUsedWithoutAskingTheRegistry() = runTest {
        val pinned = byteArrayOf(0x30, 0x01, 0x05)
        val trust = NativeTrust(h.witness, File(h.dir, "pinned.der"), pinned)
        assertContentEquals(pinned, trust.caRoot())
        h.witness.failure = WitnessException.Network("down")
        assertContentEquals(pinned, trust.caRoot())
        trust.remember(byteArrayOf(9))
        assertContentEquals(pinned, trust.caRoot())
        assertEquals(0, h.witness.caRootCalls)
    }

    @Test
    fun trustWithoutRootIsOffline() = runTest {
        h.witness.caRoot = null
        assertFailsWith<OfflineException> { h.trust.caRoot() }
    }

    @Test
    fun vaultSignerSurfacesFailuresAsSignerErrors() {
        val signer = VaultSigner(FakeKeyVault(h.clock), KeyId.fromBytes(ByteArray(16)), emptyList())
        assertEquals(true, assertFailsWith<WitnessException.Signer> { signer.sign(byteArrayOf(1)) }.reason.contains("unknown key"))
        assertTrue(signer.certChain().isEmpty())
    }

    @Test
    fun serialIsTheCertificateMagnitudeInHex() {
        assertEquals("0102", NativeKeys.serialHex(Vectors.CERT))
        assertContentEquals(byteArrayOf(0x80.toByte()), NativeKeys.magnitude(BigInteger.valueOf(128)))
        assertContentEquals(byteArrayOf(0), NativeKeys.magnitude(BigInteger.ZERO))
        assertFailsWith<Exception> { NativeKeys.serialHex(byteArrayOf(1, 2, 3)) }
    }

    @Test
    fun activeEnrolledIgnoresUnenrolledKeys() = runTest {
        h.vault.generate(ByteString.of(byteArrayOf(1)), app.witness.core.model.Freshness.FRESH)
        assertEquals(null, h.keys.activeEnrolled())
        val k = h.enrolledKey()
        assertEquals(k, h.keys.activeEnrolled())
    }
}
