package app.witness.di

import android.app.Application
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import app.witness.BuildConfig
import app.witness.core.data.ImageFiles
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeEntitlementSource
import app.witness.core.domain.fake.FakeMedia
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.InMemoryCaptureStore
import app.witness.core.domain.fake.InMemoryCaseStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.usecase.CaseBundleCbor
import app.witness.core.domain.usecase.ShareCase
import app.witness.core.domain.usecase.ShareOutcome
import app.witness.core.icp.IcpEndpoint
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.BlurClass
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.LocationConfidence
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
import app.witness.core.model.ShareExpiry
import app.witness.core.model.TemplateId
import app.witness.nativelib.Hex
import app.witness.nativelib.NativeIndex
import app.witness.nativelib.NativeSealEngine
import app.witness.nativelib.NativeVerifyEngine
import kotlinx.coroutines.test.runTest
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import uniffi.witness_android.Begun
import uniffi.witness_android.CertifiedKey
import uniffi.witness_android.CertifiedRecord
import uniffi.witness_android.Enrolled
import uniffi.witness_android.Enrollment
import uniffi.witness_android.EntitlementGrant
import uniffi.witness_android.KeystoreSigner
import uniffi.witness_android.ProbeResult
import uniffi.witness_android.Shards
import uniffi.witness_android.Submission
import uniffi.witness_android.SubmitResult
import uniffi.witness_android.Trustmark
import uniffi.witness_android.UploadTarget
import uniffi.witness_android.VerdictView
import uniffi.witness_android.WitnessInterface
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.file.Files
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

private const val ANCHOR = "3082014e30820100a003020102020101300506032b6570302c3110300e060355040a0c075769746e657373"

class NativeAvailabilityTest {
    private val endpoint = IcpEndpoint("http://10.0.2.2:8090", true, "4caro-hl777-77775-aaaba-cai")

    @Test
    fun realNativeOnlyWhenConfiguredAndTheLibraryLoads() {
        assertIs<RealNative>(NativeAvailability.select(endpoint) { true })
        assertSame(StandInNative, NativeAvailability.select(endpoint) { false })
        var probed = false
        assertSame(StandInNative, NativeAvailability.select(null) { probed = true; true })
        assertTrue(!probed)
    }

    @Test
    fun endpointNeedsBothUrlAndRegistry() {
        assertNull(NativeAvailability.endpointOf("", "id", true))
        assertNull(NativeAvailability.endpointOf("http://x", " ", true))
        assertEquals(IcpEndpoint("http://x", true, "id"), NativeAvailability.endpointOf("http://x", "id", true))
        assertEquals(IcpEndpoint("http://x", false, "id", ANCHOR), NativeAvailability.endpointOf("http://x", "id", false, ANCHOR))
    }

    @Test
    fun productionRefusesToRunWithoutAPinnedAnchor() {
        assertNull(NativeAvailability.endpointOf("https://icp-api.io", "id", false))
        assertNull(NativeAvailability.endpointOf("https://icp-api.io", "id", false, ""))
        assertNull(NativeAvailability.endpointOf("https://icp-api.io", "id", false, "30"))
        assertNull(NativeAvailability.endpointOf("https://icp-api.io", "id", false, ANCHOR.uppercase()))
        assertNull(NativeAvailability.endpointOf("https://icp-api.io", "id", false, ANCHOR + "0"))
        assertNull(NativeAvailability.endpointOf("https://icp-api.io", "id", false, "31" + ANCHOR.drop(2)))
        assertNull(NativeAvailability.endpointOf("https://icp-api.io", "id", false, ANCHOR.dropLast(2) + "zz"))
    }

    @Test
    fun localBuildsPinTheReplicaAnchorWhenTheyHaveOne() {
        assertEquals("", NativeAvailability.endpointOf("http://x", "id", true)!!.caRoot)
        assertEquals(ANCHOR, NativeAvailability.endpointOf("http://x", "id", true, ANCHOR)!!.caRoot)
        assertNull(NativeAvailability.endpointOf("http://x", "id", true, "nothex"))
        assertTrue(NativeAvailability.isAnchor(ANCHOR))
        assertTrue(!NativeAvailability.isAnchor(""))
    }

