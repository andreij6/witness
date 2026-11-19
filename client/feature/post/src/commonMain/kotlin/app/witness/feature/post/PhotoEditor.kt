package app.witness.feature.post

import androidx.compose.runtime.Immutable
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.model.EditKind
import app.witness.core.model.PostDraft
import app.witness.core.model.Rect
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin

enum class EditTool(val label: String, val icon: WitnessIcon) {
    CROP("Crop", WitnessIcon.Crop),
    STRAIGHTEN("Straighten", WitnessIcon.Straighten),
    TONE("Tone", WitnessIcon.Tonality),
    BLUR("Blur", WitnessIcon.BlurOn),
}

enum class CropAspect(val label: String, val w: Int, val h: Int) {
    FREE("Free", 0, 0),
    SQUARE("1:1", 1, 1),
    PORTRAIT("4:5", 4, 5),
    STORY("9:16", 9, 16),
}

enum class CropHandle(val sx: Int, val sy: Int) {
    MOVE(0, 0),
    TOP_LEFT(-1, -1),
    TOP_RIGHT(1, -1),
    BOTTOM_LEFT(-1, 1),
    BOTTOM_RIGHT(1, 1),
}

enum class ToneChannel(val label: String) { EXPOSURE("Exposure"), CONTRAST("Contrast"), SATURATION("Saturation") }

@Immutable
data class EditorState(
    val dims: Dims,
    val crop: Rect,
    val blurs: List<BlurProposal>,
    val tool: EditTool = EditTool.CROP,
    val aspect: CropAspect = CropAspect.FREE,
    val rotateE2: Int = 0,
    val exposureE2: Int = 0,
    val contrastE2: Int = 0,
    val saturationE2: Int = 0,
) {
    val full: Rect get() = Rect(0, 0, dims.width, dims.height)
    val enabledBlurs: Int get() = blurs.count { it.enabled }

    fun tone(channel: ToneChannel): Int = when (channel) {
        ToneChannel.EXPOSURE -> exposureE2
        ToneChannel.CONTRAST -> contrastE2
        ToneChannel.SATURATION -> saturationE2
    }
}

object PhotoEditor {
    const val MIN_CROP: Int = 64
    const val MAX_TONE_E2: Int = 100
    const val MAX_BLURS: Int = Edit.MAX_EDITS - 8
    const val BLUR_FRACTION: Int = 8
    const val MIN_BLUR: Int = 16
    val MANAGED: Set<EditKind> = setOf(EditKind.CROP, EditKind.ROTATE, EditKind.TONE)

    fun open(draft: PostDraft): EditorState {
        val dims = draft.dims
        var crop = Rect(0, 0, dims.width, dims.height)
        var rotate = 0
        var tone: Edit.Tone? = null
        for (e in draft.adjustments) when (e) {
            is Edit.Crop -> e.rect.clippedTo(dims.width, dims.height)?.let { crop = it }
            is Edit.Rotate -> rotate = e.degE2.coerceIn(-Edit.MAX_ROTATE_DEG_E2, Edit.MAX_ROTATE_DEG_E2)
            is Edit.Tone -> tone = e
            else -> Unit
        }
        return EditorState(
            dims = dims,
            crop = crop,
            blurs = draft.blurs,
            rotateE2 = rotate,
            exposureE2 = clampTone(tone?.exposureE2 ?: 0),
            contrastE2 = clampTone(tone?.contrastE2 ?: 0),
            saturationE2 = clampTone(tone?.saturationE2 ?: 0),
        )
    }

    fun adjustments(s: EditorState): List<Edit> = buildList(3) {
        if (s.crop != s.full) add(Edit.Crop(s.crop))
        if (s.rotateE2 != 0) add(Edit.Rotate(s.rotateE2))
        if (s.exposureE2 != 0 || s.contrastE2 != 0 || s.saturationE2 != 0) add(Edit.Tone(s.exposureE2, s.contrastE2, s.saturationE2))
    }

