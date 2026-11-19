package app.witness.core.platform.blur

import app.witness.core.model.BlurClass
import app.witness.core.model.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class TextBlurClassifierTest {
    @Test
    fun platesDetected() {
        listOf("7MNV562", "AB12 CDE", "B-MW 1234", "KA01AB1234".take(8)).forEach { assertEquals(BlurClass.PLATE, TextBlurClassifier.classify(it), it) }
    }

    @Test
    fun houseNumbersDetected() {
        listOf("12", "1600", "221B", " 42 ").forEach { assertEquals(BlurClass.HOUSE_NO, TextBlurClassifier.classify(it), it) }
    }

    @Test
    fun ordinaryTextIgnored() {
        listOf("", "STOP", "Main Street", "0123", "123456", "abc123", "ABCDEFGHIJ1", "12AB", "A1").forEach {
            assertNull(TextBlurClassifier.classify(it), it)
        }
    }

    @Test
    fun plateNeedsLettersAndDigits() {
        assertFalse(TextBlurClassifier.isPlate("ABCDEF"))
        assertFalse(TextBlurClassifier.isPlate("123456"))
        assertTrue(TextBlurClassifier.isPlate("ABC123"))
    }
}

class BlurGeometryTest {
    @Test
    fun sampleSizeTargetsEdge() {
        assertEquals(1, BlurGeometry.sampleSize(1280, 960))
        assertEquals(2, BlurGeometry.sampleSize(4032, 3024))
        assertEquals(4, BlurGeometry.sampleSize(8000, 6000))
        assertEquals(1, BlurGeometry.sampleSize(0, 0))
    }

    @Test
    fun scalesAndPadsAndClips() {
        assertEquals(Rect(18, 18, 24, 24), BlurGeometry.toOriginal(10, 10, 20, 20, 2, 1000, 1000))
        assertEquals(Rect(0, 0, 11, 11), BlurGeometry.toOriginal(0, 0, 10, 10, 1, 1000, 1000))
        assertEquals(Rect(989, 989, 11, 11), BlurGeometry.toOriginal(990, 990, 1000, 1000, 1, 1000, 1000))
    }

    @Test
    fun degenerateBoxesDropped() {
        assertNull(BlurGeometry.toOriginal(5, 5, 5, 10, 1, 100, 100))
        assertNull(BlurGeometry.toOriginal(200, 200, 210, 210, 1, 100, 100))
    }

    @Test
    fun dedupeRemovesContainedSameClass() {
        val big = BlurGeometry.proposal(Rect(0, 0, 100, 100), BlurClass.FACE)
        val inner = BlurGeometry.proposal(Rect(10, 10, 10, 10), BlurClass.FACE)
        val plate = BlurGeometry.proposal(Rect(10, 10, 10, 10), BlurClass.PLATE)
        assertEquals(listOf(big, plate), BlurGeometry.dedupe(listOf(inner, big, plate)))
    }

    @Test
    fun dedupeCapsCount() {
        val many = List(200) { BlurGeometry.proposal(Rect(it * 10, 0, 5, 5), BlurClass.FACE) }
        assertEquals(BlurGeometry.MAX_PROPOSALS, BlurGeometry.dedupe(many).size)
    }

    @Test
    fun confidenceClamped() {
        assertEquals(1f, BlurGeometry.proposal(Rect(0, 0, 1, 1), BlurClass.OTHER, 3f).confidence)
        assertEquals(0f, BlurGeometry.proposal(Rect(0, 0, 1, 1), BlurClass.OTHER, -1f).confidence)
    }
}
