package app.witness.core.a2ui

import app.witness.core.model.Slot
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import androidx.a2ui.model.protocol.A2uiCreateSurfaceMessage
import androidx.a2ui.model.protocol.A2uiDeleteSurfaceMessage
import androidx.a2ui.model.protocol.A2uiUpdateComponentsMessage
import androidx.a2ui.model.protocol.A2uiUpdateDataModelMessage
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNull

class A2uiBridgeTest {
    private fun parse(text: String) = Json.parseToJsonElement(text) as JsonObject

    @Test
    fun convertsCreateSurface() {
        val message = A2uiBridge.toLibrary(
            parse("""{"version": "v0.9.1", "createSurface": {"surfaceId": "r", "catalogId": "c", "theme": {"a": 1}, "sendDataModel": true}}"""),
        )
        assertEquals(A2uiCreateSurfaceMessage("r", "c", mapOf("a" to 1), true), message)
    }

    @Test
    fun createSurfaceDefaults() {
        val message = A2uiBridge.toLibrary(parse("""{"version": "v0.9", "createSurface": {"surfaceId": "r", "catalogId": "c"}}"""))
        assertEquals(A2uiCreateSurfaceMessage("r", "c", emptyMap(), false), message)
    }

    @Test
    fun convertsUpdateComponentsWithoutIdAndTypeInProperties() {
        val message = A2uiBridge.toLibrary(
            parse("""{"version": "v0.9.1", "updateComponents": {"surfaceId": "r", "components": [
                {"id": "root", "component": "Column", "children": ["a"], "spacing": 8}]}}"""),
        )
        assertIs<A2uiUpdateComponentsMessage>(message)
        val component = message.components.single()
        assertEquals("root", component.id)
        assertEquals("Column", component.type)
        assertEquals(mapOf("children" to listOf("a"), "spacing" to 8), component.properties)
    }

    @Test
    fun convertsUpdateDataModel() {
        val message = A2uiBridge.toLibrary(
            parse("""{"version": "v0.9.1", "updateDataModel": {"surfaceId": "r", "path": "/slots", "value": {"a": "x", "b": true}}}"""),
        )
        assertEquals(A2uiUpdateDataModelMessage("r", "/slots", mapOf("a" to "x", "b" to true)), message)
    }

    @Test
    fun updateDataModelDefaultsToRootAndNull() {
        val message = A2uiBridge.toLibrary(parse("""{"version": "v0.9.1", "updateDataModel": {"surfaceId": "r"}}"""))
        assertEquals(A2uiUpdateDataModelMessage("r", "/", null), message)
    }

    @Test
    fun convertsDeleteSurface() {
        assertEquals(A2uiDeleteSurfaceMessage("r"), A2uiBridge.toLibrary(A2uiMessages.deleteSurface("r")))
    }

    @Test
    fun convertsBinderOutput() {
        val template = TestTemplates.template()
        val binding = SlotBinder.bind(template, Suggestion(template.templateId, listOf(Slot("summary", text = "s"), Slot("ev_wide", flag = true))))
        assertEquals(
            A2uiUpdateDataModelMessage("report", "/slots", mapOf("summary" to "s", "ev_wide" to true)),
            A2uiBridge.toLibrary(binding.message),
        )
    }

    @Test
    fun convertsNumbersLikeTheLibraryParser() {
        assertEquals(1, A2uiBridge.toValue(JsonPrimitive(1)))
        assertEquals(5_000_000_000L, A2uiBridge.toValue(JsonPrimitive(5_000_000_000L)))
        assertEquals(1.5, A2uiBridge.toValue(JsonPrimitive(1.5)))
        assertEquals(100.0, A2uiBridge.toValue(Json.parseToJsonElement("1e2")))
    }

    @Test
    fun convertsScalarsAndContainers() {
        assertNull(A2uiBridge.toValue(JsonNull))
        assertEquals(true, A2uiBridge.toValue(JsonPrimitive(true)))
        assertEquals("true", A2uiBridge.toValue(JsonPrimitive("true")))
        assertEquals("12", A2uiBridge.toValue(JsonPrimitive("12")))
        assertEquals(listOf(1, null, mapOf("a" to listOf("b"))), A2uiBridge.toValue(Json.parseToJsonElement("""[1, null, {"a": ["b"]}]""")))
    }

    @Test
    fun rejectsUnsupportedVersion() {
        assertFailsWith<IllegalArgumentException> {
            A2uiBridge.toLibrary(parse("""{"version": "v1.0", "deleteSurface": {"surfaceId": "r"}}"""))
        }
        assertFailsWith<IllegalArgumentException> { A2uiBridge.toLibrary(parse("""{"deleteSurface": {"surfaceId": "r"}}""")) }
    }

    @Test
    fun rejectsMultipleEnvelopes() {
        assertFailsWith<IllegalArgumentException> {
            A2uiBridge.toLibrary(parse("""{"version": "v0.9.1", "deleteSurface": {"surfaceId": "r"}, "updateDataModel": {"surfaceId": "r"}}"""))
        }
    }

    @Test
    fun rejectsMissingEnvelope() {
        assertFailsWith<IllegalArgumentException> { A2uiBridge.toLibrary(parse("""{"version": "v0.9.1", "other": {}}""")) }
    }

    @Test
    fun rejectsMissingSurfaceId() {
        assertFailsWith<IllegalArgumentException> { A2uiBridge.toLibrary(parse("""{"version": "v0.9.1", "deleteSurface": {}}""")) }
        assertFailsWith<IllegalArgumentException> {
            A2uiBridge.toLibrary(parse("""{"version": "v0.9.1", "deleteSurface": {"surfaceId": ""}}"""))
        }
    }

    @Test
    fun rejectsMalformedBodies() {
        assertFailsWith<IllegalArgumentException> { A2uiBridge.toLibrary(parse("""{"version": "v0.9.1", "deleteSurface": 1}""")) }
        assertFailsWith<IllegalArgumentException> {
            A2uiBridge.toLibrary(parse("""{"version": "v0.9.1", "createSurface": {"surfaceId": "r"}}"""))
        }
        assertFailsWith<IllegalArgumentException> {
            A2uiBridge.toLibrary(parse("""{"version": "v0.9.1", "updateComponents": {"surfaceId": "r"}}"""))
        }
        assertFailsWith<IllegalArgumentException> {
            A2uiBridge.toLibrary(parse("""{"version": "v0.9.1", "updateComponents": {"surfaceId": "r", "components": [{"id": "a"}]}}"""))
        }
    }
}
