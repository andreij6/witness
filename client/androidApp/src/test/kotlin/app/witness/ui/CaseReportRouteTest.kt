package app.witness.ui

import app.witness.core.a2ui.TemplateCodec
import app.witness.core.a2ui.WitnessEvent
import app.witness.core.a2ui.ui.SummaryHeader
import app.witness.core.model.LocalCaseId
import app.witness.core.model.SummarySource
import app.witness.core.model.TemplateId
import app.witness.feature.cases.CaseReportHost
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class CaseReportRouteTest {
    private val calls = ArrayList<String>()
    private val host = CaseReportHost(
        LocalCaseId(1),
        TemplateId("property_damage"),
        onFlag = { calls += "flag:${it.target}" },
        onCaptureFor = { calls += "capture:$it" },
        summary = "Water came through the ceiling",
        onEdit = { calls += "edit:$it" },
    )

    @Test
    fun editEventsReachTheHostInsteadOfBeingDropped() {
        CaseReport.route(WitnessEvent.Edit("s", "summary"), host)
        CaseReport.route(WitnessEvent.Edit("s", "repair_notes"), host)
        CaseReport.route(WitnessEvent.CaptureFor("s", "entry_wide"), host)
        CaseReport.route(WitnessEvent.Remove("s", "x"), host)
        assertEquals(listOf("edit:summary", "edit:repair_notes", "capture:entry_wide"), calls)
    }

    @Test
    fun summaryIsWrittenIntoItsOwnSlotOnly() {
        val template = TemplateCodec.decode(java.io.File(BUNDLED, "property_damage.json").readText())
        val set = CaseReport.summaryUpdate(template, "Water came through the ceiling").getValue("updateDataModel").jsonObject
        assertEquals(JsonPrimitive(template.surfaceId), set["surfaceId"])
        assertEquals(JsonPrimitive("/slots/summary"), set["path"])
        assertEquals(JsonPrimitive("Water came through the ceiling"), set["value"])
        assertEquals(JsonNull, CaseReport.summaryUpdate(template, null).getValue("updateDataModel").jsonObject["value"])
    }

    @Test
    fun handWrittenAndEditedSummariesAreNeverShownAsAiDrafts() {
        val written = CaseReport.summaryHeader(host.copy(summarySource = SummarySource.WRITTEN))
        assertEquals(SummaryHeader("Summary · written by you", generated = false), written)
        val edited = CaseReport.summaryHeader(host.copy(summarySource = SummarySource.EDITED))
        assertEquals(SummaryHeader("Summary · edited by you", generated = false), edited)
        val legacy = CaseReport.summaryHeader(host.copy(summarySource = null))
        assertFalse(legacy.generated)
        listOf(written, edited, legacy).forEach { assertFalse("AI" in it.title("Draft summary") || "draft" in it.title("Draft summary").lowercase()) }
    }

    @Test
    fun generatedSummariesKeepTheTemplateTitleAndAiFrame() {
        val generated = CaseReport.summaryHeader(host.copy(summarySource = SummarySource.GENERATED))
        assertTrue(generated.generated)
        assertEquals("Draft summary", generated.title("Draft summary"))
        assertEquals("Room notes", generated.title("Room notes"))
    }

    @Test
    fun noSummaryIsNotAiLabelled() {
        val empty = CaseReport.summaryHeader(host.copy(summary = null, summarySource = SummarySource.GENERATED))
        assertEquals(SummaryHeader("Summary", generated = false), empty)
        assertEquals("Nothing written yet.", empty.placeholder())
    }

    @Test
    fun onlyTheSummarySlotTakesTheSummaryHeader() {
        val written = CaseReport.summaryHeader(host.copy(summarySource = SummarySource.WRITTEN))
        assertEquals(written, written.forTarget("summary"))
        listOf("affected_areas", "repair_notes", "room_notes").forEach {
            assertEquals(SummaryHeader(), written.forTarget(it))
        }
    }

    private companion object {
        const val BUNDLED = "../core/a2ui/src/commonMain/composeResources/files/templates"
    }
}
