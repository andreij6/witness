package app.witness.core.a2ui.ui

data class ZoneRect(val left: Float, val top: Float, val right: Float, val bottom: Float)

object ImpactZones {
    private val rows = mapOf("front" to (0f to 0.25f), "side" to (0.25f to 0.75f), "rear" to (0.75f to 1f))
    private val columns = mapOf("driver" to (0f to 0.5f), "center" to (0.25f to 0.75f), "passenger" to (0.5f to 1f))

    fun rect(zone: String?): ZoneRect? {
        if (zone == null) return null
        val split = zone.indexOf('_')
        if (split <= 0) return null
        val (top, bottom) = rows[zone.substring(0, split)] ?: return null
        val (left, right) = columns[zone.substring(split + 1)] ?: return null
        return ZoneRect(left, top, right, bottom)
    }
}
