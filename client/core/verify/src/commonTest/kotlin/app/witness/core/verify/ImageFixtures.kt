package app.witness.core.verify

object ImageFixtures {
    private fun be16(v: Int) = byteArrayOf((v ushr 8).toByte(), v.toByte())
    private fun le16(v: Int) = byteArrayOf(v.toByte(), (v ushr 8).toByte())
    private fun le24(v: Int) = byteArrayOf(v.toByte(), (v ushr 8).toByte(), (v ushr 16).toByte())
    private fun be32(v: Int) = be16(v ushr 16) + be16(v)
    private fun le32(v: Int) = le16(v) + le16(v ushr 16)
    private fun ascii(s: String) = s.encodeToByteArray()

    fun png(w: Int, h: Int): ByteArray =
        byteArrayOf(0x89.toByte(), 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a) + be32(13) + ascii("IHDR") + be32(w) + be32(h) + byteArrayOf(8, 2, 0, 0, 0)

    fun gif(w: Int, h: Int, version: String = "89a"): ByteArray = ascii("GIF$version") + le16(w) + le16(h) + byteArrayOf(0, 0, 0)

    private fun riff(chunk: String, body: ByteArray) = ascii("RIFF") + le32(4 + 8 + body.size) + ascii("WEBP") + ascii(chunk) + le32(body.size) + body

    fun webpLossy(w: Int, h: Int): ByteArray = riff("VP8 ", byteArrayOf(0, 0, 0, 0x9d.toByte(), 0x01, 0x2a) + le16(w) + le16(h) + ByteArray(4))

    fun webpLossless(w: Int, h: Int): ByteArray = riff("VP8L", byteArrayOf(0x2f) + le32((w - 1) or ((h - 1) shl 14)) + ByteArray(8))

    fun webpExtended(w: Int, h: Int): ByteArray = riff("VP8X", ByteArray(4) + le24(w - 1) + le24(h - 1) + ByteArray(4))

    fun exif(orientation: Int, little: Boolean = true): ByteArray {
        val u16 = if (little) ::le16 else ::be16
        val u32 = if (little) ::le32 else ::be32
        val tiff = ascii(if (little) "II" else "MM") + u16(42) + u32(8) + u16(1) + u16(0x0112) + u16(3) + u32(1) + u16(orientation) + u16(0) + u32(0)
        return ascii("Exif") + byteArrayOf(0, 0) + tiff
    }

    private fun segment(marker: Int, body: ByteArray) = byteArrayOf(0xff.toByte(), marker.toByte()) + be16(body.size + 2) + body

    fun jpeg(w: Int, h: Int, sof: Int = 0xc0, app1: ByteArray? = null, fill: Int = 0): ByteArray {
        var out = byteArrayOf(0xff.toByte(), 0xd8.toByte())
        out += segment(0xe0, ascii("JFIF") + byteArrayOf(0, 1, 1, 0, 0, 1, 0, 1, 0, 0))
        if (app1 != null) out += segment(0xe1, app1)
        out += segment(0xdb, ByteArray(65))
        out += segment(0xc4, ByteArray(20))
        out += ByteArray(fill) { 0xff.toByte() }
        out += segment(sof, byteArrayOf(8) + be16(h) + be16(w) + byteArrayOf(3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1))
        out += segment(0xda, ByteArray(10))
        out += byteArrayOf(0x12, 0x34, 0xff.toByte(), 0xd9.toByte())
        return out
    }
}
