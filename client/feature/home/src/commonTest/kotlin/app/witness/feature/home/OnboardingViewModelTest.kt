package app.witness.feature.home

import app.witness.core.domain.port.KeyVault
import app.witness.core.model.ByteString
import app.witness.core.model.DeviceKey
import app.witness.core.model.Freshness
import app.witness.core.model.KeyId
import app.witness.core.model.SecurityLevel
import app.witness.core.domain.port.GeneratedKey
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue

class SplashViewModelTest : MainDispatcherTest() {
    @Test
    fun noKeyRoutesToWelcome() = runTest {
        val vm = SplashViewModel(World().vault)
        assertEquals(HomeKey.Welcome, vm.destination.first { it != null })
    }

    @Test
    fun enrolledKeyRoutesHome() = runTest {
        val w = World()
        w.enrollment()()
        assertEquals(HomeKey.Home, SplashViewModel(w.vault).destination.first { it != null })
    }

    @Test
    fun pendingKeyRoutesHome() = runTest {
        val w = World()
        w.registry.offline = true
        w.enrollment()()
        assertEquals(HomeKey.Home, SplashViewModel(w.vault).destination.first { it != null })
    }

    @Test
    fun retiredKeyRoutesToWelcome() = runTest {
        val w = World()
        w.vault.put(DeviceKey(KeyId.fromBytes(ByteArray(KeyId.SIZE)), SecurityLevel.TEE, Freshness.FRESH, NOW, retired = true))
        assertEquals(HomeKey.Welcome, SplashViewModel(w.vault).destination.first { it != null })
    }
}

class PermissionsViewModelTest : MainDispatcherTest() {
    private val granted = PermissionProbe(granted = true, rationale = false)
    private val denied = PermissionProbe(granted = false, rationale = true)
    private val silent = PermissionProbe(granted = false, rationale = false)

    @Test
    fun startsUnknownAndAsksToRequest() {
        val s = PermissionsViewModel().state.value
        assertEquals(Grant.Unknown, s.camera)
        assertEquals(PermissionsAction.Request, s.action)
        assertEquals(PermissionsCopy.CONTINUE, s.buttonLabel)
        assertEquals(PermissionsCopy.CHANGE_LATER, s.note)
    }

    @Test
    fun syncWithGrantedCameraProceeds() {
        val vm = PermissionsViewModel()
        vm.sync(granted, silent)
        assertEquals(PermissionsAction.Proceed, vm.state.value.action)
        assertEquals(Grant.Unknown, vm.state.value.location)
    }

    @Test
    fun resultWithCameraGrantedReturnsProceed() {
        val vm = PermissionsViewModel()
        assertTrue(vm.onResult(granted, denied))
        assertEquals(Grant.Granted, vm.state.value.camera)
        assertEquals(Grant.Denied, vm.state.value.location)
    }

    @Test
    fun locationIsOptional() {
        val vm = PermissionsViewModel()
        assertTrue(vm.onResult(granted, silent))
        assertEquals(Grant.Blocked, vm.state.value.location)
        assertEquals(PermissionsAction.Proceed, vm.state.value.action)
    }

    @Test
    fun deniedCameraStaysAndWarns() {
        val vm = PermissionsViewModel()
        assertFalse(vm.onResult(denied, granted))
        val s = vm.state.value
        assertEquals(Grant.Denied, s.camera)
        assertEquals(PermissionsAction.Request, s.action)
        assertEquals(PermissionsCopy.CAMERA_NEEDED, s.note)
        assertEquals(app.witness.core.designsystem.Tone.Warn, s.noteTone)
    }

    @Test
    fun deniedWithoutRationaleAfterAskingIsBlocked() {
        val vm = PermissionsViewModel()
        assertFalse(vm.onResult(silent, silent))
        val s = vm.state.value
        assertEquals(Grant.Blocked, s.camera)
        assertEquals(PermissionsAction.OpenSettings, s.action)
        assertEquals(PermissionsCopy.OPEN_SETTINGS, s.buttonLabel)
        assertEquals(PermissionsCopy.CAMERA_BLOCKED, s.note)
    }

    @Test
    fun syncAfterDenialWithoutRationaleBlocks() {
        val vm = PermissionsViewModel()
        vm.onResult(denied, denied)
        vm.sync(silent, denied)
        assertEquals(Grant.Blocked, vm.state.value.camera)
    }

    @Test
    fun grantingInSettingsUnblocks() {
        val vm = PermissionsViewModel()
        vm.onResult(silent, silent)
        vm.sync(granted, granted)
        assertEquals(PermissionsAction.Proceed, vm.state.value.action)
        assertEquals(Grant.Granted, vm.state.value.location)
    }

