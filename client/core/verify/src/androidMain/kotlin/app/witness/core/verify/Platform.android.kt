package app.witness.core.verify

import android.graphics.BitmapFactory

actual fun createVerifyEngine(config: VerifyConfig): VerifyEngine = FakeVerifyEngine()

actual fun platformImageInfoReader(): ImageInfoReader = SniffingImageInfoReader { bytes ->
    val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeByteArray(bytes, 0, bytes.size, options)
    if (options.outWidth > 0 && options.outHeight > 0) options.outWidth to options.outHeight else null
}
