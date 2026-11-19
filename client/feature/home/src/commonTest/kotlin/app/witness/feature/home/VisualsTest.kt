package app.witness.feature.home

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class VisualsTest {
    @Test
    fun mosaicShadesStayInPalette() {
        for (r in 0..3) for (i in 0 until MOSAIC_CELLS) for (j in 0 until MOSAIC_CELLS) assertTrue(mosaicShade(r, i, j) in 0..4)
    }

    @Test
    fun mosaicIsNotUniform() {
        val shades = (0 until MOSAIC_CELLS).flatMap { i -> (0 until MOSAIC_CELLS).map { j -> mosaicShade(0, i, j) } }.toSet()
        assertTrue(shades.size >= 3)
    }

    @Test
    fun redactionsSitInsideTheHero() {
        for (r in heroRedactions) {
            assertTrue(r.cx - r.w / 2 >= 0f && r.cx + r.w / 2 <= 1f)
            assertTrue(r.cy - r.h / 2 >= 0f && r.cy + r.h / 2 <= 1f)
        }
    }

    @Test
    fun connectorTurnsGreenOnlyBetweenProgressedSteps() {
        val done = SetupStep("a", "b", StepStatus.Done)
        assertTrue(connectorDone(done, done.copy(status = StepStatus.Busy)))
        assertTrue(connectorDone(done, done))
        assertFalse(connectorDone(done, done.copy(status = StepStatus.Todo)))
        assertFalse(connectorDone(done.copy(status = StepStatus.Busy), done))
        assertFalse(connectorDone(done, null))
    }

    @Test
    fun emptyIllustrationDotsFitTheCanvas() {
        for ((dx, dy, r) in emptyDots) {
            assertTrue(kotlin.math.abs(dx) + r <= 120f)
            assertTrue(kotlin.math.abs(dy) + r <= 120f)
        }
    }

    @Test
    fun homeTabsAreHomeAndCheck() = assertEquals(listOf("Home", "Check"), listOf(homeTabs.first.label, homeTabs.second.label))
}
