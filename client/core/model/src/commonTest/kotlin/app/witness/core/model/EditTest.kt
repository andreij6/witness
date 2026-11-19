package app.witness.core.model

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EditTest {
    private val r = Rect(1, 2, 3, 4)
    private val all: List<Edit> = listOf(
        Edit.Resize(100, 50),
        Edit.Recompress(ImageFormat.JPEG, 85),
        Edit.Orient(6),
        Edit.Crop(r),
        Edit.Rotate(-250),
        Edit.Tone(10, -20, 30),
        Edit.Filter(3, 50),
        Edit.Blur(r, BlurClass.FACE, 800),
    )

    @Test
    fun kindsAreDistinctAndOrderedLikeRustCodes() {
        assertEquals(EditKind.entries, all.map { it.kind })
        assertEquals(listOf(0, 1, 2, 3, 4, 5, 6, 7), all.map { it.kind.ordinal })
    }

    @Test
    fun classesFollowSpec() {
        assertEquals(
            listOf(
                EditClass.PRESENTATION, EditClass.PRESENTATION, EditClass.PRESENTATION,
                EditClass.GLOBAL, EditClass.GLOBAL, EditClass.GLOBAL, EditClass.GLOBAL,
                EditClass.LOCAL_DECLARED,
            ),
            all.map { it.editClass },
        )
    }

    @Test
    fun allSamplesValid() {
        all.forEach { assertTrue(it.isValid, it.toString()) }
    }

    @Test
    fun invalidEditsRejected() {
        val bad = listOf(
            Edit.Resize(0, 10),
            Edit.Resize(10, -1),
            Edit.Resize(10, 10, 256),
            Edit.Recompress(ImageFormat.PNG, 101),
            Edit.Recompress(ImageFormat.PNG, -1),
            Edit.Orient(8),
            Edit.Orient(-1),
            Edit.Crop(Rect(0, 0, 0, 1)),
            Edit.Crop(Rect(-1, 0, 1, 1)),
            Edit.Rotate(1001),
            Edit.Rotate(-1001),
            Edit.Tone(40000, 0, 0),
            Edit.Filter(70000, 0),
            Edit.Filter(0, -1),
            Edit.Blur(Rect(0, 0, 1, 0), BlurClass.PLATE, 1),
            Edit.Blur(r, BlurClass.PLATE, 70000),
        )
        bad.forEach { assertFalse(it.isValid, it.toString()) }
    }

    @Test
    fun rotateBoundsInclusive() {
        assertTrue(Edit.Rotate(Edit.MAX_ROTATE_DEG_E2).isValid)
        assertTrue(Edit.Rotate(-Edit.MAX_ROTATE_DEG_E2).isValid)
        assertTrue(Edit.Orient(Edit.MAX_ORIENT).isValid)
    }

    @Test
    fun presentationOnlyAndLocal() {
        assertTrue(emptyList<Edit>().isPresentationOnly())
        assertTrue(all.take(3).isPresentationOnly())
        assertFalse(all.take(4).isPresentationOnly())
        assertFalse(all.take(7).hasLocal())
        assertTrue(all.hasLocal())
        assertFalse(emptyList<Edit>().hasLocal())
    }

    @Test
    fun rectGeometry() {
        assertEquals(12, r.area)
        assertEquals(4, r.right)
        assertEquals(6, r.bottom)
        assertTrue(Rect(0, 0, 1, 1).isValid)
        assertTrue(Rect(Int.MAX_VALUE, 0, Int.MAX_VALUE, 1).isValid)
        assertEquals(Int.MAX_VALUE.toLong() * Int.MAX_VALUE, Rect(0, 0, Int.MAX_VALUE, Int.MAX_VALUE).area)
    }

    @Test
    fun rectClipping() {
        assertEquals(Rect(0, 0, 5, 5), Rect(-5, -5, 10, 10).clippedTo(5, 5))
        assertEquals(Rect(8, 8, 2, 2), Rect(8, 8, 10, 10).clippedTo(10, 10))
        assertNull(Rect(10, 10, 3, 3).clippedTo(10, 10))
        assertNull(Rect(-10, 0, 5, 5).clippedTo(10, 10))
        assertEquals(Rect(0, 0, 10, 10), Rect(0, 0, Int.MAX_VALUE, Int.MAX_VALUE).clippedTo(10, 10))
    }
}
