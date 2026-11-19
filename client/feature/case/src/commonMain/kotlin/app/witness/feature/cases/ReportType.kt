package app.witness.feature.cases

import app.witness.core.designsystem.WitnessIcon
import app.witness.core.model.TemplateId
import app.witness.core.model.TemplateSchema

enum class ReportType(
    val templateId: TemplateId,
    val title: String,
    val subtitle: String,
    val rowIcon: WitnessIcon,
    val chipLabel: String,
    val chipIcon: WitnessIcon,
    val available: Boolean = true,
) {
    CAR_ACCIDENT(TemplateId.CAR_ACCIDENT, "Car accident", "Plates, damage, other driver, injuries", WitnessIcon.DirectionsCar, "Car accident", WitnessIcon.DirectionsCar),
    PROPERTY_DAMAGE(TemplateId.PROPERTY_DAMAGE, "Property damage", "Source, affected areas, repair notes", WitnessIcon.WaterDrop, "Property damage", WitnessIcon.WaterDrop),
    RENTAL_CONDITION(TemplateId.RENTAL_CONDITION, "Rental condition", "Room by room, move-in vs move-out", WitnessIcon.Home, "Rental home", WitnessIcon.Home),
    WORKPLACE_INCIDENT(TemplateId("workplace_incident"), "Workplace incident", "Coming in a later update", WitnessIcon.Badge, "Workplace", WitnessIcon.Badge, available = false),
    GENERAL(TemplateId.GENERAL, "Other", "Plain checklist, no form", WitnessIcon.CheckBox, "Other", WitnessIcon.EditNote);

    companion object {
        val situations: List<ReportType> = listOf(CAR_ACCIDENT, PROPERTY_DAMAGE, RENTAL_CONDITION, GENERAL)

        private val byId: Map<TemplateId, ReportType> = entries.associateBy { it.templateId }

        fun of(id: TemplateId): ReportType? = byId[id]

        fun label(id: TemplateId, schemas: List<TemplateSchema> = emptyList()): String =
            of(id)?.title ?: schemas.firstOrNull { it.id == id }?.title ?: humanize(id.value)

        fun options(schemas: List<TemplateSchema>): List<ReportType> {
            val known = schemas.mapTo(HashSet(schemas.size * 2)) { it.id }
            return entries.filter { !it.available || it.templateId in known }
        }

        fun humanize(id: String): String {
            val words = id.split('_', '-', ' ').filter { it.isNotEmpty() }
            if (words.isEmpty()) return id
            return words.joinToString(" ").replaceFirstChar { it.uppercaseChar() }
        }
    }
}
