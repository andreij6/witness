package app.witness.nativelib

import app.witness.core.domain.port.PrepareRequest
import app.witness.core.domain.port.PreparedCapture
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.SealEngine
import app.witness.core.domain.port.SealedCapture
import app.witness.core.model.ByteString
import app.witness.core.model.ClaimDraft
import app.witness.core.model.ClaimKind
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.RecordId
import app.witness.core.model.WitnessClaim
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.witness_android.CaptureMeta
import uniffi.witness_android.ClaimView
import uniffi.witness_android.PreparedHandleInterface
import java.io.File
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

fun interface EditRenderer {
    suspend fun render(jpeg: ByteString, edits: List<Edit>): ByteString
}

fun interface NativePrepare {
    fun prepare(jpeg: ByteArray, meta: CaptureMeta): PreparedHandleInterface
}

class ClaimFiles(private val dir: File) {
    suspend fun write(recordId: RecordId, cbor: ByteString) = withContext(Dispatchers.IO) {
        dir.mkdirs()
        val tmp = File(dir, recordId.text + TMP)
        tmp.writeBytes(cbor.toByteArray())
        check(tmp.renameTo(File(dir, recordId.text + EXT))) { "could not persist claim" }
    }

    suspend fun read(recordId: RecordId): ByteString? = withContext(Dispatchers.IO) {
        val bytes = File(dir, recordId.text + EXT).takeIf(File::isFile)?.readBytes() ?: return@withContext null
        bytes.takeIf { RecordId.fromDigest(MessageDigest.getInstance("SHA-256").digest(it)) == recordId }?.let(ByteString::wrap)
    }

    private companion object {
        const val EXT = ".cbor"
        const val TMP = ".cbor.tmp"
    }
}

class NativeSealEngine(
    private val keys: NativeKeys,
    private val images: suspend (RecordId) -> ByteString?,
    private val claims: ClaimFiles,
    private val remoteClaim: suspend (RecordId) -> ByteString?,
    private val renderer: EditRenderer,
    private val random: RandomSource,
    private val appVersion: String,
    private val prepareNative: NativePrepare,
    private val decode: (ByteArray) -> ClaimView,
    private val cpu: CoroutineDispatcher = Dispatchers.Default,
) : SealEngine {
    private class Pending(val jpeg: ByteString, val exifOrientation: Int)

    private val pending = ConcurrentHashMap<Long, Pending>()
    private val tokens = AtomicLong(1)

    override suspend fun prepare(request: PrepareRequest): PreparedCapture {
        val draft = ClaimDraft(request.captureId, ClaimKind.Original, ByteString.EMPTY, ByteString.EMPTY, ByteString.EMPTY, UNKNOWN, request.time, request.location, request.checks)
        return stash(Pending(request.frame.jpeg, exifOrientation(request.frame.rotationDegrees)), draft)
    }

    override suspend fun derive(parent: RecordId, edits: List<Edit>, coarse: String?): PreparedCapture {
        val p = claimOf(parent)
        val root = (p.kind as? ClaimKind.Derivative)?.root ?: p.captureId
        val jpeg = renderer.render(imageOf(parent), edits)
        val draft = ClaimDraft(root, ClaimKind.Derivative(parent, root, edits), ByteString.EMPTY, ByteString.EMPTY, ByteString.EMPTY, UNKNOWN, p.time, p.location.copy(commit = null), p.checks, coarse)
        return stash(Pending(jpeg, EXIF_UPRIGHT), draft)
    }

    override suspend fun reseal(recordId: RecordId): PreparedCapture {
        val c = claimOf(recordId)
        val draft = ClaimDraft(c.captureId, c.kind, ByteString.EMPTY, ByteString.EMPTY, ByteString.EMPTY, UNKNOWN, c.time, c.location, c.checks, c.coarse)
        return stash(Pending(imageOf(recordId), EXIF_UPRIGHT), draft)
    }

    override suspend fun seal(prepared: PreparedCapture, claim: WitnessClaim): SealedCapture {
        val p = pending.remove(prepared.token) ?: throw IllegalStateException("unknown or reused prepared capture")
        val serial = claim.certSerial ?: throw NativeException("not_enrolled", "the signing key has no certificate yet")
        val signer = keys.signer(claim.keyRef.keyId)
        val caRoot = keys.caRoot()
        val meta = CaptureMeta(
            captureId = claim.captureId.value.toULong(),
            exifOrientation = p.exifOrientation.toUInt(),
            kind = NativeMapping.kind(claim.kind),
            time = NativeMapping.time(claim.time),
            location = NativeMapping.location(claim.location),
            checks = NativeMapping.checks(claim.checks),
            keyId = claim.keyRef.keyId.toBytes(),
            certSerial = Hex.parse(serial),
            coarse = claim.coarse,
            manifestId = random.nextBytes(MANIFEST_ID_BYTES),
            appVersion = appVersion,
        )
        val sealed = withContext(cpu) {
            native {
                val handle = prepareNative.prepare(p.jpeg.toByteArray(), meta)
                try {
                    handle.finish(claim.keyRef.counter.toULong(), signer, caRoot)
                } finally {
                    (handle as? AutoCloseable)?.close()
                }
            }
        }
        val submission = NativeMapping.submission(sealed.recordId, sealed.claim, sealed.submission)
        claims.write(submission.recordId, submission.claimCbor)
        return SealedCapture(submission.recordId, NativeMapping.claim(sealed.claim), submission, ByteString.wrap(sealed.jpeg))
    }

    private fun stash(p: Pending, draft: ClaimDraft): PreparedCapture {
        val token = tokens.getAndIncrement()
        pending[token] = p
        return PreparedCapture(token, draft)
    }

    private suspend fun imageOf(recordId: RecordId): ByteString = images(recordId) ?: throw NativeException("missing_image", "no sealed image for $recordId")

    private suspend fun claimOf(recordId: RecordId): WitnessClaim {
        val cbor = claims.read(recordId) ?: remoteClaim(recordId) ?: throw NativeException("missing_claim", "no claim for $recordId")
        return NativeMapping.claim(native { decode(cbor.toByteArray()) })
    }

    companion object {
        const val MANIFEST_ID_BYTES: Int = 16
        const val EXIF_UPRIGHT: Int = 1
        val UNKNOWN: Dims = Dims(0, 0)

        fun exifOrientation(rotationDegrees: Int): Int = when (rotationDegrees.mod(360)) {
            90 -> 6
            180 -> 3
            270 -> 8
            else -> EXIF_UPRIGHT
        }
    }
}
