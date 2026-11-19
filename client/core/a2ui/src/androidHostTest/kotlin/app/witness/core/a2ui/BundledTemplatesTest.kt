package app.witness.core.a2ui

import app.witness.core.model.Slot
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.jsonPrimitive
import app.witness.core.domain.usecase.TemplateMatcher
import kotlin.test.Test
import kotlin.test.assertNotNull
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class BundledTemplatesTest {
    @Test
    fun bundledTemplatesDecode() = runTest {
        val store = BundledTemplates.store()
        assertEquals(TemplateStore.bundledIds, store.all.map { it.id })
        assertEquals(listOf("Car accident", "Property damage", "Rental condition", "Other"), store.all.map { it.title })
    }

    @Test
    fun carAccidentMatchesCaseScreen() = runTest {
        val car = BundledTemplates.store()["car_accident"]!!
        val expected = listOf(
            "impact_point", "impact_source", "other_plate", "plate_source", "injured", "summary",
            "ev_damage_closeup", "ev_both_plates", "ev_wide_shot", "ev_insurance_card",
        )
        assertEquals(expected, car.slots.map { it.id })
        assertEquals(listOf("no", "yes", "unsure"), car.slot("injured")?.choices)
        assertTrue("rear_driver" in car.slot("impact_point")!!.choices)
        val text = car.messages.toString()
        listOf("Still recommended", "Accident details", "Generated form", "Impact point", "Other driver's plate",
            "Anyone injured?", "Draft summary", "Other driver's insurance card", "Not sure").forEach {
            assertTrue(it in text, it)
        }
    }

    @Test
    fun caseScreenSuggestionBindsCleanly() = runTest {
        val store = BundledTemplates.store()
        val binding = store.bind(
            Suggestion(
                TemplateId("car_accident"),
                listOf(
                    Slot("impact_point", choice = "rear_driver"),
                    Slot("impact_source", text = "Detected from photo 2"),
                    Slot("other_plate", text = "7MNV562"),
                    Slot("plate_source", text = "Read from photo 3"),
                    Slot("injured", choice = "no"),
                    Slot("summary", text = "At 2:31 PM a silver sedan struck the rear of my stopped hatchback at the Oak St stop sign. No injuries."),
                    Slot("ev_damage_closeup", flag = true),
                    Slot("ev_both_plates", flag = true),
                    Slot("ev_wide_shot", flag = true),
                    Slot("ev_insurance_card", flag = false),
                ),
            ),
        )!!
        assertTrue(binding.dropped.isEmpty())
        assertTrue(binding.missing.isEmpty())
        assertEquals("7MNV562", binding.values["other_plate"]?.jsonPrimitive?.content)
    }

    @Test
    fun keywordMatcherPicksTheRightTemplate() = runTest {
        val schemas = BundledTemplates.store().templates()
        assertEquals(TemplateId("car_accident"), TemplateMatcher.pick("Fender bender with a silver car", schemas))
        assertEquals(TemplateId("property_damage"), TemplateMatcher.pick("Water leak from the ceiling pipe", schemas))
        assertEquals(TemplateId("rental_condition"), TemplateMatcher.pick("Apartment move out inspection", schemas))
        assertEquals(TemplateId.GENERAL, TemplateMatcher.pick("Birthday party", schemas))
    }

    @Test
    fun matcherFallbackTemplateExists() = runTest {
        assertNotNull(BundledTemplates.store()[TemplateId.GENERAL.value])
    }

    @Test
    fun everyBundledSlotFitsTheDomainSchema() = runTest {
        BundledTemplates.store().all.forEach { template ->
            assertEquals(template.slots.size, template.schema.slots.size, template.id)
            template.schema.slots.forEach { assertTrue(it.maxLength in 1..SlotSchema.MAX_TEXT_LENGTH, it.id) }
        }
    }

    @Test
    fun everyTemplateHasASummaryAndEvidenceFlags() = runTest {
        BundledTemplates.store().all.forEach { template ->
            assertEquals(SlotKind.TEXT, template.slot("summary")?.type, template.id)
            assertTrue(template.slots.count { it.type == SlotKind.FLAG } >= 3, template.id)
            assertTrue(template.slots.filter { it.type == SlotKind.FLAG }.all { it.id.startsWith("ev_") }, template.id)
        }
    }
}
