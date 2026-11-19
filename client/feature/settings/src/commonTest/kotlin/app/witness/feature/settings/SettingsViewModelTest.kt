package app.witness.feature.settings

import app.witness.core.designsystem.Tone
import app.witness.core.domain.fake.FakeAssistant
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeEntitlementSource
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.Entitlement
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import app.witness.core.model.Tier
import app.witness.feature.settings.fake.FakePlusBilling
import app.witness.feature.settings.fake.InMemoryIdentityAccount
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SettingsViewModelTest : MainDispatcherTest() {
    private val settings = InMemorySettingsStore()
    private val entitlements = FakeEntitlementSource()
    private val identity = InMemoryIdentityAccount()
    private val consent = SettingsTelemetryConsent(settings)
    private val assistant = FakeAssistant()
    private val billing = FakePlusBilling()
    private val clock = FakeClock()
    private var erase: suspend () -> EraseReport = { EraseReport() }

    private fun vm() = SettingsViewModel(settings, entitlements, identity, consent, assistant, billing, DeleteAccount({ erase() }, identity), clock)

    @Test
    fun initialStateMatchesDesignDefaults() = runTest {
        val vm = vm()
        keepActive(vm.state)
        val s = vm.state.value
        assertNull(s.account)
        assertFalse(s.plusActive)
        assertEquals("$2.99", s.monthlyPrice)
        assertEquals(LocationPrecision.AREA, s.precision)
        assertTrue(s.aiEnabled)
        assertEquals(TelemetryConsent.NONE, s.consent)
        assertEquals(DeletionState.Hidden, s.deletion)
        assertNull(s.notice)
    }

    @Test
    fun reflectsAssistantAvailability() = runTest {
        assistant.available = true
        val vm = vm()
        keepActive(vm.state)
        assertTrue(vm.state.value.aiOnDevice)
    }

    @Test
    fun assistantFailureMeansRulesPath() = runTest {
        assistant.available = true
        val vm = SettingsViewModel(
            settings, entitlements, identity, consent,
            object : app.witness.core.domain.port.Assistant by assistant {
                override suspend fun isAvailable(): Boolean = error("ml kit missing")
            },
            billing, DeleteAccount({ erase() }, identity), clock,
        )
        keepActive(vm.state)
        assertFalse(vm.state.value.aiOnDevice)
    }

    @Test
    fun monthlyPriceComesFromStore() = runTest {
        billing.offers = listOf(PlanOffer(PlusPlan.MONTHLY, "€3,49", 3_490_000))
        val vm = vm()
        keepActive(vm.state)
        assertEquals("€3,49", vm.state.value.monthlyPrice)
    }

    @Test
    fun billingFailureKeepsDefaultPrice() = runTest {
        billing.failure = IllegalStateException("not configured")
        val vm = vm()
        keepActive(vm.state)
        assertEquals("$2.99", vm.state.value.monthlyPrice)
    }

    @Test
    fun linkedAccountIsShown() = runTest {
        val vm = vm()
        keepActive(vm.state)
        identity.link(TestPrincipals.alice)
        assertEquals(TestPrincipals.alice, vm.state.value.account)
    }

    @Test
    fun activePlusIsShown() = runTest {
        val vm = vm()
        keepActive(vm.state)
        entitlements.state.value = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, clock.nowMs + 1)
        assertTrue(vm.state.value.plusActive)
    }

    @Test
    fun lapsedPlusIsNotActive() = runTest {
        entitlements.state.value = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, clock.nowMs - 1)
        val vm = vm()
        keepActive(vm.state)
        assertFalse(vm.state.value.plusActive)
    }

    @Test
    fun aiToggleWritesSettings() = runTest {
        val vm = vm()
        keepActive(vm.state)
        vm.setAiEnabled(false)
        assertFalse(settings.value.aiAssistant)
        assertFalse(vm.state.value.aiEnabled)
    }

    @Test
    fun precisionPickerOpensSelectsAndCloses() = runTest {
        val vm = vm()
        keepActive(vm.state)
        vm.openPrecisionPicker()
        assertTrue(vm.state.value.precisionPicker)
        vm.setPrecision(LocationPrecision.HIDDEN)
        assertFalse(vm.state.value.precisionPicker)
        assertEquals(LocationPrecision.HIDDEN, settings.value.defaultPrecision)
        assertEquals(LocationPrecision.HIDDEN, vm.state.value.precision)
    }

    @Test
    fun precisionPickerDismissKeepsValue() = runTest {
        val vm = vm()
        keepActive(vm.state)
        vm.openPrecisionPicker()
        vm.closePrecisionPicker()
        assertFalse(vm.state.value.precisionPicker)
        assertEquals(LocationPrecision.AREA, settings.value.defaultPrecision)
    }

    @Test
    fun analyticsAloneDoesNotEnableCollection() = runTest {
        val vm = vm()
        keepActive(vm.state)
        vm.setAnalytics(true)
        assertEquals(TelemetryConsent(analytics = true, crashReports = false), vm.state.value.consent)
        assertFalse(settings.value.telemetryOptIn)
    }

    @Test
    fun bothTogglesEnableCollection() = runTest {
        val vm = vm()
        keepActive(vm.state)
        vm.setAnalytics(true)
        vm.setCrashReports(true)
        assertEquals(TelemetryConsent.ALL, vm.state.value.consent)
        assertTrue(settings.value.telemetryOptIn)
        vm.setCrashReports(false)
        assertFalse(settings.value.telemetryOptIn)
        assertEquals(TelemetryConsent(analytics = true, crashReports = false), vm.state.value.consent)
    }

    @Test
    fun consentTogglesWorkWithoutObservers() = runTest {
        val vm = vm()
        vm.setAnalytics(true)
        vm.setCrashReports(true)
        assertTrue(settings.value.telemetryOptIn)
    }

    @Test
    fun deleteFlowConfirmsThenRuns() = runTest {
        identity.link(TestPrincipals.alice)
        erase = { EraseReport(erased = listOf(RecordId.fromBytes(ByteArray(10)))) }
        val vm = vm()
        keepActive(vm.state)
        vm.requestDelete()
        assertEquals(DeletionState.Confirming, vm.state.value.deletion)
        vm.confirmDelete()
        assertEquals(DeletionState.Hidden, vm.state.value.deletion)
        assertEquals(Tone.Ok, vm.state.value.notice?.tone)
        assertNull(identity.value)
    }

    @Test
    fun deleteShowsRunningWhileErasing() = runTest {
        val gate = CompletableDeferred<EraseReport>()
        erase = { gate.await() }
        val vm = vm()
        keepActive(vm.state)
        vm.requestDelete()
        vm.confirmDelete()
        assertEquals(DeletionState.Running, vm.state.value.deletion)
        vm.cancelDelete()
        vm.confirmDelete()
        assertEquals(DeletionState.Running, vm.state.value.deletion)
        gate.complete(EraseReport())
        assertEquals(DeletionState.Hidden, vm.state.value.deletion)
    }

    @Test
    fun confirmWithoutRequestDoesNothing() = runTest {
        var calls = 0
        erase = { calls++; EraseReport() }
        val vm = vm()
        keepActive(vm.state)
        vm.confirmDelete()
        assertEquals(0, calls)
        assertEquals(DeletionState.Hidden, vm.state.value.deletion)
    }

    @Test
    fun cancelHidesDialogWithoutErasing() = runTest {
        var calls = 0
        erase = { calls++; EraseReport() }
        val vm = vm()
        keepActive(vm.state)
        vm.requestDelete()
        vm.cancelDelete()
        assertEquals(DeletionState.Hidden, vm.state.value.deletion)
        assertEquals(0, calls)
    }

    @Test
    fun deleteFailureShowsWarningAndKeepsLink() = runTest {
        identity.link(TestPrincipals.alice)
        erase = { error("registry down") }
        val vm = vm()
        keepActive(vm.state)
        vm.requestDelete()
        vm.confirmDelete()
        assertEquals(Tone.Warn, vm.state.value.notice?.tone)
        assertEquals(TestPrincipals.alice, identity.value)
        vm.dismissNotice()
        assertNull(vm.state.value.notice)
    }

    @Test
    fun repeatedRequestWhileRunningIsIgnored() = runTest {
        val gate = CompletableDeferred<EraseReport>()
        erase = { gate.await() }
        val vm = vm()
        keepActive(vm.state)
        vm.requestDelete()
        vm.confirmDelete()
        vm.requestDelete()
        assertEquals(DeletionState.Running, vm.state.value.deletion)
        gate.complete(EraseReport())
    }
}
