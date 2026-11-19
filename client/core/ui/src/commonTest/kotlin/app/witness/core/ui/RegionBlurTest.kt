package app.witness.core.ui

import app.witness.core.model.Rect
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class RegionBlurTest {
    private fun gray(v: Int): Int = (0xFF shl 24) or (v shl 16) or (v shl 8) or v

    private fun channel(p: Int): Int = p and 0xFF

    private fun checker(w: Int, h: Int, cell: Int = 1): IntArray = IntArray(w * h) { i -> if (((i % w) / cell + (i / w) / cell) % 2 == 0) gray(255) else gray(0) }

    private fun energy(px: IntArray, w: Int, h: Int): Long {
        var e = 0L
        for (y in 0 until h) for (x in 1 until w) e += abs(channel(px[y * w + x]) - channel(px[y * w + x - 1]))
        return e
    }

    @Test
    fun zeroOrNegativeSigmaGivesZeroRadii() {
        assertContentEquals(intArrayOf(0, 0, 0), RegionBlur.boxRadii(0))
        assertContentEquals(intArrayOf(0, 0, 0), RegionBlur.boxRadii(-50))
    }

    @Test
    fun radiiGrowMonotonicallyWithSigma() {
        var previous = 0
        for (s in listOf(100, 400, 800, 1600, 3200, 6400, 65_535)) {
            val sum = RegionBlur.boxRadii(s).sum()
            assertTrue(sum >= previous, "sigma $s")
            previous = sum
        }
    }

    @Test
    fun boxVarianceApproximatesTheGaussian() {
        for (sigmaE2 in listOf(400, 1000, 2500, 6400)) {
            val variance = RegionBlur.boxRadii(sigmaE2).sumOf { r -> ((2 * r + 1) * (2 * r + 1) - 1) / 12.0 }
            val target = (sigmaE2 / 100.0) * (sigmaE2 / 100.0)
            assertTrue(abs(variance - target) / target < 0.15, "sigma $sigmaE2 variance $variance target $target")
        }
    }

    @Test
    fun radiiAreDeterministic() {
        assertContentEquals(RegionBlur.boxRadii(1234), RegionBlur.boxRadii(1234))
        assertEquals(RegionBlur.PASSES, RegionBlur.boxRadii(1234).size)
        assertEquals(5, RegionBlur.boxRadii(1234, passes = 5).size)
    }

    @Test
    fun rejectsNonPositivePasses() {
        assertFailsWith<IllegalArgumentException> { RegionBlur.boxRadii(400, passes = 0) }
    }

    @Test
    fun flatImageStaysFlat() {
        val px = IntArray(20 * 10) { gray(77) }
        RegionBlur.blur(px, 20, 10, 800)
        assertTrue(px.all { it == gray(77) })
    }

    @Test
    fun blurPreservesOpaqueAlpha() {
        val px = checker(16, 16)
        RegionBlur.blur(px, 16, 16, 600)
        assertTrue(px.all { it ushr 24 == 0xFF })
    }

    @Test
    fun blurRemovesHighFrequencyEnergy() {
        val px = checker(32, 32, cell = 2)
        val before = energy(px, 32, 32)
        RegionBlur.blur(px, 32, 32, 400)
        assertTrue(energy(px, 32, 32) * 10 < before)
    }

    @Test
    fun strongerSigmaRemovesMoreEnergy() {
        val weak = checker(48, 48, cell = 4)
        val strong = weak.copyOf()
        RegionBlur.blur(weak, 48, 48, 150)
        RegionBlur.blur(strong, 48, 48, 900)
        assertTrue(energy(strong, 48, 48) < energy(weak, 48, 48))
    }

    @Test
    fun meanBrightnessIsPreservedApproximately() {
        val px = checker(40, 40, cell = 5)
        val before = px.sumOf { channel(it).toLong() }
        RegionBlur.blur(px, 40, 40, 700)
        val after = px.sumOf { channel(it).toLong() }
        assertTrue(abs(after - before) <= px.size * 2L)
    }

    @Test
    fun channelsBlurIndependently() {
        val px = IntArray(10 * 10) { i -> if (i % 10 < 5) 0xFFFF0000.toInt() else 0xFF0000FF.toInt() }
        RegionBlur.blur(px, 10, 10, 300)
        val mid = px[5 * 10 + 5]
        assertEquals(0, (mid shr 8) and 0xFF)
        assertTrue((mid shr 16) and 0xFF in 1..254)
        assertTrue(mid and 0xFF in 1..254)
    }

    @Test
    fun deterministicAcrossRuns() {
        val a = checker(33, 21, cell = 3)
        val b = a.copyOf()
        RegionBlur.blur(a, 33, 21, 1234)
        RegionBlur.blur(b, 33, 21, 1234, IntArray(33 * 21))
        assertContentEquals(a, b)
    }

    @Test
    fun radiusLargerThanTheRegionIsSafe() {
        val px = checker(3, 2)
        RegionBlur.blur(px, 3, 2, 6_400)
        assertTrue(px.all { channel(it) in 0..255 })
    }

    @Test
    fun singlePixelAndEmptyRegionsAreNoOps() {
        val one = intArrayOf(gray(9))
        RegionBlur.blur(one, 1, 1, 800)
        assertEquals(gray(9), one[0])
        RegionBlur.blur(IntArray(0), 0, 0, 800)
        RegionBlur.blur(IntArray(0), 0, 5, 800)
    }

    @Test
    fun zeroSigmaLeavesPixelsUntouched() {
        val px = checker(8, 8)
        val copy = px.copyOf()
        RegionBlur.blur(px, 8, 8, 0)
        assertContentEquals(copy, px)
    }

    @Test
    fun rejectsBadBuffers() {
        assertFailsWith<IllegalArgumentException> { RegionBlur.blur(IntArray(3), 2, 2, 400) }
        assertFailsWith<IllegalArgumentException> { RegionBlur.blur(IntArray(4), 2, 2, 400, IntArray(1)) }
        assertFailsWith<IllegalArgumentException> { RegionBlur.blur(IntArray(4), -2, 2, 400) }
    }

    @Test
    fun largeRegionIsLinearInPixels() {
        val px = checker(512, 512, cell = 3)
        RegionBlur.blur(px, 512, 512, 6_400)
        assertTrue(energy(px, 512, 512) < energy(checker(512, 512, cell = 3), 512, 512) / 50)
    }

    @Test
    fun scaledSigmaFollowsTheScale() {
        assertEquals(400, RegionBlur.scaledSigma(800, 0.5f))
        assertEquals(800, RegionBlur.scaledSigma(800, 1f))
        assertEquals(0, RegionBlur.scaledSigma(800, 0f))
        assertEquals(0, RegionBlur.scaledSigma(800, -1f))
    }

    @Test
    fun rectScalingCoversTheSourceArea() {
        val r = Rect(10, 21, 33, 17)
        assertEquals(r, r.scaled(1f))
        assertEquals(Rect(5, 10, 17, 9), r.scaled(0.5f))
        assertEquals(Rect(0, 0, 1, 1), Rect(0, 0, 1, 1).scaled(0.1f))
        assertEquals(Rect(20, 42, 66, 34), r.scaled(2f))
    }
}
