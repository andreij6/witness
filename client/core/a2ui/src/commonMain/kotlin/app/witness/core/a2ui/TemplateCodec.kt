package app.witness.core.a2ui

import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

class TemplateException(message: String) : IllegalArgumentException(message)

object TemplateCodec {
    const val ROOT_ID = "root"
    private const val MAX_IDENTIFIER_LENGTH = 48

    private val json = Json { ignoreUnknownKeys = false }

    @Serializable
    private class TemplateFile(
        val id: String,
        val title: String,
        val subtitle: String = "",
        val keywords: List<String> = emptyList(),
        val surfaceId: String,
        val slots: List<SlotSchema>,
        val messages: List<JsonObject>,
    )

    fun decode(text: String): Template {
        val file = try {
            json.decodeFromString<TemplateFile>(text)
        } catch (e: SerializationException) {
            throw TemplateException("Malformed template: ${e.message}")
        }
        ensure(isIdentifier(file.id)) { "Invalid template id '${file.id}'" }
        ensure(file.title.isNotBlank()) { "Template '${file.id}' has a blank title" }
        ensure(file.surfaceId.isNotBlank()) { "Template '${file.id}' has a blank surfaceId" }
        ensure(file.keywords.none(String::isBlank)) { "Template '${file.id}' has a blank keyword" }
        val slotIds = validateSlots(file.slots)
        validateMessages(file.surfaceId, file.messages, slotIds)
        return Template(file.id, file.title, file.subtitle, file.keywords, file.surfaceId, file.slots, file.messages)
    }

    fun isIdentifier(value: String): Boolean {
        if (value.isEmpty() || value.length > MAX_IDENTIFIER_LENGTH || value[0] !in 'a'..'z') return false
        for (c in value) if (c !in 'a'..'z' && c !in '0'..'9' && c != '_') return false
        return true
    }

    private fun validateSlots(slots: List<SlotSchema>): Set<String> {
        val ids = HashSet<String>(slots.size * 2)
        for (spec in slots) {
            ensure(isIdentifier(spec.id)) { "Invalid slot id '${spec.id}'" }
            ensure(ids.add(spec.id)) { "Duplicate slot id '${spec.id}'" }
            if (spec.type == SlotKind.CHOICE) {
                ensure(spec.choices.isNotEmpty()) { "Choice slot '${spec.id}' has no choices" }
                ensure(spec.choices.all(::isIdentifier)) { "Choice slot '${spec.id}' has an invalid choice" }
                ensure(spec.choices.toHashSet().size == spec.choices.size) { "Choice slot '${spec.id}' repeats a choice" }
            } else {
                ensure(spec.choices.isEmpty()) { "Slot '${spec.id}' is not a choice but lists choices" }
            }
            ensure(spec.maxLength in 1..SlotSchema.MAX_TEXT_LENGTH) { "Slot '${spec.id}' has maxLength ${spec.maxLength}" }
        }
        return ids
    }

    private fun validateMessages(surfaceId: String, messages: List<JsonObject>, slotIds: Set<String>) {
        ensure(messages.isNotEmpty()) { "Template has no messages" }
        val scan = ComponentScan(slotIds)
        messages.forEachIndexed { index, message ->
            ensure(message.string("version") == A2uiMessages.VERSION) { "Message $index is not A2UI ${A2uiMessages.VERSION}" }
            val envelopes = message.keys.filter { it in A2uiMessages.envelopes }
            ensure(envelopes.size == 1) { "Message $index must hold exactly one envelope" }
            ensure(message.keys.size == 2) { "Message $index has unknown keys" }
            val kind = envelopes[0]
            val body = message[kind] as? JsonObject ?: throw TemplateException("Message $index body is not an object")
            ensure(body.string("surfaceId") == surfaceId) { "Message $index targets another surface" }
            ensure((index == 0) == (kind == A2uiMessages.CREATE_SURFACE)) { "Only the first message creates the surface" }
            when (kind) {
                A2uiMessages.CREATE_SURFACE -> ensure(body.string("catalogId") == WitnessCatalog.ID) { "Template must use the Witness catalog" }
                A2uiMessages.UPDATE_COMPONENTS -> {
                    val components = body["components"] as? JsonArray ?: throw TemplateException("Message $index has no components")
                    components.forEach { scan.component(it) }
                }
                A2uiMessages.UPDATE_DATA_MODEL -> ensure(body.string("path")?.startsWith("/") ?: true) { "Message $index has a relative path" }
                else -> throw TemplateException("Templates may not ${kind}")
            }
        }
        scan.finish()
    }

    private class ComponentScan(private val slotIds: Set<String>) {
        private val ids = HashSet<String>()
        private val references = ArrayList<String>()

        fun component(element: JsonElement) {
            val component = element as? JsonObject ?: throw TemplateException("Component is not an object")
            val id = component.string("id")?.takeIf(String::isNotEmpty) ?: throw TemplateException("Component without id")
            ensure(ids.add(id)) { "Duplicate component id '$id'" }
            val type = component.string("component")
            ensure(type != null && type in WitnessCatalog.componentNames) { "Component '$id' uses unknown type '$type'" }
            for ((key, value) in component) {
                if (key == "children") children(id, value)
                scanValue(id, value)
            }
        }

        private fun children(owner: String, value: JsonElement) {
            when (value) {
                is JsonArray -> value.forEach {
                    references += (it as? JsonPrimitive)?.takeIf(JsonPrimitive::isString)?.content
                        ?: throw TemplateException("Component '$owner' has a non-string child")
                }
                is JsonObject -> references += value.string("componentId")
                    ?: throw TemplateException("Component '$owner' has a child template without componentId")
                else -> throw TemplateException("Component '$owner' has malformed children")
            }
        }

        private fun scanValue(owner: String, value: JsonElement) {
            when (value) {
                is JsonArray -> value.forEach { scanValue(owner, it) }
                is JsonObject -> {
                    ensure("functionCall" !in value) { "Component '$owner' calls a function" }
                    (value["event"] as? JsonObject)?.let { event(owner, it) }
                    value.string("path")?.let { binding(owner, it) }
                    value.values.forEach { scanValue(owner, it) }
                }
                else -> Unit
            }
        }

        private fun event(owner: String, event: JsonObject) {
            val name = event.string("name")
            ensure(name != null && WitnessCatalog.isMappedEvent(name)) { "Component '$owner' emits unmapped event '$name'" }
            val target = (event["context"] as? JsonObject)?.string("target")
            ensure(!target.isNullOrBlank()) { "Component '$owner' event '$name' has no target" }
        }

        private fun binding(owner: String, path: String) {
            if (!path.startsWith(SLOT_PREFIX)) return
            val slotId = path.substring(SLOT_PREFIX.length).substringBefore('/')
            ensure(slotId in slotIds) { "Component '$owner' binds undeclared slot '$slotId'" }
        }

        fun finish() {
            ensure(TemplateCodec.ROOT_ID in ids) { "Template has no '${TemplateCodec.ROOT_ID}' component" }
            for (reference in references) ensure(reference in ids) { "Unknown child '$reference'" }
        }
    }

    private const val SLOT_PREFIX = A2uiMessages.SLOTS_PATH + "/"
}

private inline fun ensure(condition: Boolean, message: () -> String) {
    if (!condition) throw TemplateException(message())
}

internal fun JsonObject.string(key: String): String? =
    (this[key] as? JsonPrimitive)?.takeIf(JsonPrimitive::isString)?.content
