package app.witness.feature.share

import app.cash.turbine.test
import app.witness.core.model.BlurClass
import app.witness.core.model.BlurProposal
import app.witness.core.model.CaptureOrigin
import app.witness.core.model.CaseId
import app.witness.core.model.CaseShare
import app.witness.core.model.Edit
import app.witness.core.model.Entitlement
import app.witness.core.model.LocalCaseId
import app.witness.core.model.Rect
import app.witness.core.model.RecordId
import app.witness.core.model.ShareExpiry
import app.witness.core.model.TemplateId
import app.witness.core.model.Tier
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ShareCaseViewModelTest {
    private class FakeRedactor(val f: ShareFixture, val caseId: LocalCaseId) : ShareRedactor {
        val calls = ArrayList<Pair<RecordId, Redaction>>()
        override suspend fun redact(recordId: RecordId, redaction: Redaction): RecordId {
            calls += recordId to redaction
            val derived = capture(50 + calls.size, caseId, origin = CaptureOrigin.DERIVATIVE).copy(parent = recordId, edits = listOf(Edit.Resize(1, 1)))
            f.add(derived)
            return derived.recordId
        }
    }

    private suspend fun setup(f: ShareFixture): LocalCaseId {
        val case = f.cases.create("Fender bender", TemplateId.CAR_ACCIDENT, NOW)
        for (n in 1..3) f.add(capture(n, case.id))
        f.add(capture(4, case.id, origin = CaptureOrigin.DERIVATIVE).copy(edits = listOf(Edit.Resize(2, 2))))
        f.add(capture(5, case.id, origin = CaptureOrigin.IMPORT))
        f.add(capture(6, null))
        return case.id
    }

    private fun vm(f: ShareFixture, id: LocalCaseId, redactor: ShareRedactor? = null, detect: Boolean = true) =
        ShareCaseViewModel(id, f.cases, f.captures, f.shareCase, if (detect) f.detector else null, redactor, f.clock, UtcOffset.UTC)

    private fun face() = listOf(BlurProposal(Rect(0, 0, 1, 1), BlurClass.FACE))

    @Test
    fun buildsDesignState() = vmTest {
        val f = ShareFixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        val s = vm.state.value
        assertEquals("Fender bender · 4 items", s.subtitle)
        assertEquals("4 of 4 selected", s.selectedLabel)
        assertEquals(listOf(false, false, false, true), s.items.map { it.edited })
        assertEquals("4 of 4 verified · 1 declared edit", s.recipients)
        assertEquals(ShareExpiry.WEEK, s.expiry)
        assertTrue(s.includeSummary)
        assertEquals("Include summary", s.summaryLabel)
        assertNull(s.link)
        assertTrue(s.privacy.isEmpty())
        assertTrue(s.canShare)
    }

    @Test
    fun missingCase() = vmTest {
        val f = ShareFixture()
        val vm = vm(f, LocalCaseId(77))
        keep(vm.state)
        assertTrue(vm.state.value.missing)
    }

    @Test
    fun toggleItemUpdatesCounts() = vmTest {
        val f = ShareFixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.toggleItem(record(4))
        assertEquals("3 of 4 selected", vm.state.value.selectedLabel)
        assertEquals("3 of 3 verified", vm.state.value.recipients)
        vm.toggleItem(record(4))
        assertEquals("4 of 4 selected", vm.state.value.selectedLabel)
    }

    @Test
    fun deselectingAllDisablesSharing() = vmTest {
        val f = ShareFixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        for (n in 1..4) vm.toggleItem(record(n))
        assertFalse(vm.state.value.canShare)
        vm.shareLink()
        assertEquals(ShareCaseViewModel.NOTHING_TO_SHARE, vm.state.value.error)
        vm.clearError()
        assertNull(vm.state.value.error)
    }

    @Test
    fun facesAreDetectedAndBlurOnByDefaultWhenRedactorExists() = vmTest {
        val f = ShareFixture()
        f.detector.proposals = mapOf(record(2) to face())
        val id = setup(f)
        val vm = vm(f, id, FakeRedactor(f, id))
        keep(vm.state)
        val chips = vm.state.value.privacy
        assertEquals(listOf("Blur faces · 1", "Blur plates", "Hide location"), chips.map { it.label })
        assertEquals(listOf(true, false, false), chips.map { it.on })
        assertEquals(listOf(false, true, false, true), vm.state.value.items.map { it.edited })
        assertEquals("4 of 4 verified · 2 declared edits", vm.state.value.recipients)
    }

    @Test
    fun privacyToggleIgnoredWithoutRedactor() = vmTest {
        val f = ShareFixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.togglePrivacy(PrivacyOption.HIDE_LOCATION)
        assertTrue(vm.state.value.privacy.isEmpty())
    }

    @Test
    fun detectorFailureIsTolerated() = vmTest {
        val f = ShareFixture()
        f.detector.failure = IllegalStateException("ml kit missing")
        val id = setup(f)
        val vm = vm(f, id, FakeRedactor(f, id))
        keep(vm.state)
        assertEquals(listOf(false, false, false), vm.state.value.privacy.map { it.on })
    }

    @Test
    fun shareUploadsAndEmitsLink() = vmTest {
        val f = ShareFixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.effects.test {
            vm.shareLink()
            val sent = assertIs<ShareEvent.SendLink>(awaitItem())
            val link = assertNotNull(vm.state.value.link)
            assertEquals(sent.url, link.url)
            assertTrue(link.url.startsWith("https://case.witness.app/v/"))
            assertTrue(link.display.endsWith("#k=••••"))
            assertEquals("Expires Nov 26 · 4 items", link.meta)
            assertFalse(link.stale)
            assertEquals(1, f.media.uploads.size)
            assertTrue(f.media.uploads.values.single().committed)
        }
    }

    @Test
    fun shareSheetShowsTheExpiryTheMediaCanisterReports() = vmTest {
        val f = ShareFixture()
        f.media.caseExpiryMs = f.clock.nowEpochMillis() + 30L * 86_400_000
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.effects.test {
            vm.setExpiry(ShareExpiry.NEVER)
            vm.shareLink()
            awaitItem()
            assertEquals("Expires Dec 19 · 4 items", vm.state.value.link!!.meta)
        }
    }

    @Test
    fun resharingUnchangedReusesLink() = vmTest {
        val f = ShareFixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.effects.test {
            vm.shareLink()
            val first = awaitItem()
            vm.shareLink()
            assertEquals(first, awaitItem())
            assertEquals(1, f.media.uploads.size)
        }
    }

    @Test
    fun changingChoicesMarksLinkStaleAndReuploads() = vmTest {
        val f = ShareFixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.effects.test {
            vm.shareLink()
            awaitItem()
            vm.setExpiry(ShareExpiry.NEVER)
            assertTrue(vm.state.value.link!!.stale)
            vm.shareLink()
            awaitItem()
            assertEquals(2, f.media.uploads.size)
            assertEquals("Never expires · 4 items", vm.state.value.link!!.meta)
            assertFalse(vm.state.value.link!!.stale)
        }
    }

    @Test
    fun redactionSendsDerivativesInstead() = vmTest {
        val f = ShareFixture()
        f.detector.proposals = mapOf(record(2) to face())
        val id = setup(f)
        val redactor = FakeRedactor(f, id)
        val vm = vm(f, id, redactor)
        keep(vm.state)
        vm.effects.test {
            vm.shareLink()
            awaitItem()
        }
        assertEquals(listOf(record(2) to Redaction(setOf(BlurClass.FACE), false)), redactor.calls)
        assertEquals(4, f.cases.get(id)!!.shares.single().itemCount)
    }

    @Test
    fun hideLocationRedactsEveryItem() = vmTest {
        val f = ShareFixture()
        val id = setup(f)
        val redactor = FakeRedactor(f, id)
        val vm = vm(f, id, redactor)
        keep(vm.state)
        vm.togglePrivacy(PrivacyOption.HIDE_LOCATION)
        vm.effects.test {
            vm.shareLink()
            awaitItem()
        }
        assertEquals(4, redactor.calls.size)
        assertTrue(redactor.calls.all { it.second.hideLocation })
    }

    @Test
    fun tooLargeShowsLimit() = vmTest {
        val f = ShareFixture()
        val case = f.cases.create("Big", TemplateId.GENERAL, NOW)
        f.add(capture(1, case.id), ByteArray((Entitlement.FREE_CASE_BYTES + 1).toInt()))
        val vm = vm(f, case.id)
        keep(vm.state)
        vm.shareLink()
        assertEquals("This selection is 17 MB. Links hold up to 17 MB — remove a few items or get Witness Plus.", vm.state.value.error)
        assertFalse(vm.state.value.sharing)
        assertNull(vm.state.value.link)
    }

    @Test
    fun uploadFailureShowsRetryMessage() = vmTest {
        val f = ShareFixture()
        f.media.failPutAt = 0
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.shareLink()
        assertEquals(ShareCaseViewModel.UPLOAD_FAILED, vm.state.value.error)
        assertFalse(vm.state.value.sharing)
    }

    @Test
    fun existingActiveLinkIsAdopted() = vmTest {
        val f = ShareFixture()
        val id = setup(f)
        val active = CaseShare(CaseId.of(0, ByteArray(15) { 1 }), "https://case.witness.app/v/ABCDEFGHJK#k=x", NOW - HOUR_MS, NOW + HOUR_MS, 4)
        val expired = active.copy(url = "https://case.witness.app/v/OLD#k=y", createdAtMs = NOW, expiresAtMs = NOW - 1)
        f.cases.update(f.cases.get(id)!!.copy(shares = listOf(active, expired)))
        val vm = vm(f, id)
        keep(vm.state)
        assertEquals(active.url, vm.state.value.link!!.url)
        assertFalse(vm.state.value.link!!.stale)
        vm.effects.test {
            vm.shareLink()
            assertEquals(ShareEvent.SendLink(active.url), awaitItem())
        }
        assertTrue(f.media.uploads.isEmpty())
    }

    @Test
    fun shareableExcludesImportsAndOtherCases() {
        val id = LocalCaseId(1)
        val list = listOf(capture(2, id), capture(1, id), capture(3, id, origin = CaptureOrigin.IMPORT), capture(4, null))
        assertEquals(listOf(record(1), record(2)), ShareCaseViewModel.shareable(list, id).map { it.recordId })
    }

    @Test
    fun editedDetection() {
        assertFalse(ShareCaseViewModel.isEdited(capture(1, null)))
        assertTrue(ShareCaseViewModel.isEdited(capture(1, null, origin = CaptureOrigin.DERIVATIVE)))
        assertTrue(ShareCaseViewModel.isEdited(capture(1, null).copy(edits = listOf(Edit.Resize(1, 1)))))
    }
}

