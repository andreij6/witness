package app.witness.core.data

import app.witness.core.model.ByteString
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ImageFilesTest {
    private val images = tempImages()

    @Test
    fun writeReadRenameDelete() = runTest {
        val jpeg = ByteString.of(byteArrayOf(-1, -40, 1))
        assertNull(images.read(rid(1)))
        images.write(rid(1), jpeg)
        assertEquals(jpeg, images.read(rid(1)))
        images.write(rid(1), ByteString.of(byteArrayOf(5)))
        assertEquals(1, images.read(rid(1))!!.size)
        images.rename(rid(1), rid(2))
        assertNull(images.read(rid(1)))
        assertEquals(1, images.read(rid(2))!!.size)
        images.rename(rid(7), rid(8))
        assertTrue(images.delete(rid(2)))
        assertNull(images.read(rid(2)))
    }
}
