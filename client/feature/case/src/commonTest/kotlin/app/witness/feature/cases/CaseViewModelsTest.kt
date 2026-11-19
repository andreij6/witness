package app.witness.feature.cases

import app.witness.core.domain.usecase.CaseException
import app.witness.core.model.AnchoringStatus
import app.witness.core.model.CaptureMode
import app.witness.core.model.CaseId
import app.witness.core.model.CaseShare
import app.witness.core.model.LocalCaseId
import app.witness.core.model.Suggestion
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CasesViewModelTest {
    @Test
    fun startsLoadingThenShowsCards() = vmTest {
        val f = Fixture()
        val vm = CasesViewModel(f.cases, f.captures, f.clock, UtcOffset.UTC)
        assertTrue(vm.state.value.loading)
        val case = f.cases.create("Fender bender", TemplateId.CAR_ACCIDENT, NOW - HOUR_MS)
        f.add(capture(1, case.id, place = "Oak St & 5th Ave"), capture(2, case.id, AnchoringStatus.QUEUED))
        keep(vm.state)
        val card = vm.state.value.cases.single()
        assertEquals("Today · Oak St & 5th Ave · 2 items", card.meta)
        assertEquals(1, card.anchored)
        assertEquals(1, card.queued)
        assertEquals(listOf(record(1), record(2)), card.covers)
        assertFalse(vm.state.value.isEmpty)
    }

    @Test
    fun emptyStoreIsEmpty() = vmTest {
        val f = Fixture()
        val vm = CasesViewModel(f.cases, f.captures, f.clock, UtcOffset.UTC)
        keep(vm.state)
        assertTrue(vm.state.value.isEmpty)
    }

    @Test
    fun filterSelectsSharedCases() = vmTest {
        val f = Fixture()
        val open = f.cases.create("Open", TemplateId.GENERAL, NOW - 2 * HOUR_MS)
        val shared = f.cases.create("Shared", TemplateId.GENERAL, NOW - HOUR_MS)
        f.cases.update(shared.copy(shares = listOf(CaseShare(CaseId.of(1, ByteArray(15)), "u", NOW, null, 1))))
        val vm = CasesViewModel(f.cases, f.captures, f.clock, UtcOffset.UTC)
        keep(vm.state)
        assertEquals(2, vm.state.value.cases.size)
        vm.setFilter(CaseFilter.SHARED)
        assertEquals(listOf(shared.id), vm.state.value.cases.map { it.id })
        vm.setFilter(CaseFilter.OPEN)
        assertEquals(listOf(open.id), vm.state.value.cases.map { it.id })
        assertEquals(CaseFilter.OPEN, vm.state.value.filter)
    }

    @Test
    fun newestUpdatedCaseComesFirst() = vmTest {
        val f = Fixture()
        f.cases.create("Older", TemplateId.GENERAL, NOW - 5 * HOUR_MS)
        f.cases.create("Newer", TemplateId.GENERAL, NOW - HOUR_MS)
        val vm = CasesViewModel(f.cases, f.captures, f.clock, UtcOffset.UTC)
        keep(vm.state)
        assertEquals(listOf("Newer", "Older"), vm.state.value.cases.map { it.title })
    }
}

class CaseReportViewModelTest {
    private suspend fun setup(f: Fixture): LocalCaseId {
        val case = f.cases.create("Fender bender", TemplateId.CAR_ACCIDENT, NOW - 60_000)
        f.add(
            capture(4, case.id, createdAtMs = NOW - 4_000, place = "Oak St & 5th Ave"),
            capture(1, case.id, createdAtMs = NOW - 1_000, status = AnchoringStatus.QUEUED),
            capture(3, case.id, createdAtMs = NOW - 3_000),
            capture(2, case.id, createdAtMs = NOW - 2_000),
            capture(9, null),
        )
        return case.id
    }

    private fun vm(f: Fixture, id: LocalCaseId) =
        CaseReportViewModel(id, f.cases, f.captures, f.manage, f.catalog, f.feedback, f.clock, UtcOffset.UTC)