class ExportViewModelTest {
    private class RecordingExporter(var result: ExportResult = ExportResult.Saved("a.zip", "loc", 10, 2), var failure: Exception? = null) : CaseExporter {
        val requests = ArrayList<ExportRequest>()
        override suspend fun export(request: ExportRequest): ExportResult {
            requests += request
            failure?.let { throw it }
            return result
        }
    }

    private suspend fun setup(f: ShareFixture): LocalCaseId {
        val case = f.cases.create("Fender bender", TemplateId.CAR_ACCIDENT, NOW)
        for (n in 1..4) f.add(capture(n, case.id))
        return case.id
    }

    @Test
    fun freeTierDefaultsToZipWithPdfLocked() = vmTest {
        val f = ShareFixture()
        val vm = ExportViewModel(setup(f), f.cases, f.captures, f.entitlements, RecordingExporter(), f.clock)
        keep(vm.state)
        val s = vm.state.value
        assertEquals("Fender bender · 4 items", s.subtitle)
        assertEquals(ExportFormat.ZIP, s.format)
        assertTrue(s.pdfLocked)
        assertEquals("Export ZIP", s.buttonLabel)
        assertEquals(ExportUiState.ROUNDED_HINT, s.locationHint)
        assertTrue(s.canExport)
    }

    @Test
    fun lockedPdfAsksForUpgrade() = vmTest {
        val f = ShareFixture()
        val vm = ExportViewModel(setup(f), f.cases, f.captures, f.entitlements, RecordingExporter(), f.clock)
        keep(vm.state)
        vm.effects.test {
            vm.select(ExportFormat.PDF)
            assertEquals(ExportEvent.Upgrade, awaitItem())
        }
        assertEquals(ExportFormat.ZIP, vm.state.value.format)
    }

