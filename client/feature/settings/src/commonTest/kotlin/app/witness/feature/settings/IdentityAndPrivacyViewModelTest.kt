package app.witness.feature.settings

import app.witness.core.designsystem.Tone
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.RecordId
import app.witness.feature.settings.fake.FakeDataExporter
import app.witness.feature.settings.fake.InMemoryIdentityAccount
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class IdentityLinkViewModelTest : MainDispatcherTest() {
    private val identity = InMemoryIdentityAccount()
    private val flow = IdentityLinkFlow(identity, SeededRandom(3))
    private fun vm() = IdentityLinkViewModel(flow, identity)

    @Test
    fun startOpensBrowserAndWaits() = runTest {
        val vm = vm()
        keepActive(vm.state)
        val effects = record(vm.effect)
        vm.start()
        val open = assertIs<IdentityEffect.OpenBrowser>(effects.single())
        assertTrue(open.url.startsWith(IiLinkContract.AUTHORIZE_URL))
        assertTrue(vm.state.value.waiting)
        assertNull(vm.state.value.error)
    }

    @Test
    fun callbackFromShellLinksAndUpdatesState() = runTest {
        val vm = vm()
        keepActive(vm.state)
        val effects = record(vm.effect)
        vm.start()
        val nonce = (effects.single() as IdentityEffect.OpenBrowser).url.substringAfter("nonce=").substringBefore('&')
        flow.complete("${IiLinkContract.REDIRECT}?v=1&nonce=$nonce&principal=${TestPrincipals.alice.text}")
        assertEquals(TestPrincipals.alice, vm.state.value.linked)
        assertFalse(vm.state.value.waiting)
    }

    @Test
    fun failureIsShownAndDismissible() = runTest {
        val vm = vm()
        keepActive(vm.state)
        vm.start()
        flow.complete("${IiLinkContract.REDIRECT}?v=9")
        assertEquals(SettingsCopy.linkError(LinkError.UNSUPPORTED_VERSION), vm.state.value.error)
        vm.dismissError()
        assertNull(vm.state.value.error)
        assertEquals(LinkStatus.Idle, vm.state.value.status)
    }

    @Test
    fun dismissErrorDoesNotAbortWaiting() = runTest {
        val vm = vm()
        keepActive(vm.state)
        vm.start()
        vm.dismissError()
        assertTrue(vm.state.value.waiting)
    }

    @Test
    fun notNowResetsAndCloses() = runTest {
        val vm = vm()
        keepActive(vm.state)
        val effects = record(vm.effect)
        vm.start()
        vm.notNow()
        assertEquals(IdentityEffect.Close, effects.last())
        assertEquals(LinkStatus.Idle, flow.status.value)
    }

    @Test
    fun unlinkClearsAccount() = runTest {
        identity.link(TestPrincipals.alice)
        val vm = vm()
        keepActive(vm.state)
        vm.unlink()
        assertNull(vm.state.value.linked)
        assertEquals(LinkStatus.Idle, vm.state.value.status)
    }

    @Test
    fun unlinkOfflineKeepsAccount() = runTest {
        identity.link(TestPrincipals.alice)
        identity.offline = true
        val vm = vm()
        keepActive(vm.state)
        vm.unlink()
        assertEquals(TestPrincipals.alice, vm.state.value.linked)
    }
}

class PrivacyDataViewModelTest : MainDispatcherTest() {
    private val exporter = FakeDataExporter(ExportOutcome.Ready(12))
    private var erase: suspend () -> EraseReport = { EraseReport() }
    private fun vm() = PrivacyDataViewModel(exporter) { erase() }

    @Test
    fun startsIdle() = assertEquals(PrivacyDataUiState(), vm().state.value)

    @Test
    fun exportReportsCount() = runTest {
        val vm = vm()
        vm.export()
        assertEquals("Export ready · 12 records", vm.state.value.notice?.text)
        assertNull(vm.state.value.busy)
    }

    @Test
    fun exportFailureIsCritical() = runTest {
        exporter.failure = IllegalStateException("disk full")
        val vm = vm()
        vm.export()
        assertEquals(Tone.Crit, vm.state.value.notice?.tone)
    }

    @Test
    fun exportIsNotReentrant() = runTest {
        val gate = CompletableDeferred<Unit>()
        val slow = object : DataExporter {
            var calls = 0
            override suspend fun export(): ExportOutcome {
                calls++
                gate.await()
                return ExportOutcome.Ready(1)
            }
        }
        val vm = PrivacyDataViewModel(slow) { EraseReport() }
        vm.export()
        vm.export()
        vm.requestErase()
        assertEquals(PrivacyBusy.Exporting, vm.state.value.busy)
        assertFalse(vm.state.value.confirmErase)
        gate.complete(Unit)
        assertEquals(1, slow.calls)
        assertNull(vm.state.value.busy)
    }

    @Test
    fun eraseNeedsConfirmation() = runTest {
        var calls = 0
        erase = { calls++; EraseReport() }
        val vm = vm()
        vm.confirmErase()
        assertEquals(0, calls)
        vm.requestErase()
        assertTrue(vm.state.value.confirmErase)
        vm.cancelErase()
        vm.confirmErase()
        assertEquals(0, calls)
    }

    @Test
    fun confirmedEraseReportsResult() = runTest {
        erase = { EraseReport(erased = listOf(RecordId.fromBytes(ByteArray(10)))) }
        val vm = vm()
        vm.requestErase()
        vm.confirmErase()
        assertFalse(vm.state.value.confirmErase)
        assertEquals("1 public copy erased. Proofs stay.", vm.state.value.notice?.text)
        vm.dismissNotice()
        assertNull(vm.state.value.notice)
    }

    @Test
    fun eraseFailureIsTreatedAsOffline() = runTest {
        erase = { error("network") }
        val vm = vm()
        vm.requestErase()
        vm.confirmErase()
        assertEquals(Tone.Warn, vm.state.value.notice?.tone)
        assertTrue(vm.state.value.notice!!.text.startsWith("You're offline"))
    }

    @Test
    fun eraseShowsBusyUntilDone() = runTest {
        val gate = CompletableDeferred<EraseReport>()
        erase = { gate.await() }
        val vm = vm()
        vm.requestErase()
        vm.confirmErase()
        assertEquals(PrivacyBusy.Erasing, vm.state.value.busy)
        vm.export()
        assertEquals(0, exporter.calls)
        gate.complete(EraseReport())
        assertNull(vm.state.value.busy)
    }
}
