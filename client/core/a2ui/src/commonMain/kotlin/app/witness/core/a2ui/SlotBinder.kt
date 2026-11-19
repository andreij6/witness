package app.witness.core.a2ui

import app.witness.core.model.Slot
import app.witness.core.model.Suggestion
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

enum class DropReason { WRONG_TEMPLATE, UNKNOWN_ID, DUPLICATE, MISSING_VALUE, INVALID_VALUE }

data class DroppedSlot(val id: String, val reason: DropReason)

class Binding(
    val template: Template,
    val values: JsonObject,
    val dropped: List<DroppedSlot>,
) {
    val message: JsonObject get() = A2uiMessages.updateDataModel(template.surfaceId, A2uiMessages.SLOTS_PATH, values)

    val missing: List<String> get() = template.slots.mapNotNull { spec -> spec.id.takeIf { it !in values } }
}

object SlotBinder {
    fun bind(template: Template, suggestion: Suggestion, fallback: Suggestion? = null): Binding {
        val accepted = arrayOfNulls<JsonElement>(template.slots.size)
        val dropped = ArrayList<DroppedSlot>()
        collect(template, suggestion, accepted, dropped)
        if (fallback != null) collect(template, fallback, accepted, null)
        val values = LinkedHashMap<String, JsonElement>(accepted.size * 2)
        accepted.forEachIndexed { index, value -> if (value != null) values[template.slots[index].id] = value }
        return Binding(template, JsonObject(values), dropped)
    }

    internal fun validate(spec: SlotSchema, choices: Set<String>, slot: Slot): JsonElement? = when (spec.type) {
        SlotKind.TEXT -> slot.text?.let { cleanText(it, spec.maxLength) }?.let(::JsonPrimitive)
        SlotKind.PLATE -> slot.text?.let(::normalizePlate)?.let(::JsonPrimitive)
        SlotKind.CHOICE -> slot.choice?.let { normalizeChoice(it, choices) }?.let(::JsonPrimitive)
        SlotKind.FLAG -> slot.flag?.let(::JsonPrimitive)
    }

    fun cleanText(raw: String, maxLength: Int): String? {
        val text = raw.trim()
        if (text.isEmpty() || text.length > maxLength) return null
        for (c in text) if (c < ' ' && c != '\n') return null
        return text
    }

    fun normalizePlate(raw: String): String? {
        val out = StringBuilder(minOf(raw.length, SlotSchema.MAX_PLATE_LENGTH))
        for (c in raw) {
            when (c) {
                ' ', '-', '·' -> Unit
                in 'A'..'Z', in '0'..'9' -> out.append(c)
                in 'a'..'z' -> out.append(c.uppercaseChar())
                else -> return null
            }
            if (out.length > SlotSchema.MAX_PLATE_LENGTH) return null
        }
        return out.takeIf { it.isNotEmpty() }?.toString()
    }

    fun sanitizePlateInput(raw: String): String {
        val out = StringBuilder(minOf(raw.length, SlotSchema.MAX_PLATE_LENGTH))
        for (c in raw) {
            if (out.length == SlotSchema.MAX_PLATE_LENGTH) break
            when (c) {
                in 'A'..'Z', in '0'..'9' -> out.append(c)
                in 'a'..'z' -> out.append(c.uppercaseChar())
            }
        }
        return out.toString()
    }

    fun normalizeChoice(raw: String, choices: Set<String>): String? {
        val choice = raw.trim().lowercase()
        return choice.takeIf { it in choices }
    }

    private fun collect(
        template: Template,
        suggestion: Suggestion,
        accepted: Array<JsonElement?>,
        dropped: MutableList<DroppedSlot>?,
    ) {
        if (suggestion.templateId != template.templateId) {
            dropped?.let { sink -> suggestion.slots.forEach { sink += DroppedSlot(it.id, DropReason.WRONG_TEMPLATE) } }
            return
        }
        val seen = BooleanArray(template.slots.size)
        for (slot in suggestion.slots) {
            val index = template.indexOf(slot.id)
            val reason = when {
                index < 0 -> DropReason.UNKNOWN_ID
                seen[index] -> DropReason.DUPLICATE
                else -> {
                    seen[index] = true
                    if (accepted[index] != null) null else accept(template, index, slot, accepted)
                }
            }
            if (reason != null) dropped?.add(DroppedSlot(slot.id, reason))
        }
    }

    private fun accept(template: Template, index: Int, slot: Slot, accepted: Array<JsonElement?>): DropReason? {
        val spec = template.slots[index]
        val hasValue = when (spec.type) {
            SlotKind.TEXT, SlotKind.PLATE -> slot.text != null
            SlotKind.CHOICE -> slot.choice != null
            SlotKind.FLAG -> slot.flag != null
        }
        if (!hasValue) return DropReason.MISSING_VALUE
        val value = validate(spec, template.choiceSets[index], slot) ?: return DropReason.INVALID_VALUE
        accepted[index] = value
        return null
    }
}