    @Test
    fun plusUnlocksPdf() = vmTest {
        val f = ShareFixture()
        f.entitlements.state.value = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, NOW + HOUR_MS)
        val exporter = RecordingExporter()
        val id = setup(f)
        val vm = ExportViewModel(id, f.cases, f.captures, f.entitlements, exporter, f.clock)
        keep(vm.state)
        vm.select(ExportFormat.PDF)
        assertEquals("Export PDF", vm.state.value.buttonLabel)
        vm.export()
        assertEquals(listOf(ExportRequest(id, ExportFormat.PDF, false)), exporter.requests)
    }

    @Test
    fun lapsedPlusFallsBackToZip() = vmTest {
        val f = ShareFixture()
        f.entitlements.state.value = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, NOW + HOUR_MS)
        val vm = ExportViewModel(setup(f), f.cases, f.captures, f.entitlements, RecordingExporter(), f.clock)
        keep(vm.state)
        vm.select(ExportFormat.PDF)
        f.entitlements.state.value = Entitlement(Tier.PLUS, Entitlement.PLUS_QUOTA_BYTES, NOW - 1)
        assertEquals(ExportFormat.ZIP, vm.state.value.format)
        assertTrue(vm.state.value.pdfLocked)
    }

    @Test
    fun exportEmitsSavedResultWithLocationChoice() = vmTest {
        val f = ShareFixture()
        val exporter = RecordingExporter()
        val id = setup(f)
        val vm = ExportViewModel(id, f.cases, f.captures, f.entitlements, exporter, f.clock)
        keep(vm.state)
        vm.setExactLocation(true)
        assertEquals(ExportUiState.EXACT_HINT, vm.state.value.locationHint)
        vm.effects.test {
            vm.export()
            assertEquals(ExportEvent.Exported(ExportResult.Saved("a.zip", "loc", 10, 2)), awaitItem())
        }
        assertEquals(listOf(ExportRequest(id, ExportFormat.ZIP, true)), exporter.requests)
        assertFalse(vm.state.value.exporting)
    }

    @Test
    fun exportOutcomesMapToMessages() = vmTest {
        val f = ShareFixture()
        val exporter = RecordingExporter(ExportResult.Unsupported)
        val vm = ExportViewModel(setup(f), f.cases, f.captures, f.entitlements, exporter, f.clock)
        keep(vm.state)
        vm.export()
        assertEquals(ExportViewModel.UNSUPPORTED, vm.state.value.error)
        exporter.result = ExportResult.NothingToExport
        vm.export()
        assertEquals(ExportViewModel.NOTHING, vm.state.value.error)
        exporter.failure = IllegalStateException("io")
        vm.export()
        assertEquals(ExportViewModel.FAILED, vm.state.value.error)
        assertFalse(vm.state.value.exporting)
    }

    @Test
    fun missingCaseCannotExport() = vmTest {
        val f = ShareFixture()
        val exporter = RecordingExporter()
        val vm = ExportViewModel(LocalCaseId(5), f.cases, f.captures, f.entitlements, exporter, f.clock)
        keep(vm.state)
        assertTrue(vm.state.value.missing)
        vm.export()
        assertTrue(exporter.requests.isEmpty())
    }

    @Test
    fun endToEndWithZipExporter() = vmTest {
        val f = ShareFixture()
        val id = setup(f)
        val sink = MemorySink()
        val vm = ExportViewModel(id, f.cases, f.captures, f.entitlements, ZipCaseExporter(f.cases, f.captures, sink, f.clock), f.clock)
        keep(vm.state)
        vm.effects.test {
            vm.export()
            val saved = assertIs<ExportEvent.Exported>(awaitItem()).result
            assertEquals(4, saved.items)
            assertEquals(sink.last!!.toByteArray().size.toLong(), saved.bytes)
        }
    }
}
