package app.witness.feature.check

import androidx.compose.ui.graphics.ImageBitmap
import app.witness.core.model.ByteString
import app.witness.core.model.Dims
import app.witness.core.ui.Formatters
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class DimsTextTest {
    private val bytes = ByteString.of(byteArrayOf(1, 2, 3))

    @Test
    fun pickedSizeWinsOverASubsampledBitmap() =
        assertEquals(Formatters.dimensions(4032, 3024), dimsText(PickedImage(bytes, dims = Dims(4032, 3024)), ImageBitmap(2016, 1512)))

    @Test
    fun bitmapSizeIsUsedWhenThePickerGaveNone() =
        assertEquals(Formatters.dimensions(640, 480), dimsText(PickedImage(bytes), ImageBitmap(640, 480)))

    @Test
    fun pickedSizeIsShownBeforeTheBitmapDecodes() =
        assertEquals(Formatters.dimensions(10, 20), dimsText(PickedImage(bytes, dims = Dims(10, 20)), null))

    @Test
    fun nothingKnownGivesNoText() = assertNull(dimsText(PickedImage(bytes), null))
}
