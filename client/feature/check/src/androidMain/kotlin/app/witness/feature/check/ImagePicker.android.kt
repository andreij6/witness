package app.witness.feature.check

import android.content.ContentResolver
import android.graphics.BitmapFactory
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.platform.LocalContext
import app.witness.core.model.ByteString
import app.witness.core.model.Dims
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
actual fun rememberImagePicker(onPicked: (PickedImage) -> Unit): ImagePicker {
    val resolver = LocalContext.current.contentResolver
    val scope = rememberCoroutineScope()
    val callback = rememberUpdatedState(onPicked)
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) scope.launch {
            val image = withContext(Dispatchers.IO) { readImage(resolver, uri) }
            if (image != null) callback.value(image)
        }
    }
    return remember(launcher) {
        ImagePicker { launcher.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)) }
    }
}

fun readImage(resolver: ContentResolver, uri: Uri): PickedImage? {
    val bytes = resolver.openInputStream(uri)?.use { input -> readBounded(input, PickedImage.MAX_BYTES) } ?: return null
    val name = resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
        if (c.moveToFirst()) c.getString(0) else null
    }
    return PickedImage(ByteString.wrap(bytes), name, boundsOf(bytes))
}

internal fun readBounded(input: java.io.InputStream, limit: Int): ByteArray? {
    val out = java.io.ByteArrayOutputStream()
    val buffer = ByteArray(64 * 1024)
    var total = 0
    while (true) {
        val n = input.read(buffer)
        if (n < 0) break
        total += n
        if (total > limit) return null
        out.write(buffer, 0, n)
    }
    return out.toByteArray()
}

internal fun boundsOf(bytes: ByteArray): Dims? {
    val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeByteArray(bytes, 0, bytes.size, options)
    return Dims(options.outWidth, options.outHeight).takeIf { it.isValid }
}
