package app.witness.nativelib

import app.witness.core.domain.fake.InMemoryCounterStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.fake.SoftwareSha256
import app.witness.core.domain.port.CaptureFrame
import app.witness.core.domain.port.PrepareRequest
import app.witness.core.domain.usecase.SealSigner
import app.witness.core.model.BootState
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureId
import app.witness.core.model.Checks
import app.witness.core.model.ClaimKind
import app.witness.core.model.DeviceKey
import app.witness.core.model.Edit
import app.witness.core.model.EnumBits
import app.witness.core.model.LocationEvidence
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
import app.witness.core.model.SecurityLevel
import app.witness.core.model.TimeEvidence
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.test.runTest
import uniffi.witness_android.CaptureMeta
import uniffi.witness_android.Kind
import uniffi.witness_android.SealedCapture
import uniffi.witness_android.Submission
import uniffi.witness_android.WitnessException
import java.io.File
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class NativeSealEngineTest {
    private val h = Harness()
    private val metas = ArrayList<Pair<ByteArray, CaptureMeta>>()
    private val handles = ArrayList<FakeHandle>()
    private val images = HashMap<RecordId, ByteString>()
    private val rendered = ArrayList<Pair<ByteString, List<Edit>>>()
    private val remote = HashMap<RecordId, ByteString>()
    private val claimFiles = ClaimFiles(File(h.dir, "claims"))
    private var prepareFailure: WitnessException? = null
    private val claimCbor = "claim-cbor".encodeToByteArray()
    private val recordId = RecordId.fromDigest(SoftwareSha256.digest(claimCbor))

    private val engine = NativeSealEngine(
        keys = h.keys,
        images = { images[it] },
        claims = claimFiles,
        remoteClaim = { remote[it] },
        renderer = { jpeg, edits -> rendered += jpeg to edits; ByteString.of(byteArrayOf(9, 9)) },
        random = SeededRandom(3),
        appVersion = "0.1.0",
        prepareNative = { jpeg, meta ->
            prepareFailure?.let { throw it }
            metas += jpeg to meta
            FakeHandle(sealedFor(meta)).also(handles::add)
        },
        decode = { Vectors.claimView(ByteArray(16) { 5 }, kind = if (it.contentEquals(claimCbor)) Kind.Original else Kind.Derivative(recordId.text, 0x00AB_CDEFuL, emptyList())) },
        cpu = Dispatchers.Unconfined,
    )
    private val signer = SealSigner(engine, InMemoryCounterStore())

    private fun sealedFor(meta: CaptureMeta) = SealedCapture(
        jpeg = byteArrayOf(-1, -40, 42),
        recordId = recordId.text,
        claim = Vectors.claimView(meta.keyId, kind = meta.kind),
        submission = Submission(claimCbor, byteArrayOf(2), byteArrayOf(3), byteArrayOf(4)),
        watermarked = false,
    )

    private val checks = Checks(SecurityLevel.TEE, BootState.VERIFIED, true, false, EnumBits.none(), 0)

    private fun request(rotation: Int = 90) = PrepareRequest(
        CaptureFrame(ByteString.of(byteArrayOf(-1, -40, 1)), rotation, 5, 1),
        CaptureId.fromLong(77)!!,
        TimeEvidence(1_000, null, 5, 1),
        LocationEvidence.NONE,
        checks,
    )

    private suspend fun key(): DeviceKey = h.vault.key(h.enrolledKey())!!

    @Test
    fun sealBuildsTheNativeMetaFromTheFinalClaim() = runTest {
        val k = key()
        val sealed = signer.seal(engine.prepare(request()), k)
        val (jpeg, meta) = metas.single()
        assertContentEquals(byteArrayOf(-1, -40, 1), jpeg)
        assertEquals(77uL, meta.captureId)
        assertEquals(6u, meta.exifOrientation)
        assertEquals(Kind.Original, meta.kind)
        assertContentEquals(k.keyId.toBytes(), meta.keyId)
        assertContentEquals(byteArrayOf(1, 2), meta.certSerial)
        assertEquals(16, meta.manifestId.size)
        assertEquals("0.1.0", meta.appVersion)
        assertEquals(1_000uL, meta.time.wallMs)
        assertEquals(recordId, sealed.recordId)
        assertEquals(ByteString.of(byteArrayOf(-1, -40, 42)), sealed.jpeg)
        assertEquals(ByteString.of(byteArrayOf(4)), sealed.submission.fingerprints)
        assertEquals(0, sealed.claim.keyRef.counter)
        assertTrue(handles.single().closed)
    }

    @Test
    fun signerCarriesTheC2paChainAndTheCounter() = runTest {
        val k = key()
        signer.seal(engine.prepare(request()), k)
        signer.seal(engine.prepare(request()), k)
        val (counter, s, ca) = handles[1].finishedWith!!
        assertEquals(1uL, counter)
        assertContentEquals(Vectors.CA, ca)
        assertEquals(listOf(Vectors.CERT.toList(), Vectors.CA.toList()), s.certChain().map { it.toList() })
        assertEquals(h.vault.sign(k.keyId, byteArrayOf(5)), ByteString.of(s.sign(byteArrayOf(5))))
    }

    @Test
    fun sealPersistsTheClaimContentAddressed() = runTest {
        signer.seal(engine.prepare(request()), key())
        assertEquals(ByteString.of(claimCbor), claimFiles.read(recordId))
        File(h.dir, "claims/${recordId.text}.cbor").writeBytes(byteArrayOf(1))
        assertNull(claimFiles.read(recordId))
    }

    @Test
    fun rotationMapsToExif() {
        assertEquals(listOf(1, 6, 3, 8, 1, 8), listOf(0, 90, 180, 270, 360, -90).map(NativeSealEngine::exifOrientation))
    }

    @Test
    fun unenrolledKeysCannotSeal() = runTest {
        val k = key().copy(certSerial = null)
        assertEquals("not_enrolled", assertFailsWith<NativeException> { signer.seal(engine.prepare(request()), k) }.code)
    }

    @Test
    fun preparedCapturesAreSingleUse() = runTest {
        val k = key()
        val p = engine.prepare(request())
        signer.seal(p, k)
        assertFailsWith<IllegalStateException> { signer.seal(p, k) }
    }

    @Test
    fun nativeFailuresAreTranslated() = runTest {
        prepareFailure = WitnessException.Image("not a jpeg")
        assertEquals("image", assertFailsWith<NativeException> { signer.seal(engine.prepare(request()), key()) }.code)
    }

    @Test
    fun deriveRendersEditsOnTheParentImage() = runTest {
        val k = key()
        val parent = signer.seal(engine.prepare(request()), k)
        images[parent.recordId] = ByteString.of(byteArrayOf(1, 1))
        val edits = listOf(Edit.Crop(Rect(0, 0, 10, 10)))
        val d = engine.derive(parent.recordId, edits, "9xj6")
        val kind = assertIs<ClaimKind.Derivative>(d.draft.kind)
        assertEquals(parent.recordId, kind.parent)
        assertEquals("9xj6", d.draft.coarse)
        signer.seal(d, k)
        assertEquals(ByteString.of(byteArrayOf(1, 1)) to edits, rendered.single())
        val (jpeg, meta) = metas.last()
        assertContentEquals(byteArrayOf(9, 9), jpeg)
        assertEquals(1u, meta.exifOrientation)
        assertEquals("9xj6", meta.coarse)
        val parentLocation = Vectors.claimView(ByteArray(16) { 5 }).location
        assertTrue(parentLocation.commit != null)
        assertNull(meta.location.commit)
        assertEquals(parentLocation.confidence, meta.location.confidence)
        assertEquals(parentLocation.reasons, meta.location.reasons)
        val nk = assertIs<Kind.Derivative>(meta.kind)
        assertEquals(parent.recordId.text, nk.parent)
        assertEquals(0x00AB_CDEFuL, nk.root)
    }

    @Test
    fun deriveFallsBackToTheRegistryClaimAndNeedsTheImage() = runTest {
        val id = RecordId.fromBytes(ByteArray(10) { 3 })
        assertEquals("missing_claim", assertFailsWith<NativeException> { engine.derive(id, emptyList(), null) }.code)
        remote[id] = ByteString.of(claimCbor)
        assertEquals("missing_image", assertFailsWith<NativeException> { engine.derive(id, emptyList(), null) }.code)
        images[id] = ByteString.of(byteArrayOf(1))
        assertIs<ClaimKind.Derivative>(engine.derive(id, emptyList(), null).draft.kind)
    }

    @Test
    fun resealKeepsTheClaimAndUsesTheStoredImage() = runTest {
        val k = key()
        val first = signer.seal(engine.prepare(request()), k)
        images[first.recordId] = ByteString.of(byteArrayOf(7))
        val p = engine.reseal(first.recordId)
        assertEquals(ClaimKind.Original, p.draft.kind)
        assertEquals(0x00AB_CDEFL, p.draft.captureId.value)
        signer.seal(p, k)
        assertContentEquals(byteArrayOf(7), metas.last().first)
        assertEquals(1u, metas.last().second.exifOrientation)
    }
}
