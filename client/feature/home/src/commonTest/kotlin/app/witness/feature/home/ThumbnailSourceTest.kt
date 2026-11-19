package app.witness.feature.home

import kotlin.test.Test
import kotlin.test.assertEquals

class ThumbnailSourceTest {
    @Test
    fun bucketRoundsUpToPowerOfTwo() {
        assertEquals(64, ThumbnailSource.bucket(1))
        assertEquals(64, ThumbnailSource.bucket(64))
        assertEquals(128, ThumbnailSource.bucket(65))
        assertEquals(256, ThumbnailSource.bucket(168))
    }

    @Test
    fun bucketIsCapped() = assertEquals(ThumbnailSource.MAX_BUCKET, ThumbnailSource.bucket(Int.MAX_VALUE))

    @Test
    fun bucketHandlesNonPositive() {
        assertEquals(64, ThumbnailSource.bucket(0))
        assertEquals(64, ThumbnailSource.bucket(-5))
    }

    @Test
    fun sampleSizeKeepsShortSideAtLeastTarget() {
        assertEquals(8, ThumbnailSource.sampleSize(4032, 3024, 256))
        assertEquals(1, ThumbnailSource.sampleSize(4032, 3024, 3024))
        assertEquals(2, ThumbnailSource.sampleSize(4032, 3024, 1512))
        assertEquals(1, ThumbnailSource.sampleSize(4032, 3024, 1513))
    }

    @Test
    fun sampleSizeUsesShortSideForPortrait() = assertEquals(8, ThumbnailSource.sampleSize(3024, 4032, 256))

    @Test
    fun sampleSizeRejectsBadInput() {
        assertEquals(1, ThumbnailSource.sampleSize(0, 100, 10))
        assertEquals(1, ThumbnailSource.sampleSize(100, -1, 10))
        assertEquals(1, ThumbnailSource.sampleSize(100, 100, 0))
    }

    @Test
    fun sampleSizeIsPowerOfTwo() {
        for (target in 1..2000 step 37) {
            val s = ThumbnailSource.sampleSize(4000, 3000, target)
            assertEquals(0, s and (s - 1))
        }
    }

    @Test
    fun noneSourceLoadsNothing() = kotlinx.coroutines.test.runTest { assertEquals(null, ThumbnailSource.None.load(rid(1), 100)) }
}
