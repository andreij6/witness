package app.witness.core.telemetry

import android.content.Context
import android.os.Bundle
import app.witness.core.domain.port.AnalyticsEvent
import com.google.firebase.FirebaseApp
import com.google.firebase.analytics.FirebaseAnalytics
import com.google.firebase.crashlytics.FirebaseCrashlytics
import app.witness.core.domain.port.Telemetry as PortTelemetry

class FirebaseTelemetry(private val analytics: FirebaseAnalytics, private val crashlytics: FirebaseCrashlytics) : Telemetry {
    override fun setCollectionEnabled(enabled: Boolean) {
        analytics.setAnalyticsCollectionEnabled(enabled)
        crashlytics.isCrashlyticsCollectionEnabled = enabled
    }

    override fun event(event: AnalyticsEvent, metrics: Map<String, Long>) =
        analytics.logEvent(TelemetryRules.eventName(event), metrics.toBundle())

    override fun breadcrumb(message: String) = crashlytics.log(message)

    override fun nonFatal(error: Throwable) = crashlytics.recordException(error)
}

internal fun Map<String, Long>.toBundle(): Bundle {
    val b = Bundle(minOf(size, TelemetryRules.MAX_PARAMS))
    TelemetryRules.forEachParam(this, b::putLong)
    return b
}

object AndroidTelemetry {
    fun isConfigured(context: Context): Boolean = FirebaseApp.getApps(context).isNotEmpty()

    fun create(context: Context): PortTelemetry = selectTelemetry(isConfigured(context)) {
        FirebaseTelemetry(FirebaseAnalytics.getInstance(context), FirebaseCrashlytics.getInstance())
    }
}