    @Test
    fun buildsDesignState() = vmTest {
        val f = Fixture()
        val id = setup(f)
        val vm = vm(f, id)
        keep(vm.state)
        val s = vm.state.value
        assertEquals("Fender bender", s.title)
        assertEquals("Oak St & 5th Ave · Today 2:31 PM", s.subtitle)
        assertEquals("Report built on-device · Car accident", s.reportLabel)
        assertEquals("Evidence · 4 sealed", s.evidenceTitle)
        assertEquals("3 anchored · 1 queued", s.anchoringLine)
        assertEquals(listOf(record(4), record(3), record(2), record(1)), s.items.map { it.recordId })
        assertFalse(s.loading)
        assertFalse(s.missing)
    }

    @Test
    fun unknownCaseIsMissing() = vmTest {
        val f = Fixture()
        val vm = vm(f, LocalCaseId(42))
        keep(vm.state)
        assertTrue(vm.state.value.missing)
    }

    @Test
    fun reportTypesSheetCarriesSelectionAndSuggestion() = vmTest {
        val f = Fixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.openReportTypes()
        val sheet = assertIs<CaseSheet.ReportTypes>(vm.state.value.sheet)
        assertEquals(TemplateId.CAR_ACCIDENT, sheet.selected)
        assertEquals(TemplateId.CAR_ACCIDENT, sheet.suggested)
        assertEquals(ReportType.entries.toList(), sheet.options)
    }

    @Test
    fun suggestionFailureStillOpensSheet() = vmTest {
        val f = Fixture()
        f.assistant.available = true
        f.assistant.failure = IllegalStateException("model busy")
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.openReportTypes()
        assertIs<CaseSheet.ReportTypes>(vm.state.value.sheet)
    }

    @Test
    fun choosingTypeChangesTemplateAndCloses() = vmTest {
        val f = Fixture()
        val id = setup(f)
        val vm = vm(f, id)
        keep(vm.state)
        vm.openReportTypes()
        vm.chooseReportType(ReportType.PROPERTY_DAMAGE)
        assertEquals(TemplateId.PROPERTY_DAMAGE, f.cases.get(id)!!.templateId)
        assertEquals("Report built on-device · Property damage", vm.state.value.reportLabel)
        assertNull(vm.state.value.sheet)
    }

    @Test
    fun choosingCurrentTypeOnlyCloses() = vmTest {
        val f = Fixture()
        val id = setup(f)
        val before = f.cases.get(id)!!.updatedAtMs
        val vm = vm(f, id)
        keep(vm.state)
        vm.openReportTypes()
        f.clock.advance(1_000)
        vm.chooseReportType(ReportType.CAR_ACCIDENT)
        assertEquals(before, f.cases.get(id)!!.updatedAtMs)
        assertNull(vm.state.value.sheet)
    }

    @Test
    fun unavailableTypeIsIgnored() = vmTest {
        val f = Fixture()
        val id = setup(f)
        val vm = vm(f, id)
        keep(vm.state)
        vm.openReportTypes()
        vm.chooseReportType(ReportType.WORKPLACE_INCIDENT)
        assertEquals(TemplateId.CAR_ACCIDENT, f.cases.get(id)!!.templateId)
        assertIs<CaseSheet.ReportTypes>(vm.state.value.sheet)
    }

    @Test
    fun flagDefaultsToWrongAndSendsCleanNote() = vmTest {
        val f = Fixture()
        val id = setup(f)
        val vm = vm(f, id)
        keep(vm.state)
        val output = GeneratedOutput("impact", "Impact point", "Rear, driver side")
        vm.openFlag(output)
        assertEquals(FlagReason.WRONG, assertIs<CaseSheet.Flag>(vm.state.value.sheet).reason)
        vm.setFlagReason(FlagReason.MISLEADING)
        vm.setFlagNote("  it was the front  ")
        vm.sendFlag()
        assertEquals(listOf(AiFlag(id, "impact", FlagReason.MISLEADING, "it was the front", NOW)), f.feedback.flags)
        assertNull(vm.state.value.sheet)
    }

    @Test
    fun blankNoteIsSentAsNull() = vmTest {
        val f = Fixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.openFlag(GeneratedOutput("t", "L", "V"))
        vm.setFlagNote("   ")
        vm.sendFlag()
        assertNull(f.feedback.flags.single().note)
    }

    @Test
    fun noteIsCappedOnInput() = vmTest {
        val f = Fixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.openFlag(GeneratedOutput("t", "L", "V"))
        vm.setFlagNote("x".repeat(10_000))
        assertEquals(CaseReportViewModel.MAX_NOTE, assertIs<CaseSheet.Flag>(vm.state.value.sheet).note.length)
    }