    @Test
    fun witnessConfigCarriesThePinnedAnchorBytes() {
        val pinned = RealNative.config(IcpEndpoint("https://icp-api.io", false, "4caro-hl777-77775-aaaba-cai", ANCHOR))
        assertTrue(pinned.caRoot!!.contentEquals(Hex.parse(ANCHOR)))
        assertTrue(!pinned.local)
        assertEquals("4caro-hl777-77775-aaaba-cai", pinned.registry)
        assertNull(RealNative.config(endpoint).caRoot)
        assertTrue(RealNative.config(endpoint).local)
        assertNull(RealNative.pinnedAnchor(endpoint))
    }

    @Test
    fun uploadsComeFromAPersistentPerInstallIdentityNotTheAnonymousPrincipal() {
        val dir = File(Files.createTempDirectory("seed").toFile(), "native")
        val seed = RealNative.identitySeed(dir, SeededRandom(1))
        assertEquals(RealNative.SEED_BYTES, seed.size)
        assertTrue(seed.any { it != 0.toByte() })
        assertTrue(seed.contentEquals(RealNative.identitySeed(dir, SeededRandom(2))))
        assertTrue(!seed.contentEquals(RealNative.identitySeed(Files.createTempDirectory("seed").toFile(), SeededRandom(2))))
        assertTrue(RealNative.config(endpoint, seed).identitySeed!!.contentEquals(seed))
        File(dir, RealNative.SEED_FILE).writeBytes(ByteArray(5))
        val healed = RealNative.identitySeed(dir, SeededRandom(3))
        assertEquals(RealNative.SEED_BYTES, healed.size)
        assertTrue(healed.contentEquals(File(dir, RealNative.SEED_FILE).readBytes()))
        assertTrue(!File(dir, RealNative.SEED_FILE + ".tmp").exists())
    }

    @Test
    fun buildConfigAnchorIsHexDerOrEmpty() {
        val anchor = BuildConfig.WITNESS_CA_ROOT
        assertTrue(anchor.isEmpty() || NativeAvailability.isAnchor(anchor), anchor)
        if (BuildConfig.ICP_FETCH_ROOT_KEY && NativeAvailability.endpoint != null) assertEquals(anchor, NativeAvailability.endpoint.caRoot)
        if (!BuildConfig.ICP_FETCH_ROOT_KEY && anchor.isEmpty()) assertNull(NativeAvailability.endpoint)
    }

    @Test
    fun hostJvmCannotLoadTheAndroidLibrarySoTestsUseStandIns() {
        assertTrue(!NativeAvailability.loads())
        assertSame(StandInNative, NativeAvailability.factory)
    }

