package app.witness.feature.post

import android.graphics.Bitmap
import android.graphics.Color
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.graphics.asImageBitmap
import app.witness.core.model.BlurClass
import app.witness.core.model.Edit
import app.witness.core.model.Rect
import app.witness.core.ui.decodeBitmap
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import java.io.ByteArrayOutputStream
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNotSame

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36])
class BlurPhotoTest {
    private fun checker(w: Int, h: Int): Bitmap = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888).apply {
        for (y in 0 until h) for (x in 0 until w) setPixel(x, y, if ((x + y) % 2 == 0) Color.BLACK else Color.WHITE)
    }

    @Test
    fun blursOnlyTheScaledRectOnACopy() {
        val source = checker(40, 40).asImageBitmap()
        val out = blurPhoto(source, listOf(Edit.Blur(Rect(0, 0, 40, 40), BlurClass.FACE, 800)), 0.5f).asAndroidBitmap()
        assertNotSame(source.asAndroidBitmap(), out)
        assertNotEquals(Color.BLACK, out.getPixel(2, 2))
        assertEquals(Color.BLACK, out.getPixel(30, 30))
        assertEquals(Color.BLACK, source.asAndroidBitmap().getPixel(2, 2))
    }

    @Test
    fun noBlursKeepsPixels() {
        val source = checker(8, 8).asImageBitmap()
        val out = blurPhoto(source, emptyList(), 1f).asAndroidBitmap()
        assertEquals(Color.WHITE, out.getPixel(1, 0))
    }

    @Test
    fun decodeDownsamplesLargePhotos() {
        val out = ByteArrayOutputStream()
        checker(400, 200).compress(Bitmap.CompressFormat.PNG, 100, out)
        val decoded = assertNotNull(decodeBitmap(out.toByteArray(), 100))
        assertEquals(100 to 50, decoded.width to decoded.height)
    }
}
