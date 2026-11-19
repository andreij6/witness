package app.witness.core.platform.assist

import app.witness.core.domain.port.Assistant
import app.witness.core.domain.port.AssistantContext
import app.witness.core.model.BlurClass
import app.witness.core.model.CaptureMode
import app.witness.core.model.Slot
import app.witness.core.model.SlotSpec
import app.witness.core.model.SlotType
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import app.witness.core.model.TemplateSchema
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

val general = TemplateSchema(TemplateId.GENERAL, "General", emptyList(), listOf(SlotSpec("notes", SlotType.TEXT)))
val car = TemplateSchema(
    TemplateId.CAR_ACCIDENT,
    "Car accident",
    listOf("car", "crash", "fender", "bumper", "accident"),
    listOf(
        SlotSpec("other_plate", SlotType.TEXT, maxLength = 10),
        SlotSpec("damage", SlotType.CHOICE, listOf("minor", "major")),
        SlotSpec("injuries", SlotType.FLAG),
    ),
)
val rental = TemplateSchema(TemplateId.RENTAL_CONDITION, "Rental", listOf("rental", "apartment", "lease"), emptyList())
val templates = listOf(general, car, rental)

class TemplateMatcherTest {
    @Test
    fun tokenizesLowercaseAlnum() {
        assertEquals(setOf("fender", "bender", "on", "5th", "ave"), TemplateMatcher.tokens("Fender-bender on 5th Ave.", null))
    }

    @Test
    fun emptyTokens() {
        assertTrue(TemplateMatcher.tokens(null, "", "  ").isEmpty())
    }

    @Test
    fun bestByKeywordCount() {
        assertEquals(car, TemplateMatcher.best(templates, TemplateMatcher.tokens("Rear bumper crash")))
        assertEquals(rental, TemplateMatcher.best(templates, TemplateMatcher.tokens("Apartment lease move-out")))
    }

    @Test
    fun noMatchFallsBackToGeneralThenFirst() {
        assertEquals(general, TemplateMatcher.best(templates, setOf("sunset")))
        assertEquals(car, TemplateMatcher.best(listOf(car, rental), setOf("sunset")))
        assertNull(TemplateMatcher.best(emptyList(), setOf("car")))
    }

    @Test
    fun tieKeepsFirstListed() {
        assertEquals(car, TemplateMatcher.best(listOf(car, rental), setOf("car", "rental")))
    }
}

class SuggestionValidatorTest {
    @Test
    fun keepsValidSlotsInSchemaOrder() {
        val s = Suggestion(TemplateId.CAR_ACCIDENT, listOf(Slot("injuries", flag = false), Slot("damage", choice = "MAJOR"), Slot("other_plate", text = " 7MNV562 ")), " Hail ", null)
        val v = SuggestionValidator.validate(s, templates)!!
        assertEquals(listOf(Slot("other_plate", text = "7MNV562"), Slot("damage", choice = "major"), Slot("injuries", flag = false)), v.slots)
        assertEquals("Hail", v.caption)
    }

    @Test
    fun dropsInvalidSlots() {
        val s = Suggestion(
            TemplateId.CAR_ACCIDENT,
            listOf(
                Slot("unknown", text = "x"),
                Slot("damage", choice = "totaled"),
                Slot("injuries"),
                Slot("other_plate", text = "   "),
            ),
        )
        assertTrue(SuggestionValidator.validate(s, templates)!!.slots.isEmpty())
    }

    @Test
    fun textIsTruncatedToMaxLength() {
        val s = Suggestion(TemplateId.CAR_ACCIDENT, listOf(Slot("other_plate", text = "ABCDEFGHIJKLMNOP")))
        assertEquals("ABCDEFGHIJ", SuggestionValidator.validate(s, templates)!!.slots.single().text)
    }

    @Test
    fun duplicateSlotsFirstWins() {
        val s = Suggestion(TemplateId.CAR_ACCIDENT, listOf(Slot("damage", choice = "minor"), Slot("damage", choice = "major")))
        assertEquals("minor", SuggestionValidator.validate(s, templates)!!.slots.single().choice)
    }

    @Test
    fun unknownTemplateIsNull() {
        assertNull(SuggestionValidator.validate(Suggestion(TemplateId("nope"), emptyList()), templates))
    }

    @Test
    fun mergeFillsMissingFromRules() {
        val rules = Suggestion(TemplateId.CAR_ACCIDENT, listOf(Slot("damage", choice = "minor")), "rules caption", "rules alt")
        val model = Suggestion(TemplateId.CAR_ACCIDENT, listOf(Slot("injuries", flag = true), Slot("damage", choice = "bogus")), null, "model alt")
        val m = SuggestionValidator.merge(model, rules, templates)
        assertEquals(listOf(Slot("damage", choice = "minor"), Slot("injuries", flag = true)), m.slots)
        assertEquals("rules caption", m.caption)
        assertEquals("model alt", m.altText)
    }

