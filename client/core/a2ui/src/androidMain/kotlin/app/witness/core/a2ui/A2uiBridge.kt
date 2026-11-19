package app.witness.core.a2ui

import androidx.a2ui.model.protocol.A2uiComponentPayload
import androidx.a2ui.model.protocol.A2uiCreateSurfaceMessage
import androidx.a2ui.model.protocol.A2uiDeleteSurfaceMessage
import androidx.a2ui.model.protocol.A2uiServerToClientMessage
import androidx.a2ui.model.protocol.A2uiUpdateComponentsMessage
import androidx.a2ui.model.protocol.A2uiUpdateDataModelMessage
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull

object A2uiBridge {
    private val supportedVersions = hashSetOf("v0.9", A2uiMessages.VERSION)

    fun toLibrary(message: JsonObject): A2uiServerToClientMessage {
        val version = message.string("version")
        require(version in supportedVersions) { "Unsupported A2UI version '$version'" }
        var result: A2uiServerToClientMessage? = null
        for ((key, value) in message) {
            if (key == "version") continue
            val body = value as? JsonObject ?: throw IllegalArgumentException("Envelope '$key' is not an object")
            val parsed = envelope(key, body) ?: continue
            require(result == null) { "Multiple envelopes in one message" }
            result = parsed
        }
        return requireNotNull(result) { "Message has no envelope" }
    }

    fun toValue(element: JsonElement): Any? = when (element) {
        JsonNull -> null
        is JsonPrimitive -> primitive(element)
        is JsonArray -> ArrayList<Any?>(element.size).apply { element.forEach { add(toValue(it)) } }
        is JsonObject -> toMap(element)
    }

    private fun envelope(kind: String, body: JsonObject): A2uiServerToClientMessage? {
        if (kind !in A2uiMessages.envelopes) return null
        val surfaceId = body.string("surfaceId")?.takeIf(String::isNotEmpty)
            ?: throw IllegalArgumentException("Envelope '$kind' has no surfaceId")
        return when (kind) {
            A2uiMessages.CREATE_SURFACE -> A2uiCreateSurfaceMessage(
                surfaceId = surfaceId,
                catalogId = requireNotNull(body.string("catalogId")) { "createSurface has no catalogId" },
                theme = (body["theme"] as? JsonObject)?.let(::toMap).orEmpty(),
                shouldSendDataModel = (body["sendDataModel"] as? JsonPrimitive)?.booleanOrNull ?: false,
            )
            A2uiMessages.UPDATE_COMPONENTS -> A2uiUpdateComponentsMessage(
                surfaceId,
                (body["components"] as? JsonArray ?: throw IllegalArgumentException("updateComponents has no components"))
                    .map(::component),
            )
            A2uiMessages.UPDATE_DATA_MODEL -> A2uiUpdateDataModelMessage(
                surfaceId,
                body.string("path") ?: "/",
                body["value"]?.let(::toValue),
            )
            else -> A2uiDeleteSurfaceMessage(surfaceId)
        }
    }

    private fun component(element: JsonElement): A2uiComponentPayload {
        val obj = element as? JsonObject ?: throw IllegalArgumentException("Component is not an object")
        val properties = HashMap<String, Any?>(obj.size * 2)
        for ((key, value) in obj) if (key != "id" && key != "component") properties[key] = toValue(value)
        return A2uiComponentPayload(
            id = requireNotNull(obj.string("id")) { "Component has no id" },
            type = requireNotNull(obj.string("component")) { "Component has no type" },
            properties = properties,
        )
    }

    private fun toMap(obj: JsonObject): Map<String, Any?> =
        HashMap<String, Any?>(obj.size * 2).apply { for ((key, value) in obj) put(key, toValue(value)) }

    private fun primitive(primitive: JsonPrimitive): Any {
        if (primitive.isString) return primitive.content
        primitive.booleanOrNull?.let { return it }
        val raw = primitive.content
        val decimal = raw.any { it == '.' || it == 'e' || it == 'E' }
        return if (decimal) raw.toDoubleOrNull() ?: raw else raw.toIntOrNull() ?: raw.toLongOrNull() ?: raw
    }
}
