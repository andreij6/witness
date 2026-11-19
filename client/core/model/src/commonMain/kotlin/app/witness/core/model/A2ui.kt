package app.witness.core.model

import kotlin.jvm.JvmInline

@JvmInline
value class TemplateId(val value: String) {
    override fun toString(): String = value

    companion object {
        val GENERAL: TemplateId = TemplateId("general")
        val CAR_ACCIDENT: TemplateId = TemplateId("car_accident")
        val PROPERTY_DAMAGE: TemplateId = TemplateId("property_damage")
        val RENTAL_CONDITION: TemplateId = TemplateId("rental_condition")
    }
}

enum class SlotType { TEXT, CHOICE, FLAG }

data class SlotSpec(
    val id: String,
    val type: SlotType,
    val choices: List<String> = emptyList(),
    val required: Boolean = false,
    val maxLength: Int = DEFAULT_MAX_LENGTH,
) {
    companion object {
        const val DEFAULT_MAX_LENGTH: Int = 280
    }
}

data class TemplateSchema(val id: TemplateId, val title: String, val keywords: List<String>, val slots: List<SlotSpec>)

data class Slot(val id: String, val text: String? = null, val choice: String? = null, val flag: Boolean? = null)

data class Suggestion(
    val templateId: TemplateId,
    val slots: List<Slot>,
    val caption: String? = null,
    val altText: String? = null,
)
