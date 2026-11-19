package app.witness.core.domain.usecase

import app.witness.core.model.Slot
import app.witness.core.model.SlotSpec
import app.witness.core.model.SlotType
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import app.witness.core.model.TemplateSchema

object TemplateMatcher {
    fun pick(text: String, templates: List<TemplateSchema>): TemplateId {
        val words = Tokens.of(text).toHashSet()
        var best: TemplateSchema? = null
        var bestScore = 0
        for (t in templates) {
            val score = t.keywords.count { k -> Tokens.of(k).let { kw -> kw.isNotEmpty() && kw.all { it in words } } }
            if (score > bestScore) {
                best = t
                bestScore = score
            }
        }
        return best?.id ?: TemplateId.GENERAL
    }
}

object SuggestionValidator {
    fun validate(suggestion: Suggestion, schema: TemplateSchema, fallback: List<Slot> = emptyList()): Suggestion {
        val specs = schema.slots.associateBy { it.id }
        val accepted = LinkedHashMap<String, Slot>()
        for (slot in suggestion.slots) {
            if (slot.id in accepted) continue
            val spec = specs[slot.id] ?: continue
            clean(slot, spec)?.let { accepted[slot.id] = it }
        }
        for (slot in fallback) {
            if (slot.id in accepted) continue
            val spec = specs[slot.id] ?: continue
            clean(slot, spec)?.let { accepted[slot.id] = it }
        }
        val ordered = schema.slots.mapNotNull { accepted[it.id] }
        return Suggestion(
            templateId = schema.id,
            slots = ordered,
            caption = PostText.clean(suggestion.caption, PostText.MAX_CAPTION),
            altText = PostText.clean(suggestion.altText, PostText.MAX_ALT),
        )
    }

    fun missingRequired(suggestion: Suggestion, schema: TemplateSchema): List<String> {
        val filled = suggestion.slots.mapTo(HashSet()) { it.id }
        return schema.slots.filter { it.required && it.id !in filled }.map { it.id }
    }

    private fun clean(slot: Slot, spec: SlotSpec): Slot? = when (spec.type) {
        SlotType.TEXT -> PostText.clean(slot.text, spec.maxLength)?.takeIf { slot.choice == null && slot.flag == null }?.let { Slot(slot.id, text = it) }
        SlotType.CHOICE -> slot.choice?.takeIf { it in spec.choices && slot.text == null && slot.flag == null }?.let { Slot(slot.id, choice = it) }
        SlotType.FLAG -> slot.flag?.takeIf { slot.text == null && slot.choice == null }?.let { Slot(slot.id, flag = it) }
    }
}

object Tokens {
    fun of(text: String): List<String> {
        val out = ArrayList<String>()
        var start = -1
        for (i in 0..text.length) {
            val word = i < text.length && text[i].isLetterOrDigit()
            if (word && start < 0) start = i
            if (!word && start >= 0) {
                out += text.substring(start, i).lowercase()
                start = -1
            }
        }
        return out
    }
}
