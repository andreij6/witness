package app.witness.feature.settings

import app.witness.core.designsystem.Tone
import app.witness.core.domain.usecase.EraseReport
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SettingsCopyTest {
    private fun ids(n: Int) = List(n) { i -> RecordId.fromBytes(ByteArray(10) { i.toByte() }) }

    @Test
    fun accountCopyWithoutLink() {
        assertEquals("No account", SettingsCopy.accountTitle(null))
        assertEquals("Optional: sign in with Internet Identity to sync", SettingsCopy.accountSubtitle(null))
    }

    @Test
    fun accountCopyWithLinkShowsShortPrincipal() {
        val p = TestPrincipals.alice
        assertEquals("Internet Identity", SettingsCopy.accountTitle(p))
        assertEquals("Linked · ${p.short}", SettingsCopy.accountSubtitle(p))
    }

    @Test
    fun everyPrecisionHasALabel() {
        assertEquals("City or area", SettingsCopy.precisionLabel(LocationPrecision.AREA))
        assertEquals(LocationPrecision.entries.size, LocationPrecision.entries.map(SettingsCopy::precisionLabel).toSet().size)
    }

    @Test
    fun aiSubtitleReflectsOnDeviceModel() {
        assertEquals("Gemini Nano available", SettingsCopy.aiSubtitle(true))
        assertEquals(SettingsCopy.AI_RULES, SettingsCopy.aiSubtitle(false))
    }

    @Test
    fun priceCopy() {
        assertEquals("$2.99 / month", SettingsCopy.monthlyPrice("$2.99"))
        assertEquals("per year · $2.08 a month", SettingsCopy.perYear(PlanOffer.YEARLY))
        assertEquals("per year", SettingsCopy.perYear(PlanOffer(PlusPlan.YEARLY, "gratis", 0)))
        assertEquals("Save 30%", SettingsCopy.savings(30))
        assertNull(SettingsCopy.savings(0))
        assertNull(SettingsCopy.savings(-3))
    }

    @Test
    fun eraseNoticeSuccess() {
        assertEquals(Notice("3 public copies erased. Proofs stay.", Tone.Ok), SettingsCopy.eraseNotice(EraseReport(erased = ids(3))))
        assertEquals(Notice("1 public copy erased. Proofs stay.", Tone.Ok), SettingsCopy.eraseNotice(EraseReport(erased = ids(1))))
    }

    @Test
    fun eraseNoticeNothingToErase() = assertEquals(Tone.Muted, SettingsCopy.eraseNotice(EraseReport()).tone)

    @Test
    fun eraseNoticeOfflineWinsOverStuck() {
        val n = SettingsCopy.eraseNotice(EraseReport(erased = ids(2), rejected = ids(1), offline = true))
        assertEquals(Tone.Warn, n.tone)
        assertTrue(n.text.startsWith("You're offline. 2 public copies"))
    }

    @Test
    fun eraseNoticeCountsRejectedAndMissingKeys() {
        val n = SettingsCopy.eraseNotice(EraseReport(erased = ids(1), rejected = ids(1), missingKey = ids(2)))
        assertEquals(Tone.Warn, n.tone)
        assertTrue(n.text.startsWith("1 public copy erased. 3 public copies need"), n.text)
    }

    @Test
    fun deleteNotice() {
        assertEquals(Tone.Ok, SettingsCopy.deleteNotice(DeleteOutcome.Deleted(4)).tone)
        val partial = SettingsCopy.deleteNotice(DeleteOutcome.Incomplete(EraseReport(offline = true)))
        assertEquals(Tone.Warn, partial.tone)
        assertTrue(partial.text.startsWith("Deletion incomplete. You're offline."))
    }

    @Test
    fun exportNotice() {
        assertEquals("Export ready · 1 record", SettingsCopy.exportNotice(ExportOutcome.Ready(1)).text)
        assertEquals("Export ready · 0 records", SettingsCopy.exportNotice(ExportOutcome.Ready(0)).text)
        assertEquals(Tone.Crit, SettingsCopy.exportNotice(ExportOutcome.Failed("disk")).tone)
    }

    @Test
    fun purchaseNotice() {
        assertEquals(Tone.Ok, SettingsCopy.purchaseNotice(PurchaseOutcome.Purchased)?.tone)
        assertNull(SettingsCopy.purchaseNotice(PurchaseOutcome.Cancelled))
        assertEquals(Tone.Muted, SettingsCopy.purchaseNotice(PurchaseOutcome.NothingToRestore)?.tone)
        assertEquals(Tone.Crit, SettingsCopy.purchaseNotice(PurchaseOutcome.Failed("x"))?.tone)
    }

    @Test
    fun everyLinkErrorHasCopy() {
        for (e in LinkError.entries) assertTrue(SettingsCopy.linkError(e).isNotBlank(), e.name)
    }

    @Test
    fun linkErrorsNeverLeakDetails() {
        for (e in LinkError.entries) assertTrue(!SettingsCopy.linkError(e).contains("nonce", ignoreCase = true), e.name)
    }
}
