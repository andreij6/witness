package app.witness.core.a2ui

import app.witness.core.model.Slot
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SlotBinderTest {
    private val template = TestTemplates.template()

    private fun bind(vararg slots: Slot, templateId: String = template.id, fallback: Suggestion? = null): Binding =
        SlotBinder.bind(template, Suggestion(TemplateId(templateId), slots.toList()), fallback)

    private fun Binding.text(id: String): String? = (values[id] as? JsonPrimitive)?.content

    @Test
    fun acceptsValidSlotsOfEveryType() {
        val binding = bind(
            Slot("impact_point", choice = "rear_driver"),
            Slot("other_plate", text = "7MNV562"),
            Slot("summary", text = "No injuries."),
            Slot("ev_wide", flag = true),
        )
        assertEquals("rear_driver", binding.text("impact_point"))
        assertEquals("7MNV562", binding.text("other_plate"))
        assertEquals("No injuries.", binding.text("summary"))
        assertEquals(true, binding.values["ev_wide"]?.jsonPrimitive?.boolean)
        assertTrue(binding.dropped.isEmpty())
        assertTrue(binding.missing.isEmpty())
    }

    @Test
    fun flagFalseIsAValue() {
        val binding = bind(Slot("ev_wide", flag = false))
        assertEquals(false, binding.values["ev_wide"]?.jsonPrimitive?.boolean)
    }

    @Test
    fun flagValuesAreJsonBooleansNotStrings() {
        val value = bind(Slot("ev_wide", flag = true)).values["ev_wide"]?.jsonPrimitive
        assertEquals(false, value?.isString)
    }

    @Test
    fun dropsUnknownIds() {
        val binding = bind(Slot("injured", choice = "no"))
        assertEquals(listOf(DroppedSlot("injured", DropReason.UNKNOWN_ID)), binding.dropped)
        assertTrue(binding.values.isEmpty())
    }

    @Test
    fun keepsFirstOfDuplicateIds() {
        val binding = bind(Slot("summary", text = "first"), Slot("summary", text = "second"))
        assertEquals("first", binding.text("summary"))
        assertEquals(listOf(DroppedSlot("summary", DropReason.DUPLICATE)), binding.dropped)
    }

    @Test
    fun duplicateAfterInvalidStillCountsAsDuplicate() {
        val binding = bind(Slot("summary", text = ""), Slot("summary", text = "ok"))
        assertNull(binding.values["summary"])
        assertEquals(
            listOf(DroppedSlot("summary", DropReason.INVALID_VALUE), DroppedSlot("summary", DropReason.DUPLICATE)),
            binding.dropped,
        )
    }

    @Test
    fun dropsWrongTypedFields() {
        val binding = bind(
            Slot("impact_point", text = "rear_driver"),
            Slot("other_plate", choice = "7MNV562"),
            Slot("summary", flag = true),
            Slot("ev_wide", text = "true"),
        )
        assertTrue(binding.values.isEmpty())
        assertEquals(4, binding.dropped.size)
        assertTrue(binding.dropped.all { it.reason == DropReason.MISSING_VALUE })
    }

    @Test
    fun ignoresExtraFieldsWhenTypedFieldIsValid() {
        val binding = bind(Slot("impact_point", text = "junk", choice = "rear_driver", flag = true))
        assertEquals("rear_driver", binding.text("impact_point"))
        assertTrue(binding.dropped.isEmpty())
    }

    @Test
    fun dropsChoicesOutsideTheSchema() {
        val binding = bind(Slot("impact_point", choice = "roof"))
        assertEquals(listOf(DroppedSlot("impact_point", DropReason.INVALID_VALUE)), binding.dropped)
    }

    @Test
    fun normalizesChoiceCaseAndWhitespace() {
        assertEquals("rear_driver", bind(Slot("impact_point", choice = "  REAR_Driver ")).text("impact_point"))
    }

    @Test
    fun dropsChoiceThatOnlyMatchesALabel() {
        assertTrue(bind(Slot("impact_point", choice = "Rear, driver side")).values.isEmpty())
    }

    @Test
    fun trimsText() {
        assertEquals("hello", bind(Slot("summary", text = "  hello \n")).text("summary"))
    }

    @Test
    fun dropsBlankText() {
        assertEquals(DropReason.INVALID_VALUE, bind(Slot("summary", text = "   ")).dropped.single().reason)
    }

    @Test
    fun dropsTextOverMaxLength() {
        assertEquals(DropReason.INVALID_VALUE, bind(Slot("summary", text = "x".repeat(21))).dropped.single().reason)
    }

    @Test
    fun acceptsTextAtMaxLength() {
        assertEquals("x".repeat(20), bind(Slot("summary", text = "x".repeat(20))).text("summary"))
    }

    @Test
    fun dropsTextWithControlCharacters() {
        assertTrue(bind(Slot("summary", text = "a\u0000b")).values.isEmpty())
        assertTrue(bind(Slot("summary", text = "a\u001Bb")).values.isEmpty())
    }

    @Test
    fun keepsInnerNewlines() {
        assertEquals("a\nb", bind(Slot("summary", text = "a\nb")).text("summary"))
    }

    @Test
    fun normalizesPlates() {
        assertEquals("7MNV562", bind(Slot("other_plate", text = " 7mnv-562 ")).text("other_plate"))
    }

    @Test
    fun dropsPlatesWithForeignCharacters() {
        assertTrue(bind(Slot("other_plate", text = "<script>")).values.isEmpty())
        assertTrue(bind(Slot("other_plate", text = "ÄB123")).values.isEmpty())
    }

    @Test
    fun dropsEmptyAndOverlongPlates() {
        assertTrue(bind(Slot("other_plate", text = " - ")).values.isEmpty())
        assertTrue(bind(Slot("other_plate", text = "A".repeat(11))).values.isEmpty())
    }

    @Test
    fun dropsAllSlotsForAnotherTemplate() {
        val binding = bind(Slot("summary", text = "x"), Slot("ev_wide", flag = true), templateId = "rental_condition")
        assertTrue(binding.values.isEmpty())
        assertTrue(binding.dropped.all { it.reason == DropReason.WRONG_TEMPLATE })
        assertEquals(2, binding.dropped.size)
    }

    @Test
    fun fallbackFillsOnlyMissingOrDroppedSlots() {
        val fallback = Suggestion(template.templateId, listOf(Slot("summary", text = "rules"), Slot("ev_wide", flag = false)))
        val binding = bind(Slot("summary", text = "model"), Slot("ev_wide", text = "bad"), fallback = fallback)
        assertEquals("model", binding.text("summary"))
        assertEquals(false, binding.values["ev_wide"]?.jsonPrimitive?.boolean)
        assertEquals(listOf(DroppedSlot("ev_wide", DropReason.MISSING_VALUE)), binding.dropped)
    }

    @Test
    fun fallbackProblemsAreNotReported() {
        val fallback = Suggestion(template.templateId, listOf(Slot("ghost", text = "x"), Slot("summary", text = "")))
        assertTrue(bind(fallback = fallback).dropped.isEmpty())
    }

    @Test
    fun fallbackForAnotherTemplateIsIgnored() {
        val fallback = Suggestion(TemplateId("other"), listOf(Slot("summary", text = "x")))
        val binding = bind(fallback = fallback)
        assertTrue(binding.values.isEmpty())
        assertTrue(binding.dropped.isEmpty())
    }

    @Test
    fun outputFollowsSchemaOrderNotSuggestionOrder() {
        val binding = bind(Slot("ev_wide", flag = true), Slot("summary", text = "s"), Slot("impact_point", choice = "front_driver"))
        assertEquals(listOf("impact_point", "summary", "ev_wide"), binding.values.keys.toList())
    }

    @Test
    fun bindingIsDeterministic() {
        val slots = arrayOf(Slot("ev_wide", flag = true), Slot("summary", text = "s"), Slot("ghost", text = "g"))
        assertEquals(bind(*slots).message, bind(*slots).message)
        assertEquals(bind(*slots).dropped, bind(*slots).dropped)
    }

    @Test
    fun reportsMissingSlotsInSchemaOrder() {
        assertEquals(listOf("impact_point", "other_plate", "ev_wide"), bind(Slot("summary", text = "s")).missing)
    }

    @Test
    fun emptySuggestionBindsAnEmptyObject() {
        val binding = bind()
        assertEquals(JsonObject(emptyMap()), binding.values)
        assertEquals(template.slots.map { it.id }, binding.missing)
    }

    @Test
    fun emitsUpdateDataModelForSlotsPath() {
        val message = bind(Slot("summary", text = "s")).message
        assertEquals(A2uiMessages.VERSION, message["version"]?.jsonPrimitive?.content)
        val body = message[A2uiMessages.UPDATE_DATA_MODEL]!!.jsonObject
        assertEquals(TestTemplates.SURFACE, body["surfaceId"]?.jsonPrimitive?.content)
        assertEquals("/slots", body["path"]?.jsonPrimitive?.content)
        assertEquals("s", body["value"]!!.jsonObject["summary"]?.jsonPrimitive?.content)
        assertEquals(setOf("version", A2uiMessages.UPDATE_DATA_MODEL), message.keys)
    }

    @Test
    fun scalesLinearlyWithManySlots() {
        val big = TestTemplates.manySlots(5_000)
        val suggestion = Suggestion(big.templateId, List(20_000) { Slot("s${it % 6_000}", flag = it % 2 == 0) })
        val binding = SlotBinder.bind(big, suggestion)
        assertEquals(5_000, binding.values.size)
        assertEquals(15_000, binding.dropped.size)
    }

    @Test
    fun sanitizesPlateInput() {
        assertEquals("7MNV562", SlotBinder.sanitizePlateInput("7mnv-562"))
        assertEquals("", SlotBinder.sanitizePlateInput("--"))
        assertEquals("ABCDEFGHIJ", SlotBinder.sanitizePlateInput("abcdefghijklmnop"))
        assertEquals("AB", SlotBinder.sanitizePlateInput("a<b>"))
    }

    @Test
    fun cleanTextHelpers() {
        assertEquals("a", SlotBinder.cleanText(" a ", 1))
        assertNull(SlotBinder.cleanText("ab", 1))
        assertNull(SlotBinder.cleanText("a\tb", 10))
    }

    @Test
    fun normalizeChoiceHelper() {
        assertEquals("no", SlotBinder.normalizeChoice(" No ", setOf("no")))
        assertNull(SlotBinder.normalizeChoice("nope", setOf("no")))
        assertNull(SlotBinder.normalizeChoice("", setOf("no")))
    }

    @Test
    fun normalizePlateHelper() {
        assertEquals("AB1", SlotBinder.normalizePlate("ab 1"))
        assertEquals("AB1", SlotBinder.normalizePlate("A·B-1"))
        assertNull(SlotBinder.normalizePlate(""))
        assertNull(SlotBinder.normalizePlate("A_B"))
    }
}
