package app.witness.core.domain.usecase

import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class PostTextTest {
    private fun b(c: BlurClass, enabled: Boolean = true) = BlurProposal(Rect(0, 0, 10, 10), c, enabled)

    @Test
    fun geohashMatchesReferenceVectors() {
        assertEquals("u4pruydqqvj8", Geohash.encode(576_491_100, 104_074_400, 12))
        assertEquals("ezs42", Geohash.encode(426_050_000, -56_030_000, 5))
        assertEquals("s00000000000", Geohash.encode(0, 0, 12))
        assertEquals("000000000000", Geohash.encode(-900_000_000, -1_800_000_000, 12))
        assertEquals("zzzzzzzzzzzz", Geohash.encode(900_000_000, 1_800_000_000, 12))
        assertEquals("9xj64fk3sey1", Geohash.encode(397_392_000, -1_049_903_000, 12))
        assertEquals("r3gx2f77bn44", Geohash.encode(-338_688_000, 1_512_093_000, 12))
    }

    @Test
    fun geohashPrefixesAreConsistent() {
        val full = Geohash.encode(397_392_000, -1_049_903_000, 12)
        for (n in 1..12) assertEquals(full.take(n), Geohash.encode(397_392_000, -1_049_903_000, n))
    }

    @Test
    fun geohashRejectsOutOfRange() {
        assertFailsWith<IllegalArgumentException> { Geohash.encode(0, 0, 0) }
        assertFailsWith<IllegalArgumentException> { Geohash.encode(0, 0, 13) }
        assertFailsWith<IllegalArgumentException> { Geohash.encode(900_000_001, 0, 5) }
        assertFailsWith<IllegalArgumentException> { Geohash.encode(0, -1_800_000_001, 5) }
    }

    @Test
    fun cleanTrimsCollapsesAndCaps() {
        assertEquals("a b c", PostText.clean("  a \n\t b   c ", 100))
        assertEquals("abc", PostText.clean("abcdef", 3))
        assertEquals("ab", PostText.clean("ab cd", 3))
        assertNull(PostText.clean("   ", 10))
        assertNull(PostText.clean(null, 10))
        assertNull(PostText.clean("abc", 0))
    }

    @Test
    fun rulesAltTextSummarizesBlurs() {
        assertEquals("Photo.", PostText.rulesAltText(emptyList(), null))
        assertEquals("Photo taken in Denver area.", PostText.rulesAltText(emptyList(), " Denver area "))
        assertEquals("Photo. 1 face is blurred.", PostText.rulesAltText(listOf(b(BlurClass.FACE)), ""))
        assertEquals("Photo. 2 faces are blurred.", PostText.rulesAltText(listOf(b(BlurClass.FACE), b(BlurClass.FACE)), null))
        assertEquals(
            "Photo. 1 face and 1 license plate are blurred.",
            PostText.rulesAltText(listOf(b(BlurClass.PLATE), b(BlurClass.FACE)), null),
        )
        assertEquals(
            "Photo. 1 face, 2 license plates and 1 house number are blurred.",
            PostText.rulesAltText(listOf(b(BlurClass.HOUSE_NO), b(BlurClass.PLATE), b(BlurClass.FACE), b(BlurClass.PLATE), b(BlurClass.OTHER, false)), null),
        )
        assertEquals("Photo.", PostText.rulesAltText(listOf(b(BlurClass.FACE, false)), null))
    }
}
