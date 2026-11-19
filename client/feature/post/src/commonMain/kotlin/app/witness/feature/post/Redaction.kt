package app.witness.feature.post

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import app.witness.core.domain.usecase.BuildPost
import app.witness.core.model.BlurProposal
import app.witness.core.model.Dims
import app.witness.core.model.Edit
import app.witness.core.ui.NormRect

fun BlurProposal.normalized(dims: Dims): NormRect? {
    if (!dims.isValid) return null
    val r = rect.clippedTo(dims.width, dims.height) ?: return null
    val w = dims.width.toFloat()
    val h = dims.height.toFloat()
    return NormRect(r.x / w, r.y / h, r.right / w, r.bottom / h)
}

fun fitFrame(dims: Dims, size: Size): Rect {
    if (!dims.isValid || size.width <= 0f || size.height <= 0f) return Rect(Offset.Zero, size)
    val k = minOf(size.width / dims.width, size.height / dims.height)
    val w = dims.width * k
    val h = dims.height * k
    val left = (size.width - w) / 2f
    val top = (size.height - h) / 2f
    return Rect(left, top, left + w, top + h)
}

fun Rect.place(norm: NormRect): Rect = Rect(left + norm.left * width, top + norm.top * height, left + norm.right * width, top + norm.bottom * height)

fun blurEdits(blurs: List<BlurProposal>, dims: Dims): List<Edit.Blur> {
    if (!dims.isValid) return emptyList()
    val out = ArrayList<Edit.Blur>(blurs.size)
    for (b in blurs) {
        if (!b.enabled) continue
        val r = b.rect.clippedTo(dims.width, dims.height) ?: continue
        out += Edit.Blur(r, b.blurClass, BuildPost.sigmaFor(r.w, r.h))
    }
    return out
}

fun previewScale(decodedWidth: Int, sourceWidth: Int): Float = if (decodedWidth <= 0 || sourceWidth <= 0) 1f else decodedWidth.toFloat() / sourceWidth
