package app.witness.feature.settings

import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.model.Settings
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class TelemetryConsentTest {
    private val settings = InMemorySettingsStore()
    private val store = SettingsTelemetryConsent(settings)

    @Test
    fun collectionNeedsBothConsents() {
        assertTrue(TelemetryConsent.ALL.collectionAllowed)
        assertFalse(TelemetryConsent.NONE.collectionAllowed)
        assertFalse(TelemetryConsent(analytics = true, crashReports = false).collectionAllowed)
        assertFalse(TelemetryConsent(analytics = false, crashReports = true).collectionAllowed)
    }

    @Test
    fun defaultsToNoConsentAndNoCollection() = runTest {
        assertEquals(TelemetryConsent.NONE, store.consent.first())
        assertFalse(settings.value.telemetryOptIn)
    }

    @Test
    fun persistedOptInReadsAsFullConsent() = runTest {
        val opted = SettingsTelemetryConsent(InMemorySettingsStore(Settings(telemetryOptIn = true)))
        assertEquals(TelemetryConsent.ALL, opted.consent.first())
    }

    @Test
    fun partialConsentIsShownButNeverEnablesCollection() = runTest {
        val crashOnly = TelemetryConsent(analytics = false, crashReports = true)
        store.set(crashOnly)
        assertEquals(crashOnly, store.consent.first())
        assertFalse(settings.value.telemetryOptIn)
    }

    @Test
    fun fullConsentEnablesCollection() = runTest {
        store.set(TelemetryConsent.ALL)
        assertTrue(settings.value.telemetryOptIn)
        assertEquals(TelemetryConsent.ALL, store.consent.first())
    }

    @Test
    fun withdrawingEitherConsentDisablesCollection() = runTest {
        store.set(TelemetryConsent.ALL)
        store.set(TelemetryConsent(analytics = false, crashReports = true))
        assertFalse(settings.value.telemetryOptIn)
        assertEquals(TelemetryConsent(analytics = false, crashReports = true), store.consent.first())
    }

    @Test
    fun partialConsentDoesNotSurviveRestart() = runTest {
        store.set(TelemetryConsent(analytics = true, crashReports = false))
        assertEquals(TelemetryConsent.NONE, SettingsTelemetryConsent(settings).consent.first())
    }

    @Test
    fun externalOptOutWins() = runTest {
        store.set(TelemetryConsent.ALL)
        settings.update { it.copy(telemetryOptIn = false) }
        assertEquals(TelemetryConsent.NONE, store.consent.first())
    }

    @Test
    fun otherSettingsAreUntouched() = runTest {
        settings.update { it.copy(aiAssistant = false) }
        store.set(TelemetryConsent.ALL)
        assertFalse(settings.value.aiAssistant)
    }
}
