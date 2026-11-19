package app.witness.feature.check

import app.witness.core.model.ByteString
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class PickedImageTest {
    private fun sniff(vararg parts: Any): String? {
        val bytes = parts.flatMap { p ->
            when (p) {
                is String -> p.encodeToByteArray().toList()
                is Int -> listOf(p.toByte())
                else -> error("bad part")
            }
        }.toByteArray()
        return PickedImage.sniffFormat(ByteString.of(bytes))
    }

    @Test
    fun recognisesCommonFormats() {
        assertEquals("JPEG", sniff(0xFF, 0xD8, 0xFF, 0xE0))
        assertEquals("PNG", sniff(0x89, "PNG", 0x0D, 0x0A, 0x1A, 0x0A, 0))
        assertEquals("WEBP", sniff("RIFF", 0, 0, 0, 0, "WEBPVP8 "))
        assertEquals("AVIF", sniff(0, 0, 0, 0x20, "ftypavif"))
        assertEquals("HEIC", sniff(0, 0, 0, 0x18, "ftypheic"))
        assertEquals("HEIC", sniff(0, 0, 0, 0x18, "ftypmif1"))
    }

    @Test
    fun unknownTruncatedAndEmptyInputsGiveNull() {
        assertNull(sniff())
        assertNull(sniff(0xFF, 0xD8))
        assertNull(sniff(0x89, "PNG"))
        assertNull(sniff("RIFF", 0, 0, 0, 0, "WAVE"))
        assertNull(sniff("GIF89a"))
        assertNull(sniff(0, 0, 0, 0x18, "ftypmp42"))
    }

    @Test
    fun sizeAndFormatReflectBytes() {
        val img = Fixtures.image(1, 2, 3)
        assertEquals(6, img.size)
        assertEquals("JPEG", img.format)
    }
}
