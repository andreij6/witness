package app.witness.core.verify

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class ImageSnifferTest {
    private fun dims(bytes: ByteArray) = ImageSniffer.sniff(bytes)?.let { Triple(it.format, it.width, it.height) }

    @Test
    fun png() {
        assertEquals(Triple(ImageFormat.PNG, 1080, 1920), dims(ImageFixtures.png(1080, 1920)))
        assertEquals(29, ImageSniffer.sniff(ImageFixtures.png(1, 1))?.byteCount)
    }

    @Test
    fun gifBothVersions() {
        assertEquals(Triple(ImageFormat.GIF, 640, 480), dims(ImageFixtures.gif(640, 480)))
        assertEquals(Triple(ImageFormat.GIF, 1, 65535), dims(ImageFixtures.gif(1, 65535, "87a")))
        assertNull(dims(ImageFixtures.gif(1, 1, "88a")))
    }

    @Test
    fun webpVariants() {
        assertEquals(Triple(ImageFormat.WEBP, 1080, 1350), dims(ImageFixtures.webpLossy(1080, 1350)))
        assertEquals(Triple(ImageFormat.WEBP, 16383, 1), dims(ImageFixtures.webpLossless(16383, 1)))
        assertEquals(Triple(ImageFormat.WEBP, 16384, 16384), dims(ImageFixtures.webpLossless(16384, 16384)))
        assertEquals(Triple(ImageFormat.WEBP, 4032, 3024), dims(ImageFixtures.webpExtended(4032, 3024)))
        assertEquals(Triple(ImageFormat.WEBP, 1 shl 24, 1), dims(ImageFixtures.webpExtended(1 shl 24, 1)))
    }

    @Test
    fun jpegBaselineAndProgressive() {
        assertEquals(Triple(ImageFormat.JPEG, 4032, 3024), dims(ImageFixtures.jpeg(4032, 3024)))
        assertEquals(Triple(ImageFormat.JPEG, 1080, 1080), dims(ImageFixtures.jpeg(1080, 1080, sof = 0xc2)))
        assertEquals(Triple(ImageFormat.JPEG, 7, 9), dims(ImageFixtures.jpeg(7, 9, sof = 0xcf)))
    }

    @Test
    fun jpegSkipsFillBytes() {
        assertEquals(Triple(ImageFormat.JPEG, 100, 50), dims(ImageFixtures.jpeg(100, 50, fill = 3)))
    }

    @Test
    fun jpegOrientationSwapsForRotatedFrames() {
        for (o in 1..8) {
            for (little in listOf(true, false)) {
                val want = if (o >= 5) 3024 to 4032 else 4032 to 3024
                val info = ImageSniffer.sniff(ImageFixtures.jpeg(4032, 3024, app1 = ImageFixtures.exif(o, little)))!!
                assertEquals(want, info.width to info.height, "orientation $o little=$little")
            }
        }
    }

    @Test
    fun invalidOrientationIsIgnored() {
        for (o in listOf(0, 9, 255)) {
            val info = ImageSniffer.sniff(ImageFixtures.jpeg(40, 30, app1 = ImageFixtures.exif(o)))!!
            assertEquals(40 to 30, info.width to info.height)
        }
    }

    @Test
    fun nonExifApp1IsIgnored() {
        val info = ImageSniffer.sniff(ImageFixtures.jpeg(40, 30, app1 = "http://ns.adobe.com/xap/1.0/\u0000<x/>".encodeToByteArray()))!!
        assertEquals(40 to 30, info.width to info.height)
    }

    @Test
    fun jpegWithoutFrameBeforeScanIsUnknown() {
        val bytes = byteArrayOf(0xff.toByte(), 0xd8.toByte(), 0xff.toByte(), 0xda.toByte(), 0, 4, 0, 0)
        assertNull(ImageSniffer.sniff(bytes))
    }

    @Test
    fun zeroDimensionsAreRejected() {
        assertNull(ImageSniffer.sniff(ImageFixtures.png(0, 10)))
        assertNull(ImageSniffer.sniff(ImageFixtures.gif(0, 10)))
        assertNull(ImageSniffer.sniff(ImageFixtures.jpeg(0, 10)))
        assertNull(ImageSniffer.sniff(ImageFixtures.png(-5, 10)))
    }

    @Test
    fun formatDetectionWithoutDimensions() {
        assertEquals(ImageFormat.JPEG, ImageSniffer.format(byteArrayOf(-1, -40, -1)))
        assertEquals(ImageFormat.UNKNOWN, ImageSniffer.format(ByteArray(0)))
        assertEquals(ImageFormat.UNKNOWN, ImageSniffer.format("BM".encodeToByteArray() + ByteArray(40)))
        assertEquals(ImageFormat.UNKNOWN, ImageSniffer.format("RIFF....WAVE".encodeToByteArray()))
    }

    @Test
    fun everyTruncationIsSafe() {
        val samples = listOf(
            ImageFixtures.png(10, 20),
            ImageFixtures.gif(10, 20),
            ImageFixtures.webpLossy(10, 20),
            ImageFixtures.webpLossless(10, 20),
            ImageFixtures.webpExtended(10, 20),
            ImageFixtures.jpeg(10, 20, app1 = ImageFixtures.exif(6)),
        )
        for (s in samples) {
            for (n in 0..s.size) {
                val r = ImageSniffer.sniff(s.copyOf(n))
                if (r != null) assertEquals(r.byteCount, n)
            }
        }
    }

    @Test
    fun corruptSegmentLengthsAreSafe() {
        val j = ImageFixtures.jpeg(10, 20, app1 = ImageFixtures.exif(6))
        for (i in 2 until j.size) {
            for (v in listOf(0, 1, 0xff)) {
                ImageSniffer.sniff(j.copyOf().also { it[i] = v.toByte() })
            }
        }
    }

    @Test
    fun randomBytesNeverThrow() {
        var s = 12345
        repeat(2000) {
            val n = (s ushr 20) and 255
            val bytes = ByteArray(n) {
                s = s xor (s shl 13)
                s = s xor (s ushr 17)
                s = s xor (s shl 5)
                s.toByte()
            }
            if (n >= 3 && it % 3 == 0) {
                bytes[0] = -1
                bytes[1] = -40
                bytes[2] = -1
            }
            ImageSniffer.sniff(bytes)
        }
    }

    @Test
    fun exifOrientationBounds() {
        val e = ImageFixtures.exif(6)
        assertEquals(6, ImageSniffer.exifOrientation(e, 0, e.size))
        assertEquals(1, ImageSniffer.exifOrientation(e, 0, 10))
        assertEquals(1, ImageSniffer.exifOrientation(e, 0, e.size - 6))
    }
}