    @Test
    fun buildConfigPointsDebugBuildsAtTheLocalReplica() {
        if (BuildConfig.DEBUG) {
            assertEquals("http://127.0.0.1:8090", BuildConfig.ICP_REPLICA_URL)
            assertTrue(BuildConfig.ICP_FETCH_ROOT_KEY)
            assertEquals(BuildConfig.ICP_REGISTRY_ID.isNotEmpty(), NativeAvailability.endpoint != null)
        } else {
            assertEquals("", BuildConfig.ICP_REGISTRY_ID)
            assertNull(NativeAvailability.endpoint)
        }
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36], application = Application::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class RealNativeWiringTest {
    private val clock = FakeClock()
    private val images = ImageFiles(Files.createTempDirectory("real-images").toFile())
    private val endpoint = IcpEndpoint("http://10.0.2.2:8090", true, "registry")

    @Test
    fun realPortsAreNativeAdaptersAndLinked() {
        val ports = RealNative(endpoint).ports(UnusedWitness, env(clock, images))
        assertTrue(ports.linked)
        assertIs<NativeSealEngine>(ports.seal)
        assertIs<NativeVerifyEngine>(ports.verify)
        assertIs<NativeIndex>(ports.index)
        assertSame(CaseBundleCbor, ports.bundles)
        assertIs<app.witness.nativelib.NativeMedia>(ports.previews)
    }

    @Test
    fun previewsAreUprightBoundedJpegs() {
        val big = jpeg(2400, 1200, checker = true).toByteArray()
        val out = PreviewJpeg.encode(big)
        assertTrue(out.size <= PreviewJpeg.MAX_BYTES)
        val o = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(out, 0, out.size, o)
        assertEquals("image/jpeg", o.outMimeType)
        assertEquals(1080 to 540, o.outWidth to o.outHeight)
        val small = PreviewJpeg.encode(jpeg(64, 48).toByteArray())
        BitmapFactory.decodeByteArray(small, 0, small.size, o)
        assertEquals(64 to 48, o.outWidth to o.outHeight)
        assertFailsWith<IllegalArgumentException> { PreviewJpeg.encode(byteArrayOf(1, 2, 3)) }
    }

    @Test
    fun previewGeometry() {
        assertEquals(1080 to 810, PreviewJpeg.fit(4000, 3000, 1080))
        assertEquals(810 to 1080, PreviewJpeg.fit(3000, 4000, 1080))
        assertEquals(100 to 50, PreviewJpeg.fit(100, 50, 1080))
        assertEquals(1080 to 1, PreviewJpeg.fit(100_000, 1, 1080))
        assertEquals(1, PreviewJpeg.sampleSize(1080, 900))
        assertEquals(1, PreviewJpeg.sampleSize(2159, 100))
        assertEquals(2, PreviewJpeg.sampleSize(2160, 100))
        assertEquals(2, PreviewJpeg.sampleSize(4000, 3000))
        assertEquals(8, PreviewJpeg.sampleSize(8640, 100))
        assertEquals(4, PreviewJpeg.sampleSize(8639, 100))
    }

    @Test
    fun referenceImagesComeFromThisPhoneFirst() = runTest {
        val ports = RealNative(endpoint).ports(UnusedWitness, env(clock, images))
        val id = RecordId.fromBytes(ByteArray(10) { 4 })
        images.write(id, ByteString.of(byteArrayOf(-1, -40, 9)))
        assertEquals(ByteString.of(byteArrayOf(-1, -40, 9)), ports.media.reference(id))
    }

    @Test
    fun cropAndResizeChangeTheRenderedSize() = runTest {
        val out = decode(BitmapEditRenderer.render(jpeg(64, 48), listOf(Edit.Crop(Rect(8, 8, 32, 16)))))
        assertEquals(32 to 16, out.width to out.height)
        val resized = decode(BitmapEditRenderer.render(jpeg(64, 48), listOf(Edit.Resize(20, 10))))
        assertEquals(20 to 10, resized.width to resized.height)
        val clipped = decode(BitmapEditRenderer.render(jpeg(64, 48), listOf(Edit.Crop(Rect(60, 40, 100, 100)))))
        assertEquals(4 to 8, clipped.width to clipped.height)
    }

    @Test
    fun blurChangesOnlyInsideTheRect() = runTest {
        val source = jpeg(96, 96, checker = true)
        val out = decode(BitmapEditRenderer.render(source, listOf(Edit.Blur(Rect(0, 0, 48, 48), BlurClass.FACE, 800))))
        val before = decode(source)
        assertEquals(96 to 96, out.width to out.height)
        assertNotEquals(before.getPixel(1, 0), out.getPixel(1, 0))
        assertEquals(before.getPixel(80, 80), out.getPixel(80, 80))
    }

    @Test
    fun declaredOnlyEditsKeepTheParentBytes() = runTest {
        val source = jpeg(16, 16)
        assertSame(source, BitmapEditRenderer.render(source, listOf(Edit.Tone(10, 0, 0), Edit.Rotate(50))))
        assertSame(source, BitmapEditRenderer.render(source, emptyList()))
    }

    @Test
    fun undecodableParentFails() = runTest {
        assertFailsWith<IllegalArgumentException> { BitmapEditRenderer.render(ByteString.of(byteArrayOf(1, 2)), listOf(Edit.Crop(Rect(0, 0, 1, 1)))) }
    }

    @Test
    fun strongerDeclaredSigmaBlursMore() = runTest {
        val source = jpeg(96, 96, checker = true)
        val weak = decode(BitmapEditRenderer.render(source, listOf(Edit.Blur(Rect(0, 0, 96, 96), BlurClass.FACE, 100))))
        val strong = decode(BitmapEditRenderer.render(source, listOf(Edit.Blur(Rect(0, 0, 96, 96), BlurClass.FACE, 1600))))
        assertTrue(edgeEnergy(strong) < edgeEnergy(weak))
    }

    private fun edgeEnergy(b: Bitmap): Long {
        var e = 0L
        for (y in 0 until b.height) for (x in 1 until b.width) e += kotlin.math.abs((b.getPixel(x, y) and 0xFF) - (b.getPixel(x - 1, y) and 0xFF))
        return e
    }

    @Test
    fun jcaSealedCaseChunkMatchesTheRustWitnessWebVector() = runTest {
        val cases = InMemoryCaseStore()
        val captures = InMemoryCaptureStore()
        val media = FakeMedia()
        val case = cases.create("Rear bumper", TemplateId("general"), 0)
        cases.update(case.copy(summary = "two photos"))
        val a = RecordId.fromBytes(ByteArray(10) { 1 })
        captures.insert(Capture(a, null, CaptureOrigin.ORIGINAL, AnchoringStatus.ANCHORED, 0, null, LocationConfidence.HIGH, Dims(1, 1), caseId = case.id), null, null, ByteString.of(byteArrayOf(-1, -40, 1, 2, 3)))
        val random = Scripted(ByteArray(32) { 0x42 }, ByteArray(2), ByteArray(15) { 0xab.toByte() }, ByteArray(12) { 7 })
        val share = ShareCase(cases, captures, CaseBundleCbor, JcaCrypto, media, FakeRegistry(clock), FakeEntitlementSource(), random, clock)
        val shared = assertIs<ShareOutcome.Shared>(share(case.id, listOf(a), ShareExpiry.entries.first()))
        val upload = media.uploads.values.single()
        val h = java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(upload.request.bundleHash.toByteArray())
        assertTrue(shared.share.url.endsWith("02NTQAXBNENTQAXBNENTQAXBNC#k=QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI&h=$h"))
        val chunk = upload.chunks.getValue(0)
        assertEquals(RUST_SINGLE_ITEM_CHUNK, ByteString.of(chunk).hex())
    }

    private class Scripted(vararg val draws: ByteArray) : RandomSource {
        private var i = 0

        override fun nextBytes(size: Int): ByteArray = draws[i++].also { check(it.size == size) { "draw $i wants $size bytes" } }
    }

    private fun jpeg(w: Int, h: Int, checker: Boolean = false): ByteString {
        val b = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        for (y in 0 until h) for (x in 0 until w) b.setPixel(x, y, if (checker && (x + y) % 2 == 0) Color.BLACK else if (checker) Color.WHITE else Color.rgb(x * 3 % 256, y * 5 % 256, 90))
        val out = ByteArrayOutputStream()
        b.compress(Bitmap.CompressFormat.PNG, 100, out)
        return ByteString.wrap(out.toByteArray())
    }

    private fun decode(b: ByteString): Bitmap = b.toByteArray().let { BitmapFactory.decodeByteArray(it, 0, it.size) }

    private companion object {
        const val RUST_SINGLE_ITEM_CHUNK = "070707070707070707070707dce25caddea3179a51748e45b02b430ef7307b4a92f5382d0f00502019acba7465e91bd175b2744c46f744899f60b854846c0dc9c1eaaa9f75025c9c04ea053e4a0b"
    }
}

object UnusedWitness : WitnessInterface {
    private fun unused(): Nothing = error("not used by this test")

