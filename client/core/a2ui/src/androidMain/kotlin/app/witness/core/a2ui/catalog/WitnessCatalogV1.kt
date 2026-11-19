package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.ui.A2uiCatalog
import androidx.a2ui.compose.ui.A2uiComponent
import app.witness.core.a2ui.WitnessCatalog

object WitnessCatalogV1 {
    val components: List<A2uiComponent> = listOf(
        ColumnComponent,
        GeneratedSurfaceComponent,
        EvidenceChecklistComponent,
        PlateFieldComponent,
        ImpactDiagramComponent,
        ChoiceRowComponent,
        SummaryDraftComponent,
        FlagButtonComponent,
    )

    val catalog: A2uiCatalog by lazy { A2uiCatalog(catalogId = WitnessCatalog.ID, components = components) }
}
