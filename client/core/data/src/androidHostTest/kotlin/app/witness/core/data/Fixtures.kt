package app.witness.core.data

import androidx.room3.Room
import androidx.test.core.app.ApplicationProvider
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.ByteString
import app.witness.core.model.Capture
import app.witness.core.model.CaptureId
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.Dims
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.LocationConfidence
import app.witness.core.model.OutboxEntry
import app.witness.core.model.RecordId
import app.witness.core.model.Submission
import kotlinx.coroutines.Dispatchers
import java.nio.file.Files
import javax.crypto.spec.SecretKeySpec

fun inMemoryDb(): WitnessDatabase =
    Room.inMemoryDatabaseBuilder(ApplicationProvider.getApplicationContext(), WitnessDatabase::class.java)
        .setQueryCoroutineContext(Dispatchers.Unconfined)
        .build()

fun testCipher() = AesGcmBlobCipher { SecretKeySpec(ByteArray(32) { 7 }, "AES") }

fun tempImages() = ImageFiles(Files.createTempDirectory("captures").toFile(), Dispatchers.Unconfined)

fun rid(n: Int) = RecordId.fromBytes(ByteArray(10) { (n + it).toByte() })

val KEY1 = KeyId.fromBytes(ByteArray(16) { 1 })
val KEY2 = KeyId.fromBytes(ByteArray(16) { 2 })

fun capture(n: Int, createdAtMs: Long = n.toLong(), status: AnchoringStatus = AnchoringStatus.QUEUED, counter: Long = n.toLong(), key: KeyId = KEY1) = Capture(
    recordId = rid(n),
    captureId = CaptureId.of(n.toLong(), 0),
    origin = CaptureOrigin.ORIGINAL,
    status = status,
    createdAtMs = createdAtMs,
    keyRef = KeyRef(key, counter),
    locationConfidence = LocationConfidence.HIGH,
    dims = Dims(4032, 3024),
)

fun entry(n: Int, counter: Long = n.toLong(), key: KeyId = KEY1, recordId: RecordId = rid(n)) = OutboxEntry(
    Submission(recordId, KeyRef(key, counter), true, 1_000L + n, ByteString.of(byteArrayOf(1, n.toByte())), ByteString.of(byteArrayOf(2)), ByteString.of(byteArrayOf(3))),
)