    @Test
    fun flagActionsWithoutFlagSheetAreNoOps() = vmTest {
        val f = Fixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.setFlagReason(FlagReason.OTHER)
        vm.setFlagNote("x")
        vm.sendFlag()
        vm.removeFlagged()
        assertTrue(f.feedback.flags.isEmpty())
        assertTrue(f.feedback.removals.isEmpty())
        assertNull(vm.state.value.sheet)
    }

    @Test
    fun removingSummaryClearsIt() = vmTest {
        val f = Fixture()
        val id = setup(f)
        f.manage.updateSummary(id, "A silver sedan struck my car.")
        val vm = vm(f, id)
        keep(vm.state)
        assertEquals("A silver sedan struck my car.", vm.state.value.summary)
        vm.openFlag(GeneratedOutput(CaseReportViewModel.SUMMARY_TARGET, SUMMARY_LABEL, "A silver sedan struck my car."))
        vm.removeFlagged()
        assertEquals(listOf(id to CaseReportViewModel.SUMMARY_TARGET), f.feedback.removals)
        assertNull(vm.state.value.summary)
    }

    @Test
    fun removingOtherTargetKeepsSummary() = vmTest {
        val f = Fixture()
        val id = setup(f)
        f.manage.updateSummary(id, "Keep me")
        val vm = vm(f, id)
        keep(vm.state)
        vm.openFlag(GeneratedOutput("impact", "Impact point", "Rear"))
        vm.removeFlagged()
        assertEquals("Keep me", vm.state.value.summary)
        assertEquals(listOf(id to "impact"), f.feedback.removals)
    }

    @Test
    fun feedbackFailureSurfacesErrorAndReleasesSheet() = vmTest {
        val f = Fixture()
        val failing = object : AiFeedback {
            override suspend fun flag(flag: AiFlag) = throw IllegalStateException("disk full")
            override suspend fun remove(caseId: LocalCaseId, target: String) = Unit
        }
        val vm = CaseReportViewModel(setup(f), f.cases, f.captures, f.manage, f.catalog, failing, f.clock, UtcOffset.UTC)
        keep(vm.state)
        vm.openFlag(GeneratedOutput("t", "L", "V"))
        vm.sendFlag()
        assertEquals("disk full", vm.state.value.error)
        assertFalse(assertIs<CaseSheet.Flag>(vm.state.value.sheet).busy)
        vm.clearError()
        assertNull(vm.state.value.error)
    }

    @Test
    fun summaryEditAndSave() = vmTest {
        val f = Fixture()
        val id = setup(f)
        val vm = vm(f, id)
        keep(vm.state)
        vm.editSummary()
        assertEquals("", assertIs<CaseSheet.Summary>(vm.state.value.sheet).text)
        vm.setSummaryDraft("  Rear-ended at the stop sign.  ")
        vm.saveSummary()
        assertEquals("Rear-ended at the stop sign.", f.cases.get(id)!!.summary)
        assertNull(vm.state.value.sheet)
    }

    @Test
    fun savedSummaryIsLabelledWrittenNotAiDrafted() = vmTest {
        val f = Fixture()
        val id = setup(f)
        val vm = vm(f, id)
        keep(vm.state)
        assertNull(vm.state.value.summarySource)
        vm.editSummary()
        vm.setSummaryDraft("Rear-ended at the stop sign.")
        vm.saveSummary()
        assertEquals(SummarySource.WRITTEN, vm.state.value.summarySource)
        assertEquals(SummaryCopy.WRITTEN, SummaryCopy.label(vm.state.value.summarySource))
        assertFalse(SummaryCopy.isGenerated(vm.state.value.summarySource))
    }

    @Test
    fun editingAGeneratedSummaryRelabelsItEdited() = vmTest {
        val f = Fixture()
        val id = setup(f)
        f.manage.draftSummary(id, "A silver sedan struck my car.")
        val vm = vm(f, id)
        keep(vm.state)
        assertEquals(SummarySource.GENERATED, vm.state.value.summarySource)
        assertEquals(SummaryCopy.DRAFT, SummaryCopy.label(vm.state.value.summarySource))
        vm.editSummary()
        assertEquals("A silver sedan struck my car.", assertIs<CaseSheet.Summary>(vm.state.value.sheet).text)
        vm.saveSummary()
        assertEquals(SummarySource.GENERATED, vm.state.value.summarySource)
        vm.editSummary()
        vm.setSummaryDraft("A silver sedan struck my car at the light.")
        vm.saveSummary()
        assertEquals(SummarySource.EDITED, vm.state.value.summarySource)
        assertEquals(SummaryCopy.EDITED, SummaryCopy.label(vm.state.value.summarySource))
    }

