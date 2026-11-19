package app.witness.core.platform

import android.Manifest

object CapturePermissions {
    val required: List<String> = listOf(Manifest.permission.CAMERA)
    val optional: List<String> = listOf(Manifest.permission.ACCESS_FINE_LOCATION)

    fun missing(granted: Set<String>): List<String> = required.filterNot(granted::contains)
}
