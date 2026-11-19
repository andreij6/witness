package app.witness.core.a2ui

object WitnessCatalog {
    const val ID = "https://witness.app/a2ui/catalog/v1.json"

    const val COLUMN = "Column"
    const val GENERATED_SURFACE = "GeneratedSurface"
    const val EVIDENCE_CHECKLIST = "EvidenceChecklist"
    const val PLATE_FIELD = "PlateField"
    const val IMPACT_DIAGRAM = "ImpactDiagram"
    const val CHOICE_ROW = "ChoiceRow"
    const val SUMMARY_DRAFT = "SummaryDraft"
    const val FLAG_BUTTON = "FlagButton"

    val componentNames: Set<String> = hashSetOf(
        COLUMN, GENERATED_SURFACE, EVIDENCE_CHECKLIST, PLATE_FIELD, IMPACT_DIAGRAM, CHOICE_ROW, SUMMARY_DRAFT, FLAG_BUTTON,
    )

    const val CAPTURE_FOR = "capture_for"
    const val EDIT = "edit"
    const val REMOVE = "remove"
    const val FLAG = "flag"

    private val mappedEvents = hashSetOf(CAPTURE_FOR, EDIT, REMOVE, FLAG)

    fun isMappedEvent(name: String): Boolean = name in mappedEvents
}