    @Test
    fun mergeIgnoresRulesSlotsFromOtherTemplate() {
        val rules = Suggestion(TemplateId.GENERAL, listOf(Slot("notes", text = "n")))
        val m = SuggestionValidator.merge(Suggestion(TemplateId.CAR_ACCIDENT, emptyList()), rules, templates)
        assertEquals(TemplateId.CAR_ACCIDENT, m.templateId)
        assertTrue(m.slots.isEmpty())
    }

    @Test
    fun mergeWithInvalidPrimaryReturnsFallback() {
        val rules = Suggestion(TemplateId.GENERAL, emptyList())
        assertSame(rules, SuggestionValidator.merge(null, rules, templates))
        assertSame(rules, SuggestionValidator.merge(Suggestion(TemplateId("x"), emptyList()), rules, templates))
    }
}

class RulesAssistantTest {
    @Test
    fun alwaysAvailable() = runTest { assertTrue(RulesAssistant().isAvailable()) }

    @Test
    fun picksTemplateAndWritesAltText() = runTest {
        val s = RulesAssistant().suggest(AssistantContext(CaptureMode.CASE, "Fender bender", "Seattle", listOf(BlurClass.PLATE, BlurClass.FACE, BlurClass.FACE)), templates)!!
        assertEquals(TemplateId.CAR_ACCIDENT, s.templateId)
        assertEquals("Fender bender", s.caption)
        assertTrue(s.altText!!.startsWith("Photo taken in Seattle."))
        assertTrue("2 faces" in s.altText!!)
        assertTrue("1 license plate" in s.altText!!)
    }

    @Test
    fun noTemplatesGivesNull() = runTest {
        assertNull(RulesAssistant().suggest(AssistantContext(CaptureMode.POST), emptyList()))
    }

    @Test
    fun deterministic() = runTest {
        val ctx = AssistantContext(CaptureMode.POST, "hail on car")
        assertEquals(RulesAssistant().suggest(ctx, templates), RulesAssistant().suggest(ctx, templates))
    }
}

class FallbackAssistantTest {
    private class Fake(val available: Boolean, val answer: () -> Suggestion?) : Assistant {
        var calls = 0
        override suspend fun isAvailable() = available
        override suspend fun suggest(context: AssistantContext, templates: List<TemplateSchema>): Suggestion? {
            calls++
            return answer()
        }
    }

    private val ctx = AssistantContext(CaptureMode.CASE, "car crash")

    @Test
    fun noModelUsesRules() = runTest {
        assertEquals(TemplateId.CAR_ACCIDENT, FallbackAssistant(null).suggest(ctx, templates)!!.templateId)
    }

    @Test
    fun unavailableModelNotCalled() = runTest {
        val model = Fake(false) { error("no") }
        FallbackAssistant(model).suggest(ctx, templates)
        assertEquals(0, model.calls)
    }

    @Test
    fun backgroundSkipsModel() = runTest {
        val model = Fake(true) { null }
        FallbackAssistant(model) { false }.suggest(ctx, templates)
        assertEquals(0, model.calls)
    }

    @Test
    fun modelAnswerIsValidatedAndMerged() = runTest {
        val model = Fake(true) { Suggestion(TemplateId.CAR_ACCIDENT, listOf(Slot("damage", choice = "minor"), Slot("evil", text = "x")), "From model", null) }
        val s = FallbackAssistant(model).suggest(ctx, templates)!!
        assertEquals(listOf(Slot("damage", choice = "minor")), s.slots)
        assertEquals("From model", s.caption)
        assertTrue(s.altText!!.startsWith("Photo"))
    }

    @Test
    fun modelFailureFallsBackToRules() = runTest {
        val s = FallbackAssistant(Fake(true) { throw IllegalStateException("boom") }).suggest(ctx, templates)!!
        assertEquals("car crash", s.caption)
    }

    @Test
    fun alwaysReportsAvailable() = runTest { assertTrue(FallbackAssistant(null).isAvailable()) }
}

class PromptBuilderTest {
    @Test
    fun includesContextAndSlotSchema() {
        val p = PromptBuilder.build(AssistantContext(CaptureMode.CASE, "Fender", "Seattle", listOf(BlurClass.PLATE)), listOf(car))
        assertTrue("Mode: case" in p)
        assertTrue("Title: Fender" in p)
        assertTrue("Place: Seattle" in p)
        assertTrue("Detected: plate" in p)
        assertTrue("car_accident (Car accident): other_plate text; damage choice of minor|major; injuries flag" in p)
    }

    @Test
    fun sanitizeStripsControlAndCaps() {
        assertEquals("a b", PromptBuilder.sanitize("a\nb"))
        assertEquals(PromptBuilder.MAX_FIELD, PromptBuilder.sanitize("x".repeat(500)).length)
    }

    @Test
    fun omitsAbsentFields() {
        val p = PromptBuilder.build(AssistantContext(CaptureMode.POST), emptyList())
        assertFalse("Title:" in p)
        assertFalse("Detected:" in p)
    }
}