    @Test
    fun nextStateTable() {
        assertEquals(Grant.Granted, PermissionsViewModel.next(Grant.Blocked, granted, asked = false))
        assertEquals(Grant.Denied, PermissionsViewModel.next(Grant.Unknown, denied, asked = false))
        assertEquals(Grant.Unknown, PermissionsViewModel.next(Grant.Unknown, silent, asked = false))
        assertEquals(Grant.Unknown, PermissionsViewModel.next(Grant.Granted, silent, asked = false))
        assertEquals(Grant.Blocked, PermissionsViewModel.next(Grant.Unknown, silent, asked = true))
        assertEquals(Grant.Blocked, PermissionsViewModel.next(Grant.Denied, silent, asked = false))
        assertEquals(Grant.Blocked, PermissionsViewModel.next(Grant.Blocked, silent, asked = false))
    }
}

class DeviceSetupViewModelTest : MainDispatcherTest() {
    @Test
    fun onlineEnrollmentEndsReady() = runTest {
        val w = World()
        val s = DeviceSetupViewModel(w.vault, w.enrollment()).state.first { !it.isWorking }
        assertEquals(SetupPhase.Ready, s.phase)
        assertTrue(s.canOpenCamera)
        assertFalse(s.canRetry)
        assertTrue(s.steps.all { it.status == StepStatus.Done })
        assertEquals("Hardware key created · StrongBox", s.steps[0].title)
        assertEquals(null, s.footnote)
        assertEquals(1, w.registry.enrollments.size)
    }

    @Test
    fun attestationDetailNeverClaimsAVerifiedPhoneForADevKey() {
        for (phase in listOf(SetupPhase.Verifying, SetupPhase.Ready, SetupPhase.Offline)) {
            assertEquals(DeviceSetupCopy.ATTESTATION_DEV, DeviceSetupUiState.steps(phase, SecurityLevel.DEV)[1].detail)
            assertEquals(DeviceSetupCopy.ATTESTATION_DETAIL, DeviceSetupUiState.steps(phase, SecurityLevel.STRONG_BOX)[1].detail)
            assertEquals(DeviceSetupCopy.ATTESTATION_DETAIL, DeviceSetupUiState.steps(phase, null)[1].detail)
        }
    }

    @Test
    fun offlineEnrollmentLetsUserCaptureNow() = runTest {
        val w = World()
        w.registry.offline = true
        val s = DeviceSetupViewModel(w.vault, w.enrollment()).state.first { !it.isWorking }
        assertEquals(SetupPhase.Offline, s.phase)
        assertTrue(s.canOpenCamera)
        assertEquals(listOf(StepStatus.Done, StepStatus.Waiting, StepStatus.Waiting, StepStatus.Done), s.steps.map { it.status })
        assertEquals(DeviceSetupCopy.CERTIFICATE_LATER, s.steps[2].detail)
        assertEquals(DeviceSetupCopy.FINISHES_LATER, s.footnote)
        assertEquals(DeviceSetupCopy.UNREACHABLE, s.banner)
    }

    @Test
    fun unreachableNetworkIsSaidPlainlyOnlyWhenItHappened() {
        assertEquals(DeviceSetupCopy.UNREACHABLE, DeviceSetupUiState.of(SetupPhase.Offline, SecurityLevel.DEV).banner)
        for (phase in listOf(SetupPhase.CreatingKey, SetupPhase.Verifying, SetupPhase.Ready, SetupPhase.Failed, SetupPhase.Unsupported("x"))) {
            assertEquals(DeviceSetupCopy.OFFLINE, DeviceSetupUiState.of(phase, null).banner)
        }
        assertTrue("can't reach" in DeviceSetupCopy.UNREACHABLE.lowercase())
    }

    @Test
    fun rejectedDeviceIsUnsupported() = runTest {
        val w = World()
        w.registry.rejectEnrollment = "bootloader_unlocked"
        val s = DeviceSetupViewModel(w.vault, w.enrollment()).state.first { !it.isWorking }
        assertEquals(SetupPhase.Unsupported("bootloader_unlocked"), s.phase)
        assertFalse(s.canOpenCamera)
    }

