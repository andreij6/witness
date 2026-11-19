package app.witness.core.verify

import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals

class AndroidPlatformTest {
    @Test
    fun engineIsInMemoryUntilNativeLands() = runTest {
        assertEquals(Judgement.NO_RECORD, createVerifyEngine().judge(byteArrayOf(1, 2, 3)))
    }

    @Test
    fun readerSniffsWithoutTouchingBitmapFactory() = runTest {
        assertEquals(ImageInfo(ImageFormat.JPEG, 4032, 3024, ImageFixtures.jpeg(4032, 3024).size), platformImageInfoReader().read(ImageFixtures.jpeg(4032, 3024)))
    }
}
