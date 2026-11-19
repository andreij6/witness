package app.witness.core.a2ui

import app.witness.core.model.SlotSpec
import app.witness.core.model.SlotType
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
enum class SlotKind {
    @SerialName("text") TEXT,
    @SerialName("plate") PLATE,
    @SerialName("choice") CHOICE,
    @SerialName("flag") FLAG,
}

@Serializable
data class SlotSchema(
    val id: String,
    val type: SlotKind,
    val hint: String = "",
    val choices: List<String> = emptyList(),
    val maxLength: Int = DEFAULT_MAX_LENGTH,
    val required: Boolean = false,
) {
    fun toSpec(): SlotSpec = SlotSpec(
        id = id,
        type = when (type) {
            SlotKind.TEXT, SlotKind.PLATE -> SlotType.TEXT
            SlotKind.CHOICE -> SlotType.CHOICE
            SlotKind.FLAG -> SlotType.FLAG
        },
        choices = choices,
        required = required,
        maxLength = if (type == SlotKind.PLATE) MAX_PLATE_LENGTH else maxLength,
    )

    companion object {
        const val DEFAULT_MAX_LENGTH = SlotSpec.DEFAULT_MAX_LENGTH
        const val MAX_TEXT_LENGTH = 4000
        const val MAX_PLATE_LENGTH = 10
    }
}
