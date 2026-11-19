package app.witness.core.a2ui

import app.witness.core.model.TemplateId
import app.witness.core.model.TemplateSchema
import kotlinx.serialization.json.JsonObject

class Template internal constructor(
    val id: String,
    val title: String,
    val subtitle: String,
    val keywords: List<String>,
    val surfaceId: String,
    val slots: List<SlotSchema>,
    val messages: List<JsonObject>,
) {
    private val slotIndex: Map<String, Int> = HashMap<String, Int>(slots.size * 2).apply {
        slots.forEachIndexed { index, spec -> put(spec.id, index) }
    }

    internal val choiceSets: Array<Set<String>> = Array(slots.size) { slots[it].choices.toHashSet() }

    val templateId: TemplateId = TemplateId(id)

    val schema: TemplateSchema = TemplateSchema(templateId, title, keywords, slots.map(SlotSchema::toSpec))

    fun indexOf(slotId: String): Int = slotIndex[slotId] ?: -1

    fun slot(slotId: String): SlotSchema? = slotIndex[slotId]?.let(slots::get)

    override fun toString(): String = "Template($id, ${slots.size} slots, ${messages.size} messages)"
}
