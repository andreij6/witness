package app.witness.core.platform

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class CapturePermissionsTest {
    @Test
    fun cameraIsMissingWhenNothingGranted() {
        assertEquals(listOf("android.permission.CAMERA"), CapturePermissions.missing(emptySet()))
    }

    @Test
    fun nothingMissingWhenRequiredGranted() {
        assertTrue(CapturePermissions.missing(CapturePermissions.required.toSet()).isEmpty())
    }

    @Test
    fun locationIsNeverRequired() {
        assertTrue(CapturePermissions.optional.none(CapturePermissions.required::contains))
    }
}
