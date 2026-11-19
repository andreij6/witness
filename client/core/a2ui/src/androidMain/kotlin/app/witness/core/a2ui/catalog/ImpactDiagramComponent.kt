package app.witness.core.a2ui.catalog

import androidx.a2ui.compose.runtime.A2uiComponentProperties
import androidx.a2ui.compose.runtime.A2uiComponentScope
import androidx.a2ui.compose.runtime.A2uiProperty
import androidx.a2ui.compose.ui.A2uiComponent
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import app.witness.core.a2ui.WitnessCatalog
import app.witness.core.a2ui.ui.ImpactDiagramBlock

internal object ImpactDiagramComponent : A2uiComponent {
    private val label = A2uiProperty.string("label", required = true)
    private val value = A2uiProperty.dynamicString("value", description = "Impact zone, e.g. rear_driver.")
    private val caption = A2uiProperty.dynamicString("caption", description = "Provenance, e.g. Detected from photo 2.")
    private val options = OptionProperties.list("Zone values with display labels.")
    private val flagAction = A2uiProperty.action("flagAction")

    override val name = WitnessCatalog.IMPACT_DIAGRAM
    override val description = "Top-down car diagram highlighting the impact zone."
    override val properties = listOf(label, value, caption, options, flagAction)

    @Composable
    override fun A2uiComponentScope.Content(properties: A2uiComponentProperties, modifier: Modifier) {
        val zones = remember(properties) { OptionProperties.read(properties[options]) }
        val zone = properties.bind(value)
        ImpactDiagramBlock(
            label = properties[label].orEmpty(),
            valueLabel = zone?.let { selected -> zones.firstOrNull { it.value == selected }?.label },
            caption = properties.bind(caption),
            zone = zone,
            onFlag = dispatcher(properties[flagAction]),
            modifier = modifier,
        )
    }
}
