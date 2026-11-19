package app.witness.nativelib

import app.witness.core.domain.port.EnrollOutcome
import app.witness.core.domain.port.EnrollmentRequest
import app.witness.core.domain.port.EraseOutcome
import app.witness.core.domain.port.EraseRequest
import app.witness.core.domain.port.OfflineException
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.SubmitResult
import app.witness.core.model.Anchor
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.Provisioning
import app.witness.core.model.RecordId
import app.witness.core.model.Submission
import kotlinx.coroutines.test.runTest
import uniffi.witness_android.Anchored
import uniffi.witness_android.CertifiedRecord
import uniffi.witness_android.Enrolled
import uniffi.witness_android.SubmitErr
import uniffi.witness_android.WitnessException
import uniffi.witness_android.SubmitResult as NSubmitResult
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class NativeRegistryTest {
    private val h = Harness()
    private val decoded = ArrayList<ByteArray>()
    private val registry = NativeRegistry(h.witness, h.keys, h.trust) { decoded += it; Vectors.claimView(ByteArray(16) { 5 }) }
    private val keyBytes = ByteArray(16) { 9 }
    private val keyId = KeyId.fromBytes(keyBytes)
    private val chain = listOf(ByteString.of(byteArrayOf(1)), ByteString.of(byteArrayOf(2)))
    private fun request(freshness: Freshness = Freshness.FRESH) = EnrollmentRequest(keyId, chain, freshness, ByteString.of(byteArrayOf(7)))

    private fun answerEnroll(id: ByteArray = keyBytes) {
        h.witness.enrollAnswer = { Enrolled(id, Vectors.CERT, byteArrayOf(0x30, 0, 9)) }
        h.witness.keys[ByteString.of(id).hex()] = Vectors.keyInfo(id)
    }

    @Test
    fun enrollSendsTheAttestationChainAndReportsDevProvisioning() = runTest {
        answerEnroll()
        val out = assertIs<EnrollOutcome.Enrolled>(registry.enroll(request()))
        val sent = h.witness.enrollments.single()
        assertNull(sent.keyId)
        assertNull(sent.late)
        assertEquals(listOf(listOf<Byte>(1), listOf<Byte>(2)), sent.chain.map { it.toList() })
        assertEquals(Provisioning.DEV, out.info.provisioning)
        assertTrue(out.info.isDev)
        assertEquals("0102", out.certSerial)
        assertEquals(ByteString.of(Vectors.CERT), out.c2paCert)
        assertContentEquals(byteArrayOf(0x30, 0, 9), h.trust.caRoot())
        assertEquals(0, h.witness.caRootCalls)
    }

    @Test
    fun attestationRejectionIsFinal() = runTest {
        h.witness.failure = WitnessException.Rejected("attestation: BadChain")
        assertEquals(EnrollOutcome.Rejected("attestation: BadChain"), registry.enroll(request()))
    }

    @Test
    fun quotaAndReadinessRejectionsAreRetriedLater() = runTest {
        for (reason in listOf("quota: retry after 9 ns", "not ready: witness ca", "enrollment pending")) {
            h.witness.failure = WitnessException.Rejected(reason)
            assertFailsWith<OfflineException> { registry.enroll(request()) }
        }
        h.witness.failure = WitnessException.Network("down")
        assertFailsWith<OfflineException> { registry.enroll(request()) }
    }

    @Test
    fun otherNativeErrorsSurface() = runTest {
        h.witness.failure = WitnessException.InvalidInput("attestation chain is empty")
        assertEquals("invalid_input", assertFailsWith<NativeException> { registry.enroll(request()) }.code)
    }

    @Test
    fun enrollingADifferentKeyIsRejected() = runTest {
        answerEnroll(ByteArray(16) { 1 })
        assertIs<EnrollOutcome.Rejected>(registry.enroll(request()))
    }

    @Test
    fun lateEnrollmentWaitsWithoutTouchingTheNetwork() = runTest {
        assertFailsWith<OfflineException> { registry.enroll(request(Freshness.LATE)) }
        assertTrue(h.witness.enrollments.isEmpty())
    }

    @Test
    fun enrolledKeyMustBeReadable() = runTest {
        h.witness.enrollAnswer = { Enrolled(keyBytes, Vectors.CERT, Vectors.CA) }
        assertFailsWith<OfflineException> { registry.enroll(request()) }
    }

    @Test
    fun submitMapsResultsInOrder() = runTest {
        val sub = Submission(RecordId.fromBytes(ByteArray(10)), KeyRef(keyId, 1), true, 5, ByteString.of(byteArrayOf(1)), ByteString.of(byteArrayOf(2)), ByteString.of(byteArrayOf(3)), ByteString.of(byteArrayOf(4)))
        h.witness.submitAnswer = { batch ->
            assertContentEquals(byteArrayOf(4), batch[0].fingerprints)
            listOf(NSubmitResult.Ok(Anchored(1uL, "x", 2_000_000uL)), NSubmitResult.Err(SubmitErr.CounterReused))
        }
        assertEquals(listOf(SubmitResult.Anchored(Anchor(1, 2)), SubmitResult.Rejected("counter_reused")), registry.submit(listOf(sub, sub)))
        assertEquals(emptyList(), registry.submit(emptyList()))
        assertFailsWith<IllegalArgumentException> { registry.submit(List(RegistryGateway.MAX_BATCH + 1) { sub }) }
        h.witness.failure = WitnessException.Network("down")
        assertFailsWith<OfflineException> { registry.submit(listOf(sub)) }
    }

    @Test
    fun getDecodesTheCertifiedClaim() = runTest {
        val id = "WERC8GMRZGE196QV"
        h.witness.records[id] = CertifiedRecord(11uL, id, byteArrayOf(0xa1.toByte()), byteArrayOf(), byteArrayOf(), 4_000_000uL, emptyList(), 12uL, byteArrayOf())
        val r = registry.get(RecordId.parse(id))!!
        assertEquals(11, r.recordIx)
        assertEquals(4, r.anchoredAtMs)
        assertContentEquals(byteArrayOf(0xa1.toByte()), decoded.single())
        assertNull(registry.get(RecordId.fromBytes(ByteArray(10))))
        assertNull(registry.getByIndex(11))
    }

    @Test
    fun getKeyAndByCapture() = runTest {
        h.witness.keys[ByteString.of(keyBytes).hex()] = Vectors.keyInfo(keyBytes)
        assertEquals(keyId, registry.getKey(keyId)!!.keyId)
        assertNull(registry.getKey(KeyId.fromBytes(ByteArray(16))))
        h.witness.byCaptureAnswer = listOf("WERC8GMRZGE196QV", "not-an-id")
        assertEquals(listOf(RecordId.parse("WERC8GMRZGE196QV")), registry.byCapture(CaptureId.fromLong(5)!!))
    }

    @Test
    fun shardsAndChallenge() = runTest {
        assertEquals(2, registry.shards().media.size)
        assertEquals(ByteString.of(byteArrayOf(1, 2, 3)), registry.challenge())
    }

    @Test
    fun eraseSignsWithTheVaultKey() = runTest {
        val key = h.enrolledKey()
        val id = RecordId.parse("WERC8GMRZGE196QV")
        assertSame(EraseOutcome.Erased, registry.eraseMine(EraseRequest(id, key, ByteString.of(byteArrayOf(3)), ByteString.EMPTY)))
        assertEquals(id.text, h.witness.erased.single().first)
        val signer = h.witness.erasedSigners.single()
        assertEquals(h.vault.sign(key, byteArrayOf(1)), ByteString.of(signer.sign(byteArrayOf(1))))
        assertEquals(listOf(Vectors.CERT.toList(), Vectors.CA.toList()), signer.certChain().map { it.toList() })
    }

    @Test
    fun eraseRejectionIsAnOutcome() = runTest {
        val key = h.enrolledKey()
        h.witness.eraseFailure = WitnessException.Rejected("not yours")
        assertEquals(EraseOutcome.Rejected("not yours"), registry.eraseMine(EraseRequest(RecordId.fromBytes(ByteArray(10)), key, ByteString.of(byteArrayOf(1)), ByteString.EMPTY)))
        h.witness.eraseFailure = WitnessException.Network("down")
        assertFailsWith<OfflineException> { registry.eraseMine(EraseRequest(RecordId.fromBytes(ByteArray(10)), key, ByteString.of(byteArrayOf(1)), ByteString.EMPTY)) }
    }

    @Test
    fun transientReasonsAreRecognisedByPrefix() {
        assertTrue(NativeRegistry.isTransient("quota: retry after 1 ns"))
        assertTrue(!NativeRegistry.isTransient("attestation: quota"))
    }
}
