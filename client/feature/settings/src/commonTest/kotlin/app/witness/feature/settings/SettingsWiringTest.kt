package app.witness.feature.settings

import app.witness.core.domain.fake.FakeAssistant
import app.witness.core.domain.fake.FakeClock
import app.witness.core.domain.fake.FakeEntitlementSource
import app.witness.core.domain.fake.InMemorySettingsStore
import app.witness.core.domain.fake.SeededRandom
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.Entitlement
import app.witness.core.model.Tier
import app.witness.feature.settings.fake.FakeDataExporter
import app.witness.feature.settings.fake.FakePlusBilling
import app.witness.feature.settings.fake.InMemoryIdentityAccount
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotSame
import kotlin.test.assertTrue

class SettingsWiringTest : MainDispatcherTest() {
    private val settings = InMemorySettingsStore()
    private val identity = InMemoryIdentityAccount()
    private val deps = SettingsDependencies(
        settings = settings,
        entitlements = FakeEntitlementSource(),
        identity = identity,
        linkFlow = IdentityLinkFlow(identity, SeededRandom(1)),
        consent = SettingsTelemetryConsent(settings),
        assistant = FakeAssistant(),
        billing = FakePlusBilling(),
        exporter = FakeDataExporter(),
        eraseAll = { EraseReport() },
        clock = FakeClock(),
    )

    @Test
    fun factoriesBuildFreshViewModels() {
        assertNotSame(deps.settingsViewModel(), deps.settingsViewModel())
        assertNotSame(deps.plusViewModel(), deps.plusViewModel())
        assertNotSame(deps.identityLinkViewModel(), deps.identityLinkViewModel())
        assertNotSame(deps.privacyDataViewModel(), deps.privacyDataViewModel())
    }

    @Test
    fun keysAreDistinct() {
        val keys = listOf(SettingsKey.Root, SettingsKey.Plus, SettingsKey.LinkIdentity, SettingsKey.PrivacyData)
        assertEquals(keys.size, keys.toSet().size)
    }

    @Test
    fun deleteFromSettingsUnlinksSharedIdentity() = runTest {
        identity.link(TestPrincipals.alice)
        val vm = deps.settingsViewModel()
        vm.requestDelete()
        vm.confirmDelete()
        assertEquals(null, identity.value)
    }

    @Test
    fun fakeBillingGrantsEntitlementOnPurchase() = runTest {
        val ent = MutableStateFlow(Entitlement.FREE)
        val billing = FakePlusBilling(entitlement = ent)
        billing.purchase(PlusPlan.YEARLY, TestPrincipals.alice)
        assertEquals(Tier.PLUS, ent.value.tier)
        assertEquals(Entitlement.PLUS_QUOTA_BYTES, ent.value.quotaBytes)
    }

    @Test
    fun fakeBillingDoesNotGrantOnCancel() = runTest {
        val ent = MutableStateFlow(Entitlement.FREE)
        FakePlusBilling(outcome = PurchaseOutcome.Cancelled, entitlement = ent).purchase(PlusPlan.MONTHLY, TestPrincipals.alice)
        assertEquals(Entitlement.FREE, ent.value)
    }

    @Test
    fun fakeIdentityRejectsWhenConfigured() = runTest {
        val rejecting = InMemoryIdentityAccount(accept = false)
        assertEquals(false, rejecting.link(TestPrincipals.alice))
        assertEquals(null, rejecting.value)
    }

    @Test
    fun privacySectionsMatchDesign() {
        assertEquals(listOf(DataPlace.Permanent, DataPlace.Erasable, DataPlace.PhoneOnly), PrivacyDataCopy.SECTIONS.map { it.place })
        assertTrue(PrivacyDataCopy.SECTIONS.all { it.rows.size == 3 })
        assertEquals("Hashes and commitments", PrivacyDataCopy.SECTIONS[0].rows[0].second)
    }

    @Test
    fun deleteDialogListsWhatGoesAndStays() {
        assertEquals(4, DeleteAccountCopy.DELETED.size)
        assertTrue("Your Internet Identity link" in DeleteAccountCopy.DELETED)
        assertEquals(1, DeleteAccountCopy.STAYS.size)
    }
}
