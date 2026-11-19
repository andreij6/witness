package app.witness.feature.post

import android.graphics.Bitmap
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.graphics.asImageBitmap
import app.witness.core.model.Edit
import app.witness.core.ui.applyBlurs

internal actual fun blurPhoto(image: ImageBitmap, blurs: List<Edit.Blur>, scale: Float): ImageBitmap =
    image.asAndroidBitmap().copy(Bitmap.Config.ARGB_8888, true).apply { applyBlurs(blurs, scale) }.asImageBitmap()
