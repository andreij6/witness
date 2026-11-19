package app.witness.core.verify

import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class SniffingImageInfoReaderTest {
    @Test
    fun sniffedFormatsSkipFallback() = runTest {
        val reader = SniffingImageInfoReader { error("fallback must not run") }
        assertEquals(ImageInfo(ImageFormat.PNG, 3, 4, 29), reader.read(ImageFixtures.png(3, 4)))
    }

    @Test
    fun unknownFormatsUseFallback() = runTest {
        val bmp = "BM".encodeToByteArray() + ByteArray(60)
        assertEquals(ImageInfo(ImageFormat.UNKNOWN, 2, 3, 62), SniffingImageInfoReader { 2 to 3 }.read(bmp))
    }

    @Test
    fun fallbackFailuresYieldNull() = runTest {
        val junk = ByteArray(10)
        assertNull(SniffingImageInfoReader { null }.read(junk))
        assertNull(SniffingImageInfoReader { 0 to 3 }.read(junk))
        assertNull(SniffingImageInfoReader { 3 to -1 }.read(junk))
    }

    @Test
    fun truncatedJpegFallsBackKeepingFormat() = runTest {
        val head = ImageFixtures.jpeg(10, 10).copyOf(8)
        assertEquals(ImageInfo(ImageFormat.JPEG, 5, 6, 8), SniffingImageInfoReader { 5 to 6 }.read(head))
    }
}
