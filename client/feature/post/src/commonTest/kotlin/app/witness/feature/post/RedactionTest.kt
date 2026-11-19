package app.witness.feature.post

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import app.witness.core.domain.usecase.BuildPost
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.PostDraft
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import app.witness.core.ui.NormRect
import kotlinx.serialization.serializer
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class RedactionTest {
    private val dims = Dims(1000, 500)

    @Test
    fun normalizedDividesByDims() = assertEquals(NormRect(0.1f, 0.2f, 0.3f, 0.6f), BlurProposal(app.witness.core.model.Rect(100, 100, 200, 200), BlurClass.FACE).normalized(dims))

    @Test
    fun normalizedClipsAndRejects() {
        assertEquals(1f, BlurProposal(app.witness.core.model.Rect(900, 400, 500, 500), BlurClass.FACE).normalized(dims)?.right)
        assertNull(BlurProposal(app.witness.core.model.Rect(2000, 0, 5, 5), BlurClass.FACE).normalized(dims))
        assertNull(BlurProposal(app.witness.core.model.Rect(0, 0, 5, 5), BlurClass.FACE).normalized(Dims(0, 10)))
    }

    @Test
    fun fitFramePillarbox() = assertEquals(Rect(150f, 0f, 250f, 200f), fitFrame(Dims(100, 200), Size(400f, 200f)))

    @Test
    fun fitFrameLetterbox() = assertEquals(Rect(0f, 75f, 400f, 125f), fitFrame(Dims(800, 100), Size(400f, 200f)))

    @Test
    fun fitFrameExactAspect() = assertEquals(Rect(0f, 0f, 380f, 508f), fitFrame(Dims(380, 508), Size(380f, 508f)))

    @Test
    fun fitFrameDegenerateInputs() {
        assertEquals(Rect(0f, 0f, 10f, 10f), fitFrame(Dims(0, 0), Size(10f, 10f)))
        assertEquals(Rect(0f, 0f, 0f, 0f), fitFrame(Dims(10, 10), Size(0f, 0f)))
    }

    @Test
    fun placeMapsIntoFrame() = assertEquals(Rect(15f, 30f, 20f, 50f), Rect(10f, 10f, 20f, 50f).place(NormRect(0.5f, 0.5f, 1f, 1f)))

    @Test
    fun blurEditsKeepOnlyEnabledClippedRects() {
        val on = BlurProposal(app.witness.core.model.Rect(900, 400, 500, 500), BlurClass.PLATE)
        val off = BlurProposal(app.witness.core.model.Rect(0, 0, 50, 50), BlurClass.FACE, enabled = false)
        val outside = BlurProposal(app.witness.core.model.Rect(5000, 0, 5, 5), BlurClass.FACE)
        val edits = blurEdits(listOf(on, off, outside), dims)
        assertEquals(listOf(Edit.Blur(app.witness.core.model.Rect(900, 400, 100, 100), BlurClass.PLATE, BuildPost.sigmaFor(100, 100))), edits)
    }

    @Test
    fun blurEditsMatchWhatBuildPostSeals() {
        val blurs = listOf(
            BlurProposal(app.witness.core.model.Rect(10, 20, 300, 80), BlurClass.PLATE),
            BlurProposal(app.witness.core.model.Rect(500, 100, 40, 40), BlurClass.FACE),
            BlurProposal(app.witness.core.model.Rect(600, 100, 40, 40), BlurClass.HOUSE_NO, enabled = false),
        )
        val draft = PostDraft(RecordId.fromBytes(ByteArray(10)), dims, blurs, LocationPrecision.AREA, null, "", "")
        assertEquals(BuildPost.derivativeEdits(draft), blurEdits(blurs, dims))
    }

    @Test
    fun blurEditsNeedValidDims() {
        assertTrue(blurEdits(listOf(BlurProposal(app.witness.core.model.Rect(0, 0, 5, 5), BlurClass.FACE)), Dims(0, 0)).isEmpty())
        assertTrue(blurEdits(emptyList(), dims).isEmpty())
    }

    @Test
    fun previewScaleFollowsDecodedWidth() {
        assertEquals(0.25f, previewScale(1000, 4000))
        assertEquals(1f, previewScale(1000, 0))
        assertEquals(1f, previewScale(0, 4000))
        assertEquals(1f, previewScale(800, 800))
    }

    @Test
    fun composerKeyRoundTrips() {
        val id = RecordId.fromBytes(ByteArray(10) { (it * 7).toByte() })
        assertEquals(id, PostComposerKey.of(id).recordId)
        assertEquals("app.witness.feature.post.PostComposerKey", serializer<PostComposerKey>().descriptor.serialName)
    }

    @Test
    fun sharedKeyRoundTrips() {
        val id = RecordId.fromBytes(ByteArray(10) { it.toByte() })
        val key = PostSharedKey.of(id, LocationPrecision.EXACT)
        assertEquals(id, key.recordId)
        assertEquals(LocationPrecision.EXACT, key.precision)
        assertEquals(2, serializer<PostSharedKey>().descriptor.elementsCount)
    }
}
