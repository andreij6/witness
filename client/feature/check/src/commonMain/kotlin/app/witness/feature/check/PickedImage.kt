package app.witness.feature.check

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import app.witness.core.model.ByteString
import app.witness.core.model.Dims

@Immutable
class PickedImage(val bytes: ByteString, val name: String? = null, val dims: Dims? = null) {
    val format: String? get() = sniffFormat(bytes)

    val size: Int get() = bytes.size

    companion object {
        const val MAX_BYTES: Int = 64 * 1024 * 1024

        fun sniffFormat(b: ByteString): String? = when {
            b.size >= 3 && b[0] == 0xFF.toByte() && b[1] == 0xD8.toByte() && b[2] == 0xFF.toByte() -> "JPEG"
            b.size >= 8 && b.startsWith(PNG) -> "PNG"
            b.size >= 12 && b.startsWith(RIFF) && b.matches(8, WEBP) -> "WEBP"
            b.size >= 12 && b.matches(4, FTYP) && b.matches(8, AVIF) -> "AVIF"
            b.size >= 12 && b.matches(4, FTYP) && (b.matches(8, HEIC) || b.matches(8, HEIX) || b.matches(8, MIF1)) -> "HEIC"
            else -> null
        }

        private val PNG = byteArrayOf(0x89.toByte(), 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A)
        private val RIFF = "RIFF".encodeToByteArray()
        private val WEBP = "WEBP".encodeToByteArray()
        private val FTYP = "ftyp".encodeToByteArray()
        private val AVIF = "avif".encodeToByteArray()
        private val HEIC = "heic".encodeToByteArray()
        private val HEIX = "heix".encodeToByteArray()
        private val MIF1 = "mif1".encodeToByteArray()

        private fun ByteString.startsWith(prefix: ByteArray): Boolean = matches(0, prefix)

        private fun ByteString.matches(at: Int, part: ByteArray): Boolean {
            if (at + part.size > size) return false
            for (i in part.indices) if (this[at + i] != part[i]) return false
            return true
        }
    }
}

fun interface ImagePicker {
    fun launch()
}

@Composable
expect fun rememberImagePicker(onPicked: (PickedImage) -> Unit): ImagePicker
