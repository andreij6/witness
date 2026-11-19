package app.witness.feature.cases

import app.witness.core.model.TemplateId

data class CoachUi(val prompt: String, val step: Int, val total: Int) {
    val filled: Int get() = step
    val progressLabel: String get() = "$step of $total"
}

object EvidenceCoach {
    private val steps: Map<TemplateId, List<String>> = mapOf(
        TemplateId.CAR_ACCIDENT to listOf(
            "Get a close-up of the damage.",
            "Capture both license plates.",
            "Now get a wide shot showing both cars and the intersection.",
            "Photograph the other driver's insurance card.",
            "Add any road signs or signals nearby.",
        ),
        TemplateId.PROPERTY_DAMAGE to listOf(
            "Start with a wide shot of the affected area.",
            "Get a close-up of the damage.",
            "Show where the damage came from.",
            "Capture anything that was damaged.",
        ),
        TemplateId.RENTAL_CONDITION to listOf(
            "Start with a wide shot of each room.",
            "Get close-ups of any existing damage.",
            "Photograph appliances and fixtures.",
            "Capture meter readings and keys.",
        ),
        TemplateId.GENERAL to listOf(
            "Start with a wide shot that sets the scene.",
            "Get close-ups of the details that matter.",
        ),
    )

    fun steps(templateId: TemplateId): List<String> = steps[templateId] ?: steps.getValue(TemplateId.GENERAL)

    fun current(captured: Int, skipped: Int, total: Int): Int? {
        if (total <= 0) return null
        val done = captured.coerceAtLeast(0).toLong() + skipped.coerceAtLeast(0)
        return if (done >= total) null else done.toInt()
    }

    fun coach(templateId: TemplateId, captured: Int, skipped: Int): CoachUi? {
        val list = steps(templateId)
        val index = current(captured, skipped, list.size) ?: return null
        return CoachUi(list[index], index + 1, list.size)
    }
}
