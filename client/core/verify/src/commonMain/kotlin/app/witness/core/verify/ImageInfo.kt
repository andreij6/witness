package app.witness.core.verify

enum class ImageFormat { JPEG, PNG, GIF, WEBP, UNKNOWN }

data class ImageInfo(val format: ImageFormat, val width: Int, val height: Int, val byteCount: Int)

fun interface ImageInfoReader {
    suspend fun read(bytes: ByteArray): ImageInfo?
}

expect fun platformImageInfoReader(): ImageInfoReader

class SniffingImageInfoReader(private val fallback: suspend (ByteArray) -> Pair<Int, Int>?) : ImageInfoReader {
    override suspend fun read(bytes: ByteArray): ImageInfo? {
        ImageSniffer.sniff(bytes)?.let { return it }
        val (w, h) = fallback(bytes) ?: return null
        if (w <= 0 || h <= 0) return null
        return ImageInfo(ImageSniffer.format(bytes), w, h, bytes.size)
    }
}
