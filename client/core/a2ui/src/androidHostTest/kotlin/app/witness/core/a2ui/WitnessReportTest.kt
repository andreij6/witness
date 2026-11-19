package app.witness.core.a2ui

import app.witness.core.model.Slot
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import androidx.a2ui.engine.model.A2uiCoreSurfaceModel
import androidx.a2ui.model.protocol.A2uiClientErrorMessage
import androidx.a2ui.model.protocol.A2uiEventAction
import androidx.a2ui.model.protocol.A2uiFunctionCallAction
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

@OptIn(ExperimentalCoroutinesApi::class)
class WitnessReportTest {
    private val events = ArrayList<WitnessEvent>()
    private val report = WitnessReport(WitnessEventSink { events += it })

    private fun TestScope.started(): MutableList<A2uiClientErrorMessage> {
        val errors = ArrayList<A2uiClientErrorMessage>()
        backgroundScope.launch { report.run() }
        backgroundScope.launch { report.errors.collect { errors += it } }
        runCurrent()
        return errors
    }

    private val caseSuggestion = Suggestion(
        TemplateId("car_accident"),
        listOf(
            Slot("impact_point", choice = "rear_driver"),
            Slot("other_plate", text = "7mnv 562"),
            Slot("injured", choice = "No"),
            Slot("summary", text = "No injuries."),
            Slot("ev_wide_shot", flag = true),
            Slot("ev_insurance_card", flag = false),
            Slot("injected", text = "drop me"),
        ),
    )

    @Test
    fun everyBundledTemplateRendersIntoTheEngineWithoutErrors() = runTest {
        val errors = started()
        BundledTemplates.store().all.forEach { template -> report.show(template) }
        runCurrent()
        assertEquals(emptyList(), errors)
        assertEquals(setOf("report"), report.processor.activeSurfaces.value.map { it.id }.toSet())
    }

    @Test
    fun showingAnotherTemplateReplacesTheSurfaceAndItsData() = runTest {
        val errors = started()
        val store = BundledTemplates.store()
        report.show(store["car_accident"]!!, store.bind(caseSuggestion))
        runCurrent()
        report.show(store["property_damage"]!!)
        runCurrent()
        assertEquals(emptyList(), errors)
        assertEquals(1, report.processor.activeSurfaces.value.size)
        assertEquals(emptyMap(), report.slots("report"))
    }

    @Test
    fun bindingLandsInTheSurfaceDataModel() = runTest {
        val errors = started()
        val store = BundledTemplates.store()
        val binding = store.bind(caseSuggestion)!!
        report.show(binding.template, binding)
        runCurrent()
        assertEquals(emptyList(), errors)
        val slots = report.slots("report")
        assertEquals("rear_driver", slots["impact_point"])
        assertEquals("7MNV562", slots["other_plate"])
        assertEquals("no", slots["injured"])
        assertEquals(true, slots["ev_wide_shot"])
        assertEquals(false, slots["ev_insurance_card"])
        assertNull(slots["injected"])
        assertEquals(listOf(DroppedSlot("injected", DropReason.UNKNOWN_ID)), binding.dropped)
    }

    @Test
    fun singleSlotEditsAndRemovalsApply() = runTest {
        started()
        val binding = BundledTemplates.store().bind(caseSuggestion)!!
        report.show(binding.template, binding)
        report.send(A2uiMessages.updateDataModel("report", A2uiMessages.slotPath("summary"), JsonPrimitive("Edited.")))
        report.send(A2uiMessages.updateDataModel("report", A2uiMessages.slotPath("impact_point"), JsonNull))
        runCurrent()
        val slots = report.slots("report")
        assertEquals("Edited.", slots["summary"])
        assertTrue("impact_point" !in slots)
    }

    @Test
    fun rebindingReplacesAllSlots() = runTest {
        started()
        val store = BundledTemplates.store()
        report.show(store["car_accident"]!!, store.bind(caseSuggestion))
        report.send(store.bind(Suggestion(TemplateId("car_accident"), listOf(Slot("injured", choice = "yes"))))!!.message)
        runCurrent()
        assertEquals(mapOf<String, Any?>("injured" to "yes"), report.slots("report"))
    }

    @Test
    fun deleteSurfaceRemovesIt() = runTest {
        started()
        report.show(BundledTemplates.store()["rental_condition"]!!)
        runCurrent()
        report.send(A2uiMessages.deleteSurface("report"))
        runCurrent()
        assertTrue(report.processor.activeSurfaces.value.isEmpty())
        assertEquals(emptyMap(), report.slots("report"))
    }

    @Test
    fun surfaceDispatchedEventsReachTheSink() = runTest {
        started()
        report.show(BundledTemplates.store()["car_accident"]!!)
        runCurrent()
        val surface = report.processor.activeSurfaces.value.single() as A2uiCoreSurfaceModel
        surface.dispatchAction("summary", mapOf("event" to mapOf("name" to "edit", "context" to mapOf("target" to "summary"))))
        surface.dispatchAction("x", mapOf("event" to mapOf("name" to "open_url", "context" to mapOf("target" to "x"))))
        runCurrent()
        assertEquals(listOf<WitnessEvent>(WitnessEvent.Edit("report", "summary")), events)
    }

    @Test
    fun interceptorConsumesEveryAction() = runTest {
        val interceptor = WitnessActionInterceptor(WitnessEventSink { events += it })
        assertNull(interceptor.onInterceptAction(A2uiEventAction("s", "c", 0, "flag", mapOf("target" to "impact_point"))))
        assertNull(interceptor.onInterceptAction(A2uiEventAction("s", "c", 0, "submit", mapOf("target" to "x"))))
        assertNull(interceptor.onInterceptAction(A2uiFunctionCallAction("s", "c", 0, "openUrl", emptyMap())))
        assertEquals(listOf<WitnessEvent>(WitnessEvent.Flag("s", "impact_point")), events)
    }

    @Test
    fun slotsOfUnknownSurfaceIsEmpty() = assertEquals(emptyMap(), report.slots("nope"))
}
