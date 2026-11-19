package app.witness.core.platform.assist

import app.witness.core.domain.port.Assistant
import app.witness.core.domain.port.AssistantContext
import app.witness.core.domain.usecase.PostText
import app.witness.core.model.BlurProposal
import app.witness.core.model.Rect
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateSchema

class RulesAssistant : Assistant {
    override suspend fun isAvailable(): Boolean = true

    override suspend fun suggest(context: AssistantContext, templates: List<TemplateSchema>): Suggestion? = rules(context, templates)

    companion object {
        private val NO_RECT = Rect(0, 0, 1, 1)

        fun rules(context: AssistantContext, templates: List<TemplateSchema>): Suggestion? {
            val template = TemplateMatcher.best(templates, TemplateMatcher.tokens(context.title, context.place)) ?: return null
            val blurs = context.detected.map { BlurProposal(NO_RECT, it) }
            return Suggestion(
                templateId = template.id,
                slots = emptyList(),
                caption = PostText.clean(context.title, PostText.MAX_CAPTION),
                altText = PostText.rulesAltText(blurs, context.place),
            )
        }
    }
}

class FallbackAssistant(private val primary: Assistant?, private val foreground: () -> Boolean = { true }) : Assistant {
    override suspend fun isAvailable(): Boolean = true

    override suspend fun suggest(context: AssistantContext, templates: List<TemplateSchema>): Suggestion? {
        val rules = RulesAssistant.rules(context, templates) ?: return null
        val model = primary?.takeIf { foreground() && it.isAvailable() } ?: return rules
        val candidate = runCatching { model.suggest(context, templates) }.getOrNull()
        return SuggestionValidator.merge(candidate, rules, templates)
    }
}
