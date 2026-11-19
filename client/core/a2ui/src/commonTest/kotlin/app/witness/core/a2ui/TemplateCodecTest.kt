package app.witness.core.a2ui

import app.witness.core.model.SlotType
import app.witness.core.model.TemplateId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class TemplateCodecTest {
    private val rootOnly = """[{"id": "root", "component": "Column", "children": []}]"""

    private fun rejects(text: String, fragment: String) {
        val error = assertFailsWith<TemplateException> { TemplateCodec.decode(text) }
        assertTrue(error.message.orEmpty().contains(fragment), "expected '$fragment' in '${error.message}'")
    }

    private fun rejectsComponents(components: String, fragment: String) =
        rejects(TestTemplates.json(components = components), fragment)

    @Test
    fun decodesValidTemplate() {
        val template = TestTemplates.template()
        assertEquals("car_accident", template.id)
        assertEquals("Car accident", template.title)
        assertEquals("Plates", template.subtitle)
        assertEquals(TestTemplates.SURFACE, template.surfaceId)
        assertEquals(listOf("impact_point", "other_plate", "summary", "ev_wide"), template.slots.map { it.id })
        assertEquals(2, template.messages.size)
    }

    @Test
    fun parsesSlotTypesAndDefaults() {
        val template = TestTemplates.template()
        assertEquals(SlotKind.CHOICE, template.slot("impact_point")?.type)
        assertEquals(SlotKind.PLATE, template.slot("other_plate")?.type)
        assertEquals(SlotKind.TEXT, template.slot("summary")?.type)
        assertEquals(SlotKind.FLAG, template.slot("ev_wide")?.type)
        assertEquals(20, template.slot("summary")?.maxLength)
        assertEquals(SlotSchema.DEFAULT_MAX_LENGTH, template.slot("other_plate")?.maxLength)
        assertEquals("", template.slot("other_plate")?.hint)
    }

    @Test
    fun indexesSlotsInDeclaredOrder() {
        val template = TestTemplates.template()
        assertEquals(0, template.indexOf("impact_point"))
        assertEquals(3, template.indexOf("ev_wide"))
        assertEquals(-1, template.indexOf("nope"))
        assertNull(template.slot("nope"))
    }

    @Test
    fun decodingIsDeterministic() {
        val text = TestTemplates.json()
        val a = TemplateCodec.decode(text)
        val b = TemplateCodec.decode(text)
        assertEquals(a.slots, b.slots)
        assertEquals(a.messages, b.messages)
    }

    @Test
    fun acceptsDataModelSeedMessage() {
        val seed = """{"version": "v0.9.1", "updateDataModel": {"surfaceId": "report", "path": "/evidence", "value": {"wide": "Photo 4"}}}"""
        val messages = "[${TestTemplates.createSurface()}, ${TestTemplates.updateComponents()}, $seed]"
        assertEquals(3, TemplateCodec.decode(TestTemplates.json(messages = messages)).messages.size)
    }

    @Test
    fun acceptsChildTemplateObject() {
        val components = """[
            {"id": "root", "component": "Column", "children": {"componentId": "flag", "path": "/items"}},
            {"id": "flag", "component": "FlagButton", "action": {"event": {"name": "flag", "context": {"target": "x"}}}}
        ]"""
        assertNotNull(TemplateCodec.decode(TestTemplates.json(components = components)))
    }

    @Test
    fun allowsBindingsOutsideSlots() {
        val components = """[
            {"id": "root", "component": "SummaryDraft", "title": "t", "text": {"path": "/evidence/anything"}}
        ]"""
        assertNotNull(TemplateCodec.decode(TestTemplates.json(components = components)))
    }

    @Test
    fun keywordsDefaultToEmpty() = assertEquals(emptyList<String>(), TestTemplates.template().keywords)

    @Test
    fun parsesKeywords() {
        val template = TemplateCodec.decode(TestTemplates.json(extra = """, "keywords": ["car", "license plate"]"""))
        assertEquals(listOf("car", "license plate"), template.keywords)
        assertEquals(listOf("car", "license plate"), template.schema.keywords)
    }

    @Test
    fun rejectsBlankKeyword() = rejects(TestTemplates.json(extra = """, "keywords": ["car", " "]"""), "blank keyword")

    @Test
    fun exposesDomainSchema() {
        val schema = TestTemplates.template().schema
        assertEquals(TemplateId("car_accident"), schema.id)
        assertEquals("Car accident", schema.title)
        assertEquals(
            listOf(SlotType.CHOICE, SlotType.TEXT, SlotType.TEXT, SlotType.FLAG),
            schema.slots.map { it.type },
        )
        assertEquals(SlotSchema.MAX_PLATE_LENGTH, schema.slots[1].maxLength)
        assertEquals(20, schema.slots[2].maxLength)
        assertEquals(listOf("front_driver", "rear_driver"), schema.slots[0].choices)
    }

    @Test
    fun parsesRequiredFlag() {
        val template = TestTemplates.template(slots = """[{"id": "a", "type": "flag", "required": true}]""", components = rootOnly)
        assertTrue(template.slots.single().required)
        assertTrue(template.schema.slots.single().required)
    }

    @Test
    fun rejectsMalformedJson() = rejects("{not json", "Malformed")

    @Test
    fun rejectsEmptyInput() = rejects("", "Malformed")

    @Test
    fun rejectsUnknownTopLevelKey() = rejects(TestTemplates.json(extra = """, "evil": true"""), "Malformed")

    @Test
    fun rejectsMissingSlots() =
        rejects("""{"id": "a", "title": "t", "surfaceId": "s", "messages": []}""", "Malformed")

    @Test
    fun rejectsUnknownSlotType() =
        rejects(TestTemplates.json(slots = """[{"id": "a", "type": "number"}]"""), "Malformed")

    @Test
    fun rejectsInvalidTemplateId() = rejects(TestTemplates.json(id = "Car-Accident"), "Invalid template id")

    @Test
    fun rejectsBlankTitle() = rejects(TestTemplates.json().replace("\"Car accident\"", "\" \""), "blank title")

    @Test
    fun rejectsInvalidSlotId() = rejects(TestTemplates.json(slots = """[{"id": "9lives", "type": "flag"}]"""), "Invalid slot id")

    @Test
    fun rejectsDuplicateSlotId() =
        rejects(TestTemplates.json(slots = """[{"id": "a", "type": "flag"}, {"id": "a", "type": "text"}]"""), "Duplicate slot id")

    @Test
    fun rejectsChoiceWithoutChoices() =
        rejects(TestTemplates.json(slots = """[{"id": "a", "type": "choice"}]"""), "has no choices")

    @Test
    fun rejectsInvalidChoiceValue() =
        rejects(TestTemplates.json(slots = """[{"id": "a", "type": "choice", "choices": ["Yes!"]}]"""), "invalid choice")

    @Test
    fun rejectsRepeatedChoice() =
        rejects(TestTemplates.json(slots = """[{"id": "a", "type": "choice", "choices": ["no", "no"]}]"""), "repeats")

    @Test
    fun rejectsChoicesOnNonChoiceSlot() =
        rejects(TestTemplates.json(slots = """[{"id": "a", "type": "text", "choices": ["no"]}]"""), "not a choice")

    @Test
    fun rejectsZeroMaxLength() =
        rejects(TestTemplates.json(slots = """[{"id": "a", "type": "text", "maxLength": 0}]"""), "maxLength")

    @Test
    fun rejectsHugeMaxLength() =
        rejects(TestTemplates.json(slots = """[{"id": "a", "type": "text", "maxLength": 4001}]"""), "maxLength")

    @Test
    fun rejectsNoMessages() = rejects(TestTemplates.json(messages = "[]"), "no messages")

    @Test
    fun rejectsWrongVersion() =
        rejects(TestTemplates.json(messages = "[${TestTemplates.createSurface().replace("v0.9.1", "v0.8")}]"), "is not A2UI")

    @Test
    fun rejectsTwoEnvelopesInOneMessage() {
        val message = """{"version": "v0.9.1", "createSurface": {"surfaceId": "report", "catalogId": "${WitnessCatalog.ID}"},
            "deleteSurface": {"surfaceId": "report"}}"""
        rejects(TestTemplates.json(messages = "[$message]"), "exactly one envelope")
    }

    @Test
    fun rejectsUnknownMessageKey() {
        val message = """{"version": "v0.9.1", "createSurface": {"surfaceId": "report", "catalogId": "${WitnessCatalog.ID}"}, "x": 1}"""
        rejects(TestTemplates.json(messages = "[$message]"), "unknown keys")
    }

    @Test
    fun rejectsFirstMessageNotCreate() =
        rejects(TestTemplates.json(messages = "[${TestTemplates.updateComponents()}]"), "first message creates")

    @Test
    fun rejectsSecondCreate() =
        rejects(TestTemplates.json(messages = "[${TestTemplates.createSurface()}, ${TestTemplates.createSurface()}]"), "first message creates")

    @Test
    fun rejectsForeignCatalog() =
        rejects(TestTemplates.json(messages = "[${TestTemplates.createSurface(catalogId = "https://a2ui.org/basic.json")}]"), "Witness catalog")

    @Test
    fun rejectsOtherSurface() =
        rejects(TestTemplates.json(messages = "[${TestTemplates.createSurface(surfaceId = "other")}]"), "another surface")

    @Test
    fun rejectsDeleteSurface() {
        val delete = """{"version": "v0.9.1", "deleteSurface": {"surfaceId": "report"}}"""
        rejects(TestTemplates.json(messages = "[${TestTemplates.createSurface()}, ${TestTemplates.updateComponents()}, $delete]"), "may not")
    }

    @Test
    fun rejectsRelativeDataPath() {
        val seed = """{"version": "v0.9.1", "updateDataModel": {"surfaceId": "report", "path": "slots", "value": 1}}"""
        rejects(TestTemplates.json(messages = "[${TestTemplates.createSurface()}, ${TestTemplates.updateComponents()}, $seed]"), "relative")
    }

    @Test
    fun rejectsNonObjectBody() {
        val message = """{"version": "v0.9.1", "createSurface": 5}"""
        rejects(TestTemplates.json(messages = "[$message]"), "not an object")
    }

    @Test
    fun rejectsMissingComponentsArray() {
        val message = """{"version": "v0.9.1", "updateComponents": {"surfaceId": "report"}}"""
        rejects(TestTemplates.json(messages = "[${TestTemplates.createSurface()}, $message]"), "no components")
    }

    @Test
    fun rejectsMissingRoot() = rejectsComponents("""[{"id": "top", "component": "Column", "children": []}]""", "no 'root'")

    @Test
    fun rejectsComponentWithoutId() = rejectsComponents("""[{"component": "Column", "children": []}]""", "without id")

    @Test
    fun rejectsNonObjectComponent() = rejectsComponents("""["root"]""", "not an object")

    @Test
    fun rejectsDuplicateComponentId() = rejectsComponents(
        """[{"id": "root", "component": "Column", "children": []}, {"id": "root", "component": "Column", "children": []}]""",
        "Duplicate component id",
    )

    @Test
    fun rejectsUnknownComponentType() =
        rejectsComponents("""[{"id": "root", "component": "WebView", "url": "https://x"}]""", "unknown type")

    @Test
    fun rejectsBasicCatalogComponent() =
        rejectsComponents("""[{"id": "root", "component": "Text", "text": "hi"}]""", "unknown type")

    @Test
    fun rejectsUnknownChild() =
        rejectsComponents("""[{"id": "root", "component": "Column", "children": ["ghost"]}]""", "Unknown child 'ghost'")

    @Test
    fun rejectsNonStringChild() =
        rejectsComponents("""[{"id": "root", "component": "Column", "children": [1]}]""", "non-string child")

    @Test
    fun rejectsMalformedChildren() =
        rejectsComponents("""[{"id": "root", "component": "Column", "children": "root"}]""", "malformed children")

    @Test
    fun rejectsChildTemplateWithoutComponentId() =
        rejectsComponents("""[{"id": "root", "component": "Column", "children": {"path": "/x"}}]""", "without componentId")

    @Test
    fun rejectsUnmappedEvent() = rejectsComponents(
        """[{"id": "root", "component": "FlagButton", "action": {"event": {"name": "open_url", "context": {"target": "x"}}}}]""",
        "unmapped event 'open_url'",
    )

    @Test
    fun rejectsUnmappedEventNestedInItems() = rejectsComponents(
        """[{"id": "root", "component": "EvidenceChecklist", "title": "t",
            "items": [{"key": "a", "label": "A", "action": {"event": {"name": "share", "context": {"target": "a"}}}}]}]""",
        "unmapped event 'share'",
    )

    @Test
    fun rejectsEventWithoutTarget() = rejectsComponents(
        """[{"id": "root", "component": "FlagButton", "action": {"event": {"name": "flag"}}}]""",
        "has no target",
    )

    @Test
    fun rejectsEventWithBlankTarget() = rejectsComponents(
        """[{"id": "root", "component": "FlagButton", "action": {"event": {"name": "flag", "context": {"target": " "}}}}]""",
        "has no target",
    )

    @Test
    fun rejectsFunctionCalls() = rejectsComponents(
        """[{"id": "root", "component": "FlagButton", "action": {"functionCall": {"call": "openUrl", "args": {}}}}]""",
        "calls a function",
    )

    @Test
    fun rejectsBindingToUndeclaredSlot() = rejectsComponents(
        """[{"id": "root", "component": "SummaryDraft", "title": "t", "text": {"path": "/slots/secret"}}]""",
        "undeclared slot 'secret'",
    )

    @Test
    fun rejectsNestedBindingToUndeclaredSlot() = rejectsComponents(
        """[{"id": "root", "component": "SummaryDraft", "title": "t", "text": {"path": "/slots/secret/deep"}}]""",
        "undeclared slot 'secret'",
    )

    @Test
    fun identifierRules() {
        listOf("a", "car_accident", "ev_2", "z".repeat(48)).forEach { assertTrue(TemplateCodec.isIdentifier(it), it) }
        listOf("", "A", "_a", "1a", "a-b", "a b", "a.b", "é", "z".repeat(49)).forEach { assertFalse(TemplateCodec.isIdentifier(it), it) }
    }
}
