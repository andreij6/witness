package app.witness.core.ui

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class BitmapsTest {
    @Test
    fun smallImageIsNotSampled() = assertEquals(1, sampleSize(1000, 800, DISPLAY_PX))

    @Test
    fun twelveMegapixelPhotoIsSampledBelowTwiceTheDisplaySize() = assertEquals(2, sampleSize(4032, 3024, DISPLAY_PX))

    @Test
    fun fortyEightMegapixelPortraitUsesLongestSide() = assertEquals(4, sampleSize(6000, 8000, DISPLAY_PX))

    @Test
    fun sampledLongestSideStaysWithinOneToTwiceTheTarget() {
        for (w in listOf(1281, 2560, 2561, 4032, 8000, 12000)) {
            val s = sampleSize(w, w / 3, DISPLAY_PX)
            assertTrue(w / s >= DISPLAY_PX && w / (s * 2) < DISPLAY_PX, "w=$w s=$s")
        }
    }

    @Test
    fun malformedSizesAreNotSampled() {
        assertEquals(1, sampleSize(0, 100, DISPLAY_PX))
        assertEquals(1, sampleSize(100, -1, DISPLAY_PX))
        assertEquals(1, sampleSize(5000, 5000, 0))
    }
}