    override suspend fun beginUpload(shard: UByte, target: UploadTarget, size: ULong, sha256: ByteArray, keyId: ByteArray, signer: KeystoreSigner, grant: EntitlementGrant?): Begun = unused()
    override suspend fun byCapture(captureId: ULong): List<String> = unused()
    override suspend fun caRoot(): ByteArray? = unused()
    override suspend fun challenge(): ByteArray = unused()
    override suspend fun check(image: ByteArray, watermark: Trustmark?): VerdictView = unused()
    override suspend fun commit(shard: UByte, upload: ULong): String = unused()
    override suspend fun enroll(enrollment: Enrollment): Enrolled = unused()
    override suspend fun eraseMine(recordId: String, nonce: ByteArray, signer: KeystoreSigner) = unused()
    override suspend fun get(recordId: String): CertifiedRecord? = unused()
    override suspend fun getKey(keyId: ByteArray): CertifiedKey? = unused()
    override suspend fun probe(pdq: List<ByteArray>, watermarks: List<ULong>, exact: List<ByteArray>): ProbeResult = unused()
    override suspend fun putChunk(shard: UByte, upload: ULong, ix: UInt, bytes: ByteArray) = unused()
    override suspend fun shards(): Shards = unused()
    override suspend fun submit(batch: List<Submission>): List<SubmitResult> = unused()
}
