package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.runtime.A2uiComponentProperties
import androidx.a2ui.compose.runtime.A2uiComponentScope
import androidx.a2ui.compose.runtime.A2uiProperty
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.witness.core.a2ui.WitnessCatalog

internal object ColumnComponent : A2uiComponent {
    private val children = A2uiProperty.childList("children", required = true, description = "Child component ids, top to bottom.")
    private val spacing = A2uiProperty.number("spacing", defaultValue = 16, description = "Gap between children in dp.")

    override val name = WitnessCatalog.COLUMN
    override val description = "Vertical stack that lays out the report sections."
    override val properties = listOf(children, spacing)

    @Composable
    override fun A2uiComponentScope.Content(properties: A2uiComponentProperties, modifier: Modifier) {
        val gap = (properties[spacing]?.toInt() ?: 16).coerceIn(0, 48)
        Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(gap.dp)) {
            properties.bindChildReferences(children)?.forEach { Child(it) }
        }
    }
}