    @Test
    fun unexpectedErrorFailsAndRetrySucceeds() = runTest {
        val w = World()
        val flaky = FlakyVault(w.vault, failures = 1)
        val vm = DeviceSetupViewModel(flaky, w.enrollment(flaky))
        val failed = vm.state.first { !it.isWorking }
        assertEquals(SetupPhase.Failed, failed.phase)
        assertTrue(failed.canRetry)
        assertEquals(StepStatus.Failed, failed.steps[0].status)
        assertEquals(DeviceSetupCopy.FAILED, failed.footnote)
        vm.retry()
        assertEquals(SetupPhase.Ready, vm.state.first { it.phase == SetupPhase.Ready }.phase)
    }

    @Test
    fun retryIgnoredUnlessFailed() = runTest {
        val w = World()
        val vm = DeviceSetupViewModel(w.vault, w.enrollment())
        vm.state.first { !it.isWorking }
        vm.retry()
        assertEquals(1, w.registry.enrollments.size)
    }

    @Test
    fun alreadyEnrolledKeyFinishesWithoutNewEnrollment() = runTest {
        val w = World()
        w.enrollment()()
        val s = DeviceSetupViewModel(w.vault, w.enrollment()).state.first { !it.isWorking }
        assertEquals(SetupPhase.Ready, s.phase)
        assertEquals(1, w.registry.enrollments.size)
    }

    @Test
    fun pendingKeyCompletesWhenBackOnline() = runTest {
        val w = World()
        w.registry.offline = true
        w.enrollment()()
        w.registry.offline = false
        val s = DeviceSetupViewModel(w.vault, w.enrollment()).state.first { !it.isWorking }
        assertEquals(SetupPhase.Ready, s.phase)
    }

    @Test
    fun stepsForEachPhase() {
        fun statuses(p: SetupPhase, sec: SecurityLevel? = SecurityLevel.TEE) = DeviceSetupUiState.steps(p, sec).map { it.status }
        assertEquals(listOf(StepStatus.Busy, StepStatus.Todo, StepStatus.Todo, StepStatus.Todo), statuses(SetupPhase.CreatingKey, null))
        assertEquals(listOf(StepStatus.Done, StepStatus.Busy, StepStatus.Todo, StepStatus.Todo), statuses(SetupPhase.Verifying))
        assertEquals(listOf(StepStatus.Done, StepStatus.Failed, StepStatus.Todo, StepStatus.Todo), statuses(SetupPhase.Failed))
        assertEquals(listOf(StepStatus.Done, StepStatus.Failed, StepStatus.Todo, StepStatus.Todo), statuses(SetupPhase.Unsupported("x")))
    }

    @Test
    fun keyTitleOmitsSecurityWhenUnknown() {
        assertEquals(DeviceSetupCopy.KEY, DeviceSetupUiState.steps(SetupPhase.CreatingKey, null)[0].title)
        assertEquals("Hardware key created · TEE", DeviceSetupUiState.steps(SetupPhase.Ready, SecurityLevel.TEE)[0].title)
    }

    @Test
    fun workingPhasesShowUsualDurationAndDisableCamera() {
        for (p in listOf(SetupPhase.CreatingKey, SetupPhase.Verifying)) {
            val s = DeviceSetupUiState.of(p, null)
            assertTrue(s.isWorking)
            assertFalse(s.canOpenCamera)
            assertEquals(DeviceSetupCopy.USUALLY_FAST, s.footnote)
        }
    }

    @Test
    fun readyStepDetailsSwitchWhenDone() {
        val ready = DeviceSetupUiState.steps(SetupPhase.Ready, SecurityLevel.STRONG_BOX)
        assertEquals(DeviceSetupCopy.CERTIFICATE_DONE, ready[2].detail)
        assertEquals(DeviceSetupCopy.READY_NOW, ready[3].detail)
        val busy = DeviceSetupUiState.steps(SetupPhase.Verifying, SecurityLevel.STRONG_BOX)
        assertEquals(DeviceSetupCopy.CERTIFICATE_DETAIL, busy[2].detail)
        assertEquals(DeviceSetupCopy.READY_DETAIL, busy[3].detail)
    }

    @Test
    fun phaseMapping() {
        assertIs<SetupPhase.Offline>(DeviceSetupUiState.phaseOf(app.witness.core.domain.usecase.EnrollmentStatus.Offline))
        assertEquals(null, DeviceSetupUiState.securityOf(app.witness.core.domain.usecase.EnrollmentStatus.Offline))
    }
}

private class FlakyVault(private val inner: KeyVault, private var failures: Int) : KeyVault by inner {
    override suspend fun generate(challenge: ByteString, freshness: Freshness): GeneratedKey {
        if (failures-- > 0) throw IllegalStateException("keystore busy")
        return inner.generate(challenge, freshness)
    }
}
