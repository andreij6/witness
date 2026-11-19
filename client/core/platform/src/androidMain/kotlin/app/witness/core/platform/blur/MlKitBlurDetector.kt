package app.witness.core.platform.blur

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.camera.core.ExperimentalGetImage
import androidx.camera.core.ImageProxy
import app.witness.core.domain.port.BlurDetector
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.ByteString
import app.witness.core.model.RecordId
import com.google.android.gms.tasks.Task
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.face.FaceDetection
import com.google.mlkit.vision.face.FaceDetector
import com.google.mlkit.vision.face.FaceDetectorOptions
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.TextRecognizer
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

class MlKitBlurDetector(
    private val image: suspend (RecordId) -> ByteString?,
    private val io: CoroutineDispatcher = Dispatchers.Default,
) : BlurDetector {
    private val faces: FaceDetector by lazy {
        FaceDetection.getClient(
            FaceDetectorOptions.Builder()
                .setPerformanceMode(FaceDetectorOptions.PERFORMANCE_MODE_FAST)
                .setMinFaceSize(MIN_FACE)
                .build(),
        )
    }
    private val text: TextRecognizer by lazy { TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS) }

    override suspend fun detect(recordId: RecordId): List<BlurProposal> {
        val jpeg = image(recordId) ?: return emptyList()
        val decoded = withContext(io) { decode(jpeg.toByteArray()) } ?: return emptyList()
        val (bitmap, scale, width, height) = decoded
        try {
            return process(InputImage.fromBitmap(bitmap, 0), scale, width, height)
        } finally {
            bitmap.recycle()
        }
    }

    @OptIn(ExperimentalGetImage::class)
    suspend fun detectFrame(frame: ImageProxy): List<BlurProposal> {
        val media = frame.image ?: return emptyList()
        val rotation = frame.imageInfo.rotationDegrees
        val swapped = rotation % 180 != 0
        val width = if (swapped) frame.height else frame.width
        val height = if (swapped) frame.width else frame.height
        return process(InputImage.fromMediaImage(media, rotation), 1, width, height)
    }

    private suspend fun process(input: InputImage, scale: Int, width: Int, height: Int): List<BlurProposal> = coroutineScope {
        val f = async { faces.process(input).await() }
        val t = async { text.process(input).await() }
        val out = ArrayList<BlurProposal>()
        for (face in f.await()) {
            val b = face.boundingBox
            BlurGeometry.toOriginal(b.left, b.top, b.right, b.bottom, scale, width, height)?.let { out += BlurGeometry.proposal(it, BlurClass.FACE) }
        }
        for (block in t.await().textBlocks) for (line in block.lines) {
            val cls = TextBlurClassifier.classify(line.text) ?: continue
            val b = line.boundingBox ?: continue
            BlurGeometry.toOriginal(b.left, b.top, b.right, b.bottom, scale, width, height)
                ?.let { out += BlurGeometry.proposal(it, cls, line.confidence) }
        }
        BlurGeometry.dedupe(out)
    }

    fun close() {
        faces.close()
        text.close()
    }

    private data class Decoded(val bitmap: Bitmap, val scale: Int, val width: Int, val height: Int)

    private fun decode(jpeg: ByteArray): Decoded? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        val sample = BlurGeometry.sampleSize(bounds.outWidth, bounds.outHeight)
        val opts = BitmapFactory.Options().apply {
            inSampleSize = sample
            inPreferredConfig = Bitmap.Config.ARGB_8888
        }
        val bitmap = BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size, opts) ?: return null
        return Decoded(bitmap, sample, bounds.outWidth, bounds.outHeight)
    }

    companion object {
        const val MIN_FACE = 0.05f
    }
}

suspend fun <T> Task<T>.await(): T = suspendCancellableCoroutine { cont ->
    addOnSuccessListener { cont.resume(it) }
    addOnFailureListener { cont.resumeWithException(it) }
    addOnCanceledListener { cont.cancel() }
}
