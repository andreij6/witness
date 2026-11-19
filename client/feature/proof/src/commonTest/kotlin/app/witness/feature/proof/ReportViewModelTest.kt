package app.witness.feature.proof

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class ReportViewModelTest {
    @BeforeTest
    fun setUp() = Dispatchers.setMain(UnconfinedTestDispatcher())

    @AfterTest
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun sendsTrimmedReport() = runTest {
        val sender = InMemoryReportSender()
        val vm = ReportViewModel("p/H4IL", sender)
        vm.select(ReportReason.COPYRIGHT)
        vm.onDetails("  my photo  ")
        vm.onEmail(" me@example.com ")
        vm.send()
        assertEquals(ReportPhase.Sent, vm.state.value.phase)
        assertEquals(PageReport("p/H4IL", ReportReason.COPYRIGHT, "my photo", "me@example.com"), sender.sent.single())
    }

    @Test
    fun emptyEmailIsOptional() = runTest {
        val sender = InMemoryReportSender()
        ReportViewModel("p", sender).send()
        assertNull(sender.sent.single().email)
        assertEquals(ReportReason.PRIVATE_INFO, sender.sent.single().reason)
    }

    @Test
    fun invalidEmailBlocksSending() = runTest {
        val sender = InMemoryReportSender()
        val vm = ReportViewModel("p", sender)
        vm.onEmail("nope")
        vm.send()
        assertTrue(sender.sent.isEmpty())
        assertEquals(ReportPhase.Editing, vm.state.value.phase)
        vm.onEmail("a@b.co")
        assertNull(vm.state.value.emailError)
    }

    @Test
    fun failuresAllowRetry() = runTest {
        var ok = false
        val vm = ReportViewModel("p") { if (ok) true else error("down") }
        vm.send()
        assertEquals(ReportPhase.Failed, vm.state.value.phase)
        assertTrue(vm.state.value.canSend)
        ok = true
        vm.send()
        assertEquals(ReportPhase.Sent, vm.state.value.phase)
        val rejected = ReportViewModel("p") { false }
        rejected.send()
        assertEquals(ReportPhase.Failed, rejected.state.value.phase)
    }

    @Test
    fun editsAreLockedWhileSendingAndAfterSent() = runTest {
        val gate = CompletableDeferred<Boolean>()
        val vm = ReportViewModel("p") { gate.await() }
        vm.send()
        assertEquals(ReportPhase.Sending, vm.state.value.phase)
        vm.onDetails("late")
        vm.send()
        assertEquals("", vm.state.value.details)
        gate.complete(true)
        assertEquals(ReportPhase.Sent, vm.state.value.phase)
        vm.select(ReportReason.OTHER)
        assertEquals(ReportReason.PRIVATE_INFO, vm.state.value.reason)
    }

    @Test
    fun inputsAreCapped() {
        val vm = ReportViewModel("p") { true }
        vm.onDetails("x".repeat(5_000))
        vm.onEmail("y".repeat(500))
        assertEquals(ReportViewModel.MAX_DETAILS, vm.state.value.details.length)
        assertEquals(ReportViewModel.MAX_EMAIL, vm.state.value.email.length)
    }

    @Test
    fun emailValidation() {
        listOf("a@b.co", "first.last@sub.example.org", "x+tag@d.io").forEach { assertTrue(ReportViewModel.isEmail(it), it) }
        listOf("", "@b.co", "a@", "a@b", "a@b.c", "a b@c.co", "a@@b.co", "a@.co", "a@b..co", "a@b.co.").forEach { assertFalse(ReportViewModel.isEmail(it), it) }
    }

    @Test
    fun everyReasonHasALabel() {
        assertEquals(6, ReportReason.entries.map { it.label }.distinct().size)
    }
}
