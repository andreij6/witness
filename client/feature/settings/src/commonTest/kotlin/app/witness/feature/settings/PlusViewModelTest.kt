package app.witness.feature.settings

import app.witness.core.designsystem.Tone
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeEntitlementSource
import app.witness.core.model.Entitlement
import app.witness.core.model.Tier
import app.witness.feature.settings.fake.FakePlusBilling
import app.witness.feature.settings.fake.InMemoryIdentityAccount
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PlusViewModelTest : MainDispatcherTest() {
    private val entitlements = FakeEntitlementSource()
    private val clock = FakeClock()
    private val billing = FakePlusBilling(entitlement = entitlements.state, grantExpiresAtMs = clock.nowMs + 30L * 86_400_000)
    private val identity = InMemoryIdentityAccount()

    private fun vm() = PlusViewModel(billing, entitlements, identity, clock)

    @Test
    fun defaultsToYearlyWithDesignCopy() = runTest {
        val vm = vm()
        keepActive(vm.state)
        val s = vm.state.value
        assertEquals(PlusPlan.YEARLY, s.selected)
        assertEquals("$2.99", s.monthly.price)
        assertEquals("$24.99", s.yearly.price)
        assertEquals("Save 30%", s.savings)
        assertEquals("per year · $2.08 a month", s.yearlyDetail)
        assertFalse(s.active)
        assertNull(s.busy)
    }

    @Test
    fun storeOffersReplaceDefaults() = runTest {
        billing.offers = listOf(PlanOffer(PlusPlan.MONTHLY, "3,49 €", 3_490_000), PlanOffer(PlusPlan.YEARLY, "29,99 €", 29_990_000))
        val vm = vm()
        keepActive(vm.state)
        assertEquals("3,49 €", vm.state.value.monthly.price)
        assertEquals("per year · 2,50 € a month", vm.state.value.yearlyDetail)
        assertEquals("Save 28%", vm.state.value.savings)
    }

    @Test
    fun emptyOrFailingStoreKeepsDefaults() = runTest {
        billing.offers = emptyList()
        val a = vm()
        keepActive(a.state)
        assertEquals(PlanOffer.YEARLY, a.state.value.yearly)
        billing.failure = IllegalStateException("offline")
        val b = vm()
        keepActive(b.state)
        assertEquals(PlanOffer.MONTHLY, b.state.value.monthly)
    }

    @Test
    fun selectingPlan() = runTest {
        val vm = vm()
        keepActive(vm.state)
        vm.select(PlusPlan.MONTHLY)
        assertEquals(PlusPlan.MONTHLY, vm.state.value.selected)
    }

    @Test
    fun subscribeWithoutIdentityAsksToLink() = runTest {
        val vm = vm()
        keepActive(vm.state)
        val effects = record(vm.effect)
        vm.subscribe()
        assertEquals(listOf<PlusEffect>(PlusEffect.LinkIdentity), effects)
        assertTrue(billing.purchases.isEmpty())
        assertNull(vm.state.value.busy)
    }

    @Test
    fun subscribePurchasesSelectedPlanForLinkedIdentity() = runTest {
        identity.link(TestPrincipals.alice)
        val vm = vm()
        keepActive(vm.state)
        vm.select(PlusPlan.MONTHLY)
        vm.subscribe()
        assertEquals(listOf(PlusPlan.MONTHLY to TestPrincipals.alice), billing.purchases)
        assertTrue(vm.state.value.active)
        assertEquals(Tone.Ok, vm.state.value.notice?.tone)
        assertNull(vm.state.value.busy)
    }

    @Test
    fun cancelledPurchaseShowsNothing() = runTest {
        identity.link(TestPrincipals.alice)
        billing.outcome = PurchaseOutcome.Cancelled
        val vm = vm()
        keepActive(vm.state)
        vm.subscribe()
        assertNull(vm.state.value.notice)
        assertFalse(vm.state.value.active)
    }

    @Test
    fun purchaseExceptionBecomesFailureNotice() = runTest {
        identity.link(TestPrincipals.alice)
        val vm = vm()
        keepActive(vm.state)
        billing.failure = IllegalStateException("play services")
        vm.subscribe()
        assertEquals(Tone.Crit, vm.state.value.notice?.tone)
        assertNull(vm.state.value.busy)
    }

    @Test
    fun restoreUsesLinkedIdentityWhenPresent() = runTest {
        identity.link(TestPrincipals.bob)
        billing.restoreOutcome = PurchaseOutcome.Purchased
        val vm = vm()
        keepActive(vm.state)
        vm.restore()
        assertEquals(listOf<Principal?>(TestPrincipals.bob), billing.restores)
        assertTrue(vm.state.value.active)
    }

    @Test
    fun restoreWithNothingToRestore() = runTest {
        val vm = vm()
        keepActive(vm.state)
        vm.restore()
        assertEquals(listOf<Principal?>(null), billing.restores)
        assertEquals(Tone.Muted, vm.state.value.notice?.tone)
        vm.dismissNotice()
        assertNull(vm.state.value.notice)
    }

    @Test
    fun activeEntitlementShowsManage() = runTest {
        entitlements.state.value = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, null)
        val vm = vm()
        keepActive(vm.state)
        assertTrue(vm.state.value.active)
        val effects = record(vm.effect)
        vm.manage()
        vm.linkIdentity()
        assertEquals(listOf(PlusEffect.ManageSubscription, PlusEffect.LinkIdentity), effects)
    }

    @Test
    fun accountIsExposedForIdentityCard() = runTest {
        val vm = vm()
        keepActive(vm.state)
        assertNull(vm.state.value.account)
        identity.link(TestPrincipals.alice)
        assertEquals(TestPrincipals.alice, vm.state.value.account)
    }
}
