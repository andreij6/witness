package app.witness.di

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import app.witness.core.data.ImageFiles
import app.witness.core.domain.fake.FakeCaseBundleCodec
import app.witness.core.domain.fake.FakeIndex
import app.witness.core.domain.fake.FakeMedia
import app.witness.core.domain.fake.FakeRegistry
import app.witness.core.domain.fake.FakeSealEngine
import app.witness.core.domain.fake.FakeVerifyEngine
import app.witness.core.domain.port.EnrollOutcome
import app.witness.core.domain.port.EnrollmentRequest
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.PrepareRequest
import app.witness.core.domain.port.PreparedCapture
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.SealEngine
import app.witness.core.domain.port.SealedCapture
import app.witness.core.domain.port.SubmitResult
import app.witness.core.model.ByteString
import app.witness.core.model.DeviceKey
import app.witness.core.model.Edit
import app.witness.core.model.KeyHistory
import app.witness.core.model.KeyInfo
import app.witness.core.model.Provisioning
import app.witness.core.model.RecordId
import app.witness.core.model.SecurityLevel
import app.witness.core.model.Submission
import app.witness.core.model.WitnessClaim
import app.witness.feature.proof.CaseLinkResult
import app.witness.feature.proof.InMemoryReportSender
import app.witness.feature.settings.fake.InMemoryIdentityAccount
import app.witness.nativelib.EditRenderer
import java.io.ByteArrayOutputStream
import java.util.concurrent.ConcurrentHashMap

object StandInNative : NativePortsFactory {
    const val SHARD: Int = 0

    override fun create(env: NativeEnvironment): NativePorts {
        val engine = FakeSealEngine()
        val index = FakeIndex()
        val registry = FakeRegistry(env.clock, engine::claim)
        return NativePorts(
            seal = ImageKeepingSealEngine(engine, env.images, JpegOrientation::upright, BitmapEditRenderer),
            verify = FakeVerifyEngine(),
            registry = LoopbackRegistry(registry, index, engine::claim, env.vault),
            index = index,
            media = FakeMedia(),
            bundles = FakeCaseBundleCodec(),
            identity = InMemoryIdentityAccount(),
            caseLinks = { _, _ -> CaseLinkResult.NotFound },
            reports = InMemoryReportSender(),
            linked = false,
            restore = { vault -> vault.activeKey()?.toDevInfo()?.let(registry::putKey) },
        )
    }

    fun DeviceKey.toDevInfo(): KeyInfo? {
        val enrolled = enrolledAtMs ?: return null
        return KeyInfo(keyId, security, Provisioning.DEV, freshness, enrolled, activeUntilMs ?: enrolled, KeyHistory.enrolled(enrolled))
    }
}

class ImageKeepingSealEngine(
    private val delegate: SealEngine,
    private val images: ImageFiles,
    private val upright: (ByteString, Int) -> ByteString = { jpeg, _ -> jpeg },
    private val render: EditRenderer = EditRenderer { jpeg, _ -> jpeg },
) : SealEngine {
    private val frames = ConcurrentHashMap<Long, ByteString>()
    private val parents = ConcurrentHashMap<Long, Pair<RecordId, List<Edit>>>()

    val pending: Int get() = frames.size + parents.size

    override suspend fun prepare(request: PrepareRequest): PreparedCapture {
        val jpeg = upright(request.frame.jpeg, request.frame.rotationDegrees)
        return delegate.prepare(request.copy(frame = request.frame.copy(jpeg = jpeg))).also { frames[it.token] = jpeg }
    }

    override suspend fun derive(parent: RecordId, edits: List<Edit>, coarse: String?): PreparedCapture =
        delegate.derive(parent, edits, coarse).also { parents[it.token] = parent to edits }

    override suspend fun reseal(recordId: RecordId): PreparedCapture = delegate.reseal(recordId)

    override suspend fun seal(prepared: PreparedCapture, claim: WitnessClaim): SealedCapture {
        val frame = frames.remove(prepared.token)
        val parent = parents.remove(prepared.token)
        val sealed = delegate.seal(prepared, claim)
        val jpeg = frame ?: parent?.let { (id, edits) -> images.read(id)?.let { render.render(it, edits) } }
        if (jpeg != null) images.write(sealed.recordId, jpeg)
        return sealed
    }
}

class LoopbackRegistry(
    private val delegate: FakeRegistry,
    private val index: FakeIndex,
    private val claims: (RecordId) -> WitnessClaim?,
    private val vault: KeyVault? = null,
) : RegistryGateway by delegate {
    override suspend fun enroll(request: EnrollmentRequest): EnrollOutcome {
        val outcome = delegate.enroll(request)
        if (outcome !is EnrollOutcome.Enrolled) return outcome
        val security = vault?.key(request.keyId)?.security ?: SecurityLevel.DEV
        val info = outcome.info.copy(security = security, provisioning = Provisioning.DEV)
        delegate.putKey(info)
        return outcome.copy(info = info)
    }

    override suspend fun submit(batch: List<Submission>): List<SubmitResult> {
        val results = delegate.submit(batch)
        for (i in results.indices) {
            val anchored = results[i] as? SubmitResult.Anchored ?: continue
            val claim = claims(batch[i].recordId) ?: continue
            index.addExact(StandInNative.SHARD, claim.assetHash, anchored.anchor.recordIx)
            if (claim.pixelHash != claim.assetHash) index.addExact(StandInNative.SHARD, claim.pixelHash, anchored.anchor.recordIx)
        }
        return results
    }
}

object JpegOrientation {
    const val QUALITY: Int = 92

    fun upright(jpeg: ByteString, degrees: Int): ByteString {
        val turn = degrees.mod(FULL_TURN)
        if (turn == 0) return jpeg
        val bytes = jpeg.toByteArray()
        val source = BitmapFactory.decodeByteArray(bytes, 0, bytes.size) ?: return jpeg
        val rotated = Bitmap.createBitmap(source, 0, 0, source.width, source.height, Matrix().apply { postRotate(turn.toFloat()) }, true)
        if (rotated !== source) source.recycle()
        val out = ByteArrayOutputStream(bytes.size)
        rotated.compress(Bitmap.CompressFormat.JPEG, QUALITY, out)
        rotated.recycle()
        return ByteString.wrap(out.toByteArray())
    }

    private const val FULL_TURN = 360
}
