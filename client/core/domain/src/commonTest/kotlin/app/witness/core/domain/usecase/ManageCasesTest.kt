package app.witness.core.domain.usecase

import app.cash.turbine.test
import app.witness.core.model.CaptureMode
import app.witness.core.model.LocalCaseId
import app.witness.core.model.RecordId
import app.witness.core.model.Suggestion
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ManageCasesTest {
    @Test
    fun createPicksTemplateByRulesAndActivates() = runTest {
        val g = TestGraph()
        val c = g.manageCases.create("  Fender bender  ")
        assertEquals("Fender bender", c.title)
        assertEquals(TemplateId.CAR_ACCIDENT, c.templateId)
        assertEquals(c.id, g.settings.value.activeCase)
        assertEquals(CaptureMode.CASE, g.settings.value.captureMode)
        assertEquals(1L, g.telemetry.events.last().second["template_known"])
    }

    @Test
    fun createWithoutActivationKeepsPostMode() = runTest {
        val g = TestGraph()
        g.manageCases.create("Birthday", activate = false)
        assertNull(g.settings.value.activeCase)
        assertEquals(CaptureMode.POST, g.settings.value.captureMode)
    }

    @Test
    fun explicitTemplateWins() = runTest {
        val g = TestGraph()
        assertEquals(TemplateId.RENTAL_CONDITION, g.manageCases.create("Car", TemplateId.RENTAL_CONDITION).templateId)
    }

    @Test
    fun assistantTemplateUsedOnlyWhenKnown() = runTest {
        val g = TestGraph()
        g.assistant.available = true
        g.assistant.answer = { _, _ -> Suggestion(TemplateId.PROPERTY_DAMAGE, emptyList()) }
        assertEquals(TemplateId.PROPERTY_DAMAGE, g.manageCases.suggestTemplate("Fender bender"))
        g.assistant.answer = { _, _ -> Suggestion(TemplateId("hallucinated"), emptyList()) }
        assertEquals(TemplateId.CAR_ACCIDENT, g.manageCases.suggestTemplate("Fender bender"))
        g.assistant.failure = IllegalStateException("x")
        assertEquals(TemplateId.CAR_ACCIDENT, g.manageCases.suggestTemplate("Fender bender"))
        g.assistant.failure = null
        g.settings.update { it.copy(aiAssistant = false) }
        g.assistant.answer = { _, _ -> Suggestion(TemplateId.PROPERTY_DAMAGE, emptyList()) }
        assertEquals(TemplateId.CAR_ACCIDENT, g.manageCases.suggestTemplate("Fender bender"))
    }

    @Test
    fun titlesAreValidated() = runTest {
        val g = TestGraph()
        assertFailsWith<CaseException> { g.manageCases.create("   ") }
        val c = g.manageCases.create("x".repeat(200))
        assertEquals(ManageCases.MAX_TITLE, c.title.length)
        assertFailsWith<CaseException> { g.manageCases.rename(c.id, "") }
        assertFailsWith<CaseException> { g.manageCases.rename(LocalCaseId(99), "ok") }
    }

    @Test
    fun renameTemplateAndSummaryBumpUpdatedAt() = runTest {
        val g = TestGraph()
        val c = g.manageCases.create("Leak")
        g.clock.advance(1_000)
        val r = g.manageCases.rename(c.id, "Kitchen leak")
        assertEquals("Kitchen leak", r.title)
        assertEquals(c.createdAtMs + 1_000, r.updatedAtMs)
        assertEquals(TemplateId.GENERAL, g.manageCases.changeTemplate(c.id, TemplateId.GENERAL).templateId)
        assertEquals("Water under sink", g.manageCases.updateSummary(c.id, " Water  under sink ").summary)
        assertNull(g.manageCases.updateSummary(c.id, "  ").summary)
    }

    @Test
    fun handWrittenSummaryIsWrittenNeverGenerated() = runTest {
        val g = TestGraph()
        val c = g.manageCases.create("Leak")
        assertNull(c.summarySource)
        val w = g.manageCases.updateSummary(c.id, "Water under the sink")
        assertEquals(SummarySource.WRITTEN, w.summarySource)
        assertEquals(SummarySource.WRITTEN, g.manageCases.updateSummary(c.id, "Water under the sink again").summarySource)
        assertEquals(SummarySource.WRITTEN, g.cases.get(c.id)!!.summarySource)
    }

    @Test
    fun editingAGeneratedDraftMarksItEdited() = runTest {
        val g = TestGraph()
        val c = g.manageCases.create("Crash")
        val drafted = g.manageCases.draftSummary(c.id, " Rear-ended at a light. ")
        assertEquals("Rear-ended at a light.", drafted.summary)
        assertEquals(SummarySource.GENERATED, drafted.summarySource)
        assertEquals(SummarySource.GENERATED, g.manageCases.updateSummary(c.id, "Rear-ended at a light.").summarySource)
        assertEquals(SummarySource.EDITED, g.manageCases.updateSummary(c.id, "Rear-ended at a red light.").summarySource)
        assertEquals(SummarySource.EDITED, g.manageCases.updateSummary(c.id, "Rear-ended at a red light on 5th.").summarySource)
        assertEquals(SummarySource.GENERATED, g.manageCases.draftSummary(c.id, "New draft").summarySource)
    }

    @Test
    fun clearingASummaryClearsItsProvenance() = runTest {
        val g = TestGraph()
        val c = g.manageCases.create("Crash")
        g.manageCases.draftSummary(c.id, "Draft")
        val cleared = g.manageCases.updateSummary(c.id, " ")
        assertNull(cleared.summary)
        assertNull(cleared.summarySource)
        assertNull(g.manageCases.draftSummary(c.id, "  ").summarySource)
        assertEquals(SummarySource.WRITTEN, g.manageCases.updateSummary(c.id, "Mine").summarySource)
        assertFailsWith<CaseException> { g.manageCases.draftSummary(LocalCaseId(99), "x") }
    }

    @Test
    fun moveAndDelete() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        val case = g.manageCases.create("Crash")
        val item = g.sealOne()
        assertEquals(case.id, item.caseId)
        g.manageCases.move(item.recordId, null)
        assertNull(g.captures.get(item.recordId)!!.caseId)
        g.manageCases.move(item.recordId, case.id)
        assertEquals(case.id, g.captures.get(item.recordId)!!.caseId)
        assertFailsWith<CaseException> { g.manageCases.move(item.recordId, LocalCaseId(42)) }
        assertFailsWith<CaseException> { g.manageCases.move(RecordId.fromBytes(ByteArray(10)), case.id) }
        g.manageCases.delete(case.id)
        assertNull(g.cases.get(case.id))
        assertNull(g.captures.get(item.recordId)!!.caseId)
        assertNull(g.settings.value.activeCase)
        assertEquals(CaptureMode.POST, g.settings.value.captureMode)
    }

    @Test
    fun setActiveRejectsUnknown() = runTest {
        val g = TestGraph()
        assertFailsWith<CaseException> { g.manageCases.setActive(LocalCaseId(5)) }
    }

    @Test
    fun observeSummarizesCounts() = runTest {
        val g = TestGraph()
        g.enrolledKey()
        g.manageCases.observe().test {
            assertEquals(emptyList(), awaitItem())
            val case = g.manageCases.create("Crash")
            assertEquals(0, awaitItem().single().itemCount)
            repeat(4) { g.sealOne() }
            val queued = expectMostRecentItem().single()
            assertEquals(4, queued.itemCount)
            assertEquals(4, queued.queuedCount)
            assertEquals(ManageCases.COVER_COUNT, queued.coverRecords.size)
            g.anchor()
            val final = expectMostRecentItem().single()
            assertEquals(case.id, final.case.id)
            assertEquals(4, final.anchoredCount)
            assertEquals(0, final.queuedCount)
            cancelAndIgnoreRemainingEvents()
        }
    }

    @Test
    fun summarizeSortsByRecencyAndIgnoresUnfiled() {
        val a = app.witness.core.model.Case(LocalCaseId(1), "a", TemplateId.GENERAL, 0, 10)
        val b = app.witness.core.model.Case(LocalCaseId(2), "b", TemplateId.GENERAL, 0, 20)
        val s = ManageCases.summarize(listOf(a, b), emptyList())
        assertEquals(listOf(LocalCaseId(2), LocalCaseId(1)), s.map { it.case.id })
        assertTrue(s.all { it.itemCount == 0 && it.coverRecords.isEmpty() })
    }
}
