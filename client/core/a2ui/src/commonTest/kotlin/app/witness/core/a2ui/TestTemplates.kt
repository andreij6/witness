package app.witness.core.a2ui

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

internal object TestTemplates {
    const val SURFACE = "report"

    val defaultSlots: String = """
        [
          {"id": "impact_point", "type": "choice", "choices": ["front_driver", "rear_driver"]},
          {"id": "other_plate", "type": "plate"},
          {"id": "summary", "type": "text", "maxLength": 20},
          {"id": "ev_wide", "type": "flag"}
        ]
    """.trimIndent()

    val defaultComponents: String = """
        [
          {"id": "root", "component": "Column", "children": ["impact", "plate", "summary"]},
          {"id": "impact", "component": "ImpactDiagram", "label": "Impact point", "value": {"path": "/slots/impact_point"},
           "options": [{"value": "rear_driver", "label": "Rear, driver side"}],
           "flagAction": {"event": {"name": "flag", "context": {"target": "impact_point"}}}},
          {"id": "plate", "component": "PlateField", "label": "Plate", "value": {"path": "/slots/other_plate"}},
          {"id": "summary", "component": "SummaryDraft", "title": "Draft summary", "text": {"path": "/slots/summary"},
           "editAction": {"event": {"name": "edit", "context": {"target": "summary"}}}}
        ]
    """.trimIndent()

    fun createSurface(surfaceId: String = SURFACE, catalogId: String = WitnessCatalog.ID): String =
        """{"version": "v0.9.1", "createSurface": {"surfaceId": "$surfaceId", "catalogId": "$catalogId"}}"""

    fun updateComponents(components: String = defaultComponents, surfaceId: String = SURFACE): String =
        """{"version": "v0.9.1", "updateComponents": {"surfaceId": "$surfaceId", "components": $components}}"""

    fun json(
        id: String = "car_accident",
        slots: String = defaultSlots,
        components: String = defaultComponents,
        messages: String = "[${createSurface()}, ${updateComponents(components)}]",
        extra: String = "",
    ): String = """
        {"id": "$id", "title": "Car accident", "subtitle": "Plates", "surfaceId": "$SURFACE",
         "slots": $slots, "messages": $messages $extra}
    """.trimIndent()

    fun template(id: String = "car_accident", slots: String = defaultSlots, components: String = defaultComponents): Template =
        TemplateCodec.decode(json(id = id, slots = slots, components = components))

    fun manySlots(count: Int): Template {
        val slots = JsonArray(List(count) { index ->
            buildJsonObject {
                put("id", "s$index")
                put("type", "flag")
            }
        })
        val components = JsonArray(listOf(buildJsonObject {
            put("id", "root")
            put("component", "Column")
            put("children", JsonArray(emptyList<JsonElement>()))
        }))
        return TemplateCodec.decode(json(slots = slots.toString(), components = components.toString()))
    }

    fun parse(text: String): JsonObject = Json.parseToJsonElement(text) as JsonObject

    fun string(value: String): JsonPrimitive = JsonPrimitive(value)
}