    fun apply(draft: PostDraft, s: EditorState): PostDraft =
        draft.copy(adjustments = draft.adjustments.filter { it.kind !in MANAGED } + adjustments(s), blurs = s.blurs)

    fun declaredCount(s: EditorState): Int = adjustments(s).size + s.enabledBlurs

    fun minCrop(dims: Dims): Int = min(MIN_CROP, min(dims.width, dims.height))

    fun withAspect(s: EditorState, aspect: CropAspect): EditorState {
        if (aspect == CropAspect.FREE) return s.copy(aspect = aspect)
        val dw = s.dims.width.toLong()
        val dh = s.dims.height.toLong()
        val w: Long
        val h: Long
        if (dw * aspect.h <= dh * aspect.w) {
            w = dw
            h = dw * aspect.h / aspect.w
        } else {
            h = dh
            w = dh * aspect.w / aspect.h
        }
        val cx = s.crop.x.toLong() + s.crop.w / 2
        val cy = s.crop.y.toLong() + s.crop.h / 2
        val x = (cx - w / 2).coerceIn(0, dw - w)
        val y = (cy - h / 2).coerceIn(0, dh - h)
        return s.copy(aspect = aspect, crop = Rect(x.toInt(), y.toInt(), w.toInt(), h.toInt()))
    }

    fun drag(s: EditorState, handle: CropHandle, dx: Int, dy: Int): EditorState =
        s.copy(crop = if (handle == CropHandle.MOVE) move(s.crop, s.dims, dx, dy) else resize(s.crop, s.dims, s.aspect, handle, dx, dy))

    fun move(crop: Rect, dims: Dims, dx: Int, dy: Int): Rect {
        val x = (crop.x.toLong() + dx).coerceIn(0, (dims.width - crop.w).toLong()).toInt()
        val y = (crop.y.toLong() + dy).coerceIn(0, (dims.height - crop.h).toLong()).toInt()
        return crop.copy(x = x, y = y)
    }

    fun resize(crop: Rect, dims: Dims, aspect: CropAspect, handle: CropHandle, dx: Int, dy: Int): Rect {
        val ax = if (handle.sx > 0) crop.x else crop.right
        val ay = if (handle.sy > 0) crop.y else crop.bottom
        val availW = if (handle.sx > 0) dims.width - ax else ax
        val availH = if (handle.sy > 0) dims.height - ay else ay
        val minSize = minCrop(dims)
        val wantW = crop.w.toLong() + handle.sx.toLong() * dx
        val wantH = crop.h.toLong() + handle.sy.toLong() * dy
        val w: Int
        val h: Int
        if (aspect == CropAspect.FREE) {
            w = wantW.coerceIn(min(minSize, availW).toLong(), availW.toLong()).toInt()
            h = wantH.coerceIn(min(minSize, availH).toLong(), availH.toLong()).toInt()
        } else {
            val maxW = min(availW.toLong(), availH.toLong() * aspect.w / aspect.h)
            val minW = min(maxW, max(minSize.toLong(), (minSize.toLong() * aspect.w + aspect.h - 1) / aspect.h))
            val byH = wantH * aspect.w / aspect.h
            w = max(wantW, byH).coerceIn(minW, maxW).toInt()
            h = (w.toLong() * aspect.h / aspect.w).toInt()
        }
        val x = if (handle.sx > 0) ax else ax - w
        val y = if (handle.sy > 0) ay else ay - h
        return Rect(x, y, max(w, 1), max(h, 1))
    }

    fun handleAt(crop: Rect, x: Float, y: Float, slop: Float): CropHandle? {
        var best: CropHandle? = null
        var bestD = slop * slop
        for (h in CropHandle.entries) {
            if (h == CropHandle.MOVE) continue
            val hx = if (h.sx > 0) crop.right.toFloat() else crop.x.toFloat()
            val hy = if (h.sy > 0) crop.bottom.toFloat() else crop.y.toFloat()
            val d = (hx - x) * (hx - x) + (hy - y) * (hy - y)
            if (d <= bestD) {
                bestD = d
                best = h
            }
        }
        if (best != null) return best
        return if (x >= crop.x && x <= crop.right && y >= crop.y && y <= crop.bottom) CropHandle.MOVE else null
    }

