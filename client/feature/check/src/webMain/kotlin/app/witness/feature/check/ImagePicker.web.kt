package app.witness.feature.check

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import app.witness.core.model.ByteString
import kotlinx.browser.document
import org.khronos.webgl.ArrayBuffer
import org.khronos.webgl.Int8Array
import org.khronos.webgl.toByteArray
import org.w3c.dom.HTMLInputElement
import org.w3c.files.FileReader
import org.w3c.files.get

@Composable
actual fun rememberImagePicker(onPicked: (PickedImage) -> Unit): ImagePicker {
    val callback = rememberUpdatedState(onPicked)
    return remember {
        ImagePicker {
            val input = document.createElement("input") as HTMLInputElement
            input.type = "file"
            input.accept = "image/*"
            input.onchange = {
                val file = input.files?.get(0)
                if (file != null && file.size.toDouble() <= PickedImage.MAX_BYTES) {
                    val reader = FileReader()
                    reader.onload = {
                        val buffer = reader.result as? ArrayBuffer
                        if (buffer != null) callback.value(PickedImage(ByteString.wrap(Int8Array(buffer).toByteArray()), file.name))
                    }
                    reader.readAsArrayBuffer(file)
                }
            }
            input.click()
        }
    }
}
