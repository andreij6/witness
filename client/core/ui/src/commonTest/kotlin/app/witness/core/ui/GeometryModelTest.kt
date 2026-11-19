package app.witness.core.ui

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class GeometryModelTest {
    private fun assertRect(expected: Rect, actual: Rect) {
        assertEquals(expected.left, actual.left, 1e-3f)
        assertEquals(expected.top, actual.top, 1e-3f)
        assertEquals(expected.right, actual.right, 1e-3f)
        assertEquals(expected.bottom, actual.bottom, 1e-3f)
    }

    @Test
    fun normRectValidatesBounds() {
        assertFailsWith<IllegalArgumentException> { NormRect(-0.1f, 0f, 0.5f, 0.5f) }
        assertFailsWith<IllegalArgumentException> { NormRect(0f, 0f, 1.1f, 0.5f) }
        assertFailsWith<IllegalArgumentException> { NormRect(0.6f, 0f, 0.5f, 0.5f) }
        assertFailsWith<IllegalArgumentException> { NormRect(0f, 0.6f, 0.5f, 0.5f) }
        assertFailsWith<IllegalArgumentException> { NormRect(Float.NaN, 0f, 0.5f, 0.5f) }
        assertEquals(0f, NormRect(0.5f, 0.5f, 0.5f, 0.5f).area)
        assertEquals(1f, NormRect.Full.area)
    }

    @Test
    fun centeredCropOfLandscapeToSquareTrimsSides() {
        val r = NormRect.centeredCrop(4f / 3f, 1f)
        assertEquals(0.125f, r.left, 1e-6f)
        assertEquals(0.875f, r.right, 1e-6f)
        assertEquals(0f, r.top)
        assertEquals(1f, r.bottom)
    }

    @Test
    fun centeredCropOfPortraitToSquareTrimsTopAndBottom() {
        val r = NormRect.centeredCrop(3f / 4f, 1f)
        assertEquals(0.125f, r.top, 1e-6f)
        assertEquals(0.875f, r.bottom, 1e-6f)
        assertEquals(0.75f, r.height, 1e-6f)
        assertEquals(NormRect.Full, NormRect.centeredCrop(1f, 1f))
        assertFailsWith<IllegalArgumentException> { NormRect.centeredCrop(0f, 1f) }
    }

    @Test
    fun mappingWithoutAspectScalesToFrame() {
        assertRect(Rect(10f, 20f, 50f, 60f), mapToCropped(NormRect(0.1f, 0.2f, 0.5f, 0.6f), null, Size(100f, 100f)))
    }

    @Test
    fun mappingMatchesDesignImagePoint() {
        val frame = Size(160f, 274f)
        val aspect = 896f / 1200f
        val k = maxOf(160f / 896f, 274f / 1200f)
        val expectedX = 0.567f * 896f * k - (896f * k - 160f) / 2f
        val expectedY = 0.575f * 1200f * k - (1200f * k - 274f) / 2f
        val r = mapToCropped(NormRect(0.567f, 0.575f, 0.567f, 0.575f), aspect, frame)
        assertEquals(expectedX, r.left, 1e-2f)
        assertEquals(expectedY, r.top, 1e-2f)
    }

    @Test
    fun mappingWideSourceIntoSquareFrameCropsSides() {
        val r = mapToCropped(NormRect.Full, 2f, Size(100f, 100f))
        assertRect(Rect(-50f, 0f, 150f, 100f), r)
    }

    @Test
    fun mappingTallSourceIntoWideFrameCropsTopAndBottom() {
        val r = mapToCropped(NormRect(0f, 0.5f, 1f, 0.5f), 0.5f, Size(100f, 100f))
        assertEquals(50f, r.top, 1e-3f)
        assertEquals(-50f, mapToCropped(NormRect.Full, 0.5f, Size(100f, 100f)).top, 1e-3f)
    }

    @Test
    fun mappingCenteredCropRoundTripsToFrame() {
        val frame = Size(120f, 120f)
        val crop = NormRect.centeredCrop(4f / 3f, 1f)
        assertRect(Rect(0f, 0f, 120f, 120f), mapToCropped(crop, 4f / 3f, frame))
    }

    @Test
    fun mappingDegenerateFrameIsSafe() {
        val r = mapToCropped(NormRect.Full, 1.5f, Size(0f, 0f))
        assertTrue(r.isEmpty)
    }

    @Test
    fun webLayoutIsCompactOnPhones() {
        val phone = WebLayout.spec(412.dp)
        assertEquals(WebLayoutMode.Compact, phone.mode)
        assertEquals(16.dp, phone.gutter)
        assertEquals(380.dp, phone.contentWidth)
        assertEquals(WebLayoutMode.Compact, WebLayout.spec(839.dp).mode)
        assertEquals(560.dp, WebLayout.spec(800.dp).contentWidth)
    }

    @Test
    fun webLayoutIsTwoColumnsOnDesktop() {
        val desktop = WebLayout.spec(1280.dp)
        assertEquals(WebLayoutMode.Wide, desktop.mode)
        assertEquals(64.dp, desktop.gutter)
        assertEquals(1152.dp, desktop.contentWidth)
        assertEquals(600f, desktop.primaryWidth.value, 0.01f)
        assertEquals(48.dp, desktop.columnGap)
        assertEquals(504f, (desktop.contentWidth - desktop.primaryWidth - desktop.columnGap).value, 0.01f)
    }

    @Test
    fun webLayoutCentersOnUltraWide() {
        val wide = WebLayout.spec(1920.dp)
        assertEquals(1152.dp, wide.contentWidth)
        assertEquals(384.dp, wide.gutter)
    }

    @Test
    fun webLayoutHandlesDegenerateWidths() {
        assertEquals(0.dp, WebLayout.spec(0.dp).contentWidth)
        assertEquals(0.dp, WebLayout.spec((-10).dp).contentWidth)
        assertEquals(0.dp, WebLayout.spec(20.dp).contentWidth)
        assertEquals(WebLayoutMode.Wide, WebLayout.spec(840.dp).mode)
        assertEquals(712.dp, WebLayout.spec(840.dp).contentWidth)
    }
}
