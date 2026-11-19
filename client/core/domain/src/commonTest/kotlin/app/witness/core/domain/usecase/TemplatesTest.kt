package app.witness.core.domain.usecase

import app.witness.core.domain.fake.FakeTemplateCatalog
import app.witness.core.model.Slot
import app.witness.core.model.SlotSpec
import app.witness.core.model.SlotType
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import app.witness.core.model.TemplateSchema
import kotlin.test.Test
import kotlin.test.assertEquals

class TemplatesTest {
    private val templates = FakeTemplateCatalog.BUNDLED
    private val car = templates.first { it.id == TemplateId.CAR_ACCIDENT }

    @Test
    fun tokensSplitOnNonAlphanumerics() {
        assertEquals(listOf("fender", "bender", "oak", "st", "5th"), Tokens.of("Fender-bender @ Oak St & 5th"))
        assertEquals(emptyList(), Tokens.of(""))
        assertEquals(emptyList(), Tokens.of("  -- !! "))
        assertEquals(listOf("héllo", "wörld"), Tokens.of("Héllo, Wörld"))
        assertEquals(listOf("a1"), Tokens.of("A1"))
    }

    @Test
    fun matcherPicksBestTemplate() {
        assertEquals(TemplateId.CAR_ACCIDENT, TemplateMatcher.pick("Fender bender on Oak St", templates))
        assertEquals(TemplateId.PROPERTY_DAMAGE, TemplateMatcher.pick("Hail damage to roof", templates))
        assertEquals(TemplateId.RENTAL_CONDITION, TemplateMatcher.pick("Move-out walkthrough", templates))
        assertEquals(TemplateId.GENERAL, TemplateMatcher.pick("Birthday party", templates))
        assertEquals(TemplateId.GENERAL, TemplateMatcher.pick("", templates))
        assertEquals(TemplateId.GENERAL, TemplateMatcher.pick("car", emptyList()))
    }

    @Test
    fun matcherRequiresWholeMultiWordKeywordsAndBreaksTiesByOrder() {
        assertEquals(TemplateId.GENERAL, TemplateMatcher.pick("bender", templates))
        val a = TemplateSchema(TemplateId("a"), "A", listOf("x"), emptyList())
        val b = TemplateSchema(TemplateId("b"), "B", listOf("x"), emptyList())
        assertEquals(TemplateId("a"), TemplateMatcher.pick("x", listOf(a, b)))
        assertEquals(TemplateId("b"), TemplateMatcher.pick("x", listOf(b, a)))
    }

    @Test
    fun validatorKeepsValidSlotsInSchemaOrder() {
        val s = Suggestion(
            TemplateId("whatever"),
            listOf(
                Slot("injuries", flag = true),
                Slot("impact", choice = "rear"),
                Slot("other_plate", text = "  ABC 123 "),
            ),
            caption = " c ",
            altText = "",
        )
        val v = SuggestionValidator.validate(s, car)
        assertEquals(TemplateId.CAR_ACCIDENT, v.templateId)
        assertEquals(listOf("other_plate", "impact", "injuries"), v.slots.map { it.id })
        assertEquals("ABC 123", v.slots[0].text)
        assertEquals("c", v.caption)
        assertEquals(null, v.altText)
    }

    @Test
    fun validatorDropsInvalidAndFillsFromRules() {
        val s = Suggestion(
            car.id,
            listOf(
                Slot("impact", choice = "sideways"),
                Slot("unknown", text = "x"),
                Slot("other_plate", text = "A".repeat(40)),
                Slot("insurer", choice = "x"),
                Slot("injuries", text = "yes"),
                Slot("insurer", text = "first"),
                Slot("insurer", text = "second"),
            ),
        )
        val v = SuggestionValidator.validate(s, car, fallback = listOf(Slot("impact", choice = "front"), Slot("injuries", flag = false), Slot("insurer", text = "rules")))
        assertEquals(
            listOf(Slot("other_plate", text = "A".repeat(16)), Slot("insurer", text = "first"), Slot("impact", choice = "front"), Slot("injuries", flag = false)),
            v.slots,
        )
    }

    @Test
    fun missingRequiredSlots() {
        val empty = Suggestion(car.id, emptyList())
        assertEquals(listOf("impact"), SuggestionValidator.missingRequired(empty, car))
        val filled = SuggestionValidator.validate(Suggestion(car.id, listOf(Slot("impact", choice = "left"))), car)
        assertEquals(emptyList(), SuggestionValidator.missingRequired(filled, car))
    }

    @Test
    fun textSlotRejectsBlank() {
        val schema = TemplateSchema(TemplateId("t"), "T", emptyList(), listOf(SlotSpec("n", SlotType.TEXT)))
        assertEquals(emptyList(), SuggestionValidator.validate(Suggestion(schema.id, listOf(Slot("n", text = "   "))), schema).slots)
    }
}
