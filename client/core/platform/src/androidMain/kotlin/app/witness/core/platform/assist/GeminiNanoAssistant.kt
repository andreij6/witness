package app.witness.core.platform.assist

import app.witness.core.domain.port.Assistant
import app.witness.core.domain.port.AssistantContext
import app.witness.core.model.Slot
import app.witness.core.model.Suggestion
import app.witness.core.model.TemplateId
import app.witness.core.model.TemplateSchema
import com.google.mlkit.genai.common.FeatureStatus
import com.google.mlkit.genai.prompt.GenerateContentRequest
import com.google.mlkit.genai.prompt.GenerativeModel
import com.google.mlkit.genai.prompt.Generation
import com.google.mlkit.genai.prompt.SystemInstruction
import com.google.mlkit.genai.prompt.TextPart
import com.google.mlkit.genai.prompt.generateTypedContentRequest
import com.google.mlkit.genai.schema.annotations.Generable
import com.google.mlkit.genai.schema.annotations.Guide

@Generable("A filled report template")
data class NanoSuggestion(
    @Guide(description = "The id of the chosen template")
    val templateId: String,
    @Guide(description = "Values for the chosen template's slots", maxItems = 16)
    val slots: List<NanoSlot>,
    @Guide(description = "A short neutral caption")
    val caption: String?,
    @Guide(description = "Alt text describing the photo for blind users")
    val altText: String?,
)

@Generable("One slot value")
data class NanoSlot(
    @Guide(description = "Slot id")
    val id: String,
    @Guide(description = "Text value for text slots")
    val text: String?,
    @Guide(description = "Chosen option for choice slots")
    val choice: String?,
    @Guide(description = "Value for flag slots")
    val flag: Boolean?,
)

class GeminiNanoAssistant(private val model: () -> GenerativeModel = { Generation.getClient() }) : Assistant {
    private val client by lazy(model)

    override suspend fun isAvailable(): Boolean = runCatching {
        client.checkStatus() == FeatureStatus.AVAILABLE && client.isStructuredOutputFeatureAvailable()
    }.getOrDefault(false)

    override suspend fun suggest(context: AssistantContext, templates: List<TemplateSchema>): Suggestion? {
        if (templates.isEmpty()) return null
        val base = GenerateContentRequest.Builder(TextPart(PromptBuilder.build(context, templates)))
            .apply { systemInstruction = SystemInstruction(PromptBuilder.SYSTEM) }
            .build()
        val response = client.generateContent(generateTypedContentRequest(base, NanoSuggestion::class))
        return response.candidates.firstOrNull()?.response?.toSuggestion()
    }
}

internal fun NanoSuggestion.toSuggestion() = Suggestion(
    TemplateId(templateId.trim()),
    slots.map { Slot(it.id.trim(), it.text, it.choice, it.flag) },
    caption,
    altText,
)
