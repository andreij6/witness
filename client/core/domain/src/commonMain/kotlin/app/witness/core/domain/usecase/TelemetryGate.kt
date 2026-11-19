package app.witness.core.domain.usecase

import app.witness.core.domain.port.AnalyticsEvent
import app.witness.core.domain.port.SettingsStore
import app.witness.core.domain.port.Telemetry
import kotlin.concurrent.Volatile
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

class RedactedError(val type: String, message: String) : Exception(message)

class TelemetryGate(private val delegate: Telemetry) : Telemetry {
    @Volatile
    private var enabled = false

    val isEnabled: Boolean get() = enabled

    fun follow(settings: SettingsStore, scope: CoroutineScope): Job = scope.launch {
        settings.settings.map { it.telemetryOptIn }.distinctUntilChanged().collect(::setCollectionEnabled)
    }

    override fun setCollectionEnabled(enabled: Boolean) {
        this.enabled = enabled
        delegate.setCollectionEnabled(enabled)
    }

    override fun event(event: AnalyticsEvent, metrics: Map<String, Long>) {
        if (enabled) delegate.event(event, metrics.filterKeys { SAFE_KEY.matches(it) })
    }

    override fun breadcrumb(message: String) {
        if (enabled) delegate.breadcrumb(Redactor.scrub(message))
    }

    override fun nonFatal(error: Throwable) {
        if (enabled) delegate.nonFatal(RedactedError(error::class.simpleName ?: "Throwable", Redactor.scrub(error.message.orEmpty())))
    }

    private companion object {
        val SAFE_KEY = Regex("[a-z][a-z0-9_]{0,39}")
    }
}

object Redactor {
    const val MASK: String = "[redacted]"

    private val patterns = listOf(
        Regex("https?://\\S+"),
        Regex("[0-9A-Za-z]{4}(?:-[0-9A-Za-z]{4}){2,}"),
        Regex("(?<![0-9A-Za-z])(?=[0-9A-Za-z]*[0-9])[0-9A-Za-z+/=_-]{16,}"),
        Regex("(?<![0-9A-Za-z])[0-9A-Z]{16,}(?![0-9A-Za-z])"),
        Regex("-?\\d{1,3}\\.\\d{3,}"),
        Regex("[\\w.+-]+@[\\w-]+\\.[\\w.]+"),
    )

    fun scrub(text: String): String = patterns.fold(text) { acc, r -> r.replace(acc, MASK) }
}
