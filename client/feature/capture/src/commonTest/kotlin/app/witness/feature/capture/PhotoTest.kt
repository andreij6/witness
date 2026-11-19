package app.witness.feature.capture

import app.witness.core.ui.sampleSize
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class PhotoTest {
    @Test
    fun smallImageIsNotSampled() = assertEquals(1, sampleSize(100, 80, 192))

    @Test
    fun exactlyDoubleSamplesByTwo() = assertEquals(2, sampleSize(384, 200, 192))

    @Test
    fun twelveMegapixelThumbnail() = assertEquals(16, sampleSize(4032, 3024, 192))

    @Test
    fun portraitUsesLongestSide() = assertEquals(16, sampleSize(3024, 4032, 192))

    @Test
    fun resultKeepsAtLeastMaxDimension() {
        for (w in listOf(193, 500, 1000, 4032, 8000)) {
            val s = sampleSize(w, w / 2, 192)
            assertTrue(w / s >= 192, "w=$w s=$s")
            assertTrue(w / (s * 2) < 192, "w=$w s=$s")
        }
    }

    @Test
    fun alwaysPowerOfTwo() {
        for (w in 1..5000 step 37) {
            val s = sampleSize(w, 10, 64)
            assertEquals(0, s and (s - 1))
        }
    }

    @Test
    fun invalidInputsFallBackToOne() {
        assertEquals(1, sampleSize(0, 100, 192))
        assertEquals(1, sampleSize(100, -1, 192))
        assertEquals(1, sampleSize(100, 100, 0))
    }

    @Test
    fun hugeDimensionsDoNotOverflow() = assertEquals(1 shl 23, sampleSize(Int.MAX_VALUE, 1, 192))

    @Test
    fun photoUsesIdentityEquality() {
        val bytes = byteArrayOf(1, 2)
        assertNotEquals(Photo(bytes), Photo(bytes))
    }
}
