package app.witness.feature.settings

import app.witness.core.domain.port.SettingsStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged

data class TelemetryConsent(val analytics: Boolean, val crashReports: Boolean) {
    val collectionAllowed: Boolean get() = analytics && crashReports

    companion object {
        val NONE = TelemetryConsent(analytics = false, crashReports = false)
        val ALL = TelemetryConsent(analytics = true, crashReports = true)
    }
}

interface TelemetryConsentStore {
    val consent: Flow<TelemetryConsent>

    suspend fun set(consent: TelemetryConsent)
}

class SettingsTelemetryConsent(private val settings: SettingsStore) : TelemetryConsentStore {
    private val partial = MutableStateFlow(TelemetryConsent.NONE)

    override val consent: Flow<TelemetryConsent> =
        combine(settings.settings, partial) { s, p -> if (s.telemetryOptIn) TelemetryConsent.ALL else if (p.collectionAllowed) TelemetryConsent.NONE else p }.distinctUntilChanged()

    override suspend fun set(consent: TelemetryConsent) {
        partial.value = consent
        settings.update { if (it.telemetryOptIn == consent.collectionAllowed) it else it.copy(telemetryOptIn = consent.collectionAllowed) }
    }
}
