package app.witness.core.platform.assist

import app.witness.core.domain.usecase.PostText
import app.witness.core.model.Slot
import app.witness.core.model.SlotSpec
import app.witness.core.model.SlotType
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateSchema

object SuggestionValidator {
    fun validSlot(slot: Slot, spec: SlotSpec): Slot? = when (spec.type) {
        SlotType.TEXT -> PostText.clean(slot.text, spec.maxLength)?.let { Slot(spec.id, text = it) }
        SlotType.CHOICE -> slot.choice?.trim()?.let { c -> spec.choices.firstOrNull { it.equals(c, ignoreCase = true) } }?.let { Slot(spec.id, choice = it) }
        SlotType.FLAG -> slot.flag?.let { Slot(spec.id, flag = it) }
    }

    fun validate(candidate: Suggestion, templates: List<TemplateSchema>): Suggestion? {
        val schema = templates.firstOrNull { it.id == candidate.templateId } ?: return null
        val byId = HashMap<String, Slot>(candidate.slots.size * 2)
        for (s in candidate.slots) byId.putIfAbsent(s.id, s)
        val slots = ArrayList<Slot>(schema.slots.size)
        for (spec in schema.slots) byId[spec.id]?.let { validSlot(it, spec) }?.let(slots::add)
        return Suggestion(
            schema.id,
            slots,
            PostText.clean(candidate.caption, PostText.MAX_CAPTION),
            PostText.clean(candidate.altText, PostText.MAX_ALT),
        )
    }

    fun merge(primary: Suggestion?, fallback: Suggestion, templates: List<TemplateSchema>): Suggestion {
        val valid = primary?.let { validate(it, templates) } ?: return fallback
        val schema = templates.first { it.id == valid.templateId }
        val fallbackSlots = if (fallback.templateId == valid.templateId) fallback.slots.associateBy { it.id } else emptyMap()
        val have = valid.slots.associateBy { it.id }
        val slots = schema.slots.mapNotNull { have[it.id] ?: fallbackSlots[it.id] }
        return Suggestion(valid.templateId, slots, valid.caption ?: fallback.caption, valid.altText ?: fallback.altText)
    }
}