    fun straighten(s: EditorState, degE2: Int): EditorState = s.copy(rotateE2 = degE2.coerceIn(-Edit.MAX_ROTATE_DEG_E2, Edit.MAX_ROTATE_DEG_E2))

    fun setTone(s: EditorState, channel: ToneChannel, valueE2: Int): EditorState {
        val v = clampTone(valueE2)
        return when (channel) {
            ToneChannel.EXPOSURE -> s.copy(exposureE2 = v)
            ToneChannel.CONTRAST -> s.copy(contrastE2 = v)
            ToneChannel.SATURATION -> s.copy(saturationE2 = v)
        }
    }

    fun tapBlur(s: EditorState, x: Int, y: Int): EditorState {
        if (x !in 0 until s.dims.width || y !in 0 until s.dims.height) return s
        for (i in s.blurs.indices.reversed()) {
            val r = s.blurs[i].rect
            if (x >= r.x && x < r.right && y >= r.y && y < r.bottom) {
                val out = s.blurs.toMutableList()
                out[i] = out[i].copy(enabled = !out[i].enabled)
                return s.copy(blurs = out)
            }
        }
        if (s.blurs.size >= MAX_BLURS) return s
        val side = max(MIN_BLUR, min(s.dims.width, s.dims.height) / BLUR_FRACTION)
        val rect = Rect(x - side / 2, y - side / 2, side, side).clippedTo(s.dims.width, s.dims.height) ?: return s
        return s.copy(blurs = s.blurs + BlurProposal(rect, BlurClass.OTHER))
    }

    fun reset(s: EditorState): EditorState = EditorState(s.dims, s.full, s.blurs, s.tool)

    private fun clampTone(v: Int): Int = v.coerceIn(-MAX_TONE_E2, MAX_TONE_E2)

    fun toneMatrix(exposureE2: Int, contrastE2: Int, saturationE2: Int): FloatArray {
        val gainExposure = 2.0.pow(exposureE2 / 100.0).toFloat()
        val c = 1f + contrastE2 / 100f
        val s = 1f + saturationE2 / 100f
        val g = gainExposure * c
        val offset = 128f * (1f - c)
        val lr = LUMA_R * (1 - s)
        val lg = LUMA_G * (1 - s)
        val lb = LUMA_B * (1 - s)
        return floatArrayOf(
            g * (lr + s), g * lg, g * lb, 0f, offset,
            g * lr, g * (lg + s), g * lb, 0f, offset,
            g * lr, g * lg, g * (lb + s), 0f, offset,
            0f, 0f, 0f, 1f, 0f,
        )
    }

    fun isIdentityTone(s: EditorState): Boolean = s.exposureE2 == 0 && s.contrastE2 == 0 && s.saturationE2 == 0

    fun coverScale(degE2: Int, width: Int, height: Int): Float {
        if (degE2 == 0 || width <= 0 || height <= 0) return 1f
        val rad = abs(degE2) / 100.0 * PI / 180.0
        val ratio = max(width.toDouble() / height, height.toDouble() / width)
        return (cos(rad) + sin(rad) * ratio).toFloat()
    }

    fun degreesText(degE2: Int): String {
        val tenths = (abs(degE2.toLong()) + 5) / 10
        val sign = if (degE2 < 0 && tenths != 0L) "-" else ""
        return "$sign${tenths / 10}.${tenths % 10}°"
    }

    fun toneText(valueE2: Int): String = if (valueE2 > 0) "+$valueE2" else valueE2.toString()

    private const val LUMA_R = 0.2126f
    private const val LUMA_G = 0.7152f
    private const val LUMA_B = 0.0722f
}
