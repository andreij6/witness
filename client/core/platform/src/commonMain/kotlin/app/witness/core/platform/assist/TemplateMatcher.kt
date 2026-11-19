package app.witness.core.platform.assist

import app.witness.core.model.TemplateId
import app.witness.core.model.TemplateSchema

object TemplateMatcher {
    fun tokens(vararg texts: String?): Set<String> {
        val out = HashSet<String>()
        val sb = StringBuilder()
        for (text in texts) {
            if (text == null) continue
            for (c in text) {
                if (c.isLetterOrDigit()) sb.append(c.lowercaseChar())
                else if (sb.isNotEmpty()) { out += sb.toString(); sb.clear() }
            }
            if (sb.isNotEmpty()) { out += sb.toString(); sb.clear() }
        }
        return out
    }

    fun best(templates: List<TemplateSchema>, tokens: Set<String>): TemplateSchema? {
        if (templates.isEmpty()) return null
        var best: TemplateSchema? = null
        var bestScore = 0
        for (t in templates) {
            var score = 0
            for (k in t.keywords) if (k.lowercase() in tokens) score++
            if (score > bestScore) {
                best = t
                bestScore = score
            }
        }
        return best ?: templates.firstOrNull { it.id == TemplateId.GENERAL } ?: templates.first()
    }
}
