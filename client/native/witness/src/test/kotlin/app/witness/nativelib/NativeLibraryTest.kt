package app.witness.nativelib

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NativeLibraryTest {
    @Test
    fun libraryNameMatchesCdylib() {
        assertEquals("witness_android", NativeLibrary.NAME)
    }

    @Test
    fun supportsOnlyArm64AndX86_64() {
        assertTrue(NativeLibrary.supports("arm64-v8a"))
        assertTrue(NativeLibrary.supports("x86_64"))
        assertFalse(NativeLibrary.supports("armeabi-v7a"))
        assertFalse(NativeLibrary.supports(""))
    }
}
