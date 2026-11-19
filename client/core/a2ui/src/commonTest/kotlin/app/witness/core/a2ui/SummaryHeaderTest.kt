package app.witness.core.a2ui

import app.witness.core.a2ui.ui.SummaryHeader
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame

class SummaryHeaderTest {
    private val written = SummaryHeader("Summary · written by you", generated = false)

    @Test
    fun theSummarySlotKeepsItsProvenanceHeader() = assertSame(written, written.forTarget("summary"))

    @Test
    fun otherTextSlotsKeepTheirTemplateTitle() {
        listOf("affected_areas", "repair_notes", "room_notes", "Summary", "summary ", "", null).forEach {
            assertEquals(SummaryHeader(), written.forTarget(it))
            assertEquals("Repair notes", written.forTarget(it).title("Repair notes"))
        }
    }

    @Test
    fun theDefaultHeaderIsStableForEveryTarget() {
        listOf("summary", "repair_notes", null).forEach { assertEquals(SummaryHeader(), SummaryHeader().forTarget(it)) }
    }

    @Test
    fun targetIsReadFromAnActionPayload() {
        val action = mapOf("event" to mapOf("name" to "edit", "context" to mapOf("target" to "repair_notes")))
        assertEquals("repair_notes", WitnessEventSink.targetOf(action))
    }

    @Test
    fun malformedActionPayloadsHaveNoTarget() {
        listOf(
            null,
            emptyMap(),
            mapOf("event" to "edit"),
            mapOf("event" to mapOf("name" to "edit")),
            mapOf("event" to mapOf("context" to listOf("summary"))),
            mapOf("event" to mapOf("context" to mapOf("target" to 7))),
            mapOf("context" to mapOf("target" to "summary")),
        ).forEach { assertEquals(null, WitnessEventSink.targetOf(it)) }
    }
}
