package app.witness.core.data

import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.mutablePreferencesOf
import androidx.datastore.preferences.core.stringPreferencesKey
import app.witness.core.model.ByteString
import app.witness.core.model.CaptureMode
import app.witness.core.model.LocalCaseId
import app.witness.core.model.LocationPrecision
import app.witness.core.model.Settings
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull

class DataStoreSettingsTest {
    private fun TestScope.settings(): DataStoreSettings {
        val file = File.createTempFile("settings", ".preferences_pb").also { it.delete(); it.deleteOnExit() }
        return DataStoreSettings(PreferenceDataStoreFactory.create(scope = backgroundScope) { file })
    }

    @Test
    fun defaultsTelemetryOff() = runTest {
        val s = settings().settings.first()
        assertEquals(Settings(), s)
        assertFalse(s.telemetryOptIn)
    }

    @Test
    fun updateReturnsAndPersists() = runTest {
        val store = settings()
        val nonce = ByteString.of(byteArrayOf(1, 2, 3))
        val expected = Settings(true, false, LocationPrecision.EXACT, CaptureMode.CASE, LocalCaseId(4), nonce)
        assertEquals(expected, store.update { expected })
        assertEquals(expected, store.settings.first())
        val cleared = store.update { it.copy(activeCase = null, installNonce = null) }
        assertNull(cleared.activeCase)
        assertNull(store.settings.first().installNonce)
    }

    @Test
    fun installNonceIsStableOnceSet() = runTest {
        val store = settings()
        val first = store.update { if (it.installNonce != null) it else it.copy(installNonce = ByteString.of(byteArrayOf(9))) }.installNonce
        val second = store.update { if (it.installNonce != null) it else it.copy(installNonce = ByteString.of(byteArrayOf(8))) }.installNonce
        assertEquals(first, second)
    }

    @Test
    fun unknownEnumsFallBack() {
        val prefs = mutablePreferencesOf(stringPreferencesKey("capture_mode") to "VIDEO", stringPreferencesKey("default_precision") to "GPS")
        val s = DataStoreSettings.read(prefs)
        assertEquals(CaptureMode.POST, s.captureMode)
        assertEquals(LocationPrecision.AREA, s.defaultPrecision)
    }
}
