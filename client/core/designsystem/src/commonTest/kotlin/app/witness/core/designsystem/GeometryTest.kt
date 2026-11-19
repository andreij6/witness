package app.witness.core.designsystem

import androidx.compose.ui.unit.dp
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class GeometryTest {
    @Test
    fun markProportionsFollowDesignLibrary() {
        val g = MarkGeometry.of(100f)
        assertEquals(11f, g.stroke, 1e-4f)
        assertEquals(36f, g.radius, 1e-4f)
        assertEquals(50f, g.center.x, 1e-4f)
        assertEquals(36f * 0.62f, g.shutterSize.width, 1e-4f)
        assertEquals(11f * 1.05f, g.shutterSize.height, 1e-4f)
        assertEquals(3, g.check.size)
    }

    @Test
    fun markShutterSitsAboveRingOnTheRight() {
        val g = MarkGeometry.of(100f)
        assertTrue(g.shutterTopLeft.x > g.center.x)
        assertTrue(g.shutterTopLeft.y + g.shutterSize.height < g.center.y - g.radius)
    }

    @Test
    fun markVisualBoundsAreCenteredVertically() {
        val g = MarkGeometry.of(100f)
        val top = g.shutterTopLeft.y
        val bottom = g.center.y + g.radius + g.stroke / 2
        assertTrue(top >= 0f && bottom <= 100f)
        assertTrue(abs((top + bottom) / 2 - 50f) < 1f)
    }

    @Test
    fun markCheckRisesToTheRight() {
        val (a, b, c) = MarkGeometry.of(64f).check
        assertTrue(a.x < b.x && b.x < c.x)
        assertTrue(b.y > a.y && c.y < a.y)
    }

    @Test
    fun markScalesLinearly() {
        val small = MarkGeometry.of(10f)
        val large = MarkGeometry.of(40f)
        assertEquals(small.radius * 4, large.radius, 1e-4f)
        assertEquals(small.check[2].x * 4, large.check[2].x, 1e-4f)
    }

    @Test
    fun toggleThumbMatchesDesignCenters() {
        assertEquals(13.dp, toggleThumbOffset(false) + ToggleThumb / 2)
        assertEquals(33.dp, toggleThumbOffset(true) + ToggleThumb / 2)
        assertTrue(toggleThumbOffset(true) + ToggleThumb <= ToggleWidth)
    }

    @Test
    fun bottomNavCentersFollowDesign() {
        assertEquals(74, navCenterX(0, 412))
        assertEquals(206, navCenterX(1, 412))
        assertEquals(338, navCenterX(2, 412))
        assertEquals(0, navCenterX(1, 0))
    }

    @Test
    fun overlineIsUppercase() {
        assertEquals("STILL RECOMMENDED", overlineText("Still recommended"))
        assertEquals("", overlineText(""))
    }

    @Test
    fun spacingFollowsEightPointGrid() {
        listOf(WitnessSpacing.S, WitnessSpacing.Margin, WitnessSpacing.L, WitnessSpacing.XL, WitnessSpacing.XXL, WitnessSpacing.NavHeight)
            .forEach { assertEquals(0f, it.value % 8f, "$it") }
        assertEquals(16.dp, WitnessSpacing.Margin)
    }
}
