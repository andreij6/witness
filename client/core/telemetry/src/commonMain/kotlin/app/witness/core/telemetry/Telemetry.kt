package app.witness.core.telemetry

import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.usecase.TelemetryGate
import app.witness.core.domain.port.Telemetry as PortTelemetry

interface Telemetry : PortTelemetry {
    object None : Telemetry, PortTelemetry by PortTelemetry.None
}

object TelemetryRules {
    const val MAX_NAME = 40
    const val MAX_PARAMS = 25
    private val reservedPrefixes = arrayOf("firebase_", "google_", "ga_")

    fun eventName(event: AnalyticsEvent): String = event.name.lowercase()

    fun isValidName(name: String): Boolean {
        if (name.isEmpty() || name.length > MAX_NAME || name[0] !in 'a'..'z') return false
        for (c in name) if (c !in 'a'..'z' && c !in '0'..'9' && c != '_') return false
        return reservedPrefixes.none(name::startsWith)
    }

    inline fun forEachParam(metrics: Map<String, Long>, put: (String, Long) -> Unit) {
        var n = 0
        for ((k, v) in metrics) {
            if (n == MAX_PARAMS) return
            if (isValidName(k)) {
                put(k, v)
                n++
            }
        }
    }
}

fun selectTelemetry(configured: Boolean, backend: () -> PortTelemetry): PortTelemetry =
    if (configured) TelemetryGate(backend()) else Telemetry.None
