package app.witness.di

import android.util.Log
import app.witness.core.domain.port.Telemetry
import app.witness.core.domain.usecase.TelemetryGate

class LocalLogTelemetry(val inner: Telemetry, private val log: (String, Throwable?) -> Unit = { m, e -> Log.w(TAG, m, e) }) : Telemetry by inner {
    override fun breadcrumb(message: String) {
        log(message, null)
        inner.breadcrumb(message)
    }

    override fun nonFatal(error: Throwable) {
        log("non-fatal: ${error.message}", error)
        inner.nonFatal(error)
    }

    companion object {
        const val TAG: String = "Witness"

        fun wrapIf(local: Boolean, inner: Telemetry): Telemetry = if (local) LocalLogTelemetry(inner) else inner

        fun gateOf(t: Telemetry): TelemetryGate? = ((t as? LocalLogTelemetry)?.inner ?: t) as? TelemetryGate
    }
}