    @Test
    fun summaryLabelsNeverCallUserTextAi() {
        assertEquals(SummaryCopy.DRAFT, SummaryCopy.label(SummarySource.GENERATED))
        listOf(SummarySource.EDITED, SummarySource.WRITTEN, null).forEach { source ->
            val label = SummaryCopy.label(source)
            assertFalse("AI" in label || "draft" in label.lowercase(), label)
            assertFalse(SummaryCopy.isGenerated(source))
        }
    }

    @Test
    fun clearedSummaryHasNoProvenance() = vmTest {
        val f = Fixture()
        val id = setup(f)
        f.manage.draftSummary(id, "Draft")
        val vm = vm(f, id)
        keep(vm.state)
        vm.editSummary()
        vm.setSummaryDraft("  ")
        vm.saveSummary()
        assertNull(vm.state.value.summary)
        assertNull(vm.state.value.summarySource)
    }

    @Test
    fun summaryDraftIsCapped() = vmTest {
        val f = Fixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.editSummary()
        vm.setSummaryDraft("y".repeat(10_000))
        assertEquals(4_000, assertIs<CaseSheet.Summary>(vm.state.value.sheet).text.length)
    }

    @Test
    fun deleteRemovesCaseAndMarksMissing() = vmTest {
        val f = Fixture()
        val id = setup(f)
        val vm = vm(f, id)
        keep(vm.state)
        vm.requestDelete()
        assertEquals(CaseSheet.ConfirmDelete, vm.state.value.sheet)
        vm.confirmDelete()
        assertNull(f.cases.get(id))
        assertTrue(vm.state.value.missing)
        assertTrue(f.captures.snapshot.values.none { it.caseId == id })
    }

    @Test
    fun dismissClosesAnySheet() = vmTest {
        val f = Fixture()
        val vm = vm(f, setup(f))
        keep(vm.state)
        vm.requestDelete()
        vm.dismissSheet()
        assertNull(vm.state.value.sheet)
    }

    @Test
    fun evidenceOfFiltersAndSortsOldestFirst() {
        val id = LocalCaseId(7)
        val list = listOf(capture(1, id, createdAtMs = 3), capture(2, null, createdAtMs = 1), capture(3, id, createdAtMs = 2))
        assertEquals(listOf(record(3), record(1)), CaseReportViewModel.evidenceOf(list, id).map { it.recordId })
    }
}

class CaseSwitcherViewModelTest {
    @Test
    fun listsCasesWithActiveSelected() = vmTest {
        val f = Fixture()
        val a = f.cases.create("Fender bender", TemplateId.CAR_ACCIDENT, NOW)
        f.cases.create("Kitchen ceiling leak", TemplateId.PROPERTY_DAMAGE, NOW - HOUR_MS)
        f.add(capture(1, a.id), capture(2, a.id))
        f.manage.setActive(a.id)
        val vm = CaseSwitcherViewModel(f.cases, f.captures, f.settings, f.manage)
        keep(vm.state)
        val options = vm.state.value.options
        assertEquals(listOf("Fender bender", "Kitchen ceiling leak"), options.map { it.title })
        assertEquals(listOf("Car accident · 2 items", "Property damage · 0 items"), options.map { it.subtitle })
        assertEquals(listOf(true, false), options.map { it.selected })
    }

    @Test
    fun selectActivatesCaseModeAndFinishes() = vmTest {
        val f = Fixture()
        f.cases.create("A", TemplateId.GENERAL, NOW)
        val b = f.cases.create("B", TemplateId.GENERAL, NOW - 1)
        val vm = CaseSwitcherViewModel(f.cases, f.captures, f.settings, f.manage)
        keep(vm.state)
        vm.select(b.id)
        assertEquals(b.id, f.settings.value.activeCase)
        assertEquals(CaptureMode.CASE, f.settings.value.captureMode)
        assertTrue(vm.state.value.done)
        assertTrue(vm.state.value.options.single { it.id == b.id }.selected)
    }

