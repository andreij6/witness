package app.witness.core.data

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.MutablePreferences
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.byteArrayPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.longPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import app.witness.core.domain.port.SettingsStore
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureMode
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationPrecision
import app.witness.core.model.Settings
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map

class DataStoreSettings(private val store: DataStore<Preferences>) : SettingsStore {
    override val settings: Flow<Settings> = store.data.map(::read).distinctUntilChanged()

    override suspend fun update(transform: (Settings) -> Settings): Settings {
        var result = DEFAULTS
        store.edit { prefs ->
            result = transform(read(prefs))
            write(prefs, result)
        }
        return result
    }

    companion object {
        const val FILE_NAME = "witness_settings"
        private val TELEMETRY = booleanPreferencesKey("telemetry_opt_in")
        private val AI = booleanPreferencesKey("ai_assistant")
        private val PRECISION = stringPreferencesKey("default_precision")
        private val MODE = stringPreferencesKey("capture_mode")
        private val ACTIVE_CASE = longPreferencesKey("active_case")
        private val INSTALL_NONCE = byteArrayPreferencesKey("install_nonce")
        private val DEFAULTS = Settings()

        fun read(prefs: Preferences) = Settings(
            telemetryOptIn = prefs[TELEMETRY] ?: DEFAULTS.telemetryOptIn,
            aiAssistant = prefs[AI] ?: DEFAULTS.aiAssistant,
            defaultPrecision = prefs[PRECISION]?.let { n -> LocationPrecision.entries.firstOrNull { it.name == n } } ?: DEFAULTS.defaultPrecision,
            captureMode = prefs[MODE]?.let { n -> CaptureMode.entries.firstOrNull { it.name == n } } ?: DEFAULTS.captureMode,
            activeCase = prefs[ACTIVE_CASE]?.let(::LocalCaseId),
            installNonce = prefs[INSTALL_NONCE]?.let(ByteString::of),
        )

        fun write(prefs: MutablePreferences, s: Settings) {
            prefs[TELEMETRY] = s.telemetryOptIn
            prefs[AI] = s.aiAssistant
            prefs[PRECISION] = s.defaultPrecision.name
            prefs[MODE] = s.captureMode.name
            s.activeCase.let { if (it == null) prefs.remove(ACTIVE_CASE) else prefs[ACTIVE_CASE] = it.value }
            s.installNonce.let { if (it == null) prefs.remove(INSTALL_NONCE) else prefs[INSTALL_NONCE] = it.toByteArray() }
        }
    }
}
