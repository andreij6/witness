package app.witness.core.ui

import android.graphics.Bitmap
import app.witness.core.model.Edit
import app.witness.core.model.Rect

fun Bitmap.blurRegion(rect: Rect, sigmaE2: Int): Boolean {
    require(isMutable) { "bitmap must be mutable" }
    val r = rect.clippedTo(width, height) ?: return false
    val pixels = IntArray(r.w * r.h)
    getPixels(pixels, 0, r.w, r.x, r.y, r.w, r.h)
    RegionBlur.blur(pixels, r.w, r.h, sigmaE2)
    setPixels(pixels, 0, r.w, r.x, r.y, r.w, r.h)
    return true
}

fun Bitmap.applyBlurs(blurs: List<Edit.Blur>, scale: Float = 1f) {
    for (b in blurs) blurRegion(b.rect.scaled(scale), RegionBlur.scaledSigma(b.sigmaE2, scale))
}