    @Test
    fun unknownCaseReportsError() = vmTest {
        val f = Fixture()
        val vm = CaseSwitcherViewModel(f.cases, f.captures, f.settings, f.manage)
        keep(vm.state)
        vm.select(LocalCaseId(99))
        assertEquals("unknown case", vm.state.value.error)
        assertFalse(vm.state.value.done)
    }
}

class NewCaseViewModelTest {
    @Test
    fun nameSuggestsSituation() = vmTest {
        val f = Fixture()
        val vm = NewCaseViewModel(f.manage, f.catalog)
        vm.setName("Fender bender — Oak & 5th")
        assertEquals(ReportType.CAR_ACCIDENT, vm.state.value.selected)
        assertFalse(vm.state.value.manual)
        vm.setName("Kitchen ceiling leak")
        assertEquals(ReportType.PROPERTY_DAMAGE, vm.state.value.selected)
    }

    @Test
    fun unmatchedNameSuggestsOther() = vmTest {
        val vm = NewCaseViewModel(Fixture().manage, Fixture().catalog)
        vm.setName("Something odd")
        assertEquals(ReportType.GENERAL, vm.state.value.selected)
    }

    @Test
    fun blankNameHasNoSelectionAndCannotCreate() = vmTest {
        val vm = NewCaseViewModel(Fixture().manage, Fixture().catalog)
        vm.setName("   ")
        assertNull(vm.state.value.selected)
        assertFalse(vm.state.value.canCreate)
    }

    @Test
    fun manualPickSurvivesRenaming() = vmTest {
        val vm = NewCaseViewModel(Fixture().manage, Fixture().catalog)
        vm.select(ReportType.RENTAL_CONDITION)
        vm.setName("Fender bender")
        assertEquals(ReportType.RENTAL_CONDITION, vm.state.value.selected)
        assertTrue(vm.state.value.manual)
    }

    @Test
    fun unavailablePickIgnored() = vmTest {
        val vm = NewCaseViewModel(Fixture().manage, Fixture().catalog)
        vm.select(ReportType.WORKPLACE_INCIDENT)
        assertNull(vm.state.value.selected)
    }

    @Test
    fun nameIsCappedAtMaxTitle() = vmTest {
        val vm = NewCaseViewModel(Fixture().manage, Fixture().catalog)
        vm.setName("a".repeat(500))
        assertEquals(80, vm.state.value.name.length)
    }

    @Test
    fun createWithManualTemplateActivatesCase() = vmTest {
        val f = Fixture()
        val vm = NewCaseViewModel(f.manage, f.catalog)
        vm.setName("Move-out walkthrough")
        vm.select(ReportType.RENTAL_CONDITION)
        vm.create()
        val id = vm.state.value.created!!
        val case = f.cases.get(id)!!
        assertEquals(TemplateId.RENTAL_CONDITION, case.templateId)
        assertEquals(id, f.settings.value.activeCase)
        assertFalse(vm.state.value.canCreate)
    }

    @Test
    fun createWithoutManualPickLetsAssistantDecide() = vmTest {
        val f = Fixture()
        f.assistant.available = true
        f.assistant.answer = { _, _ -> Suggestion(TemplateId.PROPERTY_DAMAGE, emptyList()) }
        val vm = NewCaseViewModel(f.manage, f.catalog)
        vm.setName("Fender bender")
        vm.create()
        assertEquals(TemplateId.PROPERTY_DAMAGE, f.cases.get(vm.state.value.created!!)!!.templateId)
    }

    @Test
    fun createIgnoredWhenBlank() = vmTest {
        val f = Fixture()
        val vm = NewCaseViewModel(f.manage, f.catalog)
        vm.create()
        assertNull(vm.state.value.created)
    }

    @Test
    fun createDoesNotActivateWhenAsked() = vmTest {
        val f = Fixture()
        val vm = NewCaseViewModel(f.manage, f.catalog, activate = false)
        vm.setName("Leak")
        vm.create()
        assertNull(f.settings.value.activeCase)
    }

    @Test
    fun catalogFailureFallsBackToOther() = vmTest {
        val f = Fixture()
        val vm = NewCaseViewModel(f.manage, { throw CaseException("offline") })
        vm.setName("Fender bender")
        assertEquals(ReportType.GENERAL, vm.state.value.selected)
    }

