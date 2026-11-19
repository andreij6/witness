package app.witness.core.verify

object ImageSniffer {
    private val PNG_SIGNATURE = byteArrayOf(0x89.toByte(), 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a)

    fun format(b: ByteArray): ImageFormat = when {
        b.size >= 3 && b.u8(0) == 0xff && b.u8(1) == 0xd8 && b.u8(2) == 0xff -> ImageFormat.JPEG
        b.size >= 8 && (0 until 8).all { b[it] == PNG_SIGNATURE[it] } -> ImageFormat.PNG
        b.size >= 6 && b.ascii(0, "GIF8") && (b.ascii(4, "7a") || b.ascii(4, "9a")) -> ImageFormat.GIF
        b.size >= 12 && b.ascii(0, "RIFF") && b.ascii(8, "WEBP") -> ImageFormat.WEBP
        else -> ImageFormat.UNKNOWN
    }

    fun sniff(b: ByteArray): ImageInfo? {
        val format = format(b)
        val dims = when (format) {
            ImageFormat.JPEG -> jpeg(b)
            ImageFormat.PNG -> png(b)
            ImageFormat.GIF -> gif(b)
            ImageFormat.WEBP -> webp(b)
            ImageFormat.UNKNOWN -> null
        } ?: return null
        val w = (dims ushr 32).toInt()
        val h = dims.toInt()
        if (w <= 0 || h <= 0) return null
        return ImageInfo(format, w, h, b.size)
    }

    private fun dims(w: Int, h: Int): Long = (w.toLong() shl 32) or (h.toLong() and 0xffff_ffffL)

    private fun png(b: ByteArray): Long? {
        if (b.size < 24 || !b.ascii(12, "IHDR")) return null
        val w = b.be32(16)
        val h = b.be32(20)
        return if (w <= 0 || h <= 0) null else dims(w, h)
    }

    private fun gif(b: ByteArray): Long? = if (b.size < 10) null else dims(b.le16(6), b.le16(8))

    private fun webp(b: ByteArray): Long? {
        if (b.size < 30) return null
        return when {
            b.ascii(12, "VP8 ") -> {
                if (b.u8(23) != 0x9d || b.u8(24) != 0x01 || b.u8(25) != 0x2a) null
                else dims(b.le16(26) and 0x3fff, b.le16(28) and 0x3fff)
            }
            b.ascii(12, "VP8L") -> {
                if (b.u8(20) != 0x2f) return null
                val bits = b.u8(21) or (b.u8(22) shl 8) or (b.u8(23) shl 16) or (b.u8(24) shl 24)
                dims((bits and 0x3fff) + 1, ((bits ushr 14) and 0x3fff) + 1)
            }
            b.ascii(12, "VP8X") -> dims(b.le24(24) + 1, b.le24(27) + 1)
            else -> null
        }
    }

    private fun jpeg(b: ByteArray): Long? {
        var at = 2
        var orientation = 1
        while (at + 4 <= b.size) {
            if (b.u8(at) != 0xff) return null
            var marker = b.u8(at + 1)
            while (marker == 0xff && at + 2 < b.size) {
                at++
                marker = b.u8(at + 1)
            }
            at += 2
            if (marker == 0xd8 || marker == 0x01 || marker in 0xd0..0xd7) continue
            if (marker == 0xd9 || marker == 0xda) return null
            if (at + 2 > b.size) return null
            val len = b.be16(at)
            if (len < 2 || at + len > b.size) return null
            if (marker == 0xe1 && orientation == 1) orientation = exifOrientation(b, at + 2, at + len)
            if (isSof(marker)) {
                if (len < 7) return null
                val h = b.be16(at + 3)
                val w = b.be16(at + 5)
                return if (orientation in 5..8) dims(h, w) else dims(w, h)
            }
            at += len
        }
        return null
    }

    private fun isSof(m: Int): Boolean = m in 0xc0..0xcf && m != 0xc4 && m != 0xc8 && m != 0xcc

    internal fun exifOrientation(b: ByteArray, start: Int, end: Int): Int {
        if (end - start < 14 || !b.ascii(start, "Exif") || b[start + 4] != 0.toByte() || b[start + 5] != 0.toByte()) return 1
        val tiff = start + 6
        val little = when {
            b.ascii(tiff, "II") -> true
            b.ascii(tiff, "MM") -> false
            else -> return 1
        }
        fun u16(at: Int) = if (little) b.le16(at) else b.be16(at)
        fun u32(at: Int) = if (little) b.le32(at) else b.be32(at)
        if (u16(tiff + 2) != 42) return 1
        val ifd = u32(tiff + 4)
        if (ifd < 8 || ifd > end - tiff - 2) return 1
        val dir = tiff + ifd
        val count = u16(dir)
        for (i in 0 until count) {
            val entry = dir + 2 + i * 12
            if (entry + 12 > end) return 1
            if (u16(entry) == 0x0112) {
                val v = u16(entry + 8)
                return if (v in 1..8) v else 1
            }
        }
        return 1
    }

    private fun ByteArray.u8(at: Int): Int = this[at].toInt() and 0xff
    private fun ByteArray.be16(at: Int): Int = (u8(at) shl 8) or u8(at + 1)
    private fun ByteArray.le16(at: Int): Int = u8(at) or (u8(at + 1) shl 8)
    private fun ByteArray.le24(at: Int): Int = u8(at) or (u8(at + 1) shl 8) or (u8(at + 2) shl 16)
    private fun ByteArray.be32(at: Int): Int = (be16(at) shl 16) or be16(at + 2)
    private fun ByteArray.le32(at: Int): Int = le16(at) or (le16(at + 2) shl 16)

    private fun ByteArray.ascii(at: Int, s: String): Boolean =
        at >= 0 && at + s.length <= size && s.indices.all { this[at + it].toInt() == s[it].code }
}
