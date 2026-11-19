package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.runtime.A2uiComponentProperties
import androidx.a2ui.compose.runtime.A2uiComponentScope
import androidx.a2ui.compose.runtime.A2uiProperty
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.witness.core.a2ui.WitnessCatalog
import app.witness.core.a2ui.ui.FieldDivider
import app.witness.core.a2ui.ui.GeneratedFrame
import app.witness.core.a2ui.ui.SectionHeading

internal object GeneratedSurfaceComponent : A2uiComponent {
    private val heading = A2uiProperty.string("heading", description = "Section heading above the surface.")
    private val tag = A2uiProperty.string("tag", description = "Generated-content tag, e.g. Generated form.")
    private val children = A2uiProperty.childList("children", required = true, description = "Fields inside the surface.")

    override val name = WitnessCatalog.GENERATED_SURFACE
    override val description = "Visibly marked surface that holds AI-generated fields, separated by dividers."
    override val properties = listOf(heading, tag, children)

    @Composable
    override fun A2uiComponentScope.Content(properties: A2uiComponentProperties, modifier: Modifier) {
        val references = properties.bindChildReferences(children).orEmpty()
        Column(modifier.fillMaxWidth()) {
            properties[heading]?.let { SectionHeading(it, properties[tag]) }
            GeneratedFrame {
                references.forEachIndexed { index, reference ->
                    if (index > 0) FieldDivider()
                    Child(reference)
                }
            }
        }
    }
}
