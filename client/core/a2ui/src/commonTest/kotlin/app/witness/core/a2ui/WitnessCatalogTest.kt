package app.witness.core.a2ui

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class WitnessCatalogTest {
    @Test
    fun mapsOnlyTheFourSinkEvents() {
        listOf("capture_for", "edit", "remove", "flag").forEach { assertTrue(WitnessCatalog.isMappedEvent(it)) }
    }

    @Test
    fun dropsUnknownAndCaseVariants() {
        listOf("", "EDIT", "delete", "capture_for ", "navigate").forEach { assertFalse(WitnessCatalog.isMappedEvent(it)) }
    }

    @Test
    fun catalogIdIsVersioned() {
        assertEquals("https://witness.app/a2ui/catalog/v1.json", WitnessCatalog.ID)
    }

    @Test
    fun catalogListsTheV1Components() {
        assertEquals(
            setOf("Column", "GeneratedSurface", "EvidenceChecklist", "PlateField", "ImpactDiagram", "ChoiceRow", "SummaryDraft", "FlagButton"),
            WitnessCatalog.componentNames,
        )
    }
}
