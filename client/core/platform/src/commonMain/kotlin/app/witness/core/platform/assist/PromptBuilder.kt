package app.witness.core.platform.assist

import app.witness.core.domain.port.AssistantContext
import app.witness.core.model.SlotType
import app.witness.core.model.TemplateSchema

object PromptBuilder {
    const val MAX_FIELD = 200

    val SYSTEM = "You help people document photos on their phone. Answer only with the requested fields. " +
        "Never invent people's names, plate numbers or addresses. Keep captions neutral and factual."

    fun build(context: AssistantContext, templates: List<TemplateSchema>): String {
        val sb = StringBuilder(256 + templates.size * 128)
        sb.append("Mode: ").append(context.mode.name.lowercase()).append('\n')
        context.title?.let { sb.append("Title: ").append(sanitize(it)).append('\n') }
        context.place?.let { sb.append("Place: ").append(sanitize(it)).append('\n') }
        if (context.detected.isNotEmpty()) {
            sb.append("Detected: ").append(context.detected.joinToString(", ") { it.name.lowercase() }).append('\n')
        }
        sb.append("Pick one templateId and fill its slots.\n")
        for (t in templates) {
            sb.append("- ").append(t.id.value).append(" (").append(sanitize(t.title)).append("): ")
            t.slots.joinTo(sb, "; ") { s ->
                when (s.type) {
                    SlotType.TEXT -> "${s.id} text"
                    SlotType.FLAG -> "${s.id} flag"
                    SlotType.CHOICE -> "${s.id} choice of ${s.choices.joinToString("|")}"
                }
            }
            sb.append('\n')
        }
        return sb.toString()
    }

    fun sanitize(text: String): String {
        val sb = StringBuilder(minOf(text.length, MAX_FIELD))
        for (c in text) {
            if (sb.length == MAX_FIELD) break
            sb.append(if (c.isISOControl()) ' ' else c)
        }
        return sb.toString().trim()
    }
}
