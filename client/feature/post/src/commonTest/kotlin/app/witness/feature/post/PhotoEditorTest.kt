package app.witness.feature.post

import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.EditKind
import app.witness.core.model.ImageFormat
import app.witness.core.model.LocationPrecision
import app.witness.core.model.PostDraft
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class PhotoEditorTest {
    private val dims = Dims(4032, 3024)
    private val parent = RecordId.fromBytes(ByteArray(10))

    private fun draft(adjustments: List<Edit> = emptyList(), blurs: List<BlurProposal> = emptyList(), d: Dims = dims) =
        PostDraft(parent, d, blurs, LocationPrecision.AREA, null, "alt", "", adjustments)

    private fun state(d: Dims = dims) = PhotoEditor.open(draft(d = d))

    @Test
    fun openWithoutAdjustmentsUsesFullFrame() {
        val s = state()
        assertEquals(Rect(0, 0, 4032, 3024), s.crop)
        assertEquals(CropAspect.FREE, s.aspect)
        assertEquals(EditTool.CROP, s.tool)
        assertTrue(PhotoEditor.adjustments(s).isEmpty())
    }

    @Test
    fun openRestoresExistingAdjustments() {
        val s = PhotoEditor.open(draft(listOf(Edit.Crop(Rect(10, 20, 100, 200)), Edit.Rotate(-250), Edit.Tone(10, -20, 30))))
        assertEquals(Rect(10, 20, 100, 200), s.crop)
        assertEquals(-250, s.rotateE2)
        assertEquals(10, s.exposureE2)
        assertEquals(-20, s.contrastE2)
        assertEquals(30, s.saturationE2)
    }

    @Test
    fun openClampsOutOfRangeValues() {
        val s = PhotoEditor.open(draft(listOf(Edit.Crop(Rect(4000, 3000, 500, 500)), Edit.Rotate(5_000), Edit.Tone(999, -999, 0))))
        assertEquals(Rect(4000, 3000, 32, 24), s.crop)
        assertEquals(Edit.MAX_ROTATE_DEG_E2, s.rotateE2)
        assertEquals(100, s.exposureE2)
        assertEquals(-100, s.contrastE2)
    }

    @Test
    fun openIgnoresCropOutsideImage() = assertEquals(Rect(0, 0, 4032, 3024), PhotoEditor.open(draft(listOf(Edit.Crop(Rect(9000, 9000, 10, 10))))).crop)

    @Test
    fun adjustmentsListOnlyChanges() {
        val s = state().copy(crop = Rect(0, 0, 100, 100), rotateE2 = 150, saturationE2 = 5)
        assertEquals(listOf(Edit.Crop(Rect(0, 0, 100, 100)), Edit.Rotate(150), Edit.Tone(0, 0, 5)), PhotoEditor.adjustments(s))
        assertTrue(PhotoEditor.adjustments(s).all { it.isValid })
    }

    @Test
    fun applyReplacesManagedAndKeepsOthers() {
        val filter = Edit.Filter(3, 50)
        val d = draft(listOf(Edit.Crop(Rect(0, 0, 10, 10)), filter, Edit.Recompress(ImageFormat.JPEG, 90)))
        val s = PhotoEditor.open(d).copy(crop = Rect(0, 0, 200, 200), rotateE2 = 0)
        val out = PhotoEditor.apply(d, s)
        assertEquals(listOf(filter, Edit.Recompress(ImageFormat.JPEG, 90), Edit.Crop(Rect(0, 0, 200, 200))), out.adjustments)
    }

    @Test
    fun applyCarriesBlurs() {
        val b = BlurProposal(Rect(1, 1, 50, 50), BlurClass.FACE)
        val d = draft(blurs = listOf(b))
        val s = PhotoEditor.tapBlur(PhotoEditor.open(d), 10, 10)
        assertEquals(listOf(b.copy(enabled = false)), PhotoEditor.apply(d, s).blurs)
    }

    @Test
    fun applyingUntouchedEditorIsNoOp() {
        val d = draft(listOf(Edit.Rotate(100)))
        assertEquals(d, PhotoEditor.apply(d, PhotoEditor.open(d)))
    }

    @Test
    fun declaredCountIncludesBlurs() {
        val s = PhotoEditor.open(draft(blurs = listOf(BlurProposal(Rect(0, 0, 9, 9), BlurClass.PLATE)))).copy(rotateE2 = 10)
        assertEquals(2, PhotoEditor.declaredCount(s))
    }

    @Test
    fun squareAspectOnLandscapeIsCentered() {
        val s = PhotoEditor.withAspect(state(), CropAspect.SQUARE)
        assertEquals(Rect(504, 0, 3024, 3024), s.crop)
        assertEquals(CropAspect.SQUARE, s.aspect)
    }

    @Test
    fun portraitAspectKeepsRatio() {
        val s = PhotoEditor.withAspect(state(), CropAspect.PORTRAIT)
        assertEquals(3024, s.crop.h)
        assertEquals(2419, s.crop.w)
        assertTrue(abs(s.crop.w * 5 - s.crop.h * 4) < 5)
    }

    @Test
    fun storyAspectOnPortraitImage() {
        val s = PhotoEditor.withAspect(state(Dims(3024, 4032)), CropAspect.STORY)
        assertEquals(4032, s.crop.h)
        assertEquals(2268, s.crop.w)
    }

    @Test
    fun aspectFitsWidthWhenImageIsTall() {
        val s = PhotoEditor.withAspect(state(Dims(1000, 5000)), CropAspect.SQUARE)
        assertEquals(Rect(0, 2000, 1000, 1000), s.crop)
    }

    @Test
    fun aspectCentersOnCurrentCropAndClamps() {
        val s = PhotoEditor.withAspect(state().copy(crop = Rect(3900, 0, 132, 100)), CropAspect.SQUARE)
        assertEquals(Rect(1008, 0, 3024, 3024), s.crop)
    }

    @Test
    fun freeAspectKeepsCrop() {
        val s = state().copy(crop = Rect(5, 5, 50, 50), aspect = CropAspect.SQUARE)
        val out = PhotoEditor.withAspect(s, CropAspect.FREE)
        assertEquals(Rect(5, 5, 50, 50), out.crop)
        assertEquals(CropAspect.FREE, out.aspect)
    }

    @Test
    fun moveClampsInsideImage() {
        val crop = Rect(100, 100, 1000, 1000)
        assertEquals(Rect(150, 80, 1000, 1000), PhotoEditor.move(crop, dims, 50, -20))
        assertEquals(Rect(0, 0, 1000, 1000), PhotoEditor.move(crop, dims, -5000, -5000))
        assertEquals(Rect(3032, 2024, 1000, 1000), PhotoEditor.move(crop, dims, 99_999, 99_999))
    }

    @Test
    fun moveHandlesExtremeDeltas() = assertEquals(Rect(0, 0, 10, 10), PhotoEditor.move(Rect(0, 0, 10, 10), dims, Int.MIN_VALUE, Int.MIN_VALUE))

    @Test
    fun resizeFreeBottomRight() {
        assertEquals(Rect(100, 100, 1100, 900), PhotoEditor.resize(Rect(100, 100, 1000, 1000), dims, CropAspect.FREE, CropHandle.BOTTOM_RIGHT, 100, -100))
    }

    @Test
    fun resizeFreeTopLeftAnchorsBottomRight() {
        val r = PhotoEditor.resize(Rect(100, 100, 1000, 1000), dims, CropAspect.FREE, CropHandle.TOP_LEFT, 50, 60)
        assertEquals(Rect(150, 160, 950, 940), r)
        assertEquals(1100, r.right)
        assertEquals(1100, r.bottom)
    }

    @Test
    fun resizeClampsToImage() {
        assertEquals(Rect(0, 0, 1100, 1100), PhotoEditor.resize(Rect(100, 100, 1000, 1000), dims, CropAspect.FREE, CropHandle.TOP_LEFT, -9999, -9999))
        assertEquals(Rect(100, 100, 3932, 2924), PhotoEditor.resize(Rect(100, 100, 1000, 1000), dims, CropAspect.FREE, CropHandle.BOTTOM_RIGHT, 9999, 9999))
    }

    @Test
    fun resizeRespectsMinimum() {
        val r = PhotoEditor.resize(Rect(100, 100, 1000, 1000), dims, CropAspect.FREE, CropHandle.BOTTOM_RIGHT, -5000, -5000)
        assertEquals(PhotoEditor.MIN_CROP, r.w)
        assertEquals(PhotoEditor.MIN_CROP, r.h)
    }

    @Test
    fun resizeWithAspectKeepsRatio() {
        val r = PhotoEditor.resize(Rect(0, 0, 400, 500), dims, CropAspect.PORTRAIT, CropHandle.BOTTOM_RIGHT, 400, 0)
        assertEquals(Rect(0, 0, 800, 1000), r)
    }

    @Test
    fun resizeWithAspectUsesLargerIntent() {
        val r = PhotoEditor.resize(Rect(0, 0, 400, 400), dims, CropAspect.SQUARE, CropHandle.BOTTOM_RIGHT, 10, 300)
        assertEquals(Rect(0, 0, 700, 700), r)
    }

    @Test
    fun resizeWithAspectClampsToBounds() {
        val r = PhotoEditor.resize(Rect(0, 0, 400, 400), dims, CropAspect.SQUARE, CropHandle.BOTTOM_RIGHT, 99_999, 0)
        assertEquals(Rect(0, 0, 3024, 3024), r)
    }

    @Test
    fun resizeWithAspectTopLeftStaysInside() {
        val r = PhotoEditor.resize(Rect(1000, 1000, 400, 500), dims, CropAspect.PORTRAIT, CropHandle.TOP_LEFT, -99_999, 0)
        assertTrue(r.x >= 0 && r.y >= 0)
        assertEquals(1400, r.right)
        assertEquals(1500, r.bottom)
        assertEquals(r.w * 5, r.h * 4)
    }

    @Test
    fun resizeWithAspectMinimum() {
        val r = PhotoEditor.resize(Rect(0, 0, 400, 400), dims, CropAspect.STORY, CropHandle.BOTTOM_RIGHT, -10_000, -10_000)
        assertTrue(r.w >= PhotoEditor.MIN_CROP && r.h >= PhotoEditor.MIN_CROP, "$r")
    }

    @Test
    fun resizeOnTinyImage() {
        val tiny = Dims(10, 10)
        val r = PhotoEditor.resize(Rect(0, 0, 10, 10), tiny, CropAspect.FREE, CropHandle.BOTTOM_RIGHT, -100, -100)
        assertEquals(Rect(0, 0, 10, 10), r)
    }

    @Test
    fun dragDispatchesByHandle() {
        val s = state().copy(crop = Rect(100, 100, 1000, 1000))
        assertEquals(Rect(110, 110, 1000, 1000), PhotoEditor.drag(s, CropHandle.MOVE, 10, 10).crop)
        assertEquals(Rect(100, 100, 1010, 1010), PhotoEditor.drag(s, CropHandle.BOTTOM_RIGHT, 10, 10).crop)
    }

    @Test
    fun handleAtFindsCorners() {
        val crop = Rect(100, 100, 1000, 1000)
        assertEquals(CropHandle.TOP_LEFT, PhotoEditor.handleAt(crop, 105f, 95f, 20f))
        assertEquals(CropHandle.TOP_RIGHT, PhotoEditor.handleAt(crop, 1100f, 100f, 20f))
        assertEquals(CropHandle.BOTTOM_LEFT, PhotoEditor.handleAt(crop, 90f, 1110f, 20f))
        assertEquals(CropHandle.BOTTOM_RIGHT, PhotoEditor.handleAt(crop, 1099f, 1101f, 20f))
    }

    @Test
    fun handleAtInsideMovesOutsideNull() {
        val crop = Rect(100, 100, 1000, 1000)
        assertEquals(CropHandle.MOVE, PhotoEditor.handleAt(crop, 600f, 600f, 20f))
        assertNull(PhotoEditor.handleAt(crop, 10f, 10f, 20f))
    }

    @Test
    fun handleAtPrefersNearestCornerOnSmallCrops() =
        assertEquals(CropHandle.BOTTOM_RIGHT, PhotoEditor.handleAt(Rect(0, 0, 10, 10), 9f, 9f, 50f))

    @Test
    fun straightenClamps() {
        assertEquals(1000, PhotoEditor.straighten(state(), 5000).rotateE2)
        assertEquals(-1000, PhotoEditor.straighten(state(), -5000).rotateE2)
        assertEquals(250, PhotoEditor.straighten(state(), 250).rotateE2)
    }

    @Test
    fun setToneClampsPerChannel() {
        var s = PhotoEditor.setTone(state(), ToneChannel.EXPOSURE, 500)
        s = PhotoEditor.setTone(s, ToneChannel.CONTRAST, -40)
        s = PhotoEditor.setTone(s, ToneChannel.SATURATION, -500)
        assertEquals(100, s.tone(ToneChannel.EXPOSURE))
        assertEquals(-40, s.tone(ToneChannel.CONTRAST))
        assertEquals(-100, s.tone(ToneChannel.SATURATION))
    }

    @Test
    fun tapBlurAddsClippedSquare() {
        val s = PhotoEditor.tapBlur(state(), 0, 0)
        val b = s.blurs.single()
        assertEquals(BlurClass.OTHER, b.blurClass)
        assertEquals(Rect(0, 0, 189, 189), b.rect)
    }

    @Test
    fun tapBlurTogglesExisting() {
        val s1 = PhotoEditor.tapBlur(state(), 2000, 1500)
        val s2 = PhotoEditor.tapBlur(s1, 2000, 1500)
        assertEquals(1, s2.blurs.size)
        assertEquals(false, s2.blurs.single().enabled)
        assertEquals(0, s2.enabledBlurs)
        assertEquals(true, PhotoEditor.tapBlur(s2, 2000, 1500).blurs.single().enabled)
    }

    @Test
    fun tapBlurHitsTopmost() {
        val a = BlurProposal(Rect(0, 0, 100, 100), BlurClass.FACE)
        val b = BlurProposal(Rect(50, 50, 100, 100), BlurClass.PLATE)
        val s = PhotoEditor.tapBlur(PhotoEditor.open(draft(blurs = listOf(a, b))), 60, 60)
        assertEquals(listOf(a, b.copy(enabled = false)), s.blurs)
    }

    @Test
    fun tapOutsideImageIgnored() {
        val s = state()
        assertSame(s, PhotoEditor.tapBlur(s, -1, 10))
        assertSame(s, PhotoEditor.tapBlur(s, 4032, 10))
    }

    @Test
    fun tapBlurCapped() {
        var s = state()
        repeat(PhotoEditor.MAX_BLURS) { s = s.copy(blurs = s.blurs + BlurProposal(Rect(it, 0, 1, 1), BlurClass.OTHER)) }
        assertSame(s, PhotoEditor.tapBlur(s, 3000, 3000))
    }

    @Test
    fun tapBlurOnTinyImageUsesMinimumSide() {
        val s = PhotoEditor.tapBlur(state(Dims(40, 40)), 20, 20)
        assertEquals(Rect(12, 12, 16, 16), s.blurs.single().rect)
    }

    @Test
    fun blurEditsStayWithinEditLimit() = assertTrue(PhotoEditor.MAX_BLURS + 3 <= Edit.MAX_EDITS)

    @Test
    fun resetRestoresIdentityAndKeepsBlurs() {
        val b = BlurProposal(Rect(0, 0, 10, 10), BlurClass.FACE)
        val s = PhotoEditor.open(draft(blurs = listOf(b))).copy(crop = Rect(0, 0, 10, 10), rotateE2 = 300, exposureE2 = 20, aspect = CropAspect.SQUARE, tool = EditTool.TONE)
        val r = PhotoEditor.reset(s)
        assertTrue(PhotoEditor.adjustments(r).isEmpty())
        assertEquals(listOf(b), r.blurs)
        assertEquals(EditTool.TONE, r.tool)
        assertEquals(CropAspect.FREE, r.aspect)
    }

    @Test
    fun identityToneMatrix() {
        val m = PhotoEditor.toneMatrix(0, 0, 0)
        val identity = floatArrayOf(1f, 0f, 0f, 0f, 0f, 0f, 1f, 0f, 0f, 0f, 0f, 0f, 1f, 0f, 0f, 0f, 0f, 0f, 1f, 0f)
        for (i in m.indices) assertTrue(abs(m[i] - identity[i]) < 1e-6f, "index $i")
        assertTrue(PhotoEditor.isIdentityTone(state()))
    }

    @Test
    fun fullDesaturationMakesRowsEqual() {
        val m = PhotoEditor.toneMatrix(0, 0, -100)
        for (c in 0..2) {
            assertEquals(m[c], m[5 + c], 1e-6f)
            assertEquals(m[c], m[10 + c], 1e-6f)
        }
    }

    @Test
    fun exposureDoublesGain() {
        val m = PhotoEditor.toneMatrix(100, 0, 0)
        assertEquals(2f, m[0], 1e-5f)
        assertEquals(2f, m[6], 1e-5f)
        assertEquals(0f, m[4])
    }

    @Test
    fun contrastPivotsAroundMidGrey() {
        val m = PhotoEditor.toneMatrix(0, 50, 0)
        assertEquals(1.5f, m[0], 1e-5f)
        assertEquals(-64f, m[4], 1e-4f)
        assertEquals(128f, 128f * m[0] + m[4], 1e-3f)
    }

    @Test
    fun alphaRowUntouched() {
        val m = PhotoEditor.toneMatrix(37, -12, 80)
        assertEquals(listOf(0f, 0f, 0f, 1f, 0f), m.slice(15..19))
        assertEquals(20, m.size)
    }

    @Test
    fun coverScale() {
        assertEquals(1f, PhotoEditor.coverScale(0, 4032, 3024))
        assertEquals(1f, PhotoEditor.coverScale(500, 0, 3024))
        val s = PhotoEditor.coverScale(1000, 4032, 3024)
        assertTrue(s > 1.1f && s < 1.3f, "$s")
        assertEquals(s, PhotoEditor.coverScale(-1000, 4032, 3024))
        assertEquals(s, PhotoEditor.coverScale(1000, 3024, 4032))
    }

    @Test
    fun degreesText() {
        assertEquals("0.0°", PhotoEditor.degreesText(0))
        assertEquals("2.5°", PhotoEditor.degreesText(250))
        assertEquals("-2.5°", PhotoEditor.degreesText(-250))
        assertEquals("10.0°", PhotoEditor.degreesText(1000))
        assertEquals("0.1°", PhotoEditor.degreesText(5))
        assertEquals("0.0°", PhotoEditor.degreesText(-4))
        assertEquals("-0.1°", PhotoEditor.degreesText(-5))
    }

    @Test
    fun toneText() {
        assertEquals("+25", PhotoEditor.toneText(25))
        assertEquals("0", PhotoEditor.toneText(0))
        assertEquals("-10", PhotoEditor.toneText(-10))
    }

    @Test
    fun managedKindsAreGlobal() = assertEquals(setOf(EditKind.CROP, EditKind.ROTATE, EditKind.TONE), PhotoEditor.MANAGED)

    @Test
    fun aspectsMatchDesignOrder() = assertEquals(listOf("Free", "1:1", "4:5", "9:16"), CropAspect.entries.map { it.label })

    @Test
    fun toolsMatchDesignOrder() = assertEquals(listOf("Crop", "Straighten", "Tone", "Blur"), EditTool.entries.map { it.label })
}