    @Test
    fun suggestIsPure() {
        assertNull(NewCaseViewModel.suggest("", emptyList()))
        assertEquals(ReportType.GENERAL, NewCaseViewModel.suggest("x", emptyList()))
    }
}

class CaseCameraViewModelTest {
    private suspend fun active(f: Fixture, captured: Int = 2): LocalCaseId {
        val case = f.cases.create("Fender bender", TemplateId.CAR_ACCIDENT, NOW)
        repeat(captured) { f.add(capture(it + 1, case.id)) }
        f.manage.setActive(case.id)
        return case.id
    }

    private fun vm(f: Fixture) = CaseCameraViewModel(f.settings, f.cases, f.captures, f.feedback, f.clock)

    @Test
    fun hiddenInPostMode() = vmTest {
        val f = Fixture()
        val vm = vm(f)
        keep(vm.state)
        assertFalse(vm.state.value.visible)
        assertEquals(CaseCameraUiState.NO_CASE, vm.state.value.pillLabel)
    }

    @Test
    fun showsActiveCaseAndCoach() = vmTest {
        val f = Fixture()
        val id = active(f)
        val vm = vm(f)
        keep(vm.state)
        val s = vm.state.value
        assertTrue(s.visible)
        assertEquals(id, s.caseId)
        assertEquals("Case: Fender bender", s.pillLabel)
        assertEquals("3 of 5", s.coach!!.progressLabel)
    }

    @Test
    fun skipAdvancesCoach() = vmTest {
        val f = Fixture()
        active(f)
        val vm = vm(f)
        keep(vm.state)
        vm.skip()
        assertEquals(4, vm.state.value.coach!!.step)
    }

    @Test
    fun newCaptureAdvancesCoach() = vmTest {
        val f = Fixture()
        val id = active(f)
        val vm = vm(f)
        keep(vm.state)
        f.add(capture(10, id))
        assertEquals(4, vm.state.value.coach!!.step)
    }

    @Test
    fun importsDoNotAdvanceCoach() = vmTest {
        val f = Fixture()
        val id = active(f)
        val vm = vm(f)
        keep(vm.state)
        f.add(capture(10, id, origin = app.witness.core.model.CaptureOrigin.IMPORT))
        assertEquals(3, vm.state.value.coach!!.step)
    }

    @Test
    fun flagRecordsAndSkips() = vmTest {
        val f = Fixture()
        val id = active(f)
        val vm = vm(f)
        keep(vm.state)
        vm.flagCoach()
        assertEquals(listOf(AiFlag(id, CaseCameraViewModel.COACH_TARGET_PREFIX + 3, FlagReason.WRONG, null, NOW)), f.feedback.flags)
        assertEquals(4, vm.state.value.coach!!.step)
    }

    @Test
    fun coachDisappearsWhenDoneAndSkipIsNoOp() = vmTest {
        val f = Fixture()
        active(f, captured = 5)
        val vm = vm(f)
        keep(vm.state)
        assertNull(vm.state.value.coach)
        vm.skip()
        vm.flagCoach()
        assertTrue(f.feedback.flags.isEmpty())
    }

    @Test
    fun skipsArePerCase() = vmTest {
        val f = Fixture()
        active(f)
        val vm = vm(f)
        keep(vm.state)
        vm.skip()
        val other = f.cases.create("Leak", TemplateId.PROPERTY_DAMAGE, NOW)
        f.manage.setActive(other.id)
        assertEquals(1, vm.state.value.coach!!.step)
    }

    @Test
    fun buildWithMissingActiveCaseShowsChooser() {
        val s = CaseCameraViewModel.build(app.witness.core.model.Settings(captureMode = CaptureMode.CASE, activeCase = LocalCaseId(5)), emptyList(), emptyList(), emptyMap())
        assertTrue(s.visible)
        assertNull(s.caseId)
        assertNull(s.coach)
    }

    @Test
    fun onlyTheSummaryTargetOpensTheSummaryEditor() {
        var opened = 0
        val actions = CaseReportActions(
            {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {},
            { opened++ }, {}, {}, {}, {}, {}, {},
        )
        val edit = editor(actions)
        edit("repair_notes")
        assertEquals(0, opened)
        edit(CaseReportViewModel.SUMMARY_TARGET)
        assertEquals(1, opened)
    }
}
